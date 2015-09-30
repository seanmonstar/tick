//! # Tick
//!
//! An implementation of Transports, Protocols, and Streams over mio.
//!
//! # Example
//!
//! ```rust
//! use tick::{Tick, Protocol, Transfer};
//!
//! struct Echo(Transfer);
//! impl Protocol<Tcp> for Echo {
//!     fn on_data(&mut self, data: &[u8]) {
//!         println!("data received: {:?}", data);
//!         self.0.write(data);
//!     }
//! }
//!
//! let mut tick = Tick::new(Echo);
//! tick.accept(listener);
//! tick.run();
//! ```

#![cfg_attr(test, deny(warnings))]
#![cfg_attr(test, deny(missing_docs))]

#[macro_use] extern crate log;
extern crate mio;

pub use tick::{Tick, Notify};
pub use protocol::Protocol;
pub use transfer::Transfer;
pub use transport::Transport;

mod handler;
mod protocol;
mod stream;
mod tick;
mod transfer;
mod transport;

#[derive(Clone, PartialEq, Debug)]
enum Action {
    Wait,
    Register(mio::EventSet),
    Queued,
    Remove,
}

#[derive(Clone, PartialEq, Debug)]
enum Queued {
    Resume,
    Pause,
    Write(Option<Vec<u8>>),
    Close,
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

/// Opaque ID returned when adding listeners and streams to the loop.
#[derive(Debug)]
pub struct Id(::mio::Token);
