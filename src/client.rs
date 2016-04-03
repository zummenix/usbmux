use std::time::Duration;
use plist::Plist;

use Stream;
use Result;
use Error;
use message_type;

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

    /// Returns a list of connected devices.
    pub fn devices(&mut self) -> Result<Vec<Device>> {
        let mut plist = try!(self.request(Plist::Dictionary(message_type("ListDevices"))));
        let mut dict = try!(plist.as_dictionary_mut().ok_or(Error::UnexpectedFormat));
        match dict.remove("DeviceList") {
            Some(Plist::Array(array)) => {
                let results = array.into_iter().filter_map(|item| {
                    match item {
                        Plist::Dictionary(mut dict) => {
                            match dict.remove("Properties") {
                                Some(plist) => Device::from_plist(plist),
                                _ => None,
                            }
                        },
                        _ => None,
                    }
                }).collect();
                Ok(results)
            },
            _ => Err(Error::UnexpectedFormat),
        }
    }

    /// Sends a request and receives a response.
    pub fn request(&mut self, message: Plist) -> Result<Plist> {
        try!(self.stream.send(message));
        Ok(try!(self.stream.receive()))
    }
}

/// Represents a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub device_id: u32,
    pub product_id: u32,
    pub location_id: u32,
    pub serial_number: String,
}

impl Device {
    /// Creates an instance of `Device` from plist.
    pub fn from_plist(mut plist: Plist) -> Option<Device> {
        let mut dict = try_opt!(plist.as_dictionary_mut());
        Some(Device {
            device_id: try_opt!(dict.get("DeviceID").and_then(Plist::as_integer).map(|x| x as u32)),
            product_id: try_opt!(dict.get("ProductID").and_then(Plist::as_integer).map(|x| x as u32)),
            location_id: try_opt!(dict.get("LocationID").and_then(Plist::as_integer).map(|x| x as u32)),
            serial_number: try_opt!(dict.remove("SerialNumber").and_then(Plist::into_string)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expectest::prelude::*;
    use plist::Plist;
    use std::collections::BTreeMap;

    #[test]
    fn test_device_from_plist() {
        let mut map = BTreeMap::new();
        map.insert("ConnectionSpeed".to_owned(), Plist::Integer(480000000));
        map.insert("ConnectionType".to_owned(), Plist::String("USB".to_owned()));
        map.insert("DeviceID".to_owned(), Plist::Integer(3));
        map.insert("LocationID".to_owned(), Plist::Integer(336592896));
        map.insert("ProductID".to_owned(), Plist::Integer(4778));
        map.insert("SerialNumber".to_owned(),Plist::String("fffffffff".to_owned()));

        let device = Device {
            device_id: 3,
            product_id: 4778,
            location_id: 336592896,
            serial_number: "fffffffff".to_owned(),
        };

        expect!(Device::from_plist(Plist::Dictionary(map))).to(be_some().value(device));
    }
}