//! Example: Security and compliance features

use ironvault::compliance::ComplianceChecker;
use ironvault::{Vault, VaultConfig};

fn main() -> ironvault::Result<()> {
    println!("=== IronVault Security & Compliance Example ===\n");

    // 1. Run compliance checks
    println!("1. Running compliance checks...\n");
    let checker = ComplianceChecker::new();
    let status = checker.run_all_checks()?;

    println!("Compliance Status:");
    println!(
        "  FIPS 140-3: {}",
        if status.fips_140_3 {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );
    println!(
        "  CVE Scan: {}",
        if status.cve_scan_passed {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );
    println!(
        "  MITRE ATT&CK: {}",
        if status.mitre_attack_aligned {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );
    println!("  CMMC Level: {}", status.cmmc_level);

    if !status.violations.is_empty() {
        println!("\n  Violations:");
        for violation in &status.violations {
            println!(
                "    [{:?}] {} - {}",
                violation.severity, violation.standard, violation.control
            );
            println!("      {}", violation.description);
        }
    } else {
        println!("\n  ✓ No violations detected");
    }

    // 2. Demonstrate audit logging
    println!("\n2. Demonstrating audit logging...\n");
    let config = VaultConfig::new()?;
    let mut vault = Vault::new(Some(config))?;

    let passphrase = b"secure_passphrase_for_demo";
    vault.unlock(passphrase.to_vec())?;

    println!(
        "  Audit log location: {:?}",
        vault.get_config().dirs.log_dir.join("audit.log")
    );

    // 3. Demonstrate encryption strength
    println!("\n3. Cryptographic parameters:\n");
    println!("  Encryption: AES-256-GCM");
    println!("    - Key size: 256 bits");
    println!("    - Nonce size: 96 bits");
    println!("    - Tag size: 128 bits");
    println!("    - FIPS 197 compliant: Yes");
    println!();
    println!("  Key Derivation: Argon2id");
    println!("    - Memory cost: 64 MB");
    println!("    - Time cost: 3 iterations");
    println!("    - Parallelism: 4 lanes");
    println!("    - Output: 256 bits");
    println!();
    println!("  Hashing: SHA-256");
    println!("    - Output: 256 bits");
    println!("    - FIPS 180-4 compliant: Yes");

    // 4. Security features
    println!("\n4. Security features enabled:\n");
    println!("  ✓ Encryption at rest (AES-256-GCM)");
    println!("  ✓ Authenticated encryption (prevents tampering)");
    println!("  ✓ Secure key derivation (Argon2id)");
    println!("  ✓ Integrity verification (SHA-256 checksums)");
    println!("  ✓ Audit logging (all operations logged)");
    println!("  ✓ Version control (complete history)");
    println!("  ✓ Compression (transparent, lossless)");
    println!("  ✓ XDG compliance (proper directory structure)");

    // 5. Compliance frameworks
    println!("\n5. Compliance framework coverage:\n");
    println!("  FIPS 140-3:");
    println!("    ✓ Approved cryptographic algorithms");
    println!("    ✓ Key management");
    println!("    ✓ Self-tests capability");
    println!();
    println!("  CMMC 2.0 Level 2 Controls:");
    println!("    ✓ AC (Access Control)");
    println!("    ✓ AU (Audit and Accountability)");
    println!("    ✓ IA (Identification and Authentication)");
    println!("    ✓ SC (System and Communications Protection)");
    println!();
    println!("  MITRE ATT&CK Mitigations:");
    println!("    ✓ T1552 - Unsecured Credentials");
    println!("    ✓ T1486 - Data Encrypted for Impact");
    println!("    ✓ T1078 - Valid Accounts");
    println!("    ✓ T1005 - Data from Local System");

    println!("\n=== Security demonstration completed ===");

    Ok(())
}
