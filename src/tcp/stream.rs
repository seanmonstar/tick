use std::mem;
use std::io::Cursor;

use eventual::{self, Async};
use mio::tcp::TcpStream;
use mio::{EventSet, Token, TryRead, TryWrite};

use ::{Action, Notify, Message, Send_};

pub struct Stream {
    raw: TcpStream,
    reading: Reading,
    writing: Writing,
}

impl Stream {
    pub fn new(raw: TcpStream) -> (Stream, ::Pair<Vec<u8>>) {
        let (read_tx, read_rx) = eventual::Pair::pair();
        let (write_tx, write_rx) = eventual::Pair::pair();
        (Stream {
            raw: raw,
            reading: Reading::New(read_tx),
            writing: Writing::New(write_rx)
        }, (write_tx, read_rx))
    }

    pub fn raw(&self) -> &TcpStream {
        &self.raw
    }

    pub fn init<T: Send_>(&mut self, notify: &Notify<T>, token: Token) -> Action {
        trace!("Stream.init token={:?}", token);
        let tx = self.reading.new_to_waiting();
        match tx.poll() {
            Ok(Ok(tx)) => {
                self.reading.waiting_to_reading(tx);
            }
            Ok(Err(_dropped)) => {
                self.reading.close();
            }
            Err(tx) => {
                self.read_wait(tx, notify, token);
            }
        }

        let rx = self.writing.new_to_waiting();
        match rx.poll() {
            Ok(Ok(Some((bytes, rx)))) => {
                self.writing.waiting_to_writing(Cursor::new(bytes), rx);
            }
            Ok(Ok(None)) => {
                self.writing.close();
            }
            Ok(Err(e)) => {
                panic!("what is this error {:?}", e);
            }
            Err(rx) => {
                self.write_wait(rx, notify, token);
            }
        }

        self.action()
    }

    fn read_wait<T, A>(&mut self, tx: A, notify: &Notify<T>, token: Token)
    where A: Async<Value=::Sender<Vec<u8>>>, T: Send_
    {
        trace!("Stream.read_wait token={:?}", token);
        let notify = notify.clone();
        tx.receive(move |res| {
            match res {
                Ok(tx) => {
                    notify.send(Message::Action(token, Action::Read(Some(tx))));
                },
                Err(_dropped) => {
                    notify.send(Message::Action(token, Action::Read(None)));
                }
            }
        });
    }

    fn write_wait<T: Send_>(&mut self, rx: ::Stream<Vec<u8>>, notify: &Notify<T>, token: Token) {
        trace!("Stream.write_wait token={:?}", token);
        let notify = notify.clone();
        rx.receive(move |res| {
            match res {
                Ok(head) => {
                    notify.send(Message::Action(token, Action::Write(head)));
                },
                Err(e) => {
                    panic!("what is this error {:?}", e);
                }
            }
        });
    }

    pub fn read_want(&mut self, tx: ::Sender<Vec<u8>>) -> Action {
        trace!("Stream.read_want");
        self.reading.waiting_to_reading(tx);
        self.action()
    }

    pub fn write_want(&mut self, bytes: Vec<u8>, rx: ::Stream<Vec<u8>>) -> Action {
        trace!("Stream.write_want");
        self.writing.waiting_to_writing(Cursor::new(bytes), rx);
        self.action()
    }

    pub fn read<T: Send_>(&mut self, notify: &Notify<T>, token: Token) {
        trace!("Stream.read token={:?}", token);
        let tx = self.reading.reading_to_waiting();

        let mut buf = vec![0u8; 1024];
        match self.raw.try_read(&mut buf) {
            Ok(Some(0)) => {
                trace!("Stream.read EOF token={:?}", token);
                self.reading.close();
            }
            Ok(Some(n)) => {
                trace!("Stream.read count={}, token={:?}", n, token);
                buf.truncate(n);
                let busy = tx.send(buf);
                self.read_wait(busy, notify, token);
            }
            Ok(None) => {
                trace!("Stream.read not ready, token={:?}", token);
                self.reading.waiting_to_reading(tx);
            }
            Err(e) => {
                trace!("Stream.read error, token={:?}, err={:?}", token, e);
                tx.fail(e.into());
                self.reading.close();
            }
        }
    }

