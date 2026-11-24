#![no_std]

use fw_api::{Buffer, FirmwareApi};

pub fn reuse(api: &FirmwareApi, buffer: Buffer) -> usize {
    (api.submit)(buffer);
    // Firmware already owns buffer.
    (api.bytes)(&buffer).len()
}
