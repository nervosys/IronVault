import CodeBlock from "@/components/DocElements";

export default function PythonPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Python Bindings</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Native PyO3 bindings for using IronVault from Python.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="install">Installation</h2>
      <CodeBlock language="bash">{`pip install ironvault

# With ML framework support
pip install "ironvault[ml]"

# With development tools
pip install "ironvault[dev]"`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="basic">Basic Usage</h2>
      <CodeBlock language="python">{`from ironvault import Vault, VaultConfig, ModelFormat

# Create a vault with XDG-compliant paths
config = VaultConfig()
vault = Vault("my-vault", config)

# Unlock with passphrase
vault.unlock("my-secure-passphrase")

# Store a model
with open("model.safetensors", "rb") as f:
    data = f.read()
vault.store_model("gpt2", data, "safetensors", description="Fine-tuned GPT-2")

# List models
models = vault.list_models()
for model in models:
    print(f"{model.name} ({model.format})")

# Retrieve a model
data = vault.get_model("gpt2")
# Or a specific version
data_v1 = vault.get_model("gpt2", version=1)

# Version control
versions = vault.list_versions("gpt2")
lineage = vault.get_lineage("gpt2", version=2)

# Lock when done
vault.lock()`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="classes">Available Classes</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Class</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["Vault", "Main vault with store, get, list, delete, version control, and passphrase management"],
              ["VaultConfig", "XDG-compliant configuration with optional custom vault directory"],
              ["ModelFormat", "23+ format detection with name and extension properties"],
              ["ModelMetadata", "Builder-style constructor for model metadata (name, format, description, framework, task)"],
            ].map(([cls, desc]) => (
              <tr key={cls} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{cls}</code></td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="metadata">Model Metadata</h2>
      <CodeBlock language="python">{`from ironvault import ModelMetadata

metadata = ModelMetadata(
    name="my-model",
    format="safetensors",
    description="Fine-tuned for code generation",
    framework="pytorch",
    task="text-generation",
    architecture="transformer",
    parameters=7_000_000_000,
)

vault.store_model("my-model", data, metadata=metadata)`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="stats">Vault Statistics</h2>
      <CodeBlock language="python">{`stats = vault.get_stats()
print(f"Models: {stats.model_count}")
print(f"Versions: {stats.version_count}")
print(f"Total size: {stats.total_size_formatted}")`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="building">Building from Source</h2>
      <CodeBlock language="bash">{`# Requires Rust 1.75+ and maturin
pip install maturin

# Build and install
cd ironvault
maturin develop --features python

# Build wheel
maturin build --features python --release`}</CodeBlock>
    </>
  );
}
