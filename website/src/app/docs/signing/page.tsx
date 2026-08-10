import CodeBlock from "@/components/DocElements";

export default function SigningPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Model Signing</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        HMAC-SHA256 model signing with detached <code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-sm">.sig</code> files for tamper detection and provenance tracking.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Sign a vault model (auto-generates key on first use)
iv sign my-model

# Sign with identity
iv sign my-model --identity "ML Team <ml@company.com>"

# Verify a signature (--key is required for a real check)
iv verify my-model --signature my-model.sig --key signing_key.json

# Sign a file on disk
iv sign my-model --file ./model.safetensors`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="how-it-works">How It Works</h2>
      <ol className="list-decimal list-inside space-y-2 text-[var(--color-text-secondary)]">
        <li><strong>Key Generation</strong> — A signing keypair is auto-generated on first use and saved to the config directory</li>
        <li><strong>Signing</strong> — HMAC-SHA256 is computed over the file content using the secret seed</li>
        <li><strong>Detached Signature</strong> — A <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">.sig</code> JSON file stores signature, public key, file hash, signer identity, and timestamp</li>
        <li><strong>Verification</strong> — The file is re-hashed and compared against the <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">.sig</code>, then the HMAC tag is recomputed from the secret seed and compared in constant time</li>
      </ol>

      <p className="mt-4 text-[var(--color-text-secondary)]">
        <strong>The verification key is not optional.</strong> Every field in a <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">.sig</code> file — including the file hash — is attacker-controlled if the attacker controls the model. Without the secret seed, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">verify</code> reports the signature as <em>not checked</em> and exits non-zero.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="sign">sign</h2>
      <CodeBlock language="bash">{`iv sign <NAME> [OPTIONS]

Options:
  -v, --version <V>   Model version (default: latest)
  -k, --key <KEY>     Path to signing key JSON file
  -i, --identity <ID> Signer identity (name/email)
  --file <PATH>       Sign a file on disk instead of vault model`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="verify">verify</h2>
      <CodeBlock language="bash">{`iv verify <NAME> --signature <SIG> [OPTIONS]

Options:
  --signature <SIG>   Path to .sig file
  -k, --key <KEY>     Path to signing key for verification
  --file <PATH>       Verify a file on disk
  -v, --version <V>   Model version`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="signature-format">Signature File Format</h2>
      <CodeBlock language="json">{`{
  "signature": "hex-encoded HMAC-SHA256",
  "public_key": "hex-encoded 32-byte key",
  "file_sha256": "hex-encoded SHA-256 of model file",
  "signer": "ML Team <ml@company.com>",
  "signed_at": "2026-04-04T12:00:00Z",
  "version": 2,
  "metadata": {}
}`}</CodeBlock>
    </>
  );
}
