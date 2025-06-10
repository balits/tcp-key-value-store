use std::{
    io::{self, BufRead, Read, Write},
    net::SocketAddr,
    vec,
};

fn main() -> io::Result<()> {
    let data = b"0005hello";
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

    let mut sock = std::net::TcpStream::connect(addr)?;

    let mut rbuf = String::with_capacity(1024);
    let mut wbuf = vec![0; 1024];
    let stdin = io::stdin();

    loop {
        print!("[you] ");
        let mut nread = stdin.lock().read_line(&mut rbuf).inspect_err(|e| eprintln!("error on stdin read {e}"))?;
        if 0 == nread {
            eprintln!("stdin: EOF");
        }

        sock.write_all(rbuf.as_bytes()).inspect_err(|e| eprintln!("error on socket write {e}"))?;
        nread = sock.read(&mut wbuf).inspect_err(|e| eprintln!("error on stdin read {e}"))?;
        if 0 == nread {
            eprintln!("sock read: EOF");
        }
        println!("[srv] {}", std::str::from_utf8(&wbuf[..nread]).unwrap());
    }
}
