//! Compliance checking and security validation
//!
//! Implements checks for:
//! - FIPS 140-3
//! - CVE scanning
//! - MITRE ATT&CK framework
//! - CMMC 2.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Result;

/// What a compliance check actually established.
///
/// Most of these checks cannot be *verified* by a program inspecting itself.
/// Reporting all of them as "PASS" made `iv compliance` look like an
/// assessment when it was mostly a set of constants — an organisation putting
/// it in a CI gate for CMMC evidence would collect a green result no matter
/// what state the system was in. This distinguishes the three cases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CheckOutcome {
    /// Checked at runtime, and it held.
    Verified { detail: String },
    /// A property of how the software is built, established by design review
    /// rather than by this program. Not evidence of certification.
    AssertedByDesign { detail: String },
    /// The check could not be run, so nothing is known either way. This is
    /// *not* a pass.
    NotVerified { reason: String },
    /// Checked at runtime, and it failed.
    Failed { detail: String },
}

impl CheckOutcome {
    /// Whether this outcome should block a compliance gate.
    #[must_use]
    pub fn is_blocking(&self) -> bool {
        matches!(self, CheckOutcome::Failed { .. })
    }

    /// Short label for terminal output.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            CheckOutcome::Verified { .. } => "✓ VERIFIED",
            CheckOutcome::AssertedByDesign { .. } => "• BY DESIGN (not verified at runtime)",
            CheckOutcome::NotVerified { .. } => "? NOT VERIFIED",
            CheckOutcome::Failed { .. } => "✗ FAILED",
        }
    }

    /// The explanatory text carried by this outcome.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            CheckOutcome::Verified { detail }
            | CheckOutcome::AssertedByDesign { detail }
            | CheckOutcome::Failed { detail } => detail,
            CheckOutcome::NotVerified { reason } => reason,
        }
    }
}

/// Compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub fips_140_3: bool,
    pub cve_scan_passed: bool,
    pub mitre_attack_aligned: bool,
    pub cmmc_level: u8,
    pub violations: Vec<ComplianceViolation>,
    /// Per-check outcomes, in the order they are reported.
    ///
    /// The booleans above are kept for API compatibility, but they cannot
    /// express "not verified" — read this instead when the distinction
    /// matters, which for an audit it always does.
    #[serde(default)]
    pub outcomes: Vec<(String, CheckOutcome)>,
}

/// Compliance violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub standard: String,
    pub control: String,
    pub severity: ViolationSeverity,
    pub description: String,
    pub remediation: Option<String>,
}

/// Violation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Compliance checker
pub struct ComplianceChecker {
    enabled_checks: HashMap<String, bool>,
}

impl ComplianceChecker {
    /// Create new compliance checker
    pub fn new() -> Self {
        let mut enabled_checks = HashMap::new();
        enabled_checks.insert("fips_140_3".to_string(), true);
        enabled_checks.insert("cve".to_string(), true);
        enabled_checks.insert("mitre_attack".to_string(), true);
        enabled_checks.insert("cmmc".to_string(), true);

        Self { enabled_checks }
    }

    /// Report the vault's relationship to FIPS 140-3.
    ///
    /// **This is not a compliance determination, and cannot be one.** FIPS
    /// 140-3 certifies a *cryptographic module* through NIST's CMVP, which
    /// issues a certificate number. This crate uses the RustCrypto
    /// implementations (`aes-gcm`, `sha2`, `argon2`), none of which hold a
    /// CMVP certificate, so no configuration of this software is FIPS 140-3
    /// validated.
    ///
    /// What is true:
    /// - AES-256-GCM is a FIPS-approved algorithm (FIPS 197, SP 800-38D)
    /// - SHA-256 is a FIPS-approved algorithm (FIPS 180-4)
    /// - **Argon2id is not FIPS-approved.** SP 800-132 approves PBKDF2 for
    ///   password-based key derivation; Argon2 is not on the approved list.
    ///   It is the better choice against modern cracking hardware, which is
    ///   why it is used — but it means the KDF is outside FIPS regardless of
    ///   the module question.
    ///
    /// Deployments with a genuine FIPS obligation need a validated module
    /// (AWS-LC-FIPS, BoringCrypto, or an HSM) and a FIPS-approved KDF.
    pub fn check_fips_140_3(&self) -> bool {
        if !self.is_check_enabled("fips_140_3") {
            return true;
        }
        // We use AES-256-GCM via the aes-gcm crate, Argon2id via argon2 crate,
        // and SHA-256 via sha2 crate. These are FIPS-approved algorithm choices.
        // The underlying implementations are NOT FIPS-validated (would require
        // an HSM or a CMVP-validated module such as AWS-LC or BoringCrypto).
        true
    }

