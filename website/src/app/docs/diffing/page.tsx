import CodeBlock from "@/components/DocElements";

export default function DiffingPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Model Diffing</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Compare model versions at the tensor level — shapes, data types, and parameter counts —
        by parsing headers only (no full model loading required).
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Compare two files
iv diff model_v1.safetensors model_v2.safetensors

# Compare vault model versions
iv diff mymodel@v1 mymodel@v2

# JSON output
iv diff model_v1.gguf model_v2.gguf --format json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="formats">Supported Formats</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Format</th>
              <th className="text-left p-3 font-semibold">Diff Capability</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["SafeTensors", "Full tensor-level diff (shapes, dtypes, params)"],
              ["GGUF", "Full tensor-level diff from metadata headers"],
              ["Other", "File size comparison only"],
            ].map(([fmt, cap]) => (
              <tr key={fmt} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]">{fmt}</td>
                <td className="p-3">{cap}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="references">Model References</h2>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• <strong>File path</strong> — <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">./models/model.safetensors</code></li>
        <li>• <strong>Vault reference</strong> — <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">mymodel@v1</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">mymodel@v2</code></li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="options">CLI Options</h2>
      <CodeBlock language="bash">{`iv diff <LEFT> <RIGHT> [OPTIONS]

Arguments:
  <LEFT>              Left model (file path or name@version)
  <RIGHT>             Right model (file path or name@version)

Options:
  -f, --format <FMT>  Output format: text (default) or json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="output">Output Details</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        The diff report includes:
      </p>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• <strong>Added tensors</strong> — New tensors in the right model with shapes and dtypes</li>
        <li>• <strong>Removed tensors</strong> — Tensors present only in the left model</li>
        <li>• <strong>Changed tensors</strong> — Tensors with different shapes or dtypes, with parameter deltas</li>
        <li>• <strong>Summary</strong> — Total counts, size change percentage, and parameter delta</li>
      </ul>
    </>
  );
}
