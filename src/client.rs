use std::time::Duration;
use plist::Plist;

use Stream;
use Result;

/// A Client for usbmuxd.
pub struct Client {
    stream: Stream,
}

impl Client {
    /// Tries to create a new instance of the `Client`.
    pub fn new() -> Result<Self> {
        let mut stream = try!(Stream::connect());
        try!(stream.set_send_tymeout(Some(Duration::new(1, 0))));
        try!(stream.set_receive_timeout(Some(Duration::new(1, 0))));
        Ok(Client {
            stream: stream,
        })
    }

    /// Sends a request and receives a response.
    pub fn request(&mut self, message: Plist) -> Result<Plist> {
        try!(self.stream.send(message));
        Ok(try!(self.stream.receive()))
    }
}