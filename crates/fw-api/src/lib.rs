#![no_std]

//! The contract shared by firmware and a separately compiled plugin.
//!
//! This crate owns the shared type definitions, not the firmware service state.
//! All function pointers use the Rust ABI. This is NOT a version-stable ABI.
//!
//! The following compile-fail examples exercise the contract across function
//! pointers. Run them with `cargo test --doc -p fw-api`.
//!
//! A borrowed slice cannot escape as a `'static` reference:
//! ```compile_fail
//! use fw_api::{Buffer, FirmwareApi};
//! fn escape(api: &FirmwareApi, buffer: &Buffer) -> &'static [u8] {
//!     (api.bytes)(buffer)
//! }
//! ```
//!
//! A submitted buffer is no longer owned by the caller:
//! ```compile_fail
//! use fw_api::{Buffer, FirmwareApi};
//! fn reuse(api: &FirmwareApi, buffer: Buffer) -> usize {
//!     (api.submit)(buffer);
//!     (api.bytes)(&buffer).len()
//! }
//! ```
//!
//! A buffer cannot be moved while its borrowed slice is still used:
//! ```compile_fail
//! use fw_api::{Buffer, FirmwareApi};
//! fn move_while_borrowed(api: &FirmwareApi, buffer: Buffer) -> usize {
//!     let data = (api.bytes)(&buffer);
//!     (api.submit)(buffer);
//!     data.len()
//! }
//! ```
//!
//! A provider accepting only `'static` references cannot implement `bytes`:
//! ```compile_fail
//! use fw_api::{Buffer, FirmwareApi};
//! fn only_static(buffer: &'static Buffer) -> &'static [u8] { &buffer.data }
//! fn invalid_provider(mut api: FirmwareApi) {
//!     api.bytes = only_static;
//! }
//! ```

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
pub struct PluginReport {
    pub length: usize,
    pub first_byte: u8,
    pub receipt: usize,
}

/// The loader must establish that the resolved entry address has this type.
pub type PluginEntry = for<'fw> fn(&'fw FirmwareApi) -> PluginReport;
