use std::io;

use mio::{Token, EventSet};
use ::{Action, Interest, Protocol, Transport};

pub struct Stream<P: Protocol<T>, T: Transport> {
    transport: T,
    protocol: P,
    token: Token,
}

impl<P: Protocol<T>, T: Transport> Stream<P, T> {

    pub fn new(token: Token, transport: T, protocol: P) -> Stream<P, T> {
        Stream {
            transport: transport,
            protocol: protocol,
            token: token,
        }
    }

    pub fn ready(&mut self, token: Token, events: EventSet) {
        trace!("ready '{:?}'", events);
        if events.is_error() {
            debug!("error event on {:?}", token);
            //TODO: self.protocol.on_error(self.transport.error());
        }

        if events.is_readable() {
            trace!("on_readable ->");
            if let Err(e) = self.protocol.on_readable(&mut self.transport) {
                trace!("on_readable {:?}", e);
            }
        }

        if events.is_writable() {
            trace!("on_writable ->");
            if let Err(e) = self.protocol.on_writable(&mut self.transport) {
                trace!("on_writable {:?}", e);
            }
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn action(&mut self) -> Action {
        match self.protocol.interest() {
            Interest::Read => Action::Register(EventSet::readable()),
            Interest::Write => Action::Register(EventSet::writable()),
            Interest::ReadWrite => Action::Register(EventSet::readable() | EventSet::writable()),
            Interest::Remove => Action::Remove
        }
    }
}
