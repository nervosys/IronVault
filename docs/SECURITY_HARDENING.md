 # Security Hardening Guide

**Document Version**: 1.0  
**Date**: November 4, 2025  
**Applies To**: IronVault v0.1.0+

---

## Overview

This guide provides security hardening recommendations for deploying IronVault in production environments. Following these guidelines will maximize the security posture of your AI model storage infrastructure.

---

## 1. Passphrase Management

### Minimum Requirements
- **Length**: Minimum 20 characters
- **Complexity**: Mix of uppercase, lowercase, numbers, and symbols
- **Entropy**: At least 80 bits of entropy
- **Uniqueness**: Never reuse passphrases across vaults

### Strong Passphrase Generation

```bash
# Generate strong passphrase (Linux/macOS)
openssl rand -base64 32

# Generate strong passphrase (Windows PowerShell)
-join ((48..57) + (65..90) + (97..122) | Get-Random -Count 32 | ForEach-Object {[char]$_})

# Using password manager (recommended)
# 1Password, LastPass, Bitwarden, etc.
```

### Passphrase Storage

**✅ DO**:
- Store in secure password manager
- Use hardware security keys when possible
- Encrypt backups of passphrases
- Use environment variables for automation
- Implement key rotation policies

**❌ DON'T**:
- Store in plain text files
- Commit to version control
- Share via email or chat
- Write on sticky notes
- Reuse across systems

### Example: Environment Variable

```bash
# Linux/macOS
export VAULT_PASSPHRASE=$(cat /secure/location/passphrase.enc | decrypt)

# Windows PowerShell
$env:VAULT_PASSPHRASE = Get-Content C:\secure\passphrase.enc | Decrypt-Secret
```

---

## 2. File System Permissions

### Unix/Linux/macOS

#### Vault Directory
```bash
# Restrict vault directory to owner only
chmod 700 ~/.local/share/ironvault/vaults/
chmod 700 ~/.local/share/ironvault/vaults/default/

# Verify permissions
ls -la ~/.local/share/ironvault/vaults/
# Should show: drwx------ (700)
```

#### Configuration Files
```bash
# Restrict config files
chmod 600 ~/.config/ironvault/config.yaml

# Verify
ls -la ~/.config/ironvault/
# Should show: -rw------- (600)
```

#### Audit Logs
```bash
# Append-only audit logs (recommended)
chattr +a ~/.local/share/ironvault/logs/audit.log

# Or restrict to read-only after creation
chmod 400 ~/.local/share/ironvault/logs/audit.log
```

### Windows

```powershell
# Restrict vault directory to current user
$path = "$env:LOCALAPPDATA\ironvault\vaults"
$acl = Get-Acl $path
$acl.SetAccessRuleProtection($true, $false)
$rule = New-Object System.Security.AccessControl.FileSystemAccessRule(
    $env:USERNAME, "FullControl", "Allow"
)
$acl.AddAccessRule($rule)
Set-Acl $path $acl

# Verify
Get-Acl $path | Format-List
```

---

## 3. Network Security

### Local-Only Access (Default)

IronVault operates locally by default. No network exposure required for basic operation.

### Cloud Storage Security

When using cloud backends:

#### AWS S3
```bash
# Use IAM roles (recommended)
export AWS_PROFILE=ironvault

# Enable MFA for sensitive operations
aws configure set profile.ironvault.mfa_serial arn:aws:iam::ACCOUNT:mfa/USER

# Enable versioning and encryption
aws s3api put-bucket-versioning \
    --bucket my-models \
    --versioning-configuration Status=Enabled

aws s3api put-bucket-encryption \
    --bucket my-models \
    --server-side-encryption-configuration \
    '{"Rules":[{"ApplyServerSideEncryptionByDefault":{"SSEAlgorithm":"AES256"}}]}'
```

#### Azure Blob Storage
```bash
# Use managed identities (recommended)
az login --identity

# Enable soft delete
az storage account blob-service-properties update \
    --account-name mystorageaccount \
    --enable-delete-retention true \
    --delete-retention-days 30
```

