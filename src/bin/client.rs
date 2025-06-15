use std::env::args;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use tcpserver::protocol;

fn write_message(sock: &mut TcpStream, cmds: Vec<String>) -> std::io::Result<()> {
    let cap = cmds.len() as u32;
    if cap as usize > protocol::MAX_ARGS {
        panic!("to many args")
    }

    let mut buf = Vec::with_capacity(cap as usize);

    buf.extend_from_slice(&cap.to_be_bytes());
    for c in cmds {
        buf.extend_from_slice(&(c.len() as u32).to_be_bytes());
        buf.extend_from_slice(c.as_bytes());
    }

    sock.write_all(&buf)
}

fn read_message(sock: &mut TcpStream) -> std::io::Result<(u32, String)> {
    let mut buf = [0u8; 1024];
    let n = sock.read(&mut buf)?;

    let numbuf = &buf[..4];
    let resp_len = u32::from_be_bytes([numbuf[0], numbuf[1], numbuf[2], numbuf[3]]) as usize;
    assert_eq!(resp_len, n);

    let numbuf = &buf[4..8];
    let status_code = u32::from_be_bytes([numbuf[0], numbuf[1], numbuf[2], numbuf[3]]);

    let str = std::str::from_utf8(&buf[8..resp_len]).unwrap().to_string();

    Ok((status_code, str))
}

fn main() -> std::io::Result<()> {
    let mut sock = TcpStream::connect("127.0.0.1:8080")?;
    sock.set_read_timeout(Some(Duration::from_secs(10)))?;
    sock.set_write_timeout(Some(Duration::from_secs(10)))?;

    let cmds: Vec<String> = args().skip(1).map(|s| s.trim().to_string()).collect();

    write_message(&mut sock, cmds)?;
    let resp = read_message(&mut sock)?;
    println!("{:?}", resp);

    Ok(())
}
