#![no_std]

use fw_api::{Buffer, FirmwareApi};

pub fn escape(api: &FirmwareApi, buffer: &Buffer) -> &'static [u8] {
    // A borrow of buffer cannot be extended to 'static.
    (api.bytes)(buffer)
}
