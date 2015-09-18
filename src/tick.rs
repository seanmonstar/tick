use std::sync::mpsc;

use mio::{self, EventLoop, Token, EventSet, PollOpt, TryAccept};

use stream::Stream;
use transfer;
use transport::Transport;
use ::{Action, Message, Protocol};


pub struct Tick<P: Protocol, T: Transport> {
    handler: LoopHandler<P, T>,
    event_loop: EventLoop<LoopHandler<P, T>>
}

impl<P: Protocol, T: Transport> Tick<P, T> {
    pub fn new() -> Tick<P, T> {
        Tick {
            handler: LoopHandler {
                transports: mio::util::Slab::new(4096)
            },
            event_loop: EventLoop::new().unwrap()
        }
    }

    pub fn accept(&mut self, listener: T::Listener) -> ::Result<()> {
        let token = try!(self.handler.transports.insert(Evented::Listener(listener))
                         .map_err(|_| ::Error::TooManySockets));
        match self.handler.transports.get(token) {
            Some(&Evented::Listener(ref lis)) => {
                try!(self.event_loop.register_opt(
                    lis,
                    token,
                    EventSet::readable(),
                    PollOpt::level()
                ));
                Ok(())
            }
            _ => unreachable!()
        }
    }

    pub fn run(&mut self) -> ::Result<()> {
        self.event_loop.run(&mut self.handler).map_err(From::from)
    }
}

struct LoopHandler<P: Protocol, T: Transport> {
    transports: mio::util::Slab<Evented<P, T>>
}

enum Evented<P: Protocol, T: Transport> {
    Listener(T::Listener),
    Stream(Stream<P, T>),
}

impl<P: Protocol, T: Transport> LoopHandler<P, T> {
    fn action(&mut self, event_loop: &mut EventLoop<Self>, token: Token, action: Action) {
        let next = match action {
            Action::Wait => {
                debug!("Action::Wait token={:?}", token);
                return;
            }
            Action::Write(data) => {
                match self.transports.get_mut(token) {
                    Some(&mut Evented::Stream(ref mut stream)) => {
                        debug!("Action::Write token={:?}, data={:?}", token, data.is_some());
                        stream.queue_writing(data);
                        stream.action()
                    }
                    Some(_) => {
                        error!("cannot write to listeners");
                        return;
                    }
                    None => {
                        error!("unknown token {:?}", token);
                        return;
                    }
                }
            }
            Action::Register(events) => {
                match self.transports.get_mut(token) {
                    Some(&mut Evented::Stream(ref mut stream)) => {
                        debug!("Action::Register token={:?}, events='{:?}'", token, events);
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
                        error!("unknown token {:?}", token);
                        return;
                    }
                }
            }
            Action::Remove => {
                debug!("Action::remove {:?}", token);
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

impl<P: Protocol, T: Transport> mio::Handler for LoopHandler<P, T> {
    type Message = Message;
    type Timeout = Token;
    fn ready(&mut self, event_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        debug!("Ready token={:?}, events='{:?}'", token, events);
        let next = match self.transports.get_mut(token) {
            Some(&mut Evented::Listener(ref lis)) => {
                match lis.accept() {
                    Ok(Some(stream)) => Ready::Insert(stream),
                    Ok(None) => return,
                    Err(e) => return,
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
                let notify = event_loop.channel();
                let maybe_token = self.transports.insert_with(move |token| {
                    trace!("inserting new stream {:?}", token);
                    let (tx, rx) = mpsc::channel();
                    let transfer = transfer::new(token, notify, tx);
                    let proto = Protocol::on_connection(transfer);
                    Evented::Stream(Stream::new(transport, proto, rx))
                });
                let token = match maybe_token {
                    Some(token) => token,
                    None => {
                        trace!("failed to insert stream");
                        // failed to insert... that means we never told Protocol about it,
                        // so we can just pretend nothing happened
                        return;
                    }
                };
                match self.transports.get(token) {
                    Some(&Evented::Stream(ref stream)) => {
                        trace!("registering initial Readable for {:?}", token);
                        event_loop.register_opt(
                            stream.transport(),
                            token,
                            EventSet::readable(),
                            PollOpt::edge() | PollOpt::oneshot()
                        ).unwrap();
                    },
                    _ => unreachable!()
                }
            }
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<Self>, msg: Message) {
        match msg {
            Message::Action(token, action) => {
                debug!("Notify Message::Action {:?}", token);
                self.action(event_loop, token, action);
            }
            Message::Shutdown => {
                debug!("Notify Message::Shutdown");
                event_loop.shutdown();
            }
        }
    }
}
