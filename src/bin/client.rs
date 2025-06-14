use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn write_message(sock: &mut TcpStream, msg: &str) -> std::io::Result<()> {
    let len = msg.len() as u32;
    sock.write_all(&len.to_be_bytes())?;
    sock.write_all(msg.as_bytes())?;
    Ok(())
}

fn read_message(sock: &mut TcpStream) -> std::io::Result<String> {
    let mut len_buf = [0u8; 4];
    sock.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut data = vec![0u8; len];
    sock.read_exact(&mut data)?;
    Ok(String::from_utf8_lossy(&data).to_string())
}

fn main() -> std::io::Result<()> {
    let mut sock = TcpStream::connect("127.0.0.1:8080")?;
    sock.set_read_timeout(Some(Duration::from_secs(10)))?;
    sock.set_write_timeout(Some(Duration::from_secs(10)))?;

    let queries = vec!["hello1", "hello2", "hello3"];

    for q in &queries {
        write_message(&mut sock, q)?;
        let resp = read_message(&mut sock)?;
        println!("response: {}", &resp); // show up to 100 chars
    }

    // Send large message
    let big_msg = "X".repeat(32 << 20); // 33,554,432 bytes
    write_message(&mut sock, &big_msg)?;
    let resp = read_message(&mut sock)?;
    println!("big response len: {}", resp.len());
    assert_eq!(32 << 20, resp.len());

    Ok(())
}

