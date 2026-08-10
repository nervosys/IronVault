import CodeBlock from "@/components/DocElements";

export default function QuantizationPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Quantization Pipeline</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Profile-based quantization management for AI/ML models. Create named quantization profiles
        with specific methods, estimate output sizes, and apply consistent quantization across model versions.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Create a quantization profile
iv quantize set fast-q4 --method q4_k_m --description "Fast 4-bit quantization"

# List profiles
iv quantize list

# Estimate output size (1GB model → Q4_K_M)
iv quantize estimate --size 1000000000 --to q4_k_m

# Remove a profile
iv quantize remove fast-q4`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="methods">Quantization Methods</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Method</th>
              <th className="text-left p-3 font-semibold">Bits/Weight</th>
              <th className="text-left p-3 font-semibold">Use Case</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["q4_0", "4.0", "Maximum compression, lower quality"],
              ["q4_k_m", "4.5", "Good balance of size and quality"],
              ["q5_k_m", "5.5", "Higher quality, moderate compression"],
              ["q8_0", "8.0", "Near-lossless, minimal compression"],
              ["f16", "16.0", "Half precision"],
              ["f32", "32.0", "Full precision (no quantization)"],
            ].map(([method, bits, useCase]) => (
              <tr key={method} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{method}</code></td>
                <td className="p-3">{bits}</td>
                <td className="p-3">{useCase}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cli">CLI Reference</h2>
      <CodeBlock language="bash">{`iv quantize <COMMAND>

Commands:
  set       Create or update a quantization profile
  remove    Remove a quantization profile
  list      List quantization profiles
  estimate  Estimate output size for a quantization method`}</CodeBlock>

      <h3 className="text-xl font-bold mt-8 mb-3">Set Profile</h3>
      <CodeBlock language="bash">{`iv quantize set <NAME> --method <METHOD> [--description <DESC>]`}</CodeBlock>

      <h3 className="text-xl font-bold mt-8 mb-3">Estimate Size</h3>
      <CodeBlock language="bash">{`iv quantize estimate --size <BYTES> --to <METHOD> [--from <METHOD>]`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="python">Python API</h2>
      <CodeBlock language="python">{`from ironvault import QuantProfileStore

store = QuantProfileStore("/path/to/vault")
store.set("fast-q4", "q4_k_m", "Fast 4-bit quantization")
profiles = store.list()
store.remove("fast-q4")

# Static method — estimate size
estimated = QuantProfileStore.estimate(1_000_000_000, "f32", "q4_k_m")`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="api">REST API</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Method</th>
              <th className="text-left p-3 font-semibold">Path</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["GET", "/api/v1/quantization/profiles", "List all profiles"],
              ["POST", "/api/v1/quantization/profiles", "Create/update a profile"],
              ["POST", "/api/v1/quantization/estimate", "Estimate quantized size"],
            ].map(([method, path, desc]) => (
              <tr key={`${method}-${path}`} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{method}</code></td>
                <td className="p-3 font-medium text-[var(--color-text)]"><code className="text-xs">{path}</code></td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h3 className="text-xl font-bold mt-8 mb-3">Example: Create Profile</h3>
      <CodeBlock language="bash">{`curl -X POST http://localhost:8080/api/v1/quantization/profiles \\
  -H "Authorization: Bearer $TOKEN" \\
  -H "Content-Type: application/json" \\
  -d '{"name": "fast-q4", "method": "q4_k_m", "description": "Fast 4-bit"}'`}</CodeBlock>
    </>
  );
}
