use crate::{protocol, storage::MAP2, util::would_block, SERVER};
use log::{error, info, trace};
use mio::{
    Interest, Token,
    net::{TcpListener, TcpStream},
};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    usize,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    WantRead,
    WantWrite,
    WantClose,
}

#[derive(Debug)]
pub struct Connection {
    pub stream: TcpStream,
    pub token: mio::Token,
    state: ConnectionState,
    pub incoming: Vec<u8>,
    pub outgoing: Vec<u8>,
}

impl Connection {
    pub fn new(stream: TcpStream, token: mio::Token) -> Self {
        Self {
            stream,
            token,
            state: ConnectionState::WantRead,
            incoming: Vec::new(),
            outgoing: Vec::new(),
        }
    }

    pub fn close(&mut self) {
        self.state = ConnectionState::WantClose;
    }

    pub fn want_read(&self) -> bool {
        matches!(self.state, ConnectionState::WantRead)
    }
    pub fn want_write(&self) -> bool {
        matches!(self.state, ConnectionState::WantWrite)
    }
    pub fn want_close(&self) -> bool {
        matches!(self.state, ConnectionState::WantClose)
    }

    pub fn on_read(&mut self) -> io::Result<()> {
        assert_eq!(
            ConnectionState::WantRead,
            self.state,
            "calling read on non WantRead state"
        );
        let mut buf = [0; 1024 * 64];
        loop {
            let n = match self.stream.read(&mut buf) {
                Ok(0) => {
                    trace!(target:"on_read", "{}", if self.incoming.is_empty() { "client dropped connection" } else { "unexpected eof" } );
                    // set state to WantClose, and let the main loop
                    // handle closing the connection
                    // instead of propagating io::Error
                    self.close();
                    return Ok(());
                }
                Ok(n) => n,
                Err(ref e) if would_block(e) => {
                    break;
                }
                Err(e) => {
                    self.close();
                    return Err(e);
                }
            };
            self.incoming.extend_from_slice(&buf[..n]);
        }

        info!("read {} bytes", self.incoming.len());

        let mut last_state;
        loop {
            // while we successfuly parse requests
            // where last_state = WantWrite == success
            last_state = self.try_one_request();
            if last_state != ConnectionState::WantWrite {
                break;
            }
        }

        if !self.outgoing.is_empty() {
            // we have at least one request ready to send
            // this way we skip one syscall to poll in the main loop
            self.state = ConnectionState::WantWrite;
            return self.on_write();
        } else {
            self.state = last_state;
        }
        Ok(())
    }

    pub fn on_write(&mut self) -> io::Result<()> {
        assert_eq!(
            ConnectionState::WantWrite,
            self.state,
            "calling write on non WantWrite state"
        );
        assert!(!self.outgoing.is_empty(), "calling write on empty buffer");

        let n = match self.stream.write(&self.outgoing) {
            Ok(0) => {
                error!("wrote 0 bytes to buffer");
                // set state to WantClose, and let the main loop
                // handle closing the connection
                // instead of propagating io::Error
                self.close();
                return Ok(());
            }
            Ok(n) => n,
            Err(ref e) if would_block(e) => return Ok(()),
            Err(e) => {
                self.close();
                return Err(e);
            }
        };

        info!("wrote {} bytes, out of {}", n, self.outgoing.len());
        self.outgoing.drain(..n);

        if self.outgoing.is_empty() {
            self.state = ConnectionState::WantRead;
        } else {
            self.state = ConnectionState::WantWrite;
        }

        Ok(())
    }

    /// Tries to parse one request, returning the new state
    /// for the connection:
    /// -   **WantWrite:** This is the "success" path, indicating that we
    ///     parsed one request, and its put onto the outgoing buffer to read
    ///
    /// -   **WantRead:** If there wasnt enough bytes to parse, we need to read more
    ///
    /// -   **WantClose:** Someting seriously went wrong - likely some protocol error - and the main loop should
    ///     close down the connection
    fn try_one_request(&mut self) -> ConnectionState {
        use protocol::ParseError::*;
        
        // dip early
        if self.incoming.is_empty() {
            return ConnectionState::WantRead
        }

        let result = protocol::parse_request(&self.incoming)
            .inspect_err(|e| info!(target:"parse_request", "{e}"));

        let (cmds, offset) = match result {
            Ok(v) => v,
            Err(ProtocolError) => return ConnectionState::WantClose,
            Err(NotEnoughBytes { .. }) => return ConnectionState::WantRead,
        };

        // consume requests
        self.incoming.drain(..offset);
        protocol::request::handle_and_encode_request(cmds, &mut self.outgoing);

        #[allow(static_mut_refs)]
        unsafe {
            dbg!(&MAP2.get());
        }
        
        ConnectionState::WantWrite
    }
}

pub struct ConnectionManager {
    pub map: HashMap<Token, Connection>,
    token_gen: TokenGen,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            token_gen: TokenGen::new(),
        }
    }

    pub fn handle_accept(&mut self, server: &TcpListener, poll: &mut mio::Poll) -> io::Result<()> {
        let stream = match server.accept() {
            Ok((s, _)) => s,
            Err(ref e) if would_block(e) => return Ok(()),
            Err(e) => return Err(e),
        };
        trace!("new connection from {}", stream.peer_addr()?);

        let token = self.token_gen.next();
        let mut conn = Connection::new(stream, token);

        poll.registry().register(
            &mut conn.stream,
            token,
            Interest::READABLE | Interest::WRITABLE,
        )?;

        self.map.insert(token, conn);
        Ok(())
    }

    pub fn handle_close(&mut self, poll: &mio::Poll, token: mio::Token) -> io::Result<()> {
        let mut conn = self.map.remove(&token).unwrap();
        poll.registry().deregister(&mut conn.stream)
    }

    pub fn get_connection_mut(&mut self, t: &Token) -> Option<&mut Connection> {
        self.map.get_mut(t)
    }
}

struct TokenGen {
    next: usize,
}

impl TokenGen {
    pub const fn new() -> Self {
        Self { next: SERVER.0 + 1 }
    }
    pub fn next(&mut self) -> mio::Token {
        let t = mio::Token(self.next);
        self.next += 1;
        t
    }
}
