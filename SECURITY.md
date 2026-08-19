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

### Cryptography

- **AES-256-GCM** for data encryption (FIPS-approved algorithm: FIPS 197, SP 800-38D)
- **Argon2id** for key derivation (RFC 9106 — **not** a FIPS-approved KDF)
- **SHA-256** for integrity verification (FIPS-approved algorithm: FIPS 180-4)
- **Ed25519** for model signing (FIPS-approved algorithm: FIPS 186-5)
- Secure random number generation via OS-provided CSPRNG

**This is not a FIPS 140-3 validated cryptographic module, and no configuration of
it is.** FIPS 140-3 validates a *module* through NIST's CMVP; the RustCrypto
implementations used here hold no CMVP certificate, and Argon2id is outside FIPS
regardless of implementation — SP 800-132 approves PBKDF2 for password-based key
derivation. Argon2id is used deliberately, because it is materially stronger
against GPU-accelerated cracking, and that trade is the right one for most
deployments.

A deployment with a genuine FIPS obligation needs a validated module (AWS-LC-FIPS,
BoringCrypto, or an HSM) *and* a FIPS-approved KDF. See `iv compliance` and
`src/compliance.rs`, which report this relationship rather than asserting a
determination.

### Transport

`iv serve` **refuses to bind a non-loopback address without TLS.** Set `tls_cert`
and `tls_key` to PEM paths and the server terminates TLS itself; loopback binds
continue to serve plain HTTP, because those packets never reach a network.

This is enforced rather than recommended because `POST /api/v1/auth/token`
carries the vault passphrase — the value the encryption key is derived from.
Disclosing it is not comparable to leaking a session token: it does not expire,
revocation cannot reach it, it decrypts any copy of the vault taken at any time,
and using it offline leaves no audit record. A single packet capture on a shared
segment during one unlock is a permanent, silent compromise.

To terminate TLS at a reverse proxy, run the proxy on the same host and leave the
server on `127.0.0.1`. There is deliberately no flag to bypass the check: a proxy
on a different host is a real network hop that needs its own TLS, and a
"the network is trusted" switch is the kind that gets set while debugging and
never unset.

### What the vault encrypts, and what it does not

**Model contents and the version index are both encrypted.** From 8.0,
`versions.json` is sealed with AES-256-GCM under the vault key, behind an
`IRONVAULT-VERSIONS-v1` marker. Model names, sizes, formats, timestamps,
checkpoint IDs, SHA-256 checksums, blob paths and user metadata are all
ciphertext at rest.

Through 7.x that file was plaintext JSON. Anything able to read the vault
directory could read the whole inventory without the passphrase — and for a
vault the inventory is not incidental: a model named
`acme-fraud-detection-v3` identifies a customer and a use case without
decrypting a byte, and the recorded checksum confirms whether a *known* model
is present.

Three consequences worth stating plainly:

- **A wrong passphrase is refused, not answered.** Opening the index is an
  AEAD decryption, so a wrong key fails the authentication tag. This is a
  second, independent key check alongside `vault.keycheck`.
- **A pre-8.0 index migrates on first unlock**, not on first write, so a vault
  that is only ever read still gets sealed. The plaintext file is replaced in
  place.
- **`iv gc`, `iv browse`, `iv vault-export` and `iv vault-import` now require
  the passphrase.** They read the index, and it cannot be read without the
  key. For `gc` this is load-bearing rather than cosmetic: it deletes blobs
  the index does not reference, so running it against an unreadable index
  would treat every blob as an orphan.

**The SQLite backend is not sealed.** `--sqlite` stores the same model names,
sizes, timestamps and checksums in a plain database file. Sealing it requires
SQLCipher or per-column encryption, which is a separate change; until then
`--sqlite` trades index confidentiality for its ACID guarantees. Treat such a
vault directory as sensitive and rely on filesystem permissions (0600/0700,
applied on creation).

What is still *not* encrypted, on either backend: file names and sizes in the
`data/` directory. An observer can count blobs and see how large each is,
which bounds how many models exist and roughly how big they are.

### Access Control
- Passphrase-protected vault access, verified at unlock (see below)
- Secure key management
- Session timeout enforcement
- File permission restrictions (Unix: 0600/0700)

### Audit & Compliance
- Comprehensive audit logging, optionally hash-chained so tampering is detectable
- Controls mapped to CMMC 2.0 Level 2 practices (see the caveat below)
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

ironvault is built to *support* the following frameworks. None of these are
certifications this software holds, and two of them are not the kind of thing
software can hold at all — FIPS 140-3 certifies a cryptographic module through
NIST's CMVP, and CMMC certifies an organization through a C3PAO, not a product.

- **FIPS 140-3**: uses FIPS-approved algorithms (AES-256-GCM, SHA-256, Ed25519).
  **Not a validated module**, and Argon2id is not an approved KDF — see above.
- **NIST SP 800-53**: controls informed by the AC, AU, IA, SC and SI families
- **CMMC 2.0 Level 2**: supports practices across Access Control, Audit &
  Accountability, Identification & Authentication, and System & Information
  Integrity. **SC.L2-3.13.11 is not met**, because it requires FIPS-validated
  cryptography for CUI.
- **MITRE ATT&CK**: mitigations mapped to T1552, T1110, T1078, T1005, T1486 and
  T1195.002; T1040/T1557 are only partially addressed while TLS is operator-supplied

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
