#![no_std]

//! Application logic; the only dependency is the shared API contract.
//! Firmware service implementations are reached through the supplied table.

use fw_api::{FirmwareApi, PluginEntry, PluginReport};

// SAFETY: This example reserves the symbol `plugin_entry` for this function.
// A real loader must check the SDK identity before calling it as PluginEntry.
// no_mangle controls the symbol name; it does not select the C ABI.
#[unsafe(no_mangle)]
pub fn plugin_entry(api: &FirmwareApi) -> PluginReport {
    // Firmware transfers an owned Buffer to the plugin.
    let buffer = (api.new_buffer)(7);

    // The slice borrows buffer, even though the function is in firmware.
    let data = (api.bytes)(&buffer);
    let length = data.len();
    let first_byte = data.first().copied().unwrap_or(0);

    // The last use of data is above. Move buffer back to firmware.
    let receipt = (api.submit)(buffer);

    PluginReport {
        length,
        first_byte,
        receipt,
    }
}

// Catch a mismatch between the entry implementation and the shared contract.
const _: PluginEntry = plugin_entry;
