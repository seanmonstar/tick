use std::sync::{mpsc, Arc};
use std::sync::atomic::AtomicBool;

use mio::{self, EventLoop, Token, EventSet, PollOpt, TryAccept};

use stream::Stream;
use transfer;
use ::{Action, Message, Protocol, ProtocolFactory, Transport};

pub type Message_ = Message;
pub type Thunk = Box<FnMut() + Send + 'static>;

pub struct LoopHandler<F: ProtocolFactory,  T: Transport> {
    pub transports: mio::util::Slab<Evented<F::Proto, T>>,
    factory: F,
}

pub enum Evented<P: Protocol, T: Transport> {
    Listener(T::Listener),
    Stream(Stream<P, T>),
}

impl<F: ProtocolFactory, T: Transport> LoopHandler<F, T> {

    pub fn new(factory: F, size: usize) -> LoopHandler<F, T> {
        LoopHandler {
            transports: mio::util::Slab::new(size),
            factory: factory,
        }
    }

    pub fn listener(&mut self, event_loop: &mut EventLoop<Self>, lis: T::Listener) -> ::Result<Token> {
        let token = try!(self.transports.insert(Evented::Listener(lis))
                         .map_err(|_| ::Error::TooManySockets));
        match self.transports.get(token) {
            Some(&Evented::Listener(ref lis)) => {
                try!(event_loop.register(
                    lis,
                    token,
                    EventSet::readable(),
                    PollOpt::level()
                ));
                Ok(token)
            }
            _ => unreachable!()
        }
    }

    pub fn stream(&mut self, event_loop: &mut EventLoop<Self>, transport: T, events: EventSet) -> ::Result<Token> {
        let notify = event_loop.channel();
        let factory = &mut self.factory;
        let maybe_token = self.transports.insert_with(move |token| {
            trace!("inserting new stream {:?}", token);
            let (tx, rx) = mpsc::channel();
            let is_queued = Arc::new(AtomicBool::new(false));
            let transfer = transfer::new(token, notify, tx, is_queued.clone());
            let proto = factory.create(transfer, ::Id(token));
            Evented::Stream(Stream::new(token, transport, proto, rx, is_queued))
        });
        let token = match maybe_token {
            Some(token) => token,
            None => {
                trace!("failed to insert stream");
                return Err(::Error::TooManySockets);
            }
        };
        match self.transports.get(token) {
            Some(&Evented::Stream(ref stream)) => {
                trace!("registering initial '{:?}' for {:?}", events, token);
                try!(event_loop.register(
                    stream.transport(),
                    token,
                    events,
                    PollOpt::edge() | PollOpt::oneshot()
                ));
                Ok(token)
            },
            _ => unreachable!()
        }
    }

    fn action(&mut self, event_loop: &mut EventLoop<Self>, token: Token, action: Action) {
        let next = match action {
            Action::Wait => {
                debug!("  Action::Wait {:?}", token);
                return;
            }
            Action::Queued => {
                match self.transports.get_mut(token) {
                    Some(&mut Evented::Stream(ref mut stream)) => {
                        debug!("  Action::Queued {:?}", token);
                        stream.queued()
                    }
                    Some(_) => {
                        error!("cannot queue on listeners");
                        return;
                    }
                    None => {
                        warn!("  Action::Queued unknown token {:?}", token);
                        return;
                    }
                }
            }
            Action::Register(events) => {
                match self.transports.get_mut(token) {
                    Some(&mut Evented::Stream(ref mut stream)) => {
                        debug!("  Action::Register {:?}, '{:?}'", token, events);
                        event_loop.reregister(
                            stream.transport(),
                            token,
                            events,
                            PollOpt::edge() | PollOpt::oneshot()
                        );
                        return;
                    }
                    Some(_) => {
                        error!("cannot register listeners");
                        return;
                    }
                    None => {
                        error!("  Action::Register unknown token {:?}, '{:?}'", token, events);
                        return;
                    }
                }
            }
            Action::Remove => {
                debug!("  Action::remove {:?}", token);
                if let Some(slot) = self.transports.remove(token) {
                    match slot {
                        Evented::Listener(lis) => {
                            let _ = event_loop.deregister(&lis);
                        }
                        Evented::Stream(stream) => {
                            let _ = event_loop.deregister(stream.transport());
                        }
                    }
                }
                return;
            }
        };
        self.action(event_loop, token, next);
    }
}

enum Ready<T: Transport> {
    Insert(T),
    Action(Token, Action)
}

impl<F: ProtocolFactory, T: Transport> mio::Handler for LoopHandler<F, T> {
    type Message = Message_;
    type Timeout = Thunk;
    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        debug!("< Ready {:?} '{:?}'", token, events);
        let next = match self.transports.get_mut(token) {
            Some(&mut Evented::Listener(ref lis)) => {
                match lis.accept() {
                    Ok(Some(stream)) => Ready::Insert(stream),
                    Ok(None) => return,
                    Err(e) => panic!("unimplemented accept error {:?}", e),
                }
            },
            Some(&mut Evented::Stream(ref mut stream)) => {
                stream.ready(token, events);
                Ready::Action(token, stream.action())
            }
            None => {
                error!("unknown token ready {:?}", token);
                return;
            }
        };

        match next {
            Ready::Action(token, action) => {
                self.action(event_loop, token, action);
            },
            Ready::Insert(transport) => {
                let _ = self.stream(event_loop, transport, EventSet::readable());
            }
        }
    }

    fn timeout(&mut self, _: &mut EventLoop<Self>, mut cb: Thunk) {
        debug!("< Timeout");
        cb();
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, msg: Message) {
        match msg {
            Message::Timeout(cb, when) => {
                debug!("< Notify Message::Timeout {}ms", when);
                event_loop.timeout_ms(cb, when);
            }
            Message::Action(token, action) => {
                debug!("< Notify Message::Action {:?}", token);
                self.action(event_loop, token, action);
            }
            Message::Shutdown => {
                debug!("< Notify Message::Shutdown");
                event_loop.shutdown();
            }
        }
    }

    fn tick(&mut self, _event_loop: &mut EventLoop<Self>) {
    
    }
}
