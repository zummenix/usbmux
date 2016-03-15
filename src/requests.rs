//! This module contains convenient functions that build `Plist` requests for usbmuxd.

use plist::Plist;
use std::collections::BTreeMap;

/// Creates a `Listen` request for usbmuxd.
pub fn listen() -> Plist {
    Plist::Dictionary(message_type("Listen"))
}

/// Creates a `ListDevices` request for usbmuxd.
pub fn list_devices() -> Plist {
    Plist::Dictionary(message_type("ListDevices"))
}

/// Creates a `ReadBUID` request for usbmuxd.
pub fn read_buid() -> Plist {
    Plist::Dictionary(message_type("ReadBUID"))
}

fn message_type(mtype: &str) -> BTreeMap<String, Plist> {
    let mut map = BTreeMap::new();
    map.insert("MessageType".to_owned(), Plist::String(mtype.to_owned()));
    map
}