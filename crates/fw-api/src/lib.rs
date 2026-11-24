#![no_std]

//! The contract shared by firmware and a separately compiled plugin.
//!
//! This crate owns the shared type definitions, not the firmware service state.
//! All function pointers use the Rust ABI. This is NOT a version-stable ABI.

/// An owned, inline buffer. Deliberately implements neither `Copy` nor `Clone`.
///
/// Inline storage avoids introducing an allocator contract into this example.
pub struct Buffer {
    pub data: [u8; 16],
}

/// Services implemented by firmware and passed to the plugin at entry.
pub struct FirmwareApi {
    /// Returns an owned buffer to the plugin.
    pub new_buffer: fn(u8) -> Buffer,
    /// Borrows the buffer and returns a slice tied to that borrow.
    pub bytes: for<'a> fn(&'a Buffer) -> &'a [u8],
    /// Takes ownership of the buffer and returns a receipt.
    pub submit: fn(Buffer) -> usize,
}

/// A by-value result returned across the same Rust ABI boundary.
#[derive(Debug, PartialEq, Eq)]
pub struct PluginReport {
    pub length: usize,
    pub first_byte: u8,
    pub receipt: usize,
}

/// The loader must establish that the resolved entry address has this type.
pub type PluginEntry = for<'fw> fn(&'fw FirmwareApi) -> PluginReport;
