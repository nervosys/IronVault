# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.2.x   | :white_check_mark: |
| 1.1.x   | :white_check_mark: |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via email to: security@nervosys.ai

You should receive a response within 48 hours. If for some reason you do not, please follow up via email to ensure we received your original message.

Please include the following information:

- Type of issue (e.g., buffer overflow, SQL injection, cross-site scripting, etc.)
- Full paths of source file(s) related to the manifestation of the issue
- The location of the affected source code (tag/branch/commit or direct URL)
- Any special configuration required to reproduce the issue
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the issue, including how an attacker might exploit it

## Security Measures

ironvault implements multiple layers of security:

### Cryptography (FIPS 140-3 Compliant)
- **AES-256-GCM** for data encryption
- **Argon2id** for key derivation
- **SHA-256** for integrity verification
- Secure random number generation via OS-provided CSPRNG

### Access Control
- Passphrase-protected vault access
- Secure key management
- Session timeout enforcement
- File permission restrictions (Unix: 0600/0700)

### Audit & Compliance
- Comprehensive audit logging
- CMMC 2.0 Level 2 compliance
- MITRE ATT&CK framework alignment
- Regular CVE scanning via cargo-audit

### Data Protection
- Encryption at rest
- Compression before encryption
- Checksum verification
- Version control with integrity checks

## Security Best Practices

When using ironvault:

1. **Use strong passphrases**: Minimum 20 characters with high entropy
2. **Protect your passphrase**: Never commit passphrases to version control
3. **Regular updates**: Keep ironvault and dependencies updated
4. **Secure storage**: Ensure vault directory has appropriate file permissions
5. **Audit logs**: Regularly review audit logs for suspicious activity
6. **Backup strategy**: Maintain encrypted backups of critical models

## Compliance Standards

ironvault is designed to comply with:

- **FIPS 140-3**: Federal cryptographic module validation
- **NIST SP 800-53**: Security and privacy controls
- **CMMC 2.0 Level 2**: Cybersecurity Maturity Model Certification
- **MITRE ATT&CK**: Threat mitigation framework

## Threat Model

ironvault protects against:

- **Unauthorized Access** (MITRE T1078): Passphrase-protected encryption
- **Data Theft** (MITRE T1005): Encryption at rest
- **Data Tampering** (MITRE T1565): Cryptographic integrity checks
- **Credential Theft** (MITRE T1552): Secure key derivation and storage
- **Ransomware** (MITRE T1486): Version control and backup capabilities

## Known Limitations

- Passphrase recovery is not possible - if lost, data cannot be recovered
- Side-channel attacks are not explicitly mitigated
- Physical access to unlocked vault allows data access
- Memory dumps may contain decrypted data during active sessions

## Security Updates

Security updates will be released as soon as possible after a vulnerability is confirmed. Updates will be announced via:

- GitHub Security Advisories
- Release notes
- Email notifications to registered users
