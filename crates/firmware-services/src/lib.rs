#![no_std]

//! Firmware-owned service implementations and state.
//!
//! Neither the plugin nor the standalone application depends on this crate.

use core::sync::atomic::{AtomicUsize, Ordering};
use fw_api::{Buffer, FirmwareApi};

static CREATED: AtomicUsize = AtomicUsize::new(0);
static SUBMITTED: AtomicUsize = AtomicUsize::new(0);
static LAST_CHECKSUM: AtomicUsize = AtomicUsize::new(0);

/// Construct the table that firmware passes to the application.
pub fn api() -> FirmwareApi {
    FirmwareApi {
        new_buffer: new_buffer_impl,
        bytes: bytes_impl,
        submit: submit_impl,
    }
}

// Keep these functions visible in the object code for the symbol inspection
// exercise. This is a demonstration choice, not a Rust ABI requirement.
#[inline(never)]
fn new_buffer_impl(value: u8) -> Buffer {
    CREATED.fetch_add(1, Ordering::Relaxed);
    Buffer { data: [value; 16] }
}

#[inline(never)]
fn bytes_impl(buffer: &Buffer) -> &[u8] {
    &buffer.data
}

#[inline(never)]
fn submit_impl(buffer: Buffer) -> usize {
    // Actual processing and service state live only in firmware.
    let checksum = buffer.data.iter().map(|byte| usize::from(*byte)).sum();
    LAST_CHECKSUM.store(checksum, Ordering::Relaxed);
    // Firmware owns buffer here; this example drops it on return.
    SUBMITTED.fetch_add(1, Ordering::Relaxed) + 1
}

/// Firmware-side inspection; deliberately not part of the shared API.
/// The demo calls this after synchronous service calls, not as a concurrent snapshot.
pub fn counters() -> (usize, usize, usize) {
    (
        CREATED.load(Ordering::Relaxed),
        SUBMITTED.load(Ordering::Relaxed),
        LAST_CHECKSUM.load(Ordering::Relaxed),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_refer_to_one_firmware_service_state() {
        let first = api();
        let second = api();
        let buffer = (first.new_buffer)(3);
        assert_eq!((first.bytes)(&buffer), &[3; 16]);
        assert_eq!((first.submit)(buffer), 1);

        let buffer = (second.new_buffer)(7);
        assert_eq!((second.submit)(buffer), 2);
        assert_eq!(counters(), (2, 2, 112));
    }
}
