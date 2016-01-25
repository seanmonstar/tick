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
extern crate slab;

pub use mio::Evented;
pub use tick::{Tick, Notify};
pub use protocol::{Protocol, Interest};
pub use protocol::Factory as ProtocolFactory;
pub use transfer::Transfer;
pub use transport::Transport;

mod handler;
mod protocol;
mod stream;
mod tick;
mod transfer;
mod transport;



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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(::mio::Token);

impl ::std::fmt::Debug for Id {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_tuple("Id")
            .field(&(self.0).0)
            .finish()
    }
}


impl slab::Index for Id {
    fn from_usize(i: usize) -> Id {
        Id(::mio::Token(i))
    }

    fn as_usize(&self) -> usize {
        (self.0).0
    }
}

pub type Slab<T> = slab::Slab<T, Id>;

mod internal {
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum Action {
        Register(::mio::EventSet),
        Wait,
        Remove,
    }

    pub enum Message {
        Interest(::mio::Token, ::Interest),
        //Timeout(Box<FnMut() + Send + 'static>, u64),
        Shutdown,
    }
}
