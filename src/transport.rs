use std::io;

use mio::{Evented, TryAccept};

pub trait Transport: Evented + io::Read + io::Write {
    type Listener: Evented + TryAccept<Output=Self>;
}


// implementations of Transports

use mio::tcp::{TcpStream, TcpListener};
impl Transport for TcpStream {
    type Listener = TcpListener;
}
