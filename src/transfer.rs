use std::fmt;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use mio;

use ::{Action, Message};
pub type Action_ = Action;
pub type Message_ = Message;

pub struct Transfer {
    token: mio::Token,
    notify: mio::Sender<Message>,
    is_notified: Arc<AtomicBool>,
    sender: mpsc::Sender<Action>,
}

pub fn new(token: mio::Token, notify: mio::Sender<Message_>, sender: mpsc::Sender<Action_>, is_notified: Arc<AtomicBool>) -> Transfer {
    Transfer {
        token: token,
        notify: notify,
        is_notified: is_notified,
        sender: sender,
    }
}

impl Transfer {

    pub fn write(&mut self, data: &[u8]) {
        self.send(Action::Write(Some(data.to_vec())));
    }

    pub fn eof(&mut self) {
        self.send(Action::Write(None));
    }

    pub fn resume(&mut self) {
        self.notify.send(
            Message::Action(self.token, Action::Register(mio::EventSet::readable()))
        ).unwrap();
    }

    pub fn close(&mut self) {
        //TODO: consume self?
        self.notify.send(
            Message::Action(self.token, Action::Close)
        ).unwrap();
    }


    fn send(&self, action: Action) {
        if self.is_notified.load(Ordering::Acquire) {
            self.sender.send(action);
        } else {
            self.notify.send(Message::Action(self.token, action)).unwrap();
            self.is_notified.store(true, Ordering::Release);
        }
    }
    // fn abort()
    // fn pause()
}

impl fmt::Debug for Transfer {
    fn fmt(&self, f: &mut fmt::Foramtter) -> fmt::Result {
        f.debug_struct("Transfer")
            .field("token", &self.token)
            .finish()
    }
}

impl Drop for Transfer {
    fn drop(&mut self) {
        self.close();
    }
}
