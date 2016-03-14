//! This crate allows to communicate with usbmuxd (USB multiplexer daemon) which is used to
//! communicate with iOS devices.

extern crate unix_socket;
extern crate byteorder;
extern crate plist;
#[cfg(test)]
#[macro_use(expect)]
extern crate expectest;

use std::io;
use std::fmt;
use std::error;
use std::time::Duration;
use unix_socket::UnixStream;
use plist::Plist;

pub mod requests;

/// Represents connection to usbmuxd.
pub struct Stream {
    stream: UnixStream,
}

impl Stream {
    /// Tries to connect to usbmuxd.
    pub fn connect() -> io::Result<Self> {
        Ok(Stream {
            stream: try!(UnixStream::connect("/var/run/usbmuxd")),
        })
    }

    /// Tries to send `plist` data to usbmuxd.
    ///
    /// You should call `receive` or `receive_with_timeout` after this call
    /// to get a response from usbmuxd.
    pub fn send(&mut self, plist: Plist) -> Result<()> {
        send(&mut self.stream, plist)
    }

    /// Tries to receive `plist` data from usbmuxd.
    ///
    /// This method will block indefinitely waiting for data. If this behaviour
    /// is not desired you can use `receive_with_timeout` method.
    pub fn receive(&mut self) -> Result<Plist> {
        receive(&mut self.stream, None)
    }

    /// Tries to receive `plist` data from usbmuxd within `timeout` limit.
    pub fn receive_with_timeout(&mut self, timeout: Duration) -> Result<Plist> {
        receive(&mut self.stream, Some(timeout))
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
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
            Error::Plist(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref e) => Some(e),
            Error::Plist(ref e) => Some(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => e.fmt(f),
            Error::Plist(ref e) => e.fmt(f),
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

fn send(stream: &mut UnixStream, plist: Plist) -> Result<()> {
    use std::io::Write;

    let data = prepare_request_data(&plist_to_data(plist));
    Ok(try!(stream.write_all(&data)))
}

fn receive(stream: &mut UnixStream, timeout: Option<Duration>) -> Result<Plist> {
    use byteorder::{LittleEndian, ByteOrder};
    use std::io::{Read, Cursor};

    try!(stream.set_read_timeout(timeout));

    // Read header and get length of the data.
    // Don't bother to check version and message type. Deserialization
    // from plist will fail anyway if message will have wrong format.
    let mut header = [0; 16];
    try!(stream.read_exact(&mut header));
    let length = LittleEndian::read_u32(&header) as usize - header.len();

    let mut data = vec![0; length];
    try!(stream.read_exact(&mut data));

    Ok(try!(Plist::read(Cursor::new(data))))
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
    use super::prepare_request_data;
    use expectest::prelude::*;

    #[test]
    fn test_prepare_data() {
        expect!(prepare_request_data(&[1, 2, 3, 4]).iter()).to(have_count(20));
    }
}
