use std::{io, net::SocketAddr};

use log::{error, trace};
use mio::{Events, Interest, Poll};
use tcpserver::{SERVER, connection::ConnectionManager, util::interrupted};

fn main() {
    if let Err(e) = try_main() {
        error!("{e}")
    }
}

fn try_main() -> io::Result<()> {
    env_logger::builder().init();

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut socket = mio::net::TcpListener::bind(addr)?;
    trace!("Listener: {:#?}", socket);

    poll.registry()
        .register(&mut socket, SERVER, Interest::READABLE | Interest::WRITABLE)?;

    let mut connection_manager = ConnectionManager::new();

    loop {
        if let Err(e) = poll.poll(&mut events, None) {
            if interrupted(&e) {
                continue;
            } else {
                return Err(e);
            }
        }

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    connection_manager.handle_accept(&socket, &mut poll)?;
                }
                token => {
                    let Some(conn) = connection_manager.get_connection_mut(&token) else {
                        continue;
                    };
                    if event.is_readable() && conn.want_read() {
                        conn.on_read()?;
                    }

                    if event.is_writable() && conn.want_write() {
                        conn.on_write()?;
                    }

                    if conn.want_close() {
                        connection_manager.handle_close(&poll, token)?;
                        trace!(target:"handle_close", "did close connection");
                    }
                }
            }
        }
    }
}