    /// Check for known CVEs in dependencies
    ///
    /// Runs `cargo audit` if available, otherwise reports the limitation.
    /// Returns (passed, list_of_cves).
    pub fn check_cve(&self) -> (bool, Vec<String>) {
        if !self.is_check_enabled("cve") {
            return (true, Vec::new());
        }

        // Only shell out inside a Cargo project. `cargo audit` audits the
        // manifest in the working directory, which for an installed binary is
        // whatever the user happened to `cd` into — meaningless as a statement
        // about this vault, and it spawned a `cargo` resolved from PATH (and,
        // on Windows, from the current directory) in a location the user may
        // not control. No manifest, no subprocess.
        if !std::path::Path::new("Cargo.toml").exists() {
            return (
                false,
                vec![
                    "no Cargo.toml in the working directory, so there was nothing to \
                     audit; dependencies were not scanned. Note that `cargo audit` \
                     inspects the current directory's project, not the installed \
                     binary's own dependency tree — that is fixed at build time"
                        .to_string(),
                ],
            );
        }

        // Attempt to run cargo-audit for real CVE scanning
        match std::process::Command::new("cargo")
            .args(["audit", "--json"])
            .output()
        {
            Ok(output) if output.status.success() => {
                // Parse the JSON output for vulnerabilities
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(vulns) = json.get("vulnerabilities").and_then(|v| v.get("found")) {
                        if vulns.as_u64().unwrap_or(0) > 0 {
                            let mut cve_list = Vec::new();
                            if let Some(list) = json
                                .get("vulnerabilities")
                                .and_then(|v| v.get("list"))
                                .and_then(|v| v.as_array())
                            {
                                for item in list {
                                    if let Some(advisory) = item.get("advisory") {
                                        let id = advisory
                                            .get("id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        let pkg = advisory
                                            .get("package")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown");
                                        cve_list.push(format!("{} ({})", id, pkg));
                                    }
                                }
                            }
                            return (false, cve_list);
                        }
                    }
                }
                (true, Vec::new())
            }
            _ => {
                // cargo-audit could not run, so nothing is known about the
                // dependency tree. This used to return `true` — a pass for a
                // scan that never happened, and the common case for an
                // installed binary, where there is no Cargo.toml in the
                // working directory to audit.
                (
                    false,
                    vec![
                        "cargo-audit could not be run, so dependencies were not scanned; \
                         install with: cargo install cargo-audit"
                            .to_string(),
                    ],
                )
            }
        }
    }

    /// Whether `check_cve` was able to scan at all.
    ///
    /// Separates "scanned, found nothing" from "could not scan", which the
    /// `(bool, Vec<String>)` return cannot express on its own.
    fn cve_outcome(&self) -> CheckOutcome {
        if !self.is_check_enabled("cve") {
            return CheckOutcome::NotVerified {
                reason: "CVE checking is disabled in this configuration".to_string(),
            };
        }
        match self.check_cve() {
            (true, _) => CheckOutcome::Verified {
                detail: "cargo-audit reported no known vulnerabilities in the dependency tree"
                    .to_string(),
            },
            (false, findings)
                if findings
                    .iter()
                    .any(|f| f.contains("could not be run") || f.contains("nothing to audit")) =>
            {
                CheckOutcome::NotVerified {
                    reason: findings.join("; "),
                }
            }
            (false, findings) => CheckOutcome::Failed {
                detail: findings.join("; "),
            },
        }
    }

    /// Verify MITRE ATT&CK framework alignment
    ///
    /// Checks architectural mitigations for relevant techniques:
    /// - T1552: Unsecured Credentials → passphrase-derived keys, zeroization
    /// - T1486: Data Encrypted for Impact → versioning, backups
    /// - T1078: Valid Accounts → passphrase auth required for vault access
    /// - T1005: Data from Local System → AES-256-GCM encryption at rest
    pub fn check_mitre_attack(&self) -> bool {
        if !self.is_check_enabled("mitre_attack") {
            return true;
        }
        // This is a design-level assessment, not a runtime pentest.
        true
    }

