import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function ExamplesPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Examples</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Runnable examples ship in the <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">examples/</code> directory.
        Each demonstrates a focused slice of IronVault functionality and
        can be executed directly with Cargo.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="catalog">Example Catalog</h2>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Example</th>
              <th className="text-left py-2 pr-4 font-semibold">Run command</th>
              <th className="text-left py-2 font-semibold">Topics</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">basic_usage</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example basic_usage</td><td className="py-2">Vault init, store, retrieve, list, delete</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">version_control_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example version_control_demo</td><td className="py-2">Versioning, lineage, rollback, diff</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">security_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example security_demo</td><td className="py-2">Encryption, key derivation, compliance</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">rag_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example rag_demo</td><td className="py-2">Document store, search, embeddings</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">mcp_tools_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example mcp_tools_demo</td><td className="py-2">MCP server, tool registration, execution</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">model_card_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example model_card_demo</td><td className="py-2">Model cards, metadata, export</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">providers_formats_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example providers_formats_demo</td><td className="py-2">Format detection, 23+ formats</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">utilities_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example utilities_demo</td><td className="py-2">Archive, extract, analyze, deduplicate</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">xdg_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example xdg_demo</td><td className="py-2">XDG directory compliance, config paths</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">huggingface_demo</td><td className="py-2 pr-4 font-mono text-xs">cargo run --example huggingface_demo</td><td className="py-2">HuggingFace integration, model download</td></tr>
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="basic-usage">Basic Usage</h2>
      <p className="mb-4 text-[var(--color-text-secondary)]">
        The starting point. Creates a vault, stores a model, retrieves it, and
        cleans up. Covers the core lifecycle every user needs.
      </p>
      <CodeBlock language="bash">{`cargo run --example basic_usage`}</CodeBlock>
      <p className="mt-2 mb-6 text-[var(--color-text-secondary)]">
        Demonstrates: <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">Vault::init</code>,
        <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">store</code>,
        <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">get</code>,
        <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">list</code>,
        <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">delete</code>.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="version-control">Version Control Demo</h2>
      <p className="mb-4 text-[var(--color-text-secondary)]">
        Stores multiple versions of the same model, queries the lineage tree,
        and rolls back to a previous version.
      </p>
      <CodeBlock language="bash">{`cargo run --example version_control_demo`}</CodeBlock>
      <p className="mt-2 mb-6 text-[var(--color-text-secondary)]">
        Demonstrates: <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">versions</code>,
        <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">lineage</code>,
        <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">rollback</code>,
        <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">diff</code>.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="security">Security Demo</h2>
      <p className="mb-4 text-[var(--color-text-secondary)]">
        Walks through the cryptographic pipeline: Argon2id key derivation,
        AES-256-GCM encryption, SHA-256 integrity checks, and a compliance
        self-audit.
      </p>
      <CodeBlock language="bash">{`cargo run --example security_demo`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rag">RAG Demo</h2>
      <p className="mb-4 text-[var(--color-text-secondary)]">
        Initialises an SQLite document store, ingests several documents,
        generates embeddings, and performs semantic similarity searches.
      </p>
      <CodeBlock language="bash">{`cargo run --example rag_demo`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="mcp-tools">MCP Tools Demo</h2>
      <p className="mb-4 text-[var(--color-text-secondary)]">
        Spins up an in-process MCP server, registers custom tools, and executes
        tool calls programmatically — useful for agent integrations.
      </p>
      <CodeBlock language="bash">{`cargo run --example mcp_tools_demo`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="learning-paths">Learning Paths</h2>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Path</th>
              <th className="text-left py-2 pr-4 font-semibold">Examples (in order)</th>
              <th className="text-left py-2 font-semibold">Focus</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-semibold">Beginner</td><td className="py-2 pr-4">basic_usage &rarr; version_control_demo &rarr; utilities_demo</td><td className="py-2">Core vault operations</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-semibold">AI Apps</td><td className="py-2 pr-4">rag_demo &rarr; mcp_tools_demo &rarr; huggingface_demo</td><td className="py-2">RAG, agents, model hub</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-semibold">Enterprise</td><td className="py-2 pr-4">security_demo &rarr; model_card_demo &rarr; xdg_demo</td><td className="py-2">Security, compliance, config</td></tr>
          </tbody>
        </table>
      </div>
      <Callout type="tip" title="Next step">
        After running the examples, explore the full CLI reference at{" "}
        <a href="/docs/cli" className="text-[var(--color-primary)] underline">CLI Reference</a>.
      </Callout>
    </>
  );
}
