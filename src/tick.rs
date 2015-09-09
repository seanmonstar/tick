use std::io;
use std::thread;

use mio::{self, Token, EventLoop, EventSet, PollOpt};
use mio::tcp::{TcpStream, TcpListener};

use ::tcp::{Listener, Stream};
use ::{Message, Action, Notify, Send_};


pub struct Tick {
    notify: Notify<mio::Sender<Message>>
}

impl Tick {
    pub fn new() -> io::Result<Tick> {
        let mut event_loop = try!(EventLoop::new());
        let sender = event_loop.channel();
        let loop_sender = sender.clone();

        thread::spawn(move || {
            let mut handler = LoopHandler {
                sockets: mio::util::Slab::new(65_535),
                notify: Notify { sender: loop_sender },
            };
            event_loop.run(&mut handler).unwrap();
            trace!("event_loop.run complete");
        });

        Ok(Tick {
            notify: Notify { sender: sender }
        })
    }

    pub fn accept(&self, raw: TcpListener) -> ::Stream<::Pair<Vec<u8>>> {
        let (listener, rx) = Listener::new(raw);
        self.notify.send(Message::Listener(listener));
        rx
    }

    pub fn stream(&self, raw: TcpStream) -> ::Pair<Vec<u8>> {
        let (stream, pair) = Stream::new(raw);
        self.notify.send(Message::Stream(stream));
        pair
    }
}

impl Drop for Tick {
    fn drop(&mut self) {
        let _ = self.notify.send(Message::Shutdown);
    }
}

struct LoopHandler {
    sockets: mio::util::Slab<Evented>,
    notify: Notify<mio::Sender<Message>>,
}

type TickLoop = EventLoop<LoopHandler>;

enum Evented {
    Listener(Listener),
    Stream(Stream),
}

impl Evented {
    fn ready<T: Send_>(&mut self, _event_loop: &mut TickLoop, token: Token, events: EventSet, notify: &Notify<T>) -> Action {
        match *self {
            Evented::Listener(ref mut listener) => {
                if events.is_readable() {
                    let (stream, action) = listener.accept(notify, token);
                    if let Some(stream) = stream {
                        trace!("Listener.ready accepted stream");
                        notify.send(Message::Stream(stream));
                    }
                    //TODO: if action is Register, we could recursive this method
                    action
                } else {
                    trace!("Listener ready, but not readable?? {:?}", events);
                    Action::Register(EventSet::readable())
                }
            },
            Evented::Stream(ref mut stream) => {
                if events.is_readable() {
                    stream.read(notify, token);
                }
                if events.is_writable() {
                    stream.write(notify, token);
                }
                stream.action()
            }
        }
    }

    fn register(&self, event_loop: &mut TickLoop, token: Token, events: EventSet) {
        match *self {
            Evented::Listener(ref listener) => {
                let res = event_loop.reregister(
                    listener.raw(),
                    token,
                    events,
                    PollOpt::edge() | PollOpt::oneshot()
                );

                if let Err(e) = res {
                    panic!("register failure: {}", e);
                }
            }
            Evented::Stream(ref stream) => {
                let res = event_loop.reregister(
                    stream.raw(),
                    token,
                    events,
                    PollOpt::edge() | PollOpt::oneshot()
                );

                if let Err(e) = res {
                    panic!("register failure: {}", e);
                }
            }
        }
    }
}

impl LoopHandler {
    fn action(&mut self, event_loop: &mut TickLoop, token: Token, action: Action) {
        match action {
            Action::Wait => trace!("Action.wait token={:?}", token),
            Action::Remove => {
                trace!("Action.remove token={:?}", token);
                //TODO: deregister with event_loop?
                self.sockets.remove(token);
            },
            Action::Register(events) => {
                trace!("Action.register token={:?}, events={:?}", token, events);
                self.sockets[token].register(event_loop, token, events);
            }
            Action::Accept(tx) => {
                trace!("Action.accept token={:?}", token);
                let next_action = match &mut self.sockets[token] {
                    &mut Evented::Listener(ref mut listener) => {
                        listener.listening(tx)
                    }
                    _ => panic!("stream cant accept")
                };
                self.action(event_loop, token, next_action);
            }
            Action::Read(tx_opt) => {
                trace!("Action.read token={:?}, closing={}", token, tx_opt.is_none());
                let next_action = match &mut self.sockets[token] {
                    &mut Evented::Stream(ref mut stream) => {
                        if let Some(tx) = tx_opt {
                            stream.read_want(tx)
                        } else {
                            stream.read_close()
                        }
                    }
                    _ => panic!("listener cant read!")
                };
                self.action(event_loop, token, next_action);
            },
            Action::Write(rx_opt) => {
                trace!("Action.write token={:?}, closing={}", token, rx_opt.is_none());
                let next_action = match &mut self.sockets[token] {
                    &mut Evented::Stream(ref mut stream) => {
                        if let Some((bytes, rx)) = rx_opt {
                            stream.write_want(bytes, rx)
                        } else {
                            stream.write_close()
                        }
                    }
                    _ => panic!("listener cant read!")
                };
                self.action(event_loop, token, next_action);
            },
        }
    }
}

impl mio::Handler for LoopHandler {
    type Message = Message;
    type Timeout = ();


    fn ready(&mut self, event_loop: &mut TickLoop, token: Token, events: EventSet) {
        trace!("ready token={:?}, events='{:?}'", token, events);
        let action = self.sockets[token].ready(event_loop, token, events, &self.notify);
        self.action(event_loop, token, action);
    }

    fn notify(&mut self, event_loop: &mut TickLoop, msg: Message) {
        match msg {
            Message::Listener(listener) => {
                let token = match self.sockets.insert(Evented::Listener(listener)) {
                    Ok(token) => token,
                    Err(Evented::Listener(mut listener)) => {
                        warn!("too many sockets to insert listener");
                        listener.err(::Error::TooManySockets);
                        return;
                    },
                    _ => unreachable!()
                };
                trace!("notify Message::Listener = {:?}", token);
                let action = match &mut self.sockets[token] {
                    &mut Evented::Listener(ref mut listener) => {
                        let action = listener.listen(&self.notify, token);
                        if let Action::Register(..) = action {
                            event_loop.register_opt(
                                listener.raw(),
                                token,
                                EventSet::none(),
                                PollOpt::edge() | PollOpt::oneshot()
                            ).unwrap();
                        }
                        action
                    },
                    _ => unreachable!()
                };
                self.action(event_loop, token, action);
            }
            Message::Stream(stream) => {
                let token = self.sockets.insert(Evented::Stream(stream))
                    .ok().expect("unimplemented when too many sockets");
                trace!("notify Message::Stream = {:?}", token);
                let action = match &mut self.sockets[token] {
                    &mut Evented::Stream(ref mut stream) => {
                        let action = stream.init(&self.notify, token);
                        if let Action::Register(..) = action {
                            event_loop.register_opt(
                                stream.raw(),
                                token,
                                EventSet::none(),
                                PollOpt::edge() | PollOpt::oneshot()
                            ).unwrap();
                        }
                        action
                    },
                    _ => unreachable!()
                };
                self.action(event_loop, token, action);
            }
            Message::Action(token, action) => {
                trace!("notify MessageAction token={:?}", token);
                self.action(event_loop, token, action);
            }
            Message::Shutdown => {
                trace!("notify Message::Shutdown");
                event_loop.shutdown()
            }
        }
    }
}
