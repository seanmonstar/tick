use std::fmt;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use mio;

use ::{Action, Message, Queued};
pub type Message_ = Message;
pub type Queued_ = Queued;

#[derive(Clone)]
pub struct Transfer {
    token: mio::Token,
    notify: mio::Sender<Message>,
    is_notified: Arc<AtomicBool>,
    sender: mpsc::Sender<Queued>,
}

pub fn new(token: mio::Token, notify: mio::Sender<Message_>, sender: mpsc::Sender<Queued_>, is_notified: Arc<AtomicBool>) -> Transfer {
    Transfer {
        token: token,
        notify: notify,
        is_notified: is_notified,
        sender: sender,
    }
}

impl Transfer {

    pub fn write(&mut self, data: &[u8]) {
        self.send(Queued::Write(Some(data.to_vec())));
    }

    pub fn eof(&mut self) {
        self.send(Queued::Write(None));
    }

    pub fn resume(&mut self) {
        self.send(Queued::Resume);
    }

    pub fn pause(&mut self) {
        self.send(Queued::Pause);
    }

    pub fn close(&mut self) {
        self.send(Queued::Close);
    }

    pub fn abort(&mut self) {
        //TODO: consume self?
        self.notify.send(Message::Action(self.token, Action::Remove)).unwrap();
    }

    fn send(&self, action: Queued) {
        if !self.is_notified.load(Ordering::Acquire) {
            debug!("> Send Action::Queued {:?}", self.token);
            self.notify.send(Message::Action(self.token, Action::Queued)).unwrap();
            self.is_notified.store(true, Ordering::Release);
        }
        debug!("+ {:?} {:?}", action, self.token);
        self.sender.send(action);
    }
}

impl fmt::Debug for Transfer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Transfer")
            .field("token", &self.token)
            .finish()
    }
}

/*
impl Drop for Transfer {
    fn drop(&mut self) {
        trace!("Transfer::drop {:?}", self.token);
        self.close();
    }
}
*/
