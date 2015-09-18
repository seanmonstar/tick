//! # Tick
//!
//! An implementation of Transports, Protocols, and Streams over mio.
//!
//! # Example
//!
//! ```rust
//! use tick::{Tick, Protocol, Tcp};
//!
//! struct Echo;
//! impl Protocol<Tcp> for Echo {
//!     fn on_connection(transport: &mut Tcp) -> Echo {
//!         Echo
//!     }
//!     fn on_data(&mut self, data: &[u8], transport: &mut Tcp) {
//!         println!("data received: {:?}", data);
//!         transport.write(data);
//!     }
//! }
//!
//! let mut tick = Tick::<Echo>::new();
//! tick.accept(listener);
//! tick.stream(tcp::connect());
//! tick.run(&mut event_loop);
//! ```

#![cfg_attr(test, deny(warnings))]
#![cfg_attr(test, deny(missing_docs))]

extern crate eventual;
#[macro_use] extern crate log;
extern crate mio;

use std::time::Duration;

pub use tick::Tick;
pub use protocol::Protocol;
pub use transfer::Transfer;
pub use transport::Transport;

mod protocol;
mod stream;
mod tick;
mod transfer;
mod transport;

enum Action {
    Wait,
    Register(mio::EventSet),
    Write(Option<Vec<u8>>),
    Remove,
}


enum Message {
    //Timeout(Duration, ::eventual::Complete<(), Error>),
    Action(mio::Token, Action),
    #[allow(unused)]
    Shutdown,
}

#[derive(Debug)]
pub enum Error {
    TooManySockets,
    Timeout,
    Io(::std::io::Error)
}

impl From<::std::io::Error> for Error {
    fn from(e: ::std::io::Error) -> Error {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
/*
pub type Stream<T> = eventual::Stream<T, Error>;
pub type Sender<T> = eventual::Sender<T, Error>;
pub type Pair<T> = (Sender<T>, Stream<T>);
pub type Future<T> = eventual::Future<T, Error>;
*/
