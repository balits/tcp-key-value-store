use std::{collections::HashMap, net::SocketAddr};

use log::trace;
use mio::{Events, Interest, Poll, Token};
use socket::LoopError;

const SERVER: Token = Token(0);

fn main() -> std::io::Result<()> {
    env_logger::builder().init();

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut socket = mio::net::TcpListener::bind(addr)?;
    trace!("Listener: {:#?}", socket);

    poll.registry()
        .register(&mut socket, SERVER, Interest::READABLE | Interest::WRITABLE)?;

    let mut map = HashMap::new();
    let mut tokgen = eventloop::token::TokenGen::new();

    loop {
        poll.poll(&mut events, None)?;

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    let stream = match socket.accept() {
                        Ok((s, _)) => s,
                        Err(ref e) if util::would_block(e) => continue,
                        Err(ref e) if util::interrupted(e) => continue,
                        Err(e) => return Err(e),
                    };
                    trace!("new connection from {}", stream.peer_addr()?);
                    let tok = tokgen.next();
                    let mut conn = app::Conn::new(stream);
                    poll.registry().register(
                        &mut conn.stream,
                        tok,
                        Interest::READABLE | Interest::WRITABLE,
                    )?;
                    map.insert(tok, conn);
                }
                token => {
                    trace!(target:"new_token", "new event with token {}", &token.0);
                    let Some(conn) = map.get_mut(&token) else {
                        trace!(target:"new_token", "spourios wake");
                        continue;
                    };
                    trace!(target: "new_token", "conn state {:?}", conn.state());

                    let last_error = match app::handle_connection(conn, event) {
                        Ok(_) => None,
                        Err(LoopError::IncompleteRequest { .. }) => continue,
                        Err(e) => Some(e),
                    };

                    if conn.want_close() {
                        if let Some(err) = last_error {
                            trace!(target:"handle_close", "closing connection due to: {}", err);
                            app::handle_close(token, &mut tokgen, &poll, &mut map)?;
                        } else {
                            unreachable!(
                                "handle_connection didnt return an error, but connection was still set to WantClose"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Application-layer logic
mod app {
    use std::{
        collections::HashMap,
        io::{self, Read, Write}, usize,
    };

    pub use conn::Conn;
    use conn::ConnState;
    use log::{error, trace};
    use mio::{Poll, Token, event::Event};
    use socket::LoopError;

    use crate::{eventloop::token::TokenGen, util};

    pub fn handle_connection(conn: &mut Conn, event: &Event) -> Result<(), LoopError> {
        if event.is_readable() && conn.want_read() {
            match handle_read(conn) {
                Err(le) => match le {
                    ioerr @ LoopError::IoError(_) => {
                        error!(target:"handle_read", "{ioerr}");
                        return Err(ioerr);
                    }
                    close @ LoopError::CloseConnection(_) => {
                        error!(target:"handle_read", "{close}");
                        return Err(close);
                    }
                    other => {
                        error!(target:"handle_read", "{other}");
                    }
                },
                Ok(_) => {
                    trace!(target:"handle_read", "OK")
                }
            }
        }

        if event.is_writable() && conn.want_write() {
            match handle_write(conn) {
                Err(e) => trace!(target:"handle_write", "error during write: {}", e),
                Ok(_) => trace!(target:"handle_write", "OK"),
            }
        }

        Ok(())
    }

    pub fn handle_read(conn: &mut Conn) -> Result<(), LoopError> {
        trace!(target:"handle_read", "start");
        let mut buf = [0; 1024];
        match conn.stream.read(&mut buf) {
            Ok(0) => {
                let msg = "read 0 bytes: EOF".to_string();
                trace!(target: "handle_read", "{}", &msg);
                conn.state = ConnState::WantClose;
                Err(LoopError::CloseConnection(msg))
            }
            Ok(n) => {
                trace!(target: "handle_read", "read {} bytes, got: {:X?}", n, &buf[..n]);
                conn.incoming.extend_from_slice(&buf[..n]);
                let _request = try_one_request(conn)?;
                Ok(())
            }
            Err(ref e) if util::would_block(e) => {
                //continue
                Ok(())
            }
            Err(ref e) if util::interrupted(e) => {
                //break
                Ok(())
            }
            Err(e) => {
                trace!(target: "handle_read", "error: {e}" );
                conn.state = ConnState::WantClose;
                Err(LoopError::IoError(e))
            }
        }
    }

    fn try_one_request(conn: &mut Conn) -> Result<(), LoopError> {
        fn get_u32(buf: &[u8]) -> u32 {
            u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]])
        }

        if conn.incoming.len() < 4 {
            // still wants read
            conn.state = ConnState::WantRead;
            return Err(LoopError::IncompleteRequest {
                expected: 4,
                got: conn.incoming.len() as u32,
            });
        }

        let len = get_u32(&conn.incoming);

        if  conn.incoming.len() < len as usize + 4 {
            conn.state = ConnState::WantRead;
            return Err(LoopError::IncompleteRequest {
                expected: len + 4,
                got: conn.incoming.len() as u32,
            });
        }

        let str = match std::str::from_utf8(&conn.incoming[4..(4 + len as usize)]) {
            Ok(s) => s,
            Err(e) => {
                conn.state = ConnState::WantClose;
                return Err(LoopError::InvalidRequest(e.to_string()));
            }
        };
        // only valid utf8 after this
        log::debug!(target:"try_one_request", "got request: len {}, str {}", len, str);

        conn.outgoing.extend_from_slice(&(len as u32).to_be_bytes());
        conn.outgoing.extend_from_slice(str.as_bytes());
        conn.incoming.drain(..(4 + len as  usize));

        conn.state = ConnState::WantWrite;

        Ok(())
    }

    pub fn handle_write(conn: &mut Conn) -> Result<(), LoopError> {
        trace!(target:"handle_write", "start");
        dbg!(&conn.incoming, &conn.outgoing);
        assert!(
            conn.want_write(),
            "calling write even if conn doesnt want to write"
        );
        assert_ne!(0, conn.outgoing.len(), "calling write on empty buffer");

        match conn.stream.write(&mut conn.outgoing) {
            Ok(0) => {
                let msg = "wrote 0 bytes: EOF".to_string();
                trace!(target: "handle_write", "{}", &msg);
                conn.state = ConnState::WantClose;
                Err(LoopError::CloseConnection(msg))
            }
            Ok(n) => {
                trace!(target: "handle_write", "wrote {} bytes out of {} ", n, conn.outgoing.len());
                assert_eq!(n, conn.outgoing.len(), "could not write full buffer");
                conn.outgoing.drain(..);
                conn.state = ConnState::WantRead;
                Ok(())
            }
            Err(ref e) if util::would_block(e) => {
                //continue
                Ok(())
            }
            Err(ref e) if util::interrupted(e) => {
                //break
                Ok(())
            }
            Err(e) => {
                trace!(target: "handle_write", "error: {e}" );
                conn.state = ConnState::WantClose;
                Err(LoopError::IoError(e))
            }
        }
    }

    pub fn handle_close(
        token: Token,
        tokgen: &mut TokenGen,
        poll: &Poll,
        map: &mut HashMap<Token, Conn>,
    ) -> io::Result<()> {
        let mut conn = map.remove(&token).unwrap();
        tokgen.free(token);
        poll.registry().deregister(&mut conn.stream)?;
        Ok(())
    }

    pub mod conn {
        use mio::net::TcpStream as MioStream;

        #[derive(Debug, PartialEq, Eq)]
        pub enum ConnState {
            WantRead,
            WantWrite,
            WantClose,
        }

        #[derive(Debug)]
        pub struct Conn {
            pub stream: MioStream,
            pub(super) state: ConnState,
            pub incoming: Vec<u8>,
            pub outgoing: Vec<u8>,
        }

        impl Conn {
            pub fn new(stream: MioStream) -> Self {
                Self {
                    stream,
                    state: ConnState::WantRead,
                    incoming: Vec::new(),
                    outgoing: Vec::new(),
                }
            }
            pub fn state(&self) -> &ConnState {
                &self.state
            }

            pub fn want_read(&self) -> bool {
                matches!(self.state, ConnState::WantRead)
            }

            pub fn want_write(&self) -> bool {
                matches!(self.state, ConnState::WantWrite)
            }
            pub fn want_close(&self) -> bool {
                matches!(self.state, ConnState::WantClose)
            }
        }
    }
}

/// Event loop / base-layer logic
mod eventloop {
    pub mod token {
        use std::collections::VecDeque;

        use mio::Token;

        pub struct TokenGen {
            next: usize,
            freed: VecDeque<usize>,
        }

        impl TokenGen {
            pub const fn new() -> Self {
                Self {
                    next: 0,
                    freed: VecDeque::new(),
                }
            }
            pub fn next(&mut self) -> Token {
                if let Some(free) = self.freed.pop_front() {
                    Token(free)
                } else {
                    self.next += 1;
                    Token(self.next)
                }
            }
            /// # Notes
            /// Its a mistake to free a token more than once
            /// altough the code will check this (yet)
            pub fn free(&mut self, token: Token) {
                self.freed.push_front(token.0);
            }
        }
    }
}

mod util {
    use std::io;
    #[inline]
    pub fn would_block(e: &io::Error) -> bool {
        e.kind() == io::ErrorKind::WouldBlock
    }
    #[inline]
    pub fn interrupted(e: &io::Error) -> bool {
        e.kind() == io::ErrorKind::Interrupted
    }
}
