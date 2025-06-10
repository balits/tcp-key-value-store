use core::panic;
use std::{collections::HashMap, net::SocketAddr};

use app::conn::ConnState;
use log::trace;
use mio::{Events, Interest, Poll, Token};

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

                    if event.is_readable() && conn.want_read() {
                        let _ = app::handle_read(conn).inspect_err(|e| {
                            trace!(target:"handle_read", "error during read: {}", e);
                        });
                    }

                    if event.is_writable() && conn.want_write() {
                        let _ = app::handle_write(conn).inspect_err(
                            |e| trace!(target:"handle_write", "error during write: {}", e),
                        );
                    }

                    if conn.want_close() {
                       if let ConnState::WantClose(err_msg) = conn.state() {
                            trace!(target:"handle_close", "closing connection due to: {}", err_msg);
                            app::handle_close(token, &mut tokgen, &poll, &mut map)?;
                        } else {
                            panic!("Handling close: ConnState was not set properly, got: {:?}", conn.state());
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
        io::{self, Read, Write},
    };

    pub use conn::Conn;
    use conn::ConnState;
    use log::trace;
    use mio::{Poll, Token};

    use crate::{eventloop::token::TokenGen, util};

    pub fn handle_read(conn: &mut Conn) -> std::io::Result<()> {
        trace!(target:"handle_read", "start");
        let mut buf = vec![0; 1024];
        match conn.stream.read(&mut buf) {
            Ok(0) => {
                trace!(target: "handle_error", "read 0 bytes: EOF");
                conn.state = ConnState::WantClose("Connection closed by EOF".into());
                Ok(())
            }
            Ok(n) => {
                trace!(target: "handle_error", "read {} bytes", n);
                conn.incoming.extend(&buf[..n]);

                let _request = parse_request(&conn.incoming)?;
                conn.state = ConnState::WantWrite;
                // handle request parsing
                conn.outgoing.extend(&buf[..n]);
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
                conn.state = ConnState::WantClose(e.to_string());
                Err(e)
            }
        }
    }

    fn parse_request(buf: &[u8]) -> io::Result<()> {
        trace!(target:"parse_request", "got {:X?}", buf);
        Ok(())
    }

    pub fn handle_write(conn: &mut Conn) -> io::Result<()> {
        trace!(target:"handle_write", "start");
        let mut buf = conn.outgoing.as_slice();
        while !buf.is_empty() {
            match conn.stream.write(&buf) {
                Ok(0) => {
                    trace!(target:"handle_write", "could not write eventhough buf is not empty");
                    conn.state = ConnState::WantClose("failed to write whole buffer".into());
                    return Err(io::ErrorKind::WriteZero.into());
                }
                Ok(n) => {
                    trace!(target:"handle_write", "sent {n} bytes, next buffer {:?}", &buf[n..]);
                    buf = &buf[n..];
                }
                Err(ref e) if util::would_block(e) => {
                    // break event loop, try again next iteration
                    return Ok(());
                }
                Err(ref e) if util::interrupted(e) => {
                    // break event loop, try again next iteration
                    return Ok(());
                }
                Err(e) => {
                    conn.state = ConnState::WantClose(e.to_string());
                    return Err(e);
                }
            }
        }

        conn.state = ConnState::WantRead;
        Ok(())
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
            WantClose(String),
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
                matches!(self.state, ConnState::WantClose(_))
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
