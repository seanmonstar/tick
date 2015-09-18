use ::Transfer;


pub trait Protocol {
    fn on_connection(transfer: Transfer) -> Self;

    fn on_data(&mut self, data: &[u8]) {
        trace!("ignored on_data({:?})", data);
    }
    fn on_eof(&mut self) {
        trace!("ignored on_eof");
    }
    fn on_end(&mut self, err: Option<::Error>) {
        trace!("ignored on_end({:?})", err);
    }
}
