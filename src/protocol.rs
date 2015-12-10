use mio::EventSet;

use ::{Action, Transport};

pub type Action_ = Action;

pub trait Protocol<T: Transport> {
    fn on_readable(&mut self, transport: &mut T) -> Interest;
    fn on_writable(&mut self, transport: &mut T) -> Interest;

    fn on_error(&mut self, error: ::Error);

    fn on_remove(self, _transport: T) where Self: Sized {
        trace!("on_remove; default just drops");
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Interest {
    Remove,
    Wait,
    Read,
    Write,
    ReadWrite,
}

impl ::std::ops::Add for Interest {
    type Output = Interest;
    fn add(self, other: Interest) -> Interest {
        match (self, other) {
            (Interest::Read, Interest::Write) => Interest::ReadWrite,
            (Interest::Read, Interest::ReadWrite) => Interest::ReadWrite,

            (Interest::Write, Interest::Read) => Interest::ReadWrite,
            (Interest::Write, Interest::ReadWrite) => Interest::ReadWrite,

            (Interest::Wait, Interest::Read) => Interest::Read,
            (Interest::Wait, Interest::Write) => Interest::Write,
            (Interest::Wait, Interest::ReadWrite) => Interest::ReadWrite,

            (_, Interest::Remove) => Interest::Remove,
            _ => self
        }
    }
}

impl ::std::ops::Sub for Interest {
    type Output = Interest;
    fn sub(self, other: Interest) -> Interest {
        match (self, other) {
            (Interest::ReadWrite, Interest::Write) => Interest::Read,
            (Interest::ReadWrite, Interest::Read) => Interest::Write,
            (_, Interest::Remove) => self,
            (_, Interest::Wait) => self,
            _ => Interest::Wait
        }
    }
}

impl Into<Action_> for Interest {
    fn into(self) -> Action {
        match self {
            Interest::Read => Action::Register(EventSet::readable()),
            Interest::Write => Action::Register(EventSet::writable()),
            Interest::ReadWrite => Action::Register(EventSet::readable() | EventSet::writable()),
            Interest::Wait => Action::Wait,
            Interest::Remove => Action::Remove
        }
    }
}

pub trait Factory<T: Transport> {
    type Protocol: Protocol<T>;
    fn create(&mut self, ::Transfer) -> (Self::Protocol, Interest);
}

impl<F, P, T> Factory<T> for F where F: FnMut(::Transfer) -> (P, Interest), P: Protocol<T>, T: Transport {
    type Protocol = P;
    fn create(&mut self, transfer: ::Transfer) -> (P, Interest) {
        self(transfer)
    }
}
