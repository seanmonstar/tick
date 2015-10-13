use std::io;

use ::Transport;

pub trait Protocol<T: Transport> {
    fn interest(&self) -> Interest;
    fn on_readable(&mut self, transport: &mut T) -> io::Result<()>;
    fn on_writable(&mut self, transport: &mut T) -> io::Result<()>;

    fn on_error(&mut self, error: ::Error);

    fn on_remove(self, _transport: T) where Self: Sized {
        trace!("on_remove; default just drops");
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Interest {
    Remove,
    Read,
    Write,
    ReadWrite,
}

/*
pub trait Protocol {
    fn on_data(&mut self, data: &[u8]) {
        trace!("ignored on_data({:?})", data);
    }

    fn on_eof(&mut self) {
        trace!("ignored on_eof");
    }

    fn on_pause(&mut self) {
        trace!("ignored on_pause");
    }

    fn on_resume(&mut self) {
        trace!("ignored on_resume");
    }

    fn on_end(&mut self, err: Option<::Error>) {
        trace!("ignored on_end({:?})", err);
    }
}
*/

pub trait Factory<T> {
    type Proto: Protocol<T>;
    fn create(&mut self, ::Transfer, ::Id) -> Self::Proto;
}

impl<F, P, T> Factory<T> for F where F: FnMut(::Transfer, ::Id) -> P, P: Protocol<T>, T: Transport {
    type Proto = P;
    fn create(&mut self, transfer: ::Transfer, id: ::Id) -> P {
        self(transfer, id)
    }
}
