use std::io;

use mio::{Token, EventSet};
use ::{Interest, Protocol, Transport};

pub struct Stream<P: Protocol<T>, T: Transport> {
    transport: T,
    protocol: P,
    token: Token,
    interest: Interest,
}

impl<P: Protocol<T>, T: Transport> Stream<P, T> {

    pub fn new(token: Token, transport: T, protocol: P, interest: Interest) -> Stream<P, T> {
        Stream {
            transport: transport,
            protocol: protocol,
            token: token,
            interest: interest,
        }
    }

    pub fn ready(&mut self, token: Token, events: EventSet) {
        trace!("ready {:?}, '{:?}'", token, events);
        if events.is_error() {
            debug!("error event on {:?}", token);
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
                            trace!("on_readable {:?} {:?}", token, e);
                            break;
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
                            trace!("on_writable {:?} {:?}", token, e);
                            break;
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