#### Google Cloud Storage
```bash
# Use service account keys
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json

# Enable uniform bucket-level access
gsutil uniformbucketlevelaccess set on gs://my-models

# Enable versioning
gsutil versioning set on gs://my-models
```

---

## 4. Audit Logging

### Enable Comprehensive Logging

```yaml
# ~/.config/ironvault/config.yaml
security:
  audit_log: true
  session_timeout: 3600  # 1 hour
  require_passphrase: true

compliance:
  audit_retention_days: 90
  fips_mode: true
```

### Monitor Audit Logs

```bash
# Real-time monitoring (Linux/macOS)
tail -f ~/.local/share/ironvault/logs/audit.log

# Search for failed authentications
grep "Authentication failed" ~/.local/share/ironvault/logs/audit.log

# Count operations by type
grep -o '"operation":"[^"]*"' audit.log | sort | uniq -c
```

### Audit Log Forwarding

#### To Syslog (Linux)
```bash
# Install rsyslog if needed
sudo apt-get install rsyslog

# Configure forwarding
echo '*.* @@remote-syslog-server:514' | \
    sudo tee -a /etc/rsyslog.d/50-ironvault.conf

sudo systemctl restart rsyslog
```

#### To Windows Event Log
```powershell
# Use nxlog or similar tool
# Configure in nxlog.conf
```

---

## 5. System Hardening

### Operating System

#### Linux
```bash
# Enable automatic security updates
sudo apt-get install unattended-upgrades
sudo dpkg-reconfigure --priority=low unattended-upgrades

# Enable firewall (if needed for remote access)
sudo ufw enable
sudo ufw default deny incoming
sudo ufw default allow outgoing

# Enable SELinux/AppArmor
sudo setenforce 1  # SELinux
sudo aa-enforce /etc/apparmor.d/*  # AppArmor
```

#### macOS
```bash
# Enable FileVault disk encryption
sudo fdesetup enable

# Enable firewall
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --setglobalstate on
```

#### Windows
```powershell
# Enable BitLocker
Enable-BitLocker -MountPoint "C:" -EncryptionMethod XtsAes256

# Enable Windows Defender
Set-MpPreference -DisableRealtimeMonitoring $false
```

### Memory Protection

#### Linux (mlock)
```bash
# Allow memory locking
sudo setcap cap_ipc_lock=+ep /path/to/iv

# Or increase locked memory limit
ulimit -l unlimited
```

---

## 6. Backup Strategy

### Encrypted Backups

```bash
# Backup vault with encryption intact
tar czf vault-backup-$(date +%Y%m%d).tar.gz \
    ~/.local/share/ironvault/vaults/

# Encrypt backup
gpg --symmetric --cipher-algo AES256 \
    vault-backup-$(date +%Y%m%d).tar.gz

# Store offsite
aws s3 cp vault-backup-*.tar.gz.gpg \
    s3://my-backups/ironvault/ \
    --storage-class GLACIER
```

### Backup Verification

```bash
# Verify backup integrity
gpg -d vault-backup-20251104.tar.gz.gpg | tar tzf - > /dev/null
echo $?  # Should return 0
```

### 3-2-1 Rule
- **3 copies** of data (original + 2 backups)
- **2 different media types** (local + cloud)
- **1 offsite backup** (different physical location)

---

## 7. Compliance Requirements

### FIPS 140-3 Mode

```yaml
# config.yaml
compliance:
  fips_mode: true  # Declarative only -- nothing reads this, and there is no
                   # FIPS mode to enter; see SECURITY.md
  cve_scanning: true
  audit_retention_days: 90
```

### CMMC 2.0 Level 2

**Required Controls**:
- ✅ Access control (AC.3.018)
- ✅ Identification and authentication (IA.3.080)
- ⚠️ System and communications protection: SC.3.191 (data at rest) met;
  SC.3.177 (FIPS-validated cryptography) **not met** — see SECURITY.md
- ✅ Audit and accountability (AU.3.046, AU.3.049)

**Verification**:
```bash
# Check compliance status
iv compliance

# Should show all checks passing
```

---

## 8. Incident Response

