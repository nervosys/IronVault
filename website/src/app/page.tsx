import Link from "next/link";
import { FeatureCard } from "@/components/DocElements";
import VideoCard from "@/components/VideoCard";

export default function HomePage() {
  return (
    <div className="min-h-[calc(100vh-var(--header-height))]">
      {/* Hero — Vault Entry */}
      <section className="relative overflow-hidden bg-gradient-to-b from-[#0d1117] via-[#161b22] to-[#0d1117] text-white">
        {/* Tactical grid */}
        <div className="absolute inset-0 bg-[linear-gradient(rgba(74,222,128,0.015)_1px,transparent_1px),linear-gradient(90deg,rgba(74,222,128,0.015)_1px,transparent_1px)] bg-[size:48px_48px]" />
        {/* Radial vault glow */}
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_600px_400px_at_center,rgba(74,222,128,0.04)_0%,transparent_100%)]" />
        {/* Scanlines */}
        <div className="absolute inset-0 scanline" />
        {/* Top classified stripe */}
        <div className="absolute top-0 left-0 right-0 h-[3px] bg-gradient-to-r from-transparent via-red-600/60 to-transparent" />

        <div className="relative max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-28 sm:py-36">
          <div className="text-center">
            {/* Classification badge */}
            <div className="inline-flex items-center gap-2 px-3 py-1 rounded text-sm font-mono font-bold uppercase tracking-widest bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 mb-6">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
              v1.5.0 — Operational
            </div>

            {/* Vault door icon */}
            <div className="flex justify-center mb-8">
              <div className="relative w-20 h-20 rounded-full border-2 border-emerald-500/30 flex items-center justify-center">
                <div className="absolute inset-2 rounded-full border border-emerald-500/15" />
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="rgba(74,222,128,0.7)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                  <path d="M7 11V7a5 5 0 0110 0v4" />
                  <circle cx="12" cy="16" r="1" />
                </svg>
                <div className="absolute inset-0 rounded-full bg-emerald-500/5 animate-ping opacity-20" />
              </div>
            </div>

            <h1 className="text-4xl sm:text-5xl lg:text-7xl font-bold tracking-tight mb-6 vault-text">
              IronVault
            </h1>
            <p className="text-xl sm:text-2xl text-gray-400 max-w-3xl mx-auto mb-10 leading-relaxed">
              Encrypted storage, versioning, and lifecycle management for AI models.
              FIPS 140-3 compliant. 23+ formats. Zero-trust architecture.
            </p>

            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <Link
                href="/docs/quickstart"
                className="inline-flex items-center justify-center px-7 py-3.5 rounded bg-emerald-500 text-black font-bold uppercase tracking-wide text-base hover:bg-emerald-400 transition-all shadow-[0_0_20px_-4px_rgba(52,211,153,0.3)] hover:shadow-[0_0_30px_-4px_rgba(52,211,153,0.5)]"
              >
                Enter Vault
                <svg className="ml-2 w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" /></svg>
              </Link>
              <Link
                href="/docs"
                className="inline-flex items-center justify-center px-7 py-3.5 rounded border border-gray-600 text-gray-300 font-semibold text-base uppercase tracking-wide hover:border-emerald-500/40 hover:text-white hover:bg-white/5 transition-all"
              >
                Documentation
              </Link>
            </div>
          </div>

          {/* Install command — terminal style */}
          <div className="mt-14 max-w-xl mx-auto">
            <div className="relative bg-[#0d1117]/90 backdrop-blur-sm rounded border border-[#21262d] p-4 font-mono text-base text-center corner-brackets">
              <span className="text-[#8b949e]">$</span>{" "}
              <span className="text-emerald-400">cargo install</span>{" "}
              <span className="text-[#c9d1d9]">ironvault</span>
            </div>
          </div>
        </div>
      </section>

      {/* Features grid */}
      <section className="relative max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
        <div className="text-center mb-14">
          <span className="inline-block text-xs font-mono font-bold uppercase tracking-[0.25em] text-[var(--color-primary)] opacity-70 mb-3">
            Capabilities
          </span>
          <h2 className="text-4xl font-bold vault-text">Secure Model Lifecycle</h2>
          <p className="text-lg text-[var(--color-text-secondary)] mt-4 max-w-2xl mx-auto">
            From encryption to deployment — a complete toolkit for managing AI models
            across their entire lifecycle with military-grade security.
          </p>
        </div>
        <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6">
          <FeatureCard
            icon="🔐"
            title="FIPS 140-3 Encryption"
            description="AES-256-GCM with Argon2id key derivation. Military-grade protection for your valuable model IP."
            href="/docs/security"
          />
          <FeatureCard
            icon="🎯"
            title="23+ Format Support"
            description="SafeTensors, GGUF, PyTorch, ONNX, TensorRT, Core ML, TFLite, and many more with auto-detection."
            href="/docs/formats"
          />
          <FeatureCard
            icon="🕐"
            title="Version Control"
            description="Git-like versioning with lineage tracking, branching, time travel, and automatic checksums."
            href="/docs/version-control"
          />
          <FeatureCard
            icon="🔄"
            title="Format Conversion"
            description="10 built-in converters with BFS multi-step path finding. Convert between any supported format."
            href="/docs/conversion"
          />
          <FeatureCard
            icon="🌐"
            title="REST API"
            description="30+ RESTful endpoints with JWT auth, OpenAPI spec, and an embedded web dashboard."
            href="/docs/api"
          />
          <FeatureCard
            icon="🐍"
            title="Python Bindings"
            description="Native PyO3 bindings — use from Python with full type hints and async support."
            href="/docs/python"
          />
          <FeatureCard
            icon="🛡️"
            title="Hardened systemd Install"
            description="One script provisions the service user, 0600 root-owned credentials, and a locked-down unit."
            href="/docs/security-hardening"
          />
          <FeatureCard
            icon="☁️"
            title="Cloud Storage"
            description="Push and pull models to AWS S3 and Azure Blob Storage with end-to-end encryption."
            href="/docs/cloud"
          />
          <FeatureCard
            icon="🤖"
            title="RAG & MCP Tools"
            description="Built-in document store, knowledge base, vector search, and Model Context Protocol agents."
            href="/docs/rag"
          />
          <FeatureCard
            icon="📥"
            title="Model Download"
            description="Pull models from HuggingFace Hub, Ollama, or URLs with SHA-256 integrity verification."
            href="/docs/download"
          />
          <FeatureCard
            icon="✍️"
            title="Model Signing"
            description="HMAC-SHA256 signatures with detached .sig files for tamper detection and provenance."
            href="/docs/signing"
          />
          <FeatureCard
            icon="🛡️"
            title="Safety Scanning"
            description="Detect dangerous pickle opcodes and malicious patterns in PyTorch model files."
            href="/docs/scanning"
          />
          <FeatureCard
            icon="🔍"
            title="Model Diffing"
            description="Compare model versions at the tensor level — shapes, dtypes, and parameter deltas."
            href="/docs/diffing"
          />
          <FeatureCard
            icon="🔗"
            title="Engine Interop"
            description="Register models with Ollama and LM Studio for local inference and experimentation."
            href="/docs/engine-interop"
          />
          <FeatureCard
            icon="📊"
            title="Benchmarks"
            description="Attach and query benchmark scores per model version with hardware context."
            href="/docs/benchmarks"
          />
          <FeatureCard
            icon="📜"
            title="License Scanning"
            description="Detect licenses from model cards, GGUF metadata, and config files with SPDX normalization."
            href="/docs/license-scanning"
          />
          <FeatureCard
            icon="🏷️"
            title="Tags & Search"
            description="Tag models with labels and key-value annotations, then search by name, tags, or metadata."
            href="/docs/tags"
          />
          <FeatureCard
            icon="📦"
            title="Vault Export/Import"
            description="Portable tar.gz vault bundles for backup, migration, and selective model export."
            href="/docs/vault-bundle"
          />
          <FeatureCard
            icon="🧹"
            title="Garbage Collection"
            description="Detect orphaned blobs, clean temp files, and reclaim disk space with dry-run support."
            href="/docs/gc"
          />
          <FeatureCard
            icon="📟"
            title="TUI Dashboard"
            description="Browse vault contents in a rich terminal UI with keyboard navigation and live search."
            href="/docs/tui"
          />
          <FeatureCard
            icon="🔔"
            title="Webhooks"
            description="HTTP notification system for store, delete, and version events with HMAC-signed payloads."
            href="/docs/webhooks"
          />
          <FeatureCard
            icon="🛂"
            title="Access Control"
            description="Role-based ACL (Reader/Writer/Admin) per principal with grant, revoke, and check commands."
            href="/docs/access-control"
          />
          <FeatureCard
            icon="🔑"
            title="KMS Integration"
            description="Fetch secrets from env, AWS Secrets Manager, Azure Key Vault, or HashiCorp Vault."
            href="/docs/kms"
          />
          <FeatureCard
            icon="✅"
            title="Model Validation"
            description="Integrity probes with SHA-256 checksums per model version for tamper detection."
            href="/docs/validation"
          />
          <FeatureCard
            icon="📋"
            title="Retention Policies"
            description="Configurable max versions, max age, and keep-minimum with dry-run enforcement."
            href="/docs/policies"
          />
          <FeatureCard
            icon="🌳"
            title="Lineage DAG"
            description="Cross-model directed acyclic graph tracking fine-tune, merge, and distill chains."
            href="/docs/lineage-graph"
          />
          <FeatureCard
            icon="🧩"
            title="Plugins"
            description="Discover, install, and manage plugins with JSON manifests for extensibility."
            href="/docs/plugins"
          />
          <FeatureCard
            icon="⚙️"
            title="Config Profiles"
            description="Named configuration profiles with activate/deactivate switching and key-value overrides."
            href="/docs/profiles"
          />
        </div>
      </section>

      {/* CLI Demo Videos */}
      <section className="relative bg-[var(--color-bg-secondary)] border-y border-[var(--color-border)] theme-transition">
        <div className="absolute inset-0 tactical-grid opacity-40" />
        <div className="relative max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
          <div className="text-center mb-14">
            <span className="inline-block text-xs font-mono font-bold uppercase tracking-[0.25em] text-[var(--color-primary)] opacity-70 mb-3">
              Operations
            </span>
            <h2 className="text-4xl font-bold vault-text">See It in Action</h2>
            <p className="text-lg text-[var(--color-text-secondary)] mt-4 max-w-2xl mx-auto">
              Watch quick CLI demos showing real workflows — from vault initialization
              to security compliance audits.
            </p>
          </div>
          <div className="grid lg:grid-cols-2 gap-8">
            <VideoCard
              src="/videos/CLIInit.mp4"
              title="Initialize a Vault"
              description="Create an encrypted vault with AES-256-GCM, unlock it, and check status."
              duration="0:11"
            />
            <VideoCard
              src="/videos/CLIStore.mp4"
              title="Store & List Models"
              description="Store multiple models with auto-format detection and list vault contents."
              duration="0:14"
            />
            <VideoCard
              src="/videos/CLIVersions.mp4"
              title="Version Control"
              description="Track version history, rollback to previous versions, and view lineage."
              duration="0:16"
            />
            <VideoCard
              src="/videos/CLIConvert.mp4"
              title="Format Conversion"
              description="Convert models between formats with quantization — GGUF, ONNX, and more."
              duration="0:13"
            />
            <div className="lg:col-span-2 flex justify-center">
              <div className="w-full lg:w-1/2">
                <VideoCard
                  src="/videos/CLICompliance.mp4"
                  title="Security Compliance"
                  description="Run a full security audit with 12 checks and review the audit log."
                  duration="0:11"
                />
              </div>
            </div>
          </div>
          <div className="text-center mt-10">
            <Link
              href="/demos"
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded border border-[var(--color-border)] text-base font-mono font-medium uppercase tracking-wider hover:border-[var(--color-primary)]/50 hover:text-[var(--color-primary)] hover:shadow-[0_0_15px_-4px_var(--color-glow)] transition-all"
            >
              View All Demos
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" /></svg>
            </Link>
          </div>
        </div>
      </section>

      {/* At a Glance — Stats */}
      <section className="relative bg-[var(--color-bg)] border-y border-[var(--color-border)] theme-transition">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
          <div className="text-center mb-14">
            <span className="inline-block text-xs font-mono font-bold uppercase tracking-[0.25em] text-[var(--color-primary)] opacity-70 mb-3">
              Status Report
            </span>
            <h2 className="text-4xl font-bold vault-text">At a Glance</h2>
          </div>
          <div className="grid md:grid-cols-3 gap-6 text-center">
            {[
              { value: "1,917+", label: "Tests Passing", status: "●" },
              { value: "23+", label: "Model Formats", status: "●" },
              { value: "30+", label: "API Endpoints", status: "●" },
            ].map((stat) => (
              <div
                key={stat.label}
                className="relative p-8 rounded border border-[var(--color-border)] bg-[var(--color-surface)] glow-border corner-brackets"
              >
                <div className="text-5xl font-bold font-mono text-[var(--color-primary)] mb-3 vault-text">
                  {stat.value}
                </div>
                <div className="text-xs uppercase tracking-[0.2em] text-[var(--color-text-secondary)] font-mono font-medium">
                  {stat.label}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-[var(--color-border)] bg-[var(--color-bg)] theme-transition">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-10">
          <div className="flex flex-col md:flex-row justify-between items-center gap-6">
            <div className="text-sm text-[var(--color-text-secondary)] font-mono uppercase tracking-wider">
              &copy; 2025-2026 NERVOSYS &middot; AGPL-3.0-or-later
            </div>
            <div className="flex gap-6 text-sm font-mono uppercase tracking-wider">
              <a href="https://github.com/nervosys/IronVault" className="text-[var(--color-text-secondary)] hover:text-[var(--color-primary)] transition-colors">GitHub</a>
              <a href="https://crates.io/crates/ironvault" className="text-[var(--color-text-secondary)] hover:text-[var(--color-primary)] transition-colors">Crates.io</a>
              <a href="https://pypi.org/project/ironvault" className="text-[var(--color-text-secondary)] hover:text-[var(--color-primary)] transition-colors">PyPI</a>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}
