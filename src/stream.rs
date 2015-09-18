use std::io;
use std::mem;
use std::sync::mpsc;

use mio::{self, Token, EventSet, TryRead, TryWrite};
use ::{Action, Protocol, Transport};

pub type Action_ = Action;

pub struct Stream<P: Protocol, T: Transport> {
    transport: T,
    protocol: P,
    rx: mpsc::Receiver<Action>,
    reading: Reading,
    writing: Writing,
}

impl<P: Protocol, T: Transport> Stream<P, T> {

    pub fn new(transport: T, protocol: P, rx: mpsc::Receiver<Action_>) -> Stream<P, T> {
        Stream {
            transport: transport,
            protocol: protocol,
            rx: rx,
            reading: Reading::Open(vec![0; 4096]),
            writing: Writing::Waiting(io::Cursor::new(vec![])),
        }
    }

    pub fn queue_writing(&mut self, data: Option<Vec<u8>>) {
        match (self.writing.can_write(), data) {
            (true, Some(bytes)) => {
                trace!("queueing writing");
                let mut buf = self.writing.close();
                buf.get_mut().extend(&bytes);
                self.writing.open(buf);
            },
            (true, None) => {
                trace!("queueing closing");
                self.reading = Reading::Closed;
                self.writing.closing();
            },
            (false, data) => {
                trace!("cannot queue writing: {:?}", data);
            },
        }
    }

    pub fn ready(&mut self, token: Token, events: EventSet) {
        if events.is_error() {
            debug!("error event on {:?}", token);
        }

        if events.is_readable() {
            loop {
                //TODO: check if protocol paused
                let mut buf = self.reading.close();
                match self.transport.try_read(&mut buf) {
                    Ok(Some(0)) => {
                        trace!("read eof {:?}", token);
                        self.protocol.on_eof();
                        break;
                    }
                    Ok(Some(n)) => {
                        trace!("read {} bytes {:?}", n, token);
                        self.protocol.on_data(&buf[..n]);
                        self.reading.open(buf);
                    }
                    Ok(None) => {
                        trace!("read would block {:?}", token);
                        self.reading.open(buf);
                        break;
                    }
                    Err(e) => {
                        self.on_error(e.into());
                        return;
                    }
                }
            }
        }

        if events.is_writable() {
            let is_closing = !self.writing.can_write();
            let mut buf = self.writing.close();
            while buf.position() < buf.get_ref().len() as u64 {
                match self.transport.try_write_buf(&mut buf) {
                    Ok(Some(0)) => {
                        trace!("===> write 0 means what again? {:?}", token);
                        break;
                    },
                    Ok(Some(n)) => {
                        trace!("wrote {} bytes {:?}", n, token);
                    },
                    Ok(None) => {
                        trace!("write would block {:?}", token);
                        break;
                    }
                    Err(e) => {
                        self.on_error(e.into());
                        return;
                    }
                }
            }


            if buf.position() == buf.get_ref().len() as u64 {
                buf.set_position(0);
                buf.get_mut().truncate(0);
                if is_closing {
                    //leave closed
                } else {
                    self.writing.open(buf);
                    self.writing.wait();
                }
            } else {
                self.writing.open(buf);
                if is_closing {
                    self.writing.closing();
                }
            }
        }

    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn action(&self) -> Action {
        match (&self.reading, &self.writing) {
            (&Reading::Open(_), &Writing::Open(_)) |(&Reading::Open(_), &Writing::Closing(_)) => Action::Register(EventSet::readable() | EventSet::writable()),
            (&Reading::Closed, &Writing::Closed) => Action::Remove,

            (&Reading::Open(_), _) => Action::Register(EventSet::readable()),
            (_, &Writing::Open(_)) | (_, &Writing::Closing(_)) => Action::Register(EventSet::writable()),

            _ => Action::Wait
        }
    }

    fn on_error(&mut self, e: ::Error) {
        self.writing = Writing::Closed;
        self.reading = Reading::Closed;
        self.protocol.on_end(Some(e));
    }
}

enum Reading {
    Open(Vec<u8>),
    Paused(Vec<u8>),
    Closed
}

impl Reading {
    fn open(&mut self, buf: Vec<u8>) {
        mem::replace(self, Reading::Open(buf));
    }

    fn pause(&mut self) {
        let buf = self.close();
        mem::replace(self, Reading::Paused(buf));
    }

    fn close(&mut self) -> Vec<u8> {
        match mem::replace(self, Reading::Closed) {
            Reading::Open(buf) => buf,
            Reading::Paused(buf) => buf,
            _ => panic!("already closed")
        }
    }
}

enum Writing {
    Open(io::Cursor<Vec<u8>>),
    Waiting(io::Cursor<Vec<u8>>),
    Closing(io::Cursor<Vec<u8>>),
    Closed
}

impl Writing {
    fn open(&mut self, buf: io::Cursor<Vec<u8>>) {
        mem::replace(self, Writing::Open(buf));
    }

    fn wait(&mut self) {
        let buf = self.close();
        mem::replace(self, Writing::Waiting(buf));
    }

    fn closing(&mut self) {
        let buf = self.close();
        mem::replace(self, Writing::Closing(buf));
    }

    fn can_write(&self) -> bool {
        match *self {
            Writing::Open(_) | Writing::Waiting(_) => true,
            _ => false
        }
    }

    fn close(&mut self) -> io::Cursor<Vec<u8>> {
        match mem::replace(self, Writing::Closed) {
            Writing::Open(buf) => buf,
            Writing::Waiting(buf) => buf,
            Writing::Closing(buf) => buf,
            _ => panic!("already closed")
        }
    }
}