### Detection

**Monitor for**:
- Multiple failed authentication attempts
- Unusual access patterns
- Large data exfiltration
- Unexpected file modifications

### Response Plan

1. **Isolate**: Disconnect affected systems
2. **Preserve**: Save logs and evidence
3. **Analyze**: Determine scope and impact
4. **Remediate**: Patch vulnerabilities
5. **Document**: Record all actions taken
6. **Review**: Update security policies

### Emergency Contacts

- **Security Team**: security@nervosys.ai
- **Incident Response**: incident@nervosys.ai
- **CVE Reporting**: https://rustsec.org/

---

## 9. Security Testing

### Regular Audits

```bash
# Vulnerability scanning
cargo audit

# Dependency checking
cargo outdated
cargo tree --duplicates

# Static analysis
cargo clippy -- -D warnings
```

### Penetration Testing

**Recommended Tests**:
- Passphrase brute-force resistance
- Encryption key extraction attempts
- File permission bypass attempts
- Memory dump analysis
- Side-channel attacks

---

## 10. Production Deployment

Containers were removed in 4.5.0 — there is no first-party `Dockerfile`, image,
or Helm chart. `iv` ships as a static binary, a crate, and a Python wheel.

For a hardened service install, use `deploy/systemd/install.sh`: it creates the
`ironvault` system user and `/var/lib/ironvault`, writes `/etc/ironvault/server.env` at `0600`
root-owned, and installs a unit that reads credentials via `EnvironmentFile=`
rather than `Environment=` — the latter is readable by any local user through
`systemctl show`. See [TELEMETRY.md](TELEMETRY.md#service-scoped-configuration).

If you build your own container, keep the properties the removed image had: run
as a non-root user, mount the vault directory as a volume rather than baking it
into a layer, and inject secrets at runtime instead of at build time.

---

## 11. Security Checklist

### Pre-Production
- [ ] Strong passphrases configured (20+ chars)
- [ ] File permissions restricted (700/600)
- [ ] Audit logging enabled
- [ ] FIPS mode enabled (if required)
- [ ] Backup strategy implemented
- [ ] Dependencies updated (`cargo update`)
- [ ] Security scan passed (`cargo audit`)
- [ ] Configuration reviewed

### Production
- [ ] Monitoring configured
- [ ] Incident response plan documented
- [ ] Regular backup testing scheduled
- [ ] Security updates automated
- [ ] Audit log review scheduled
- [ ] Access control policies enforced
- [ ] Compliance requirements met

### Ongoing
- [ ] Monthly security updates
- [ ] Quarterly audit log reviews
- [ ] Annual penetration testing
- [ ] Continuous dependency monitoring
- [ ] Regular backup verification
- [ ] Security training for team

---

## 12. Additional Resources

### Documentation
- [SECURITY.md](https://github.com/nervosys/IronVault/blob/master/SECURITY.md) - Security policy
- [SECURITY_AUDIT.md](SECURITY_AUDIT.md) - Latest audit report
- [docs/ARCHITECTURE.md](ARCHITECTURE.md) - System architecture

### Standards
- FIPS 140-3: https://csrc.nist.gov/publications/detail/fips/140/3/final
- CMMC 2.0: https://www.acq.osd.mil/cmmc/
- NIST SP 800-53: https://csrc.nist.gov/publications/detail/sp/800-53/rev-5/final
- OWASP: https://owasp.org/

### Tools
- cargo-audit: https://github.com/RustSec/rustsec/tree/main/cargo-audit
- cargo-deny: https://github.com/EmbarkStudios/cargo-deny
- cargo-outdated: https://github.com/kbknapp/cargo-outdated

---

## 13. Support

For security questions or concerns:
- **Email**: security@nervosys.ai
- **GitHub**: https://github.com/nervosys/IronVault/security
- **Documentation**: https://github.com/nervosys/IronVault/blob/master/docs/SECURITY_HARDENING.md

**Remember**: Security is a continuous process, not a one-time setup. Regular reviews and updates are essential.

---

**Last Updated**: November 4, 2025  
**Next Review**: February 4, 2026
