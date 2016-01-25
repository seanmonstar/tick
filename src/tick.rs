//use std::time::Duration;

use mio::{EventLoop, Evented, EventLoopConfig, TryAccept};

use handler::LoopHandler;
use internal::Message;
use transport::Transport;
use ::ProtocolFactory;


pub struct Tick<T: TryAccept + Evented, F: ProtocolFactory<T::Output>> where <T as TryAccept>::Output: Transport {
    handler: LoopHandler<F, T>,
    event_loop: EventLoop<LoopHandler<F, T>>
}

pub struct TickConfig {
    transports_capacity: usize,
    notify_capacity: usize,
}

impl TickConfig {
    pub fn new() -> TickConfig {
        TickConfig {
            transports_capacity: 8_192,
            notify_capacity: 8_192,
        }
    }
}

impl<T: TryAccept + Evented, F: ProtocolFactory<T::Output>> Tick<T, F> where <T as TryAccept>::Output: Transport {
    pub fn new(protocol_factory: F) -> Tick<T, F> {
        Tick::configured(protocol_factory, TickConfig::new())
    }

    pub fn configured(factory: F, config: TickConfig) -> Tick<T, F> {
        let mut loop_config = EventLoopConfig::new();
        loop_config.notify_capacity(config.notify_capacity);
        Tick {
            handler: LoopHandler::new(factory, config.transports_capacity),
            event_loop: EventLoop::configured(loop_config).unwrap()
        }
    }

    pub fn notify(&self) -> Notify {
        Notify { sender: self.event_loop.channel() }
    }

    pub fn accept(&mut self, listener: T) -> ::Result<::Id> {
        self.handler.listener(&mut self.event_loop, listener).map(::Id)
    }

    pub fn stream(&mut self, transport: T::Output) -> ::Result<::Id> {
        self.handler.stream(&mut self.event_loop, transport).map(::Id)
    }

    pub fn run_until_complete(&mut self, id: ::Id) -> ::Result<()> {
        while self.handler.transports.contains(id.0) {
            try!(self.event_loop.run_once(&mut self.handler, None));
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
    /*
    pub fn timeout<F: FnOnce() + Send + 'static>(&self, f: F, when: Duration) {
        let mut env = Some(f);
        let ms = when.as_secs() * 1_000 + (when.subsec_nanos() as u64) / 1_000_000;
        self.sender.send(Message::Timeout(Box::new(move || {
            env.take().map(|f| f());
        }), ms));
    }
    */
    pub fn shutdown(&self) {
        self.sender.send(Message::Shutdown).unwrap();
    }
}
