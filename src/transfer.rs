use std::fmt;
use mio;

use ::{Message};
pub type Message_ = Message;

#[derive(Clone)]
pub struct Transfer {
    token: mio::Token,
    notify: mio::Sender<Message>,
}

pub fn new(token: mio::Token, notify: mio::Sender<Message_>) -> Transfer {
    Transfer {
        token: token,
        notify: notify,
    }
}

impl Transfer {
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
