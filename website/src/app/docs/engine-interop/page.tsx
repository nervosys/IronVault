import CodeBlock from "@/components/DocElements";

export default function EngineInteropPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Engine Interop</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Register vault models with local inference engines — Ollama and LM Studio — for
        direct serving and experimentation.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Register with Ollama
iv register my-model --engine ollama

# Register with custom alias and system prompt
iv register my-model --engine ollama --alias my-assistant --system-prompt "You are helpful."

# Register with LM Studio
iv register my-model --engine lm-studio`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="options">CLI Options</h2>
      <CodeBlock language="bash">{`iv register <NAME> --engine <ENGINE> [OPTIONS]

Options:
  --engine <ENGINE>       Target engine: ollama or lm-studio
  -v, --version <V>       Model version (default: latest)
  --alias <NAME>          Override the registered model name
  --system-prompt <TEXT>  System prompt (Ollama only)`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="ollama">Ollama</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        The register command builds an Ollama Modelfile and runs <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">ollama create</code>:
      </p>
      <ol className="list-decimal list-inside space-y-2 text-[var(--color-text-secondary)]">
        <li>Exports the model from the vault to a temporary GGUF file</li>
        <li>Generates a Modelfile with FROM, SYSTEM, TEMPLATE, and PARAMETER directives</li>
        <li>Runs <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">ollama create &lt;name&gt; -f &lt;Modelfile&gt;</code></li>
        <li>Reports success with the registered model name</li>
      </ol>
      <CodeBlock language="bash">{`# Register and then use with Ollama
iv register llama2-finetuned --engine ollama --alias my-llama
ollama run my-llama`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="lm-studio">LM Studio</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        For LM Studio, the model is copied to the LM Studio models directory:
      </p>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Platform</th>
              <th className="text-left p-3 font-semibold">Default Path</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["Linux", "~/.cache/lm-studio/models/"],
              ["macOS", "~/.cache/lm-studio/models/"],
              ["Windows", "%LOCALAPPDATA%/lm-studio/models/"],
            ].map(([platform, path]) => (
              <tr key={platform} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]">{platform}</td>
                <td className="p-3 font-mono text-xs">{path}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="requirements">Requirements</h2>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• <strong>Ollama</strong> — Must be installed and accessible in PATH</li>
        <li>• <strong>LM Studio</strong> — Must be installed; model directory is auto-detected</li>
        <li>• Models should be in GGUF format for best compatibility</li>
      </ul>
    </>
  );
}
