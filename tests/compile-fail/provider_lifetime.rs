#![no_std]

use fw_api::{Buffer, FirmwareApi};

fn only_static(buffer: &'static Buffer) -> &'static [u8] {
    &buffer.data
}

pub fn invalid_provider(mut api: FirmwareApi) {
    // A function accepting only 'static cannot satisfy for<'a> fn(&'a Buffer).
    api.bytes = only_static;
}
