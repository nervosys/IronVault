import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function MigrationPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Migration Guide</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Upgrading from v0.x to v1.0.0.
      </p>

      <Callout type="warning" title="Breaking changes">
        v1.0.0 contains several breaking changes from the v0.x series. Review all sections before upgrading.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="api-changes">API Changes</h2>
      <h3 className="text-lg font-semibold mt-6 mb-2">Vault Initialization</h3>
      <CodeBlock language="rust" title="Before (v0.x)">{`let vault = Vault::new("my-vault")?;
vault.set_password("passphrase")?;`}</CodeBlock>
      <CodeBlock language="rust" title="After (v1.0.0)">{`let config = VaultConfig::default();
let vault = Vault::new("my-vault", config)?;
vault.unlock("passphrase")?;`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">Model Storage</h3>
      <CodeBlock language="rust" title="Before (v0.x)">{`vault.store("model", &data, "safetensors")?;`}</CodeBlock>
      <CodeBlock language="rust" title="After (v1.0.0)">{`vault.store_model("model", &data, ModelFormat::SafeTensors, None)?;`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cli-changes">CLI Changes</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">v0.x Command</th>
              <th className="text-left p-3 font-semibold">v1.0.0 Command</th>
              <th className="text-left p-3 font-semibold">Notes</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["iv store <file>", "iv store <name> --file <file>", "Explicit name parameter"],
              ["iv get <name>", "iv get <name> --output <file>", "Output file now required"],
              ["iv info <name>", "iv get <name> --info", "Merged into get command"],
              ["iv password", "iv change-passphrase", "Renamed for clarity"],
              ["iv server", "iv serve", "Renamed for consistency"],
            ].map(([old, now, notes], i) => (
              <tr key={i} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="text-xs">{old}</code></td>
                <td className="p-3"><code className="text-xs">{now}</code></td>
                <td className="p-3">{notes}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="config">Configuration</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Configuration is now fully XDG-compliant. Old config locations are no longer read:
      </p>
      <ul className="space-y-1 text-[var(--color-text-secondary)]">
        <li>• <strong>Config</strong>: <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">~/.config/ironvault/</code></li>
        <li>• <strong>Data</strong>: <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">~/.local/share/ironvault/</code></li>
        <li>• <strong>Cache</strong>: <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">~/.cache/ironvault/</code></li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="data">Data Migration</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Vaults created with v0.x are compatible with v1.0.0. No data migration is needed.
        However, to benefit from the new XDG paths:
      </p>
      <CodeBlock language="bash">{`# Move vault data to XDG directory
mv ~/.iv/vaults/ ~/.local/share/ironvault/vaults/

# Move configuration
mv ~/.iv/config.toml ~/.config/ironvault/config.toml`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="features">Feature Flags</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        The <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">full</code> feature flag now
        includes all optional features:
      </p>
      <CodeBlock language="bash">{`# v0.x — had to specify each feature
cargo install ironvault --features "api,cloud,python"

# v1.0.0 — use full for everything
cargo install ironvault --features full`}</CodeBlock>
    </>
  );
}