    /// Check CMMC 2.0 compliance level
    ///
    /// Returns the CMMC level for which controls are implemented:
    /// Level 2 controls covered:
    /// - AC (Access Control): passphrase-gated vault
    /// - AU (Audit): audit logging
    /// - IA (Identification and Authentication): Argon2id KDF
    /// - SC (System and Communications Protection): AES-256-GCM
    pub fn check_cmmc(&self) -> u8 {
        if !self.is_check_enabled("cmmc") {
            return 0;
        }
        2
    }

    /// Check if a specific compliance check is enabled
    pub fn is_check_enabled(&self, check_name: &str) -> bool {
        *self.enabled_checks.get(check_name).unwrap_or(&false)
    }

    /// Enable or disable a specific compliance check
    pub fn set_check_enabled(&mut self, check_name: &str, enabled: bool) {
        self.enabled_checks.insert(check_name.to_string(), enabled);
    }

    /// Run all compliance checks
    pub fn run_all_checks(&self) -> Result<ComplianceStatus> {
        let mut violations = Vec::new();

        let fips = self.check_fips_140_3();
        if !fips {
            violations.push(ComplianceViolation {
                standard: "FIPS 140-3".to_string(),
                control: "Cryptographic Module".to_string(),
                severity: ViolationSeverity::Critical,
                description: "Non-FIPS approved cryptographic algorithms detected".to_string(),
                remediation: Some("Use FIPS 140-3 validated cryptographic module".to_string()),
            });
        }

        let (cve_passed, cves) = self.check_cve();
        if !cve_passed {
            for cve in cves {
                violations.push(ComplianceViolation {
                    standard: "CVE".to_string(),
                    control: "Vulnerability Management".to_string(),
                    severity: ViolationSeverity::High,
                    description: format!("Known vulnerability detected: {}", cve),
                    remediation: Some("Update affected dependencies".to_string()),
                });
            }
        }

        let cve_outcome = self.cve_outcome();
        let cmmc_level = self.check_cmmc();

        let outcomes = vec![
            (
                "FIPS 140-3".to_string(),
                CheckOutcome::AssertedByDesign {
                    detail: "AES-256-GCM and SHA-256 are FIPS-approved algorithms, but the \
                             implementations are not CMVP-validated and Argon2id is not a \
                             FIPS-approved KDF. This software is not FIPS 140-3 validated."
                        .to_string(),
                },
            ),
            ("CVE scan".to_string(), cve_outcome),
            (
                "MITRE ATT&CK".to_string(),
                CheckOutcome::AssertedByDesign {
                    detail: "Design-level mitigations for T1552, T1486, T1078 and T1005. \
                             Not a runtime assessment and not a penetration test."
                        .to_string(),
                },
            ),
            (
                "CMMC 2.0".to_string(),
                CheckOutcome::AssertedByDesign {
                    detail: format!(
                        "Level {cmmc_level} control families (AC, AU, IA, SC) have supporting \
                         features. Certification is granted by a C3PAO assessment of an \
                         organisation, not by this tool."
                    ),
                },
            ),
        ];

        Ok(ComplianceStatus {
            fips_140_3: fips,
            cve_scan_passed: cve_passed,
            mitre_attack_aligned: self.check_mitre_attack(),
            cmmc_level,
            violations,
            outcomes,
        })
    }
}

impl Default for ComplianceChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_checks() {
        let checker = ComplianceChecker::new();
        let status = checker.run_all_checks().unwrap();

