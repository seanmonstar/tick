use std::sync::mpsc;
use mio;

use ::{Action, Message};
pub type Action_ = Action;
pub type Message_ = Message;

pub struct Transfer {
    token: mio::Token,
    notify: mio::Sender<Message>,
    sender: mpsc::Sender<Action>,
}

pub fn new(token: mio::Token, notify: mio::Sender<Message_>, sender: mpsc::Sender<Action_>) -> Transfer {
    Transfer {
        token: token,
        notify: notify,
        sender: sender,
    }
}

impl Transfer {

    pub fn write(&mut self, data: &[u8]) {
        self.notify.send(
            Message::Action(self.token, Action::Write(Some(data.to_vec())))
        ).unwrap();
    }

    pub fn resume(&mut self) {
        self.notify.send(
            Message::Action(self.token, Action::Register(mio::EventSet::readable()))
        ).unwrap();
    }

    pub fn close(&mut self) {
        //TODO: consume self?
        self.notify.send(
            Message::Action(self.token, Action::Write(None))
        ).unwrap();
    }

    // fn pause()
    // fn abort()
}
