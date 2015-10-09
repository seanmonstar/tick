pub trait Protocol {
    fn on_data(&mut self, data: &[u8]) {
        trace!("ignored on_data({:?})", data);
    }

    fn on_eof(&mut self) {
        trace!("ignored on_eof");
    }

    fn on_pause(&mut self) {
        trace!("ignored on_pause");
    }

    fn on_resume(&mut self) {
        trace!("ignored on_resume");
    }

    fn on_end(&mut self, err: Option<::Error>) {
        trace!("ignored on_end({:?})", err);
    }
}

pub trait Factory {
    type Proto: Protocol;
    fn create(&mut self, ::Transfer, ::Id) -> Self::Proto;
}

impl<F, P> Factory for F where F: FnMut(::Transfer, ::Id) -> P, P: Protocol {
    type Proto = P;
    fn create(&mut self, transfer: ::Transfer, id: ::Id) -> P {
        self(transfer, id)
    }
}
