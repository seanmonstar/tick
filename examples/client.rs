extern crate env_logger;
extern crate mio;
extern crate tick;

use std::io::{self, Read, Write, stdout};
type Tcp = mio::tcp::TcpStream;

struct Client {
    msg: &'static [u8],
    pos: usize,
    eof: bool,
}


impl Client {
    fn new() -> Client {
        Client {
            msg: b"GET / HTTP/1.1\r\n\r\n",
            pos: 0,
            eof: false,
        }
    }
}

impl Client {
    fn interest(&self) -> tick::Interest {
        if self.eof {
            tick::Interest::Remove
        } else if self.pos < self.msg.len() {
            tick::Interest::ReadWrite
        } else {
            tick::Interest::Read
        }
    }
}

impl tick::Protocol<Tcp> for Client {
    fn on_readable(&mut self, transport: &mut Tcp) -> tick::Interest {
        let mut buf = [0u8; 4096];
        loop {
            match transport.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    break;
                }
                Ok(n) => {
                    stdout().write_all(&buf[..n]).unwrap();
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => return tick::Interest::Remove,
            }
        }
        self.interest()
    }

    fn on_writable(&mut self, transport: &mut Tcp) -> tick::Interest {
        while self.pos < self.msg.len() {
            match transport.write(&self.msg[self.pos..]) {
                Ok(0) => return tick::Interest::Remove,
                Ok(n) => self.pos += n,
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => break,
                    _ => {
                        return tick::Interest::Remove;
                    }
                }
            }
        }
        self.interest()
    }

    fn on_error(&mut self, err: tick::Error) {
        self.eof = true;
        println!("on_error: {:?}", err);
    }
}

fn main() {
    env_logger::init().unwrap();
    let mut tick = tick::Tick::<mio::tcp::TcpListener, _>::new(|_| (Client::new(), tick::Interest::Write));

    let sock = mio::tcp::TcpStream::connect(&"127.0.0.1:1337".parse().unwrap()).unwrap();
    let id = tick.stream(sock).unwrap();
    println!("Connecting to 127.0.0.1:1337");
    tick.run_until_complete(id).unwrap();
}
