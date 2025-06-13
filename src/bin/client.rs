use std::{
    io::{self, Read, Write},
    net::SocketAddr,
    vec,
};

fn main() -> io::Result<()> {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

    let mut sock = std::net::TcpStream::connect(addr)?;

    fn input(d: &str) -> Vec<u8> {
        let mut v = Vec::with_capacity(4 + d.len());
        let d32 = d.len() as u32;
        v.extend_from_slice(&d32.to_be_bytes());
        v.extend_from_slice(d.as_bytes());
        v
    }

    {
        let w = input("hello");
        let n = sock.write(&w)?;
        assert_eq!(n, w.len());
        println!("Send data {:X?}", &w);

        let mut r = vec![0; w.len()];
        let n = sock.read_to_end(&mut r)?;
        assert_eq!(n, w.len());
        assert_eq!(w, r);
        println!("Recv data {:X?}", &r);
    }

    {
        let w = input("world");
        let n = sock.write(&w)?;
        assert_eq!(n, w.len());
        println!("Send data {:X?}", &w);

        let mut r = vec![0; w.len()];
        let n = sock.read_to_end(&mut r)?;
        assert_eq!(n, w.len());
        assert_eq!(w, r);
        println!("Recv data {:X?}", &r);
    }



    Ok(())
}
