//! Host simulation: separate crates, statically linked, no ELF loading.

use fw_api::PluginEntry;

fn main() {
    let api = firmware_services::api();

    // An actual loader would resolve an address and validate the contract.
    // Here Rust checks the conversion directly; no pointer cast is needed.
    let entry: PluginEntry = plugin::plugin_entry;
    for expected_receipt in 1..=2 {
        let report = entry(&api);
        assert_eq!(report.length, 16);
        assert_eq!(report.first_byte, 7);
        assert_eq!(report.receipt, expected_receipt);
        println!(
            "PluginReport {{ length: {}, first_byte: {}, receipt: {} }}",
            report.length, report.first_byte, report.receipt
        );
    }
    assert_eq!(firmware_services::counters(), (2, 2, 112));

    println!("PASS: firmware -> plugin -> firmware ownership transfer");
    println!("PASS: borrowed slice returned through a Rust function pointer");
    println!("PASS: both invocations use the same firmware service state");
    println!("Scope: host simulation; the plugin is statically linked.");
}
