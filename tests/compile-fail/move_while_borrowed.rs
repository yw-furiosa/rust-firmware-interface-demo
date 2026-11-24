#![no_std]

use fw_api::{Buffer, FirmwareApi};

pub fn move_while_borrowed(api: &FirmwareApi, buffer: Buffer) -> usize {
    let data = (api.bytes)(&buffer);
    (api.submit)(buffer);
    // The later use of data keeps the borrow alive across the attempted move.
    data.len()
}
