import CodeBlock from "@/components/DocElements";

export default function DownloadPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Model Download</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Download models from HuggingFace Hub, Ollama registry, or HTTPS URLs with SHA-256 integrity verification.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Download from HuggingFace
iv pull hf://TheBloke/Llama-2-7B-GGUF/llama-2-7b.Q4_K_M.gguf

# Download from Ollama
iv pull ollama://llama2:7b

# Download from URL with checksum verification
iv pull https://example.com/model.safetensors --sha256 abc123...

# Download and auto-store in vault
iv pull hf://user/repo/model.safetensors --store --name my-model`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="sources">Source Formats</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Prefix</th>
              <th className="text-left p-3 font-semibold">Source</th>
              <th className="text-left p-3 font-semibold">Example</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["hf://", "HuggingFace Hub", "hf://TheBloke/repo/file.gguf"],
              ["ollama://", "Ollama Registry", "ollama://llama2:7b"],
              ["https://", "Direct URL", "https://example.com/model.bin"],
            ].map(([prefix, source, example]) => (
              <tr key={prefix} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{prefix}</code></td>
                <td className="p-3">{source}</td>
                <td className="p-3 font-mono text-xs">{example}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="options">CLI Options</h2>
      <CodeBlock language="bash">{`iv pull <SOURCE> [OPTIONS]

Options:
  -o, --output <DIR>     Output directory (default: current directory)
  --sha256 <HASH>        Expected SHA-256 hash for verification
  --token <TOKEN>        HuggingFace API token (or HF_TOKEN env var)
  --store                Auto-store downloaded model in vault
  --name <NAME>          Model name when using --store`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="auth">Authentication</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        For private HuggingFace repositories, provide a token via CLI flag or environment variable:
      </p>
      <CodeBlock language="bash">{`# Via CLI flag
iv pull hf://private/repo/model.safetensors --token hf_xxxxx

# Via environment variable
export HF_TOKEN=hf_xxxxx
iv pull hf://private/repo/model.safetensors`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="security">Security</h2>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• Only HTTPS URLs are accepted — HTTP is rejected</li>
        <li>• SHA-256 checksums are computed for every download</li>
        <li>• Optional <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">--sha256</code> flag enforces integrity verification</li>
        <li>• Downloads are written atomically to prevent partial files</li>
      </ul>
    </>
  );
}
