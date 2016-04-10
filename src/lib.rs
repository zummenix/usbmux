//! This crate allows to communicate with usbmuxd (USB multiplexer daemon) which is used to
//! communicate with iOS devices.

extern crate unix_socket;
extern crate byteorder;
extern crate plist;
#[cfg(test)]
#[macro_use(expect)]
extern crate expectest;
#[macro_use(try_opt)]
extern crate try_opt;

use std::io;
use std::fmt;
use std::error;
use std::time::Duration;
use std::collections::BTreeMap;
use unix_socket::UnixStream;
use plist::Plist;

mod client;
pub use client::Client;
pub use client::Device;

/// Represents connection to usbmuxd.
pub struct Stream {
    inner: UnixStream,
}

impl Stream {
    /// Tries to connect to usbmuxd.
    pub fn connect() -> Result<Self> {
        Ok(Stream {
            inner: try!(UnixStream::connect("/var/run/usbmuxd")),
        })
    }

    /// Sets the send timeout for the stream.
    ///
    /// If the provided value is `None`, then `send` calls will block indefinitely.
    /// It is an error to pass the zero `Duration` to this method.
    pub fn set_send_tymeout(&mut self, timeout: Option<Duration>) -> Result<()> {
        Ok(try!(self.inner.set_write_timeout(timeout)))
    }

    /// Sets the receive timeout for the stream.
    ///
    /// If the provided value is `None`, then `receive` calls will block indefinitely.
    /// It is an error to pass the zero `Duration` to this method.
    pub fn set_receive_timeout(&mut self, timeout: Option<Duration>) -> Result<()> {
        Ok(try!(self.inner.set_read_timeout(timeout)))
    }

    /// Tries to send a `plist` message to usbmuxd.
    ///
    /// After this call you should call `receive` method to get a response from usbmuxd.
    pub fn send(&mut self, plist: Plist) -> Result<()> {
        send(&mut self.inner, plist)
    }

    /// Tries to receive a message from usbmuxd.
    ///
    /// Typically this method should be called after `send` method.
    pub fn receive(&mut self) -> Result<Plist> {
        receive(&mut self.inner)
    }

    /// Sends a request and receives a response.
    pub fn request(&mut self, message: Plist) -> Result<Plist> {
        try!(self.send(message));
        Ok(try!(self.receive()))
    }
}

/// A Result type alias.
pub type Result<T> = ::std::result::Result<T, Error>;

/// An Error type.
#[derive(Debug)]
pub enum Error {
    /// Denotes I/O error.
    Io(io::Error),
    /// Denotes error that produces plist crate.
    Plist(plist::Error),
    /// Denotes error when response has unexpected format.
    UnexpectedFormat,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
            Error::Plist(ref e) => e.description(),
            Error::UnexpectedFormat => "unexpected format",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref e) => Some(e),
            Error::Plist(ref e) => Some(e),
            Error::UnexpectedFormat => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => e.fmt(f),
            Error::Plist(ref e) => e.fmt(f),
            Error::UnexpectedFormat => writeln!(f, "unexpected format"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<plist::Error> for Error {
    fn from(e: plist::Error) -> Self {
        Error::Plist(e)
    }
}

/// Convenient function that creates a map with `MessageType` key/value pair.
pub fn message_type(mtype: &str) -> BTreeMap<String, Plist> {
    let mut map = BTreeMap::new();
    map.insert("MessageType".to_owned(), Plist::String(mtype.to_owned()));
    map
}

fn send<W>(stream: &mut W, plist: Plist) -> Result<()> where W: io::Write {
    let data = prepare_request_data(&plist_to_data(plist));
    Ok(try!(stream.write_all(&data)))
}

fn receive<R>(stream: &mut R) -> Result<Plist> where R: io::Read {
    use byteorder::{LittleEndian, ByteOrder};

    // Read header and get length of the data.
    // Don't bother to check version and message type. Deserialization
    // from plist will fail anyway if message will have wrong format.
    let mut header = [0; 16];
    try!(stream.read_exact(&mut header));
    let length = LittleEndian::read_u32(&header) as usize - header.len();

    let mut data = vec![0; length];
    try!(stream.read_exact(&mut data));

    Ok(try!(Plist::read(io::Cursor::new(data))))
}

/// Converts the `plist` to the raw xml data.
fn plist_to_data(plist: Plist) -> Vec<u8> {
    use plist::xml::EventWriter;
    let mut buffer = Vec::new();
    {
        let mut writer = EventWriter::new(&mut buffer);
        for event in plist.into_events() {
            writer.write(&event).unwrap();
        }
    }
    buffer
}

/// Prepares request data for usbmuxd by adding a header info.
fn prepare_request_data(data: &[u8]) -> Vec<u8> {
    use byteorder::{WriteBytesExt, LittleEndian};
    use std::io::{Write, Cursor};

    let mut cursor = Cursor::new(Vec::new());
    cursor.write_u32::<LittleEndian>(data.len() as u32 + 16).unwrap(); // total length
    cursor.write_u32::<LittleEndian>(1).unwrap(); // version
    cursor.write_u32::<LittleEndian>(8).unwrap(); // message type (plist)
    cursor.write_u32::<LittleEndian>(1).unwrap(); // tag
    cursor.write_all(data).unwrap();
    cursor.into_inner()
}

#[cfg(test)]
mod tests {
    use super::{prepare_request_data, message_type};
    use std::io;
    use expectest::prelude::*;
    use plist::Plist;

    #[test]
    fn test_prepare_data() {
        expect!(prepare_request_data(&[1, 2, 3, 4]).iter()).to(have_count(20));
    }

    #[test]
    fn test_send_receive_message() {
        let message = Plist::Dictionary(message_type("Listen"));
        let mut buffer = Vec::new();
        expect!(super::send(&mut buffer, message.clone())).to(be_ok());
        let mut cursor = io::Cursor::new(buffer);
        expect!(super::receive(&mut cursor)).to(be_ok().value(message));
    }
}
