import CodeBlock from "@/components/DocElements";

export default function InstallationPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Installation</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Multiple ways to install IronVault depending on your workflow.
      </p>

      <h2 className="text-2xl font-bold mt-8 mb-4" id="requirements">Requirements</h2>
      <ul className="list-disc list-inside space-y-1 text-[var(--color-text-secondary)] mb-6">
        <li>Rust 1.75+ (for building from source)</li>
        <li>Python 3.9+ (for Python bindings)</li>
      </ul>

      <h2 className="text-2xl font-bold mt-8 mb-4" id="cargo">Via Cargo (Recommended)</h2>
      <CodeBlock language="bash">{`cargo install ironvault`}</CodeBlock>
      <p className="text-[var(--color-text-secondary)] mt-2 mb-4">
        This installs the <code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-sm">iv</code> binary
        with default features (SafeTensors, ndarray, SQLite).
      </p>

      <h3 className="text-lg font-semibold mt-6 mb-2">With API server</h3>
      <CodeBlock language="bash">{`cargo install ironvault --features api`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">With all features</h3>
      <CodeBlock language="bash">{`cargo install ironvault --features full`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-8 mb-4" id="source">From Source</h2>
      <CodeBlock language="bash">{`git clone https://github.com/nervosys/IronVault.git
cd IronVault
cargo build --release

# Run directly
./target/release/iv --help

# Or install to PATH
cargo install --path .`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-8 mb-4" id="python">Python Package</h2>
      <CodeBlock language="bash">{`# Base package
pip install ironvault

# With ML frameworks
pip install "ironvault[ml]"

# With development tools
pip install "ironvault[dev]"

# With security auditing
pip install "ironvault[security]"`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-8 mb-4" id="containers">Containers</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        There is no first-party container image. The Dockerfile and Helm chart
        were removed in 4.5.0 — <code>iv</code> ships as a static binary, a
        crate, and a Python wheel. Images published to <code>ghcr.io</code>
        before then remain pullable but are no longer updated.
      </p>

      <h2 className="text-2xl font-bold mt-8 mb-4" id="features">Feature Flags</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Feature</th>
              <th className="text-left p-3 font-semibold">Default</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["default", "Yes", "SafeTensors, ndarray, SQLite"],
              ["api", "No", "REST API server with axum"],
              ["python", "No", "PyO3 Python bindings"],
              ["cloud", "No", "AWS S3 & Azure Blob Storage"],
              ["full", "No", "All features combined"],
              ["sqlite", "Yes", "SQLite database backend"],
            ].map(([feature, def, desc]) => (
              <tr key={feature} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{feature}</code></td>
                <td className="p-3">{def}</td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-8 mb-4" id="verify">Verify Installation</h2>
      <CodeBlock language="bash">{`iv --version
# iv 1.0.0

iv --help`}</CodeBlock>
    </>
  );
}
