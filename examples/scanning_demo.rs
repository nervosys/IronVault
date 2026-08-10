//! Example: Pickle safety scanning for PyTorch models
//!
//! Run with: cargo run --example scanning_demo

use ironvault::scanning::PickleScanner;
use std::path::Path;

fn main() -> ironvault::Result<()> {
    println!("=== IronVault Scanning Example ===\n");

    // 1. Scan bytes in memory (simulate a pickle file)
    println!("1. Scanning in-memory data...");
    let safe_data = b"safe binary data without pickle opcodes";
    let report = PickleScanner::scan_bytes(safe_data, "safe_model.bin");
    println!("   File: {}", report.file_path);
    println!("   Is pickle: {}", report.is_pickle_format);
    println!("   Safe: {}", report.safe);
    println!("   Findings: {}\n", report.findings.len());

    // 2. Scan with suspicious patterns
    println!("2. Scanning data with suspicious patterns...");
    let suspicious = b"some data with os.system and subprocess.call patterns";
    let report = PickleScanner::scan_bytes(suspicious, "suspicious_model.pt");
    println!("   Safe: {}", report.safe);
    println!("   Findings:");
    for finding in &report.findings {
        println!(
            "     [{:?}] {} (count: {})",
            finding.severity, finding.description, finding.count
        );
    }
    println!("   Recommendation: {}", report.recommendation);
    println!();

    // 3. Scan a file on disk (if it exists)
    println!("3. Scanning file on disk...");
    let test_path = Path::new("example_model.pt");
    if test_path.exists() {
        let report = PickleScanner::scan(test_path)?;
        println!("   File: {}", report.file_path);
        println!("   Size: {} bytes", report.file_size);
        println!("   ZIP archive: {}", report.is_zip_archive);
        println!("   Safe: {}", report.safe);
    } else {
        println!("   (No test file found — skipping disk scan)");
    }
    println!();

    // 4. Show severity levels
    println!("4. Severity levels:");
    println!("   Info     — Informational findings");
    println!("   Warning  — Potentially risky patterns (BUILD opcode)");
    println!("   Critical — Dangerous opcodes (REDUCE, GLOBAL, INST)\n");

    // 5. CI/CD usage
    println!("5. CI/CD integration:");
    println!("   iv scan --file model.pt --format json | jq -e '.safe == true'\n");

    println!("=== Scanning example complete ===");
    Ok(())
}
