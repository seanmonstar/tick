extern crate env_logger;
extern crate mio;
extern crate tick;

struct Hello(tick::Transfer);

impl tick::Protocol for Hello {
    fn on_connection(transfer: tick::Transfer) -> Hello {
        Hello(transfer)
    }

    fn on_data(&mut self, _data: &[u8]) {
        // should check data for proper http semantics, but oh well
        self.0.write(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello");
    }


    fn on_end(&mut self, err: Option<tick::Error>) {
        println!("connection closing {:?}", err);
    }

}

fn main() {
    env_logger::init().unwrap();
    let mut tick = tick::Tick::<Hello, mio::tcp::TcpStream>::new();

    let sock = mio::tcp::TcpListener::bind(&"127.0.0.1:3000".parse().unwrap()).unwrap();
    tick.accept(sock).unwrap();
    println!("Listening on 127.0.0.1:3000");
    tick.run().unwrap();
}
