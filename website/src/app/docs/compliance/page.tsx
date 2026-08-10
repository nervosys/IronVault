export default function CompliancePage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Compliance</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Standards and certifications supported by IronVault.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4">FIPS 140-3</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        All cryptographic primitives used are FIPS 140-3 approved:
      </p>
      <ul className="space-y-1 text-[var(--color-text-secondary)]">
        <li>• AES-256-GCM (NIST SP 800-38D)</li>
        <li>• SHA-256 (FIPS 180-4)</li>
        <li>• Argon2id (NIST SP 800-63B recommended)</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4">CMMC 2.0 Level 2</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Suitable for defense contractor use with Controlled Unclassified Information (CUI):
      </p>
      <ul className="space-y-1 text-[var(--color-text-secondary)]">
        <li>• Data encryption at rest (SC.L2-3.13.16)</li>
        <li>• Audit logging (AU.L2-3.3.1)</li>
        <li>• Access control (AC.L2-3.1.1)</li>
        <li>• Integrity verification (SI.L2-3.14.1)</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4">XDG Base Directory</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        100% compliant with the XDG Base Directory Specification (9/9 checks):
      </p>
      <ul className="space-y-1 text-[var(--color-text-secondary)]">
        <li>• Config files in <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">$XDG_CONFIG_HOME</code></li>
        <li>• Data files in <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">$XDG_DATA_HOME</code></li>
        <li>• Cache files in <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">$XDG_CACHE_HOME</code></li>
        <li>• No files in home directory root</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4">MITRE ATT&CK</h2>
      <p className="text-[var(--color-text-secondary)]">
        CVE scanning and threat modeling aligned with MITRE ATT&CK framework for ML systems.
      </p>
    </>
  );
}
