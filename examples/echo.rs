extern crate env_logger;
extern crate mio;
extern crate tick;

struct Echo(tick::Transfer);

impl tick::Protocol for Echo {
    fn on_data(&mut self, data: &[u8]) {
        self.0.write(data);
    }

    fn on_eof(&mut self) {
        self.0.close();
    }
}

fn main() {
    env_logger::init().unwrap();
    let mut tick = tick::Tick::<mio::tcp::TcpStream, _, _>::new(Echo);

    let sock = mio::tcp::TcpListener::bind(&"127.0.0.1:3300".parse().unwrap()).unwrap();
    tick.accept(sock).unwrap();
    println!("Listening on 127.0.0.1:3300");
    tick.run().unwrap();
}
