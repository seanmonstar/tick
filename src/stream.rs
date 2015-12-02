use std::io;

use mio::{Token, EventSet};
use ::{Interest, Protocol, Transport};

pub struct Stream<P: Protocol<T>, T: Transport> {
    transport: T,
    protocol: P,
    interest: Interest,
}

impl<P: Protocol<T>, T: Transport> Stream<P, T> {

    pub fn new(transport: T, protocol: P, interest: Interest) -> Stream<P, T> {
        Stream {
            transport: transport,
            protocol: protocol,
            interest: interest,
        }
    }

    pub fn ready(&mut self, token: Token, events: EventSet) {
        trace!("ready {:?}, '{:?}'", token, events);
        if events.is_error() {
            error!("error event on {:?}", token);
            self.interest = Interest::Remove;
            return;
            //TODO: self.protocol.on_error(self.transport.error());
        }

        if events.is_readable() {
            trace!("on_readable {:?} ->", token);
            loop {
                trace!("  on_readable loop");
                match self.protocol.on_readable(&mut self.transport) {
                    Ok(interest) => {
                        self.interest = interest;
                        break;
                    },
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => break,
                        io::ErrorKind::Interrupted => (),
                        _ => {
                            self.interest = Interest::Remove;
                            self.protocol.on_error(e.into());
                            return;
                        }
                    }
                }
            }
        }

        if events.is_writable() {
            trace!("on_writable {:?} ->", token);
            loop {
                trace!("  on_writable loop");
                match self.protocol.on_writable(&mut self.transport) {
                    Ok(interest) => {
                        self.interest = interest;
                        break;
                    },
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => break,
                        io::ErrorKind::Interrupted => (),
                        _ => {
                            self.interest = Interest::Remove;
                            self.protocol.on_error(e.into());
                            return;
                        }
                    }
                }
            }
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn interest(&self) -> Interest {
        self.interest
    }

    pub fn removed(self) {
        self.protocol.on_remove(self.transport);
    }
}
