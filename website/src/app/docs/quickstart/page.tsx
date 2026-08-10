import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function QuickStartPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Quick Start</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Get up and running with IronVault in under 5 minutes.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="install">Installation</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">From crates.io</h3>
      <CodeBlock language="bash">{`cargo install ironvault`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">From source</h3>
      <CodeBlock language="bash">{`git clone https://github.com/nervosys/IronVault.git
cd IronVault
cargo build --release
# Binary is at target/release/iv`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">Python (via PyPI)</h3>
      <CodeBlock language="bash">{`pip install ironvault
# With ML extras:
pip install "ironvault[ml]"`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="first-vault">Create Your First Vault</h2>
      <CodeBlock language="bash">{`# Initialize a new vault
iv init my-vault

# Store a model
iv store my-model --format safetensors --file model.safetensors

# List stored models
iv list

# Retrieve a model
iv get my-model --output ./retrieved/`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="version-control">Version Control</h2>
      <CodeBlock language="bash">{`# Store a new version of an existing model
iv store my-model --format safetensors --file model-v2.safetensors

# List versions
iv versions my-model

# Get a specific version
iv get my-model --version 1 --output ./v1/

# View model lineage
iv lineage my-model`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="api-server">Start the API Server</h2>
      <Callout type="info" title="Requires API feature">
        Build with <code className="px-1 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">--features api</code> to
        enable the REST API.
      </Callout>
      <CodeBlock language="bash">{`# Start the server
iv serve --host 0.0.0.0 --port 8080

# Get a JWT token
curl -X POST http://localhost:8080/auth/token \\
  -H "Content-Type: application/json" \\
  -d '{"passphrase": "your-vault-passphrase"}'

# List models
curl http://localhost:8080/models \\
  -H "Authorization: Bearer <token>"`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rust-api">Rust Library Usage</h2>
      <CodeBlock language="rust" title="Cargo.toml">{`[dependencies]
ironvault = "1.0.0"`}</CodeBlock>

      <CodeBlock language="rust" title="main.rs">{`use ironvault::vault::Vault;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and unlock vault
    let mut vault = Vault::new("my-vault", None)?;
    vault.unlock("my-passphrase")?;

    // Store a model
    let data = std::fs::read("model.safetensors")?;
    vault.store("my-model", &data, "safetensors", None)?;

    // Retrieve it
    let retrieved = vault.get("my-model", None)?;
    println!("Retrieved {} bytes", retrieved.len());

    Ok(())
}`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="next-steps">Next Steps</h2>
      <div className="grid sm:grid-cols-2 gap-3">
        {[
          { href: "/docs/cli", label: "CLI Reference", desc: "All commands and options" },
          { href: "/docs/formats", label: "Format Support", desc: "23+ supported formats" },
          { href: "/docs/security", label: "Security", desc: "Encryption and compliance" },
          { href: "/docs/api", label: "REST API", desc: "14 REST endpoints" },
        ].map((item) => (
          <a key={item.href} href={item.href} className="block p-4 rounded border border-[var(--color-border)] hover:border-[var(--color-primary)]/50 transition-colors">
            <div className="font-semibold">{item.label}</div>
            <div className="text-sm text-[var(--color-text-secondary)]">{item.desc}</div>
          </a>
        ))}
      </div>
    </>
  );
}
