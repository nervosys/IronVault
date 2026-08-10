export default function ArchitecturePage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Architecture</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        IronVault is built with a layered architecture designed for security, extensibility, and performance.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="overview">System Overview</h2>
      <div className="my-6 p-6 bg-[var(--color-bg-secondary)] rounded-xl border border-[var(--color-border)] font-mono text-sm overflow-x-auto">
        <pre className="text-[var(--color-text-secondary)]">{`┌─────────────────────────────────────────────────────────┐
│                    User Interface                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │ CLI (iv)│  │ REST API │  │  Python  │  │   Web   │ │
│  └──────────┘  └──────────┘  └──────────┘  │Dashboard│ │
│                                             └─────────┘ │
├─────────────────────────────────────────────────────────┤
│                     Vault Core                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │  Vault   │  │  Config  │  │ Version  │  │  Model  │ │
│  │  Logic   │  │ Manager  │  │ Control  │  │  Cards  │ │
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘ │
├─────────────────────────────────────────────────────────┤
│                    Processing                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │  Format  │  │Conversion│  │   RAG    │  │  MCP    │ │
│  │Detection │  │ Pipeline │  │ Engine   │  │  Tools  │ │
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘ │
├─────────────────────────────────────────────────────────┤
│                   Security Layer                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │AES-256-  │  │ Argon2id │  │  Audit   │  │  FIPS   │ │
│  │  GCM     │  │   KDF    │  │   Log    │  │ 140-3   │ │
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘ │
├─────────────────────────────────────────────────────────┤
│                   Storage Backend                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │  Local   │  │  Cloud   │  │ SQLite   │  │  XDG    │ │
│  │   FS     │  │ (S3/Az)  │  │   DB     │  │  Dirs   │ │
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘ │
└─────────────────────────────────────────────────────────┘`}</pre>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="modules">Module Structure</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Module</th>
              <th className="text-left p-3 font-semibold">Purpose</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["vault", "Core vault operations — store, retrieve, delete, lock/unlock"],
              ["crypto", "AES-256-GCM encryption, Argon2id KDF, compression"],
              ["formats", "23+ model format detection and metadata"],
              ["conversion", "Format conversion pipeline with BFS path finding"],
              ["model_card", "Standardized model documentation (JSON/YAML/Markdown)"],
              ["version", "Version control — history, lineage, branching"],
              ["api", "REST API server (axum), JWT auth, OpenAPI"],
              ["rag", "Document store, knowledge base, MCP tools"],
              ["storage", "Local filesystem and cloud storage backends"],
              ["config", "Configuration management, XDG compliance"],
              ["audit", "Security audit logging"],
              ["compliance", "FIPS 140-3, CMMC compliance checks"],
              ["utils", "Archiving, caching, analysis, quantization, pruning"],
              ["cli", "Command-line interface (clap)"],
            ].map(([mod, desc]) => (
              <tr key={mod} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{mod}</code></td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="design">Design Principles</h2>
      <ul className="space-y-3 text-[var(--color-text-secondary)]">
        <li><strong>Security First</strong> — All data encrypted at rest. Memory zeroized after use. No unsafe code.</li>
        <li><strong>Zero Cost Abstraction</strong> — Features behind compile-time flags. API server adds zero cost when unused.</li>
        <li><strong>Cross-Platform</strong> — Windows, Linux, macOS with XDG Base Directory compliance everywhere.</li>
        <li><strong>Type Safety</strong> — Rust&apos;s type system prevents entire categories of runtime errors.</li>
        <li><strong>Extensible</strong> — Custom format converters, storage backends, and MCP tools can be plugged in.</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="data-flow">Data Flow</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        When a model is stored, it flows through these stages:
      </p>
      <ol className="list-decimal list-inside space-y-2 text-[var(--color-text-secondary)]">
        <li><strong>Format Detection</strong> — Magic bytes identify the model format automatically</li>
        <li><strong>Metadata Extraction</strong> — Size, parameters, framework info captured</li>
        <li><strong>Compression</strong> — Optional gzip or LZMA compression (configurable)</li>
        <li><strong>Encryption</strong> — AES-256-GCM with unique nonce per operation</li>
        <li><strong>Version Stamping</strong> — SHA-256 checksum, version number assigned</li>
        <li><strong>Storage</strong> — Written to local filesystem or cloud backend</li>
        <li><strong>Audit Log</strong> — Operation recorded with timestamp and user context</li>
      </ol>
    </>
  );
}
