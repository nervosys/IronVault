import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function VersionControlPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Version Control</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Git-like version control for AI models with lineage tracking, time travel, and automatic checksums.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="basics">Basics</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Every <code className="text-sm px-1 bg-[var(--color-bg-secondary)] rounded">iv store</code> call to an existing model
        creates a new version automatically. Versions are numbered sequentially (1, 2, 3, ...) and each
        gets a SHA-256 checksum for integrity verification.
      </p>
      <CodeBlock language="bash">{`# Store initial version
iv store my-model --format safetensors --file v1.safetensors

# Store an updated version (auto-increments version number)
iv store my-model --format safetensors --file v2.safetensors

# List all versions
iv versions my-model
# my-model v1 (sha256:a1b2c3...) 2024-01-15 10:30:00
# my-model v2 (sha256:d4e5f6...) 2024-01-16 14:20:00`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="retrieval">Retrieving Versions</h2>
      <CodeBlock language="bash">{`# Get latest version (default)
iv get my-model --output ./latest/

# Get a specific version
iv get my-model --version 1 --output ./v1/`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="lineage">Lineage Tracking</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Track the evolution of your models — see parent-child relationships across versions.
      </p>
      <CodeBlock language="bash">{`iv lineage my-model
# v1 ← (initial)
# v2 ← v1 (fine-tuned on new data)
# v3 ← v2 (quantized to INT8)`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="checksums">Integrity Verification</h2>
      <CodeBlock language="bash">{`# Verify checksum of a specific version
iv verify my-model --version 2
# ✓ my-model v2: checksum valid (sha256:d4e5f6...)`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cleanup">Cleanup Policies</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Manage storage by automatically removing old versions.
      </p>
      <CodeBlock language="bash">{`# Keep only the last 5 versions
iv cleanup my-model --keep 5`}</CodeBlock>

      <Callout type="tip" title="Encryption at rest">
        All versions are encrypted with AES-256-GCM. Version metadata (checksums, timestamps,
        lineage) is stored separately and also encrypted.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rust-api">Rust API</h2>
      <CodeBlock language="rust">{`use ironvault::vault::Vault;

let mut vault = Vault::new("my-vault", None)?;
vault.unlock("passphrase")?;

// Store creates a new version automatically
vault.store("my-model", &data_v1, "safetensors", None)?;
vault.store("my-model", &data_v2, "safetensors", None)?;

// Get specific version
let v1 = vault.get("my-model", Some(1))?;

// List versions
let versions = vault.list_versions("my-model")?;

// Get lineage
let lineage = vault.get_lineage("my-model", 2)?;

// Verify integrity
let valid = vault.verify_checksum("my-model", 1)?;`}</CodeBlock>
    </>
  );
}
