extern crate env_logger;
extern crate mio;
extern crate tick;

use std::io::{Write, stdout};

struct Client(tick::Transfer, tick::Id);

impl tick::Protocol for Client {
    fn on_data(&mut self, data: &[u8]) {
        stdout().write(data).unwrap();
        self.0.close();
    }


    fn on_end(&mut self, err: Option<tick::Error>) {
        println!("connection closing {:?}", err);
    }

}

fn main() {
    env_logger::init().unwrap();
    let mut tick = tick::Tick::<mio::tcp::TcpStream, _, _>::new(|mut transfer| {
        transfer.write(b"GET / HTTP/1.1\r\n\r\n");
        transfer.eof();
        Client(transfer)
    });

    let sock = mio::tcp::TcpStream::connect(&"127.0.0.1:3000".parse().unwrap()).unwrap();
    let id = tick.stream(sock).unwrap();
    println!("Connecting to 127.0.0.1:3000");
    tick.run_until_complete(id).unwrap();
}
