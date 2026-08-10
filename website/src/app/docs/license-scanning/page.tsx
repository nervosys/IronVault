import CodeBlock from "@/components/DocElements";

export default function LicenseScanningPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">License Scanning</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Detect licenses from model cards, GGUF metadata, config files, and LICENSE files with SPDX
        normalization and permissiveness classification.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Scan a directory
iv license-scan ./my-model/

# Scan a single file
iv license-scan model.gguf

# JSON output
iv license-scan ./my-model/ --format json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="options">CLI Options</h2>
      <CodeBlock language="bash">{`iv license-scan <PATH> [OPTIONS]

Arguments:
  <PATH>              File or directory to scan

Options:
  -f, --format <FMT>  Output format: text (default) or json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="sources">Detection Sources</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Source</th>
              <th className="text-left p-3 font-semibold">What It Scans</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["Model Card", "YAML frontmatter in README.md (license: field)"],
              ["GGUF Metadata", "License string in GGUF file header"],
              ["LICENSE File", "Full text matching of LICENSE/LICENSE.md files"],
              ["Config File", "license field in config.json"],
            ].map(([source, desc]) => (
              <tr key={source} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]">{source}</td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="classification">License Classification</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Class</th>
              <th className="text-left p-3 font-semibold">Examples</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["Permissive", "MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause"],
              ["Copyleft", "GPL-2.0, GPL-3.0, AGPL-3.0, LGPL-3.0"],
              ["Restricted", "CC-BY-NC-4.0, CC-BY-NC-SA-4.0, Llama-2, Llama-3"],
              ["Proprietary", "Proprietary, custom license terms"],
              ["Unknown", "Unrecognized license identifiers"],
            ].map(([cls, examples]) => (
              <tr key={cls} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]">{cls}</td>
                <td className="p-3">{examples}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="spdx">SPDX Normalization</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Raw license strings are normalized to standard SPDX identifiers. 25 license families are recognized:
      </p>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Raw Input</th>
              <th className="text-left p-3 font-semibold">SPDX Output</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["mit", "MIT"],
              ["apache 2.0", "Apache-2.0"],
              ["gpl-3", "GPL-3.0-only"],
              ["cc-by-nc-4.0", "CC-BY-NC-4.0"],
              ["llama2", "Llama-2"],
            ].map(([raw, spdx]) => (
              <tr key={raw} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-mono text-xs">{raw}</td>
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{spdx}</code></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
