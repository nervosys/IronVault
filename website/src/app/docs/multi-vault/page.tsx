import CodeBlock from "@/components/DocElements";

export default function MultiVaultPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Multi-Vault Management</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Registry for managing multiple encrypted vaults with activate/deactivate switching.
        Work with multiple vaults from a single CLI instance.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Register vaults
iv vaults register production /data/vaults/prod --description "Production models"
iv vaults register staging /data/vaults/staging --description "Staging models"

# List registered vaults
iv vaults list

# Switch active vault
iv vaults activate production

# Clear active vault
iv vaults deactivate

# Remove from registry
iv vaults unregister staging`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cli">CLI Reference</h2>
      <CodeBlock language="bash">{`iv vaults <COMMAND>

Commands:
  register    Register a vault
  unregister  Unregister a vault
  activate    Set the active vault
  deactivate  Clear the active vault
  list        List registered vaults`}</CodeBlock>

      <h3 className="text-xl font-bold mt-8 mb-3">Register Vault</h3>
      <CodeBlock language="bash">{`iv vaults register <NAME> <PATH> [--description <DESC>]

Arguments:
  <NAME>    Vault name/alias
  <PATH>    Path to vault directory`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="registry">Vault Registry</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        The vault registry is stored in the XDG config directory as{" "}
        <code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">vault_registry.json</code>.
        It tracks the name, path, description, and active status of each vault.
      </p>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Only one vault can be active at a time. The active vault is used by default for all{" "}
        <code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">iv</code>{" "}
        commands unless overridden by the{" "}
        <code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">IRONVAULT_VAULT</code>{" "}
        environment variable.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="python">Python API</h2>
      <CodeBlock language="python">{`from ironvault import VaultRegistry

registry = VaultRegistry("/path/to/config")

# Register vaults
registry.register("production", "/data/vaults/prod", "Production models")
registry.register("staging", "/data/vaults/staging", "Staging models")

# Switch active vault
registry.activate("production")
active = registry.active_name()  # "production"

# Deactivate
registry.deactivate()

# List and count
vaults = registry.list()
count = registry.count()`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="api">REST API</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Method</th>
              <th className="text-left p-3 font-semibold">Path</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["GET", "/api/v1/vaults", "List registered vaults"],
              ["POST", "/api/v1/vaults", "Register a new vault"],
              ["POST", "/api/v1/vaults/:name/activate", "Activate a vault"],
            ].map(([method, path, desc]) => (
              <tr key={`${method}-${path}`} className="border-b border-[var(--color-border)]">
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{method}</code></td>
                <td className="p-3 font-medium text-[var(--color-text)]"><code className="text-xs">{path}</code></td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h3 className="text-xl font-bold mt-8 mb-3">Example: Register Vault</h3>
      <CodeBlock language="bash">{`curl -X POST http://localhost:8080/api/v1/vaults \\
  -H "Authorization: Bearer $TOKEN" \\
  -H "Content-Type: application/json" \\
  -d '{"name": "production", "path": "/data/vaults/prod", "description": "Production models"}'`}</CodeBlock>

      <h3 className="text-xl font-bold mt-8 mb-3">Example: Activate Vault</h3>
      <CodeBlock language="bash">{`curl -X POST http://localhost:8080/api/v1/vaults/production/activate \\
  -H "Authorization: Bearer $TOKEN"`}</CodeBlock>
    </>
  );
}
