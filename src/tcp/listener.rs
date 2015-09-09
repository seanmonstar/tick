use std::mem;

use eventual::{self, Async};
use mio::{Token, EventSet};
use mio::tcp::TcpListener;

use ::{Message, Action, Notify, Send_};
use super::Stream;

pub struct Listener {
    raw: TcpListener,
    state: State
}

impl Listener {
    pub fn new(raw: TcpListener) -> (Listener, ::Stream<::Pair<Vec<u8>>>) {
        let (tx, rx) = eventual::Pair::pair();
        (Listener {
            raw: raw,
            state: State::New(tx)
        }, rx)
    }

    pub fn err(&mut self, e: ::Error) {
        self.state.fail(e);
    }

    pub fn raw(&self) -> &TcpListener {
        &self.raw
    }

    pub fn listen<T: Send_>(&mut self, notify: &Notify<T>, token: Token) -> Action {
        let tx = self.state.wait();

        if tx.is_ready() {
            if tx.is_err() {
                Action::Remove
            } else {
                self.listening(tx)
            }
        } else {
            self.wait(tx, notify, token);
            Action::Wait
        }
    }

    pub fn listening(&mut self, tx: ::Sender<::Pair<Vec<u8>>>) -> Action {
        self.state.listen(tx);
        Action::Register(EventSet::readable())
    }

    pub fn accept<T: Send_>(&mut self, notify: &Notify<T>, token: Token) -> (Option<Stream>, Action) {
        if self.state.is_closed() {
            warn!("tried to accept on closed Listener {:?}", token);
            return (None, Action::Remove);
        }
        match self.raw.accept() {
            Ok(Some(sock)) => {
                let tx = self.state.wait();
                let (stream, pair) = Stream::new(sock);
                let busy = tx.send(pair);
                match busy.poll() {
                    Ok(Ok(tx)) => {
                        (Some(stream), self.listening(tx))
                    }
                    Ok(Err(_)) => {
                        trace!("Listener stream has been dropped for {:?}", token);
                        self.state = State::Closed;
                        (None, Action::Remove)
                    }
                    Err(busy) => {
                        self.wait(busy, notify, token);
                        (Some(stream), Action::Wait)
                    }
                }
            },
            Ok(None) => {
                trace!("Listener.accept was not actually ready {:?}", token);
                (None, Action::Register(EventSet::readable()))
            },
            Err(e) => {
                self.state.fail(e);
                (None, Action::Remove)
            }
        }
    }

    fn wait<T, A>(&mut self, tx: A, notify: &Notify<T>, token: Token)
    where A: Async<Value=::Sender<::Pair<Vec<u8>>>>, T: Send_ {
        trace!("Listener.wait token={:?}", token);

        let notify = notify.clone();
        tx.receive(move |res| {
            trace!("Listener.wait complete, token={:?}", token);

            match res {
                Ok(tx) => {
                    notify.send(
                        Message::Action(
                            token,
                            Action::Accept(tx)
                        )
                    );
                }
                Err(_) => {
                    notify.send(
                        Message::Action(
                            token,
                            Action::Remove
                        )
                    );
                }
            }
        });
    }
}

enum State {
    New(::Sender<::Pair<Vec<u8>>>),
    Waiting,
    Listening(::Sender<::Pair<Vec<u8>>>),
    Closed
}

impl State {
    fn is_closed(&self) -> bool {
        match *self {
            State::Closed => true,
            _ => false
        }
    }

    fn wait(&mut self) -> ::Sender<::Pair<Vec<u8>>> {
        match mem::replace(self, State::Waiting) {
            State::New(tx) | State::Listening(tx) => tx,
            _ => panic!("wrong state")
        }
    }

    fn listen(&mut self, tx: ::Sender<::Pair<Vec<u8>>>) {
        match mem::replace(self, State::Listening(tx)) {
            State::Waiting => (),
            _ => panic!("wrong state")
        }
    }

    fn fail<E: Into<::Error>>(&mut self, err: E) {
        match mem::replace(self, State::Closed) {
            State::New(tx) | State::Listening(tx) => tx.fail(err.into()),
            _ => ()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use ::Async;
    use mio::tcp::TcpListener;
    use super::Listener;

    fn bind() -> TcpListener {
        TcpListener::bind(&"0.0.0.0:0".parse().unwrap()).unwrap()
    }

    #[test]
    fn test_error() {
        let t = bind();
        let (mut listener, rx) = Listener::new(t);
        assert!(!listener.state.is_closed());
        listener.err(::Error::Io(io::Error::new(io::ErrorKind::Other, "test")));
        assert!(listener.state.is_closed());
        assert!(rx.is_err());
    }

}
