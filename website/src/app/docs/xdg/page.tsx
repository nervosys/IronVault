import CodeBlock from "@/components/DocElements";

export default function XDGPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">XDG Compliance</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        100% compliant with the XDG Base Directory Specification. No files are placed in the home directory root.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="directories">Directory Mapping</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">XDG Variable</th>
              <th className="text-left p-3 font-semibold">Default Path</th>
              <th className="text-left p-3 font-semibold">Purpose</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["$XDG_CONFIG_HOME", "~/.config/ironvault/", "Configuration files"],
              ["$XDG_DATA_HOME", "~/.local/share/ironvault/", "Vault data and databases"],
              ["$XDG_CACHE_HOME", "~/.cache/ironvault/", "Temporary and cache files"],
              ["$XDG_STATE_HOME", "~/.local/state/ironvault/", "Log files and runtime state"],
            ].map(([variable, path, purpose]) => (
              <tr key={variable} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{variable}</code></td>
                <td className="p-3"><code className="text-xs">{path}</code></td>
                <td className="p-3">{purpose}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="windows">Windows & macOS</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        On non-Linux platforms, platform-appropriate defaults are used:
      </p>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Platform</th>
              <th className="text-left p-3 font-semibold">Config</th>
              <th className="text-left p-3 font-semibold">Data</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            <tr className="border-b border-[var(--color-border)]">
              <td className="p-3 font-medium text-[var(--color-text)]">Windows</td>
              <td className="p-3"><code className="text-xs">%APPDATA%\ironvault\</code></td>
              <td className="p-3"><code className="text-xs">%LOCALAPPDATA%\ironvault\</code></td>
            </tr>
            <tr className="border-b border-[var(--color-border)]">
              <td className="p-3 font-medium text-[var(--color-text)]">macOS</td>
              <td className="p-3"><code className="text-xs">~/Library/Preferences/ironvault/</code></td>
              <td className="p-3"><code className="text-xs">~/Library/Application Support/ironvault/</code></td>
            </tr>
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="override">Custom Paths</h2>
      <CodeBlock language="bash">{`# Override via environment variables
export XDG_DATA_HOME=/custom/data
export XDG_CONFIG_HOME=/custom/config
export XDG_CACHE_HOME=/custom/cache

# Or use the IRONVAULT_VAULT_DIR override
export IRONVAULT_VAULT_DIR=/custom/vault/path`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="structure">File Layout</h2>
      <CodeBlock language="text">{`~/.config/ironvault/
  └── config.toml          # Global configuration

~/.local/share/ironvault/
  ├── vaults/              # Vault directories
  │   ├── default/
  │   │   ├── vault.db     # SQLite metadata
  │   │   └── models/      # Encrypted model data
  │   └── production/
  │       ├── vault.db
  │       └── models/
  └── cards/               # Model cards

~/.cache/ironvault/
  ├── downloads/           # Temporary downloads
  └── conversions/         # Conversion cache`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rust-api">Rust API</h2>
      <CodeBlock language="rust">{`use ironvault::xdg::XdgDirs;

let dirs = XdgDirs::new("ironvault")?;

println!("Config: {}", dirs.config_dir().display());
println!("Data:   {}", dirs.data_dir().display());
println!("Cache:  {}", dirs.cache_dir().display());
println!("State:  {}", dirs.state_dir().display());`}</CodeBlock>
    </>
  );
}
