use std::fmt;
use mio;

use ::{Message};
pub type Message_ = Message;

#[derive(Clone)]
pub struct Transfer {
    token: mio::Token,
    notify: mio::Sender<Message>,
}

#[inline]
pub fn new(token: mio::Token, notify: mio::Sender<Message_>) -> Transfer {
    Transfer {
        token: token,
        notify: notify,
    }
}

impl Transfer {
    #[inline]
    pub fn interest(&self, interest: ::Interest) -> bool {
        self.notify.send(Message::Interest(self.token, interest)).is_ok()
    }

    //pub fn timeout(&self, )
}

impl fmt::Debug for Transfer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Transfer")
            .field("token", &self.token)
            .finish()
    }
}

