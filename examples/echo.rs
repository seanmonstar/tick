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


impl tick::Protocol<Tcp> for Echo {
    fn interest(&self) -> tick::Interest {
        match (self.eof, self.read_pos, self.write_pos) {
            (false, 0, 0) => tick::Interest::Read,
            (true, 0, 0) => tick::Interest::Remove,
            (false, r, w) if r > w => tick::Interest::ReadWrite,
            (true, r, w) if r > w => tick::Interest::Write,
            _ => tick::Interest::Remove
        }
    }
    fn on_readable(&mut self, transport: &mut Tcp) -> io::Result<()> {
        while self.read_pos < self.buf.len() {
            match transport.read(&mut self.buf[self.read_pos..]) {
                Ok(0) => {
                    self.eof = true;
                    return Ok(())
                },
                Ok(n) => self.read_pos += n,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    self.eof = true;
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    fn on_writable(&mut self, transport: &mut Tcp) -> io::Result<()> {
        while self.write_pos < self.read_pos {
            match try!(transport.write(&self.buf[self.write_pos..self.read_pos])) {
                0 => panic!("write ZERO"),
                n => self.write_pos += n,
            }
            println!("left to write: {}", self.read_pos - self.write_pos);
        }
        self.read_pos = 0;
        self.write_pos = 0;
        Ok(())
    }

    fn on_error(&mut self, e: tick::Error) {
        panic!(e);
    }
}

fn main() {
    env_logger::init().unwrap();
    let mut tick = tick::Tick::<mio::tcp::TcpStream, _>::new(|_, _| Echo {
        buf: vec![0; 4096],
        read_pos: 0,
        write_pos: 0,
        eof: false,
    });

    let sock = mio::tcp::TcpListener::bind(&"127.0.0.1:3300".parse().unwrap()).unwrap();
    tick.accept(sock).unwrap();
    println!("Listening on 127.0.0.1:3300");
    tick.run().unwrap();
}
