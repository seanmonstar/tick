use std::time::Duration;

use mio::EventLoop;

use handler::LoopHandler;
use transport::Transport;
use ::{Message, Protocol};


pub struct Tick<T: Transport, F: Fn(::Transfer) -> P, P: Protocol> {
    handler: LoopHandler<F, P, T>,
    event_loop: EventLoop<LoopHandler<F, P, T>>
}

pub struct TickConfig {
    pub notify_queue_size: usize,
}

impl Default for TickConfig {
    fn default() -> TickConfig {
        TickConfig {
            notify_queue_size: 4096
        }
    }
}

impl<T: Transport, F: Fn(::Transfer) -> P, P: Protocol> Tick<T, F, P> {
    pub fn new(protocol_factory: F) -> Tick<T, F, P> {
        Tick::configured(protocol_factory, Default::default())
    }

    pub fn configured(factory: F, _config: TickConfig) -> Tick<T, F, P> {
        Tick {
            handler: LoopHandler::new(factory),
            event_loop: EventLoop::new().unwrap()
        }
    }

    pub fn notify(&self) -> Notify {
        Notify { sender: self.event_loop.channel() }
    }

    pub fn accept(&mut self, listener: T::Listener) -> ::Result<::Id> {
        self.handler.listener(&mut self.event_loop, listener).map(::Id)
    }

    pub fn stream(&mut self, transport: T) -> ::Result<::Id> {
        self.handler.stream(&mut self.event_loop, transport).map(::Id)
    }

    pub fn run_until_complete(&mut self, id: ::Id) -> ::Result<()> {
        while self.handler.transports.contains(id.0) {
            try!(self.event_loop.run_once(&mut self.handler));
        }
        Ok(())
    }

    pub fn run(&mut self) -> ::Result<()> {
        self.event_loop.run(&mut self.handler).map_err(From::from)
    }
}

#[derive(Clone)]
pub struct Notify {
    sender: ::mio::Sender<Message>
}

impl Notify {
    pub fn timeout<F: FnOnce() + Send + 'static>(&self, f: F, when: Duration) {
        let mut env = Some(f);
        let ms = when.as_secs() * 1_000 + (when.subsec_nanos() as u64) / 1_000_000;
        self.sender.send(Message::Timeout(Box::new(move || {
            env.take().map(|f| f());
        }), ms));
    }
    pub fn shutdown(&self) {
        self.sender.send(Message::Shutdown).unwrap();
    }
}
