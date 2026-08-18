import CodeBlock from "@/components/DocElements";

export default function UtilitiesPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Utilities</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Model analysis, caching, archival, and other convenience tools.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="analysis">Model Analysis</h2>
      <CodeBlock language="bash">{`# Get model information
iv info my-model

# Show detailed statistics
iv stats

# Verify model integrity
iv verify my-model`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cache">Cache Management</h2>
      <CodeBlock language="bash">{`# Show cache usage
iv cache info

# Clear cache
iv cache clear

# Set cache size limit (in MB)
iv cache set-limit 1024`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="archive">Model Archival</h2>
      <CodeBlock language="bash">{`# Archive a model (compress and store separately)
iv archive my-model

# List archived models
iv archive list

# Restore from archive
iv archive restore my-model`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cleanup">Cleanup</h2>
      <CodeBlock language="bash">{`# Remove old versions (keep last N)
iv cleanup --keep-versions 3

# Remove orphaned data
iv cleanup --orphans

# Dry run (show what would be deleted)
iv cleanup --keep-versions 3 --dry-run`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="import-export">Import & Export</h2>
      <CodeBlock language="bash">{`# Export a vault (tar.gz, encrypted)
iv export --output vault-backup.tar.gz

# Import a vault backup
iv import vault-backup.tar.gz --target-vault restored-vault`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rust-api">Rust API</h2>
      <CodeBlock language="rust">{`use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::utils::{ModelAnalyzer, ModelArchive, RetrievalOptimizer};
use std::path::Path;

// Analyze a model. analyze() is an associated function and takes the
// model's metadata, not a format string.
let metadata = ModelMetadata::new("my-model".to_string(), ModelFormat::Safetensors);
let analysis = ModelAnalyzer::analyze(&model_data, &metadata);
println!(
    "Format: {:?}, {:.1} MB, estimated params: {:?}",
    analysis.format, analysis.size_mb, analysis.estimated_parameters
);

// Retrieval cache, bounded in bytes
let mut cache = RetrievalOptimizer::new(512 * 1024 * 1024);
cache.cache_model("my-model".to_string(), model_data.clone())?;
if let Some(bytes) = cache.get_cached("my-model") {
    println!("cache hit: {} bytes", bytes.len());
}
let stats = cache.cache_stats();
println!("Cache: {} entries, {} / {} bytes", stats.total_entries, stats.total_size, stats.max_size);

// Archive models into a tar
let bytes_written = ModelArchive::create_tar(
    vec![("my-model".to_string(), model_data)],
    Path::new("./models.tar"),
)?;`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="env">Environment Variables</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Variable</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["IRONVAULT_VAULT_DIR", "Override default vault directory"],
              ["IRONVAULT_LOG_LEVEL", "Logging level (error, warn, info, debug, trace)"],
              ["IRONVAULT_CACHE_LIMIT", "Cache size limit in bytes"],
              ["IRONVAULT_JWT_SECRET", "JWT signing secret for API server"],
              ["IRONVAULT_HOST", "API server bind address"],
              ["IRONVAULT_PORT", "API server port"],
            ].map(([name, desc]) => (
              <tr key={name} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{name}</code></td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
