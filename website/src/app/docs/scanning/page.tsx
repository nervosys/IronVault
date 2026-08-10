import CodeBlock from "@/components/DocElements";

export default function ScanningPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Safety Scanning</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Static analysis of PyTorch/pickle files for dangerous opcodes and malicious patterns.
        Detects common supply-chain attack vectors in serialized Python objects.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Scan a vault model
iv scan my-model

# Scan a file on disk
iv scan --file ./model.pt

# JSON output for CI/CD
iv scan --file ./model.pt --format json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="opcodes">Dangerous Opcodes</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        The scanner checks for 7 pickle opcodes that can execute arbitrary code:
      </p>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Opcode</th>
              <th className="text-left p-3 font-semibold">Risk</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["REDUCE", "Critical", "Calls arbitrary callable with args"],
              ["GLOBAL", "Critical", "Imports any module/attribute"],
              ["BUILD", "Warning", "Calls __setstate__ on objects"],
              ["INST", "Critical", "Instantiates arbitrary classes"],
              ["NEWOBJ", "Critical", "Creates new objects via __new__"],
              ["NEWOBJ_EX", "Critical", "Extended object creation"],
              ["STACK_GLOBAL", "Critical", "Dynamic module import from stack"],
            ].map(([opcode, risk, desc]) => (
              <tr key={opcode} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{opcode}</code></td>
                <td className="p-3">
                  <span className={risk === "Critical" ? "text-red-400 font-semibold" : "text-yellow-400 font-semibold"}>{risk}</span>
                </td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="patterns">Dangerous Patterns</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        12 string patterns associated with malicious pickle payloads:
      </p>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">os</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">subprocess</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">__builtin__</code> — system command execution</li>
        <li>• <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">exec</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">eval</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">__import__</code> — dynamic code execution</li>
        <li>• <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">socket</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">http</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">urllib</code> — network access</li>
        <li>• <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">shutil</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">tempfile</code>, <code className="px-1 bg-[var(--color-bg-secondary)] rounded text-xs">ctypes</code> — file system and memory manipulation</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="options">CLI Options</h2>
      <CodeBlock language="bash">{`iv scan [<NAME>] [OPTIONS]

Options:
  --file <PATH>       Scan a file on disk
  -v, --version <V>   Model version
  -f, --format <FMT>  Output format: text (default) or json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="ci-cd">CI/CD Integration</h2>
      <CodeBlock language="bash">{`# Fail pipeline if model is unsafe
iv scan --file model.pt --format json | jq -e '.safe == true'`}</CodeBlock>
    </>
  );
}
