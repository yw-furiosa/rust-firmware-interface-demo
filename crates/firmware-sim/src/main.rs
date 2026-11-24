//! Host simulation: separate crates, statically linked, no ELF loading.

use fw_api::{Buffer, FirmwareApi, PluginEntry, PluginReport};
use std::sync::atomic::{AtomicUsize, Ordering};

static CREATED: AtomicUsize = AtomicUsize::new(0);
static SUBMITTED: AtomicUsize = AtomicUsize::new(0);

fn new_buffer_impl(value: u8) -> Buffer {
    CREATED.fetch_add(1, Ordering::Relaxed);
    Buffer { data: [value; 16] }
}

fn bytes_impl(buffer: &Buffer) -> &[u8] {
    &buffer.data
}

fn submit_impl(buffer: Buffer) -> usize {
    assert_eq!(buffer.data, [7; 16]);
    // Firmware owns buffer here. This example lets it drop on return.
    SUBMITTED.fetch_add(1, Ordering::Relaxed) + 1
}

fn main() {
    let api = FirmwareApi {
        new_buffer: new_buffer_impl,
        bytes: bytes_impl,
        submit: submit_impl,
    };

    // An actual loader would resolve an address and validate the contract.
    // Here Rust checks the conversion directly; no pointer cast is needed.
    let entry: PluginEntry = plugin::plugin_entry;
    let report = entry(&api);

    assert_eq!(
        report,
        PluginReport {
            length: 16,
            first_byte: 7,
            receipt: 1,
        }
    );
    assert_eq!(CREATED.load(Ordering::Relaxed), 1);
    assert_eq!(SUBMITTED.load(Ordering::Relaxed), 1);

    println!("PASS: firmware -> plugin -> firmware ownership transfer");
    println!("PASS: borrowed slice returned through a Rust function pointer");
    println!("{report:?}");
    println!("Scope: host simulation; the plugin is statically linked.");
}
