use std::io;

use ::Evented;

pub trait Transport: Evented + io::Read + io::Write {}

impl<T> Transport for T where T: Evented + io::Read + io::Write {}

