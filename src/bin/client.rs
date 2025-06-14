use std::io::{Read, Write};
use std::net::TcpStream;

fn to_frame(s: &String) -> Vec<u8> {
    let mut v = vec![];
    let len = s.len() as u32;
    v.extend_from_slice(&len.to_be_bytes());
    v.extend_from_slice(s.as_bytes());
    v
}

fn main() -> std::io::Result<()> {
    let mut sock = TcpStream::connect("127.0.0.1:8080")?;
    let mut queries = vec!["hello1".to_string(), "hello2".to_string(), "hello3".to_string()];
    queries.push("z".repeat(11));
    queries.push("hello5".into());

    for bytes in queries.iter().map(to_frame) {
        sock.write_all(&bytes)?;
        let mut buf = [0; 1024];
        let n = sock.read(&mut buf)?;
        println!("got: {}", String::from_utf8_lossy(&buf[..n]));
    }

    Ok(())
}
