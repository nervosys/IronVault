import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function SecurityPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Encryption & Security</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        FIPS 140-3 compliant encryption with AES-256-GCM and Argon2id key derivation.
      </p>

      <Callout type="tip" title="Military-grade by default">
        All models are encrypted at rest with zero configuration needed. Simply set a passphrase
        when creating a vault.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="encryption">Encryption Scheme</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Component</th>
              <th className="text-left p-3 font-semibold">Algorithm</th>
              <th className="text-left p-3 font-semibold">Details</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["Cipher", "AES-256-GCM", "256-bit key, 96-bit nonce, 128-bit auth tag"],
              ["Key Derivation", "Argon2id", "Memory-hard KDF, 64 MB memory, 3 iterations"],
              ["Hashing", "SHA-256", "Integrity checksums for all stored data"],
              ["Memory Safety", "zeroize", "Secrets zeroed from memory after use"],
            ].map(([component, algo, details]) => (
              <tr key={component} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]">{component}</td>
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{algo}</code></td>
                <td className="p-3">{details}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="how-it-works">How It Works</h2>
      <ol className="list-decimal list-inside space-y-2 text-[var(--color-text-secondary)]">
        <li><strong>Passphrase input</strong> — User provides a passphrase (minimum 8 characters)</li>
        <li><strong>Salt generation</strong> — Random 32-byte salt generated per vault</li>
        <li><strong>Key derivation</strong> — Argon2id derives a 256-bit key from passphrase + salt</li>
        <li><strong>Nonce generation</strong> — Unique 96-bit nonce per encryption operation</li>
        <li><strong>Encryption</strong> — AES-256-GCM encrypts data with authenticated encryption</li>
        <li><strong>Storage</strong> — Ciphertext + nonce + auth tag stored together</li>
        <li><strong>Memory cleanup</strong> — Key material zeroized from memory via <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">zeroize</code></li>
      </ol>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="compliance">FIPS 140-3 Compliance</h2>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• AES-256-GCM is FIPS 140-3 approved (NIST SP 800-38D)</li>
        <li>• Argon2id is NIST-recommended for password hashing</li>
        <li>• SHA-256 is FIPS 180-4 approved</li>
        <li>• No custom cryptography — uses audited RustCrypto crates</li>
        <li>• CMMC 2.0 Level 2 compliant for defense contractor use</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="audit">Audit Logging</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        All security-relevant operations are logged with timestamps for compliance:
      </p>
      <ul className="space-y-1 text-[var(--color-text-secondary)]">
        <li>• Authentication attempts (success/failure)</li>
        <li>• Model store/retrieve/delete operations</li>
        <li>• Passphrase changes</li>
        <li>• Security violations</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="passphrase">Passphrase Management</h2>
      <CodeBlock language="bash">{`# Change vault passphrase (re-encrypts all models)
iv change-passphrase

# Recommended: Use a strong, random passphrase
# Minimum: 8 characters (12+ recommended)
# Use: letters, numbers, symbols`}</CodeBlock>
    </>
  );
}