    pub fn write<T: Send_>(&mut self, notify: &Notify<T>, token: Token) {
        trace!("Stream.write token={:?}", token);
        let (mut cursor, rx) = self.writing.writing_to_waiting();
        let mut pos = cursor.position() as usize;
        let len = cursor.get_ref().len();
        {
            let bytes = cursor.get_ref();
            while pos < len {
                match self.raw.try_write(&bytes[pos..]) {
                    Ok(Some(0)) => {
                        warn!("unexpected write eof? token={:?}", token);
                        self.writing.close();
                        return;
                    }
                    Ok(Some(n)) => {
                        trace!("wrote {} bytes on {:?}", n, token);
                        pos += n;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        error!("io error! {}", e);
                        self.writing.close();
                        return;
                    }
                }
            }
        }
        if pos < len {
            cursor.set_position(pos as u64);
            self.writing.waiting_to_writing(cursor, rx)
        } else {
            self.write_wait(rx, notify, token);
        }

    }

    pub fn read_close(&mut self) -> Action {
        trace!("Stream.read_close");
        self.reading.close();
        self.action()
    }

    pub fn write_close(&mut self) -> Action {
        trace!("Stream.write_close");
        self.writing.close();
        self.action()
    }

    pub fn action(&self) -> Action {
        match (&self.reading, &self.writing) {
            (&Reading::Reading(..), &Writing::Writing(..)) => Action::Register(EventSet::readable() | EventSet::writable()),
            (&Reading::Reading(..), _) => Action::Register(EventSet::readable()),
            (_, &Writing::Writing(..)) => Action::Register(EventSet::writable()),
            (&Reading::Closed, &Writing::Closed) => Action::Remove,
            _ => Action::Wait
        }
    }
}

enum Reading {
    New(::Sender<Vec<u8>>),
    Waiting,
    Reading(::Sender<Vec<u8>>),
    Closed
}

impl Reading {
    fn new_to_waiting(&mut self) -> ::Sender<Vec<u8>> {
        match mem::replace(self, Reading::Waiting) {
            Reading::New(tx) => tx,
            _ => panic!("wrong reading")
        }
    }

    fn waiting_to_reading(&mut self, tx: ::Sender<Vec<u8>>) {
        match mem::replace(self, Reading::Reading(tx)) {
            Reading::Waiting => (),
            _ => panic!("wrong reading")
        }
    }

    fn reading_to_waiting(&mut self) -> ::Sender<Vec<u8>> {
        match mem::replace(self, Reading::Waiting) {
            Reading::Reading(tx) => tx,
            _ => panic!("wrong reading")
        }
    }

    fn close(&mut self) {
        mem::replace(self, Reading::Closed);
    }
}

enum Writing {
    New(::Stream<Vec<u8>>),
    Waiting,
    Writing(Cursor<Vec<u8>>, ::Stream<Vec<u8>>),
    Closed
}

impl Writing {
    fn new_to_waiting(&mut self) -> ::Stream<Vec<u8>> {
        match mem::replace(self, Writing::Waiting) {
            Writing::New(rx) => rx,
            _ => panic!("wrong writing")
        }
    }

    fn waiting_to_writing(&mut self, bytes: Cursor<Vec<u8>>, rx: ::Stream<Vec<u8>>) {
        match mem::replace(self, Writing::Writing(bytes, rx)) {
            Writing::Waiting => (),
            _ => panic!("wrong writing")
        }
    }

    fn writing_to_waiting(&mut self) -> (Cursor<Vec<u8>>, ::Stream<Vec<u8>>) {
        match mem::replace(self, Writing::Waiting) {
            Writing::Writing(bytes, rx) => (bytes, rx),
            _ => panic!("wrong writing")
        }
    }

    fn close(&mut self) {
        mem::replace(self, Writing::Closed);
    }
}
