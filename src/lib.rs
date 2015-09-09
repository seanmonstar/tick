#![cfg_attr(test, deny(warnings))]
#![cfg_attr(test, deny(missing_docs))]

extern crate eventual;
#[macro_use] extern crate log;
extern crate mio;

pub use eventual::Async;

pub use tick::Tick;
mod tick;
mod tcp;

enum Action {
    Wait,
    Register(mio::EventSet),
    Accept(Sender<Pair<Vec<u8>>>),
    Read(Option<Sender<Vec<u8>>>),
    Write(Option<(Vec<u8>, Stream<Vec<u8>>)>),
    Remove,
}


enum Message {
    Listener(tcp::Listener),
    Stream(tcp::Stream),
    Action(mio::Token, Action),
    Shutdown,
}

#[derive(Debug)]
pub enum Error {
    TooManySockets,
    Io(::std::io::Error)
}

impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
pub type Stream<T> = eventual::Stream<T, Error>;
pub type Sender<T> = eventual::Sender<T, Error>;
pub type Pair<T> = (Sender<T>, Stream<T>);
pub type Future<T> = eventual::Future<T, Error>;

#[derive(Clone)]
struct Notify<T: Send_> {
    sender: T
}

impl<T: Send_> Notify<T> {
    fn send(&self, msg: Message) -> bool {
        self.sender.send(msg)
    }
}

trait Send_: Clone + Send + 'static {
    fn send(&self, msg: Message) -> bool;
}

impl Send_ for mio::Sender<Message> {
    fn send(&self, msg: Message) -> bool {
        self.send(msg).is_ok()
    }
}

#[cfg(test)]
impl Send_ for ::std::sync::mpsc::Sender<Message> {
    fn send(&self, msg: Message) -> bool {
        self.send(msg).is_ok()
    }
}