        assert!(status.fips_140_3);
        assert!(status.mitre_attack_aligned);
        assert_eq!(status.cmmc_level, 2);
        // `cve_scan_passed` is deliberately not asserted: whether cargo-audit
        // is installed is a property of the machine running the tests, not of
        // this crate. `test_cve_outcome_distinguishes_unscannable_from_clean`
        // covers the meaningful part.
    }

    /// The three design-level checks must not claim to have been verified.
    ///
    /// They return constants — that is defensible, because no program can
    /// verify its own certification status — but reporting them as "PASS" made
    /// `iv compliance` usable as CMMC evidence it cannot support.
    #[test]
    fn test_design_level_checks_are_not_reported_as_verified() {
        let status = ComplianceChecker::new().run_all_checks().unwrap();

        for name in ["FIPS 140-3", "MITRE ATT&CK", "CMMC 2.0"] {
            let (_, outcome) = status
                .outcomes
                .iter()
                .find(|(n, _)| n == name)
                .unwrap_or_else(|| panic!("{name} missing from outcomes"));

            assert!(
                matches!(outcome, CheckOutcome::AssertedByDesign { .. }),
                "{name} must be reported as asserted-by-design, got {outcome:?}"
            );
            assert!(
                !matches!(outcome, CheckOutcome::Verified { .. }),
                "{name} cannot be verified at runtime"
            );
        }
    }

    /// The FIPS text must not claim validation the software does not have.
    #[test]
    fn test_fips_outcome_states_it_is_not_validated() {
        let status = ComplianceChecker::new().run_all_checks().unwrap();
        let (_, fips) = status
            .outcomes
            .iter()
            .find(|(n, _)| n == "FIPS 140-3")
            .unwrap();

        let detail = fips.detail();
        assert!(
            detail.contains("not CMVP-validated") || detail.contains("not FIPS 140-3 validated"),
            "FIPS detail must disclaim validation, got: {detail}"
        );
        // Argon2 is not on the FIPS-approved KDF list; saying so is the point.
        assert!(
            detail.contains("Argon2id is not a FIPS-approved KDF"),
            "FIPS detail must disclose the KDF gap, got: {detail}"
        );
    }

    /// "Could not scan" must never be reported as a pass.
    #[test]
    fn test_cve_outcome_distinguishes_unscannable_from_clean() {
        let mut checker = ComplianceChecker::new();

        // Disabled is "nothing was checked", not "everything is fine".
        checker.set_check_enabled("cve", false);
        assert!(matches!(
            checker.cve_outcome(),
            CheckOutcome::NotVerified { .. }
        ));

        // Whatever this machine reports, an unscannable result must not be
        // Verified, and a clean result must not be Failed.
        checker.set_check_enabled("cve", true);
        let outcome = checker.cve_outcome();
        match &outcome {
            CheckOutcome::Verified { .. } | CheckOutcome::Failed { .. } => {
                let (passed, _) = checker.check_cve();
                assert_eq!(passed, matches!(outcome, CheckOutcome::Verified { .. }));
            }
            CheckOutcome::NotVerified { reason } => {
                assert!(reason.contains("cargo-audit"), "got: {reason}");
            }
            CheckOutcome::AssertedByDesign { .. } => {
                panic!("a CVE scan is either run or not; it is never by design")
            }
        }
    }

    /// Only a real failure blocks a gate. "Not verified" is loud but does not
    /// pretend to be a failure, and "by design" must never block.
    #[test]
    fn test_only_failures_are_blocking() {
        assert!(CheckOutcome::Failed { detail: "x".into() }.is_blocking());
        assert!(!CheckOutcome::NotVerified { reason: "x".into() }.is_blocking());
        assert!(!CheckOutcome::AssertedByDesign { detail: "x".into() }.is_blocking());
        assert!(!CheckOutcome::Verified { detail: "x".into() }.is_blocking());
    }

    #[test]
    fn test_check_disabled_fips() {
        // Covers line 94 (is_check_enabled("fips_140_3") => false path)
        let mut checker = ComplianceChecker::new();
        checker.set_check_enabled("fips_140_3", false);
        assert!(checker.check_fips_140_3()); // returns true when disabled
    }

    #[test]
    fn test_check_disabled_cve() {
        // A disabled check short-circuits to (true, []) so it raises no
        // violations — but `cve_outcome` reports it as NotVerified rather
        // than as a pass. See
        // `test_cve_outcome_distinguishes_unscannable_from_clean`.
        let mut checker = ComplianceChecker::new();
        checker.set_check_enabled("cve", false);
        let (passed, cves) = checker.check_cve();
        assert!(passed);
        assert!(cves.is_empty());
    }

    #[test]
    fn test_check_disabled_mitre() {
        let mut checker = ComplianceChecker::new();
        checker.set_check_enabled("mitre_attack", false);
        assert!(checker.check_mitre_attack());
    }

    #[test]
    fn test_check_disabled_cmmc() {
        let mut checker = ComplianceChecker::new();
        checker.set_check_enabled("cmmc", false);
        assert_eq!(checker.check_cmmc(), 0);
    }

    #[test]
    fn test_check_enabled_unknown() {
        let checker = ComplianceChecker::new();
        assert!(!checker.is_check_enabled("nonexistent"));
    }

    #[test]
    fn test_run_all_with_disabled_checks() {
        let mut checker = ComplianceChecker::new();
        checker.set_check_enabled("cve", false);
        checker.set_check_enabled("mitre_attack", false);
        let status = checker.run_all_checks().unwrap();
        assert!(status.fips_140_3);
        assert!(status.cve_scan_passed);
    }

    #[test]
    fn test_violation_severity_debug() {
        let sev = ViolationSeverity::Critical;
        let s = format!("{:?}", sev);
        assert!(s.contains("Critical"));
    }

    #[test]
    fn test_violation_severity_all_variants() {
        let variants = vec![
            ViolationSeverity::Critical,
            ViolationSeverity::High,
            ViolationSeverity::Medium,
            ViolationSeverity::Low,
            ViolationSeverity::Info,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let _: ViolationSeverity = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_compliance_violation_serialization() {
        let violation = ComplianceViolation {
            standard: "FIPS 140-3".to_string(),
            control: "Crypto Module".to_string(),
            severity: ViolationSeverity::Critical,
            description: "Non-FIPS algorithm".to_string(),
            remediation: Some("Use approved algorithm".to_string()),
        };
        let json = serde_json::to_string(&violation).unwrap();
        assert!(json.contains("FIPS"));
        let d: ComplianceViolation = serde_json::from_str(&json).unwrap();
        assert_eq!(d.standard, "FIPS 140-3");
        assert!(d.remediation.is_some());
    }

    #[test]
    fn test_compliance_violation_without_remediation() {
        let violation = ComplianceViolation {
            standard: "CVE".to_string(),
            control: "Vuln Mgmt".to_string(),
            severity: ViolationSeverity::Low,
            description: "Minor issue".to_string(),
            remediation: None,
        };
        let json = serde_json::to_string(&violation).unwrap();
        let d: ComplianceViolation = serde_json::from_str(&json).unwrap();
        assert!(d.remediation.is_none());
    }

    #[test]
    fn test_compliance_status_serialization() {
        let status = ComplianceStatus {
            fips_140_3: true,
            cve_scan_passed: true,
            mitre_attack_aligned: true,
            cmmc_level: 2,
            violations: vec![],
            outcomes: Vec::new(),
        };
        let json = serde_json::to_string(&status).unwrap();
        let d: ComplianceStatus = serde_json::from_str(&json).unwrap();
        assert!(d.fips_140_3);
        assert_eq!(d.cmmc_level, 2);
        assert!(d.violations.is_empty());
    }

    #[test]
    fn test_compliance_status_with_violations() {
        let status = ComplianceStatus {
            fips_140_3: false,
            cve_scan_passed: false,
            mitre_attack_aligned: true,
            cmmc_level: 1,
            violations: vec![
                ComplianceViolation {
                    standard: "FIPS".to_string(),
                    control: "AES".to_string(),
                    severity: ViolationSeverity::Critical,
                    description: "Bad algo".to_string(),
                    remediation: None,
                },
                ComplianceViolation {
                    standard: "CVE".to_string(),
                    control: "Vuln".to_string(),
                    severity: ViolationSeverity::High,
                    description: "CVE-2024-1234".to_string(),
                    remediation: Some("Update dep".to_string()),
                },
            ],
            outcomes: vec![(
                "CVE scan".to_string(),
                CheckOutcome::Failed {
                    detail: "CVE-2024-1234".to_string(),
                },
            )],
        };
        let json = serde_json::to_string(&status).unwrap();
        let d: ComplianceStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(d.violations.len(), 2);
    }

    #[test]
    fn test_set_check_enabled_toggle() {
        let mut checker = ComplianceChecker::new();
        assert!(checker.is_check_enabled("fips_140_3"));
        checker.set_check_enabled("fips_140_3", false);
        assert!(!checker.is_check_enabled("fips_140_3"));
        checker.set_check_enabled("fips_140_3", true);
        assert!(checker.is_check_enabled("fips_140_3"));
    }

    #[test]
    fn test_set_custom_check() {
        let mut checker = ComplianceChecker::new();
        assert!(!checker.is_check_enabled("custom_check"));
        checker.set_check_enabled("custom_check", true);
        assert!(checker.is_check_enabled("custom_check"));
    }

    #[test]
    fn test_checker_default_trait() {
        let checker = ComplianceChecker::default();
        assert!(checker.is_check_enabled("fips_140_3"));
        assert!(checker.is_check_enabled("cve"));
        assert!(checker.is_check_enabled("mitre_attack"));
        assert!(checker.is_check_enabled("cmmc"));
    }

    #[test]
    fn test_check_fips_enabled() {
        let checker = ComplianceChecker::new();
        assert!(checker.check_fips_140_3());
    }

    #[test]
    fn test_check_mitre_enabled() {
        let checker = ComplianceChecker::new();
        assert!(checker.check_mitre_attack());
    }

    #[test]
    fn test_check_cmmc_enabled() {
        let checker = ComplianceChecker::new();
        assert_eq!(checker.check_cmmc(), 2);
    }

    #[test]
    fn test_check_cve_enabled() {
        // This test used to assert `passed == true` unconditionally, on the
        // grounds that "cargo-audit is not available" was a non-failure. That
        // was the bug: a scan that never ran reported as clean. It passed on
        // a developer machine with cargo-audit installed and would have
        // passed on CI without it, for opposite reasons.
        //
        // Whether cargo-audit is installed is a property of the machine, so
        // assert the invariant instead: `passed` is true only when a scan
        // actually ran and found nothing, and a non-pass always explains why.
        let checker = ComplianceChecker::new();
        let (passed, findings) = checker.check_cve();

        if passed {
            assert!(
                findings.is_empty(),
                "a passing scan must report no findings, got: {findings:?}"
            );
        } else {
            assert!(
                !findings.is_empty(),
                "a non-passing scan must say why it did not pass"
            );
        }

        // And the outcome must agree with the boolean.
        match checker.cve_outcome() {
            CheckOutcome::Verified { .. } => assert!(passed),
            CheckOutcome::NotVerified { .. } | CheckOutcome::Failed { .. } => assert!(!passed),
            CheckOutcome::AssertedByDesign { .. } => {
                panic!("a CVE scan is either run or not; it is never by design")
            }
        }
    }

    #[test]
    fn test_run_all_checks_full() {
        let checker = ComplianceChecker::new();
        let status = checker.run_all_checks().unwrap();
        // All checks enabled and all return passing in our build
        assert!(status.fips_140_3);
        assert!(status.mitre_attack_aligned);
        assert_eq!(status.cmmc_level, 2);
        // No crypto violations
        assert!(status.violations.iter().all(|v| v.standard != "FIPS 140-3"));
    }

    #[test]
    fn test_run_all_checks_no_violations_when_all_pass() {
        let mut checker = ComplianceChecker::new();
        // Disable CVE check to avoid dependency on cargo-audit
        checker.set_check_enabled("cve", false);
        let status = checker.run_all_checks().unwrap();
        assert!(status.violations.is_empty());
    }

    #[test]
    fn test_compliance_status_clone() {
        let status = ComplianceStatus {
            fips_140_3: true,
            cve_scan_passed: false,
            mitre_attack_aligned: true,
            cmmc_level: 2,
            violations: vec![ComplianceViolation {
                standard: "CVE".to_string(),
                control: "VM".to_string(),
                severity: ViolationSeverity::High,
                description: "CVE-2024-9999".to_string(),
                remediation: Some("upgrade".to_string()),
            }],
            outcomes: Vec::new(),
        };
        let cloned = status.clone();
        assert_eq!(cloned.violations.len(), 1);
        assert!(!cloned.cve_scan_passed);
    }

    #[test]
    fn test_checker_enable_disable_multiple() {
        let mut checker = ComplianceChecker::new();
        // Disable all
        checker.set_check_enabled("fips_140_3", false);
        checker.set_check_enabled("cve", false);
        checker.set_check_enabled("mitre_attack", false);
        checker.set_check_enabled("cmmc", false);

        let status = checker.run_all_checks().unwrap();
        assert!(status.fips_140_3); // returns true when disabled
        assert!(status.cve_scan_passed);
        assert!(status.mitre_attack_aligned);
        assert_eq!(status.cmmc_level, 0); // returns 0 when disabled
        assert!(status.violations.is_empty());
    }
}
