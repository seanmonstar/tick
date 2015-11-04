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

impl tick::Protocol<Tcp> for Client {
    fn interest(&self) -> tick::Interest {
        if self.eof {
            tick::Interest::Remove
        } else if self.pos < self.msg.len() {
            tick::Interest::ReadWrite
        } else {
            tick::Interest::Read
        }
    }

    fn on_readable(&mut self, transport: &mut Tcp) -> io::Result<tick::Interest> {
        let mut buf = [0u8; 4096];
        loop {
            match transport.read(&mut buf) {
                Ok(0) => {
                    self.eof = true;
                    break;
                }
                Ok(n) => {
                    try!(stdout().write_all(&buf[..n]));
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e)
            }
        }
        Ok(self.interest())
    }

    fn on_writable(&mut self, transport: &mut Tcp) -> io::Result<tick::Interest> {
        while self.pos < self.msg.len() {
            self.pos += try!(transport.write(&self.msg[self.pos..]));
        }
        Ok(self.interest())
    }

    fn on_error(&mut self, err: tick::Error) {
        self.eof = true;
        println!("on_error: {:?}", err);
    }
}

fn main() {
    env_logger::init().unwrap();
    let mut tick = tick::Tick::<Tcp, _>::new(|_, _| Client::new());

    let sock = mio::tcp::TcpStream::connect(&"127.0.0.1:1337".parse().unwrap()).unwrap();
    let id = tick.stream(sock).unwrap();
    println!("Connecting to 127.0.0.1:1337");
    tick.run_until_complete(id).unwrap();
}
