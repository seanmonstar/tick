extern crate env_logger;
extern crate mio;
extern crate tick;

use std::io::{self, Read, Write};
use tick::Interest;

struct Hello {
    msg: &'static [u8],
    pos: usize,
    eof: bool,
}

impl Hello {
    fn new() -> Hello {
        Hello {
            msg: b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello",
            pos: 0,
            eof: false
        }
    }

    fn interest(&self) -> tick::Interest {
        if self.pos >= self.msg.len() {
            tick::Interest::Remove
        } else if self.eof {
            tick::Interest::Write
        } else {
            tick::Interest::ReadWrite
        }
    }
}

type Tcp = mio::tcp::TcpStream;

impl tick::Protocol<Tcp> for Hello {
    fn on_readable(&mut self, transport: &mut Tcp) -> io::Result<Interest> {
        // should check data for proper http semantics, but oh well
        let mut buf = [0; 1024];
        while !self.eof {
            let n = try!(transport.read(&mut buf));
            if n == 0 {
                self.eof = true;
            }
        }
        Ok(self.interest())
    }

    fn on_writable(&mut self, transport: &mut Tcp) -> io::Result<Interest> {
        while self.pos < self.msg.len() {
            self.pos += try!(transport.write(&self.msg[self.pos..]));
        }
        if !self.eof {
            self.pos = 0;
        }
        Ok(self.interest())
    }

    fn on_error(&mut self, err: tick::Error) {
        self.msg = b"";
        self.pos = 0;
        println!("on_error: {:?}", err);
    }
}

fn main() {
    env_logger::init().unwrap();
    let mut tick = tick::Tick::new(|_, _| (Hello::new(), Interest::ReadWrite));
    let sock = mio::tcp::TcpListener::bind(&"127.0.0.1:3330".parse().unwrap()).unwrap();
    tick.accept(sock).unwrap();
    println!("Listening on 127.0.0.1:3330");
    tick.run().unwrap();
}
