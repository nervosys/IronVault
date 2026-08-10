import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function FeatureFlagsPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Feature Flags</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Cargo feature flags keep the default binary lean while enabling optional
        capabilities such as cloud storage, database backends, and API serving.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="default-features">Default Features</h2>
      <p className="mb-4">Enabled automatically with <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">cargo build</code>:</p>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Feature</th>
              <th className="text-left py-2 pr-4 font-semibold">Crate</th>
              <th className="text-left py-2 font-semibold">Purpose</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">safetensors</td><td className="py-2 pr-4">safetensors</td><td className="py-2">SafeTensors format read/write</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">ndarray</td><td className="py-2 pr-4">ndarray</td><td className="py-2">N-dimensional array support for tensors</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">sqlite</td><td className="py-2 pr-4">rusqlite</td><td className="py-2">SQLite backend for RAG and versioning</td></tr>
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="database-backends">Database Backends</h2>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Feature</th>
              <th className="text-left py-2 pr-4 font-semibold">Crate</th>
              <th className="text-left py-2 pr-4 font-semibold">Purpose</th>
              <th className="text-left py-2 font-semibold">Build Command</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">kv-store</td><td className="py-2 pr-4">sled</td><td className="py-2 pr-4">Sled embedded KV store for RAG</td><td className="py-2 font-mono text-xs">cargo build --features kv-store</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">vector-db</td><td className="py-2 pr-4">qdrant-client</td><td className="py-2 pr-4">Qdrant vector database for RAG</td><td className="py-2 font-mono text-xs">cargo build --features vector-db</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">database</td><td className="py-2 pr-4">(meta)</td><td className="py-2 pr-4">sqlite + kv-store together</td><td className="py-2 font-mono text-xs">cargo build --features database</td></tr>
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cloud-storage">Cloud Storage</h2>
      <div className="overflow-x-auto mb-4">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Feature</th>
              <th className="text-left py-2 pr-4 font-semibold">Crate</th>
              <th className="text-left py-2 pr-4 font-semibold">Purpose</th>
              <th className="text-left py-2 font-semibold">Build Command</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">s3</td><td className="py-2 pr-4">aws-config, aws-sdk-s3</td><td className="py-2 pr-4">AWS S3 backend</td><td className="py-2 font-mono text-xs">cargo build --features s3</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">azure</td><td className="py-2 pr-4">azure_storage, azure_storage_blobs</td><td className="py-2 pr-4">Azure Blob backend</td><td className="py-2 font-mono text-xs">cargo build --features azure</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">cloud</td><td className="py-2 pr-4">(meta)</td><td className="py-2 pr-4">All cloud backends</td><td className="py-2 font-mono text-xs">cargo build --features cloud</td></tr>
          </tbody>
        </table>
      </div>
      <Callout type="warning" title="GCS disabled">
        GCS support is currently disabled due to security vulnerabilities in upstream dependencies.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="api-server">API Server</h2>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Feature</th>
              <th className="text-left py-2 pr-4 font-semibold">Crates</th>
              <th className="text-left py-2 pr-4 font-semibold">Purpose</th>
              <th className="text-left py-2 font-semibold">Build Command</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">api</td><td className="py-2 pr-4">axum, tower, jsonwebtoken, utoipa</td><td className="py-2 pr-4">REST API with JWT auth</td><td className="py-2 font-mono text-xs">cargo build --features api</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">graphql</td><td className="py-2 pr-4">async-graphql (+ api)</td><td className="py-2 pr-4">GraphQL endpoint</td><td className="py-2 font-mono text-xs">cargo build --features graphql</td></tr>
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="acceleration-bindings">Acceleration &amp; Bindings</h2>
      <div className="overflow-x-auto mb-4">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Feature</th>
              <th className="text-left py-2 pr-4 font-semibold">Crate</th>
              <th className="text-left py-2 pr-4 font-semibold">Purpose</th>
              <th className="text-left py-2 font-semibold">Build Command</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">python</td><td className="py-2 pr-4">pyo3</td><td className="py-2 pr-4">Python native bindings</td><td className="py-2 font-mono text-xs">maturin develop --features python</td></tr>
          </tbody>
        </table>
      </div>
      <Callout type="info" title="System dependencies">
        python requires a Python interpreter.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="meta-features">Meta Features</h2>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Feature</th>
              <th className="text-left py-2 pr-4 font-semibold">Includes</th>
              <th className="text-left py-2 font-semibold">Purpose</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">default</td><td className="py-2 pr-4">safetensors, ndarray, sqlite</td><td className="py-2">Minimal working set</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono">full</td><td className="py-2 pr-4">safetensors, ndarray, sqlite, sled, vector-db</td><td className="py-2">All non-system features</td></tr>
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="build-recipes">Common Build Recipes</h2>
      <CodeBlock language="bash">{`# Default (SafeTensors + SQLite)
cargo build --release

# Everything except system-dependent features
cargo build --release --features full

# API server with cloud storage
cargo build --release --features api,cloud

# Full API + GraphQL server
cargo build --release --features graphql,cloud

# All features (except Python needs maturin)
cargo build --release --features full,api,graphql,cloud

# Python bindings
pip install maturin
maturin develop --features python`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="environment-variables">Environment Variables</h2>
      <div className="overflow-x-auto mb-6">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left py-2 pr-4 font-semibold">Variable</th>
              <th className="text-left py-2 pr-4 font-semibold">Feature</th>
              <th className="text-left py-2 font-semibold">Purpose</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono text-xs">IRONVAULT_SQLITE_VERSIONS</td><td className="py-2 pr-4">sqlite</td><td className="py-2">Use SQLite for version storage</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono text-xs">AWS_ACCESS_KEY_ID</td><td className="py-2 pr-4">s3</td><td className="py-2">AWS credentials</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono text-xs">AWS_SECRET_ACCESS_KEY</td><td className="py-2 pr-4">s3</td><td className="py-2">AWS credentials</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono text-xs">AWS_REGION</td><td className="py-2 pr-4">s3</td><td className="py-2">AWS region</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono text-xs">AZURE_STORAGE_ACCOUNT</td><td className="py-2 pr-4">azure</td><td className="py-2">Azure credentials</td></tr>
            <tr className="border-b border-[var(--color-border)]"><td className="py-2 pr-4 font-mono text-xs">AZURE_STORAGE_KEY</td><td className="py-2 pr-4">azure</td><td className="py-2">Azure credentials</td></tr>
          </tbody>
        </table>
      </div>
    </>
  );
}
