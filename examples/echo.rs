extern crate env_logger;
extern crate mio;
extern crate tick;

use std::io::{self, Read, Write};

type Tcp = mio::tcp::TcpStream;

struct Echo {
    buf: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
    eof: bool,
}

impl Echo {
    fn interest(&self) -> tick::Interest {
        match (self.eof, self.read_pos, self.write_pos) {
            (false, 0, 0) => tick::Interest::Read,
            (true, 0, 0) => tick::Interest::Remove,
            (false, r, w) if r > w => tick::Interest::ReadWrite,
            (true, r, w) if r > w => tick::Interest::Write,
            _ => tick::Interest::Remove
        }
    }
}


impl tick::Protocol<Tcp> for Echo {
    fn on_readable(&mut self, transport: &mut Tcp) -> tick::Interest {
        if self.read_pos < self.buf.len() {
            match transport.read(&mut self.buf[self.read_pos..]) {
                Ok(0) => self.eof = true,
                Ok(n) => self.read_pos += n,
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => {},
                    _ => {
                        println!("read error {:?}", e);
                        return tick::Interest::Remove;
                    }
                }
            }
        }

        self.interest()
    }

    fn on_writable(&mut self, transport: &mut Tcp) -> tick::Interest {
        while self.write_pos < self.read_pos {
            match transport.write(&self.buf[self.write_pos..self.read_pos]) {
                Ok(0) => panic!("write ZERO"),
                Ok(n) => self.write_pos += n,
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => break,
                    _ => {
                        println!("write error {:?}", e);
                        return tick::Interest::Remove;
                    }
                }
            }
        }
        self.read_pos = 0;
        self.write_pos = 0;
        self.interest()
    }

    fn on_error(&mut self, e: tick::Error) {
        self.eof = true;
        println!("on_error: {:?}", e);
    }
}

fn main() {
    env_logger::init().unwrap();
    let mut tick = tick::Tick::new(|_| (Echo {
        buf: vec![0; 4096],
        read_pos: 0,
        write_pos: 0,
        eof: false,
    }, tick::Interest::ReadWrite));

    let sock = mio::tcp::TcpListener::bind(&"127.0.0.1:3300".parse().unwrap()).unwrap();
    tick.accept(sock).unwrap();
    println!("Listening on 127.0.0.1:3300");
    tick.run().unwrap();
}
