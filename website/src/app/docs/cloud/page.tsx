import CodeBlock from "@/components/DocElements";

export default function CloudPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Cloud Storage</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Push and pull models to AWS S3 and Azure Blob Storage with end-to-end encryption.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="providers">Supported Providers</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Provider</th>
              <th className="text-left p-3 font-semibold">Status</th>
              <th className="text-left p-3 font-semibold">Notes</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]">
              <td className="p-3 font-medium text-[var(--color-text)]">AWS S3</td>
              <td className="p-3"><span className="text-emerald-600">Available</span></td>
              <td className="p-3">Full support via aws-sdk-s3</td>
            </tr>
            <tr className="border-b border-[var(--color-border)]">
              <td className="p-3 font-medium text-[var(--color-text)]">Azure Blob Storage</td>
              <td className="p-3"><span className="text-emerald-600">Available</span></td>
              <td className="p-3">Full support via azure_storage_blobs</td>
            </tr>
            <tr className="border-b border-[var(--color-border)]">
              <td className="p-3 font-medium text-[var(--color-text)]">Google Cloud Storage</td>
              <td className="p-3"><span className="text-amber-600">Disabled</span></td>
              <td className="p-3">Blocked by RUSTSEC-2025-0009/0010</td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="setup">Setup</h2>
      <h3 className="text-lg font-semibold mt-6 mb-2">AWS S3</h3>
      <CodeBlock language="bash">{`# Set credentials via environment variables
export AWS_ACCESS_KEY_ID=your-key
export AWS_SECRET_ACCESS_KEY=your-secret
export AWS_REGION=us-east-1

# Or configure through CLI
iv cloud config --provider s3 --show`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">Azure Blob Storage</h3>
      <CodeBlock language="bash">{`export AZURE_STORAGE_ACCOUNT=your-account
export AZURE_STORAGE_KEY=your-key`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="usage">CLI Usage</h2>
      <CodeBlock language="bash">{`# Push a model to S3
iv cloud push my-model --provider s3 --bucket my-models

# List models in cloud
iv cloud list --provider s3 --bucket my-models

# Pull a model from cloud
iv cloud pull my-model --provider s3 --bucket my-models

# Push to Azure
iv cloud push my-model --provider azure --bucket my-container`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="encryption">End-to-End Encryption</h2>
      <p className="text-[var(--color-text-secondary)]">
        Models are encrypted <strong>before</strong> upload and remain encrypted at rest in the cloud.
        The same AES-256-GCM encryption used locally is applied. Cloud providers never see unencrypted
        model data.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rust">Rust API</h2>
      <CodeBlock language="rust">{`use ironvault::storage::{StorageConfig, StorageBackend};

#[tokio::main]
async fn main() -> Result<()> {
    let config = StorageConfig::S3 {
        bucket: "my-models".to_string(),
        region: "us-east-1".to_string(),
        prefix: Some("vault/".to_string()),
    };

    let backend = StorageBackend::new(config).await?;
    backend.upload("my-model/v1.vault", &encrypted_data).await?;
    let data = backend.download("my-model/v1.vault").await?;
    Ok(())
}`}</CodeBlock>
    </>
  );
}
