//! Example: License scanning and SPDX normalization
//!
//! Run with: cargo run --example license_scan_demo

use ironvault::license_scan::LicenseScanner;

fn main() -> ironvault::Result<()> {
    println!("=== IronVault License Scanning Example ===\n");

    // 1. Scan a directory
    println!("1. Scanning a directory...");
    let test_dir = std::env::temp_dir().join("aim_license_demo");
    std::fs::create_dir_all(&test_dir)?;

    // Create a mock README.md with YAML frontmatter
    std::fs::write(
        test_dir.join("README.md"),
        "---\nlicense: apache-2.0\n---\n# My Model\nA fine-tuned model.\n",
    )?;

    let report = LicenseScanner::scan_directory(&test_dir)?;
    println!("{}", report.display());
    println!("   Has license: {}", report.has_license);
    for lic in &report.licenses {
        println!(
            "   Found: {:?} ({:?}) from {:?}",
            lic.spdx_id, lic.classification, lic.source
        );
    }
    println!();

    // 2. Scan a single file
    println!("2. Scanning a LICENSE file...");
    let license_file = test_dir.join("LICENSE");
    std::fs::write(&license_file, "MIT License\n\nCopyright (c) 2024\n")?;
    let report = LicenseScanner::scan_file(&license_file)?;
    println!("   Has license: {}", report.has_license);
    for lic in &report.licenses {
        println!("   Found: {:?} ({:?})", lic.spdx_id, lic.classification);
    }
    println!();

    // 3. Scan bytes in memory
    println!("3. In-memory scanning...");
    let data = b"---\nlicense: gpl-3.0\n---\nSome model card content";
    let report = LicenseScanner::scan_bytes(data, "README.md");
    for lic in &report.licenses {
        println!("   Found: {:?} ({:?})", lic.spdx_id, lic.classification);
    }
    println!();

    // 4. License classifications
    println!("4. License classes:");
    println!("   Permissive  — MIT, Apache-2.0, BSD-2/3-Clause");
    println!("   Copyleft    — GPL-2.0, GPL-3.0, AGPL-3.0, LGPL");
    println!("   Restricted  — CC-BY-NC-4.0, Llama-2, Llama-3");
    println!("   Proprietary — Custom/proprietary terms");
    println!("   Unknown     — Unrecognized identifiers\n");

    // 5. CLI usage
    println!("5. CLI commands:");
    println!("   iv license-scan ./my-model/");
    println!("   iv license-scan model.gguf --format json\n");

    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);

    println!("=== License scanning example complete ===");
    Ok(())
}
