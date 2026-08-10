import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function APIPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">REST API</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        14 RESTful endpoints with JWT authentication, OpenAPI spec, and an embedded web dashboard.
      </p>

      <Callout type="info" title="Feature flag required">
        Build with <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">--features api</code> to
        enable the REST API.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="start">Starting the Server</h2>
      <CodeBlock language="bash">{`iv serve --host 0.0.0.0 --port 8080

# With environment variables
IRONVAULT_HOST=0.0.0.0 IRONVAULT_PORT=8080 IRONVAULT_JWT_SECRET=my-secret iv serve`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="auth">Authentication</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Get a JWT token by providing the vault passphrase:
      </p>
      <CodeBlock language="bash">{`curl -X POST http://localhost:8080/auth/token \\
  -H "Content-Type: application/json" \\
  -d '{"passphrase": "your-vault-passphrase"}'

# Response:
# { "token": "eyJ0eXAi...", "expires_in": 3600 }`}</CodeBlock>
      <p className="text-[var(--color-text-secondary)] mt-4 mb-4">
        Use the token in subsequent requests:
      </p>
      <CodeBlock language="bash">{`curl http://localhost:8080/models \\
  -H "Authorization: Bearer eyJ0eXAi..."`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="endpoints">Endpoints</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Method</th>
              <th className="text-left p-3 font-semibold">Path</th>
              <th className="text-left p-3 font-semibold">Auth</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["GET", "/health", "No", "Health check"],
              ["POST", "/auth/token", "No", "Get JWT token"],
              ["GET", "/models", "Yes", "List all models"],
              ["POST", "/models", "Yes", "Store a new model (multipart)"],
              ["GET", "/models/{name}", "Yes", "Get model info"],
              ["DELETE", "/models/{name}", "Yes", "Delete a model"],
              ["POST", "/models/{name}/versions", "Yes", "Create new version"],
              ["GET", "/models/{name}/versions", "Yes", "List versions"],
              ["GET", "/models/{name}/versions/{ver}", "Yes", "Get version data"],
              ["DELETE", "/models/{name}/versions/{ver}", "Yes", "Delete version"],
              ["GET", "/models/{name}/lineage/{ver}", "Yes", "Get version lineage"],
              ["GET", "/conversions", "Yes", "List available conversions"],
              ["POST", "/convert", "Yes", "Convert model format"],
              ["GET", "/stats", "Yes", "Storage statistics"],
            ].map(([method, path, auth, desc], i) => (
              <tr key={i} className="border-b border-[var(--color-border)]">
                <td className="p-3">
                  <span className={`px-2 py-0.5 rounded text-xs font-bold ${
                    method === "GET" ? "bg-emerald-100 text-emerald-800 dark:bg-emerald-900 dark:text-emerald-200" :
                    method === "POST" ? "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200" :
                    "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
                  }`}>{method}</span>
                </td>
                <td className="p-3"><code className="text-xs">{path}</code></td>
                <td className="p-3">{auth}</td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="openapi">OpenAPI Spec</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        The full OpenAPI 3.1 specification is available at <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">/api/v1/openapi.json</code>.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="dashboard">Web Dashboard</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        An embedded single-page web application is served at <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">/</code> (disable
        with <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">--no-dashboard</code>). Features:
      </p>
      <ul className="space-y-1 text-[var(--color-text-secondary)]">
        <li>• Model inventory browser with version drill-down</li>
        <li>• Storage usage statistics</li>
        <li>• Audit log viewer</li>
        <li>• Conversion registry browser</li>
        <li>• Passphrase-based login with JWT sessions</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="examples">More Examples</h2>
      <CodeBlock language="bash" title="Store a model via API">{`curl -X POST http://localhost:8080/models \\
  -H "Authorization: Bearer <token>" \\
  -F "name=my-model" \\
  -F "format=safetensors" \\
  -F "file=@model.safetensors"`}</CodeBlock>

      <CodeBlock language="bash" title="Convert a model via API">{`curl -X POST http://localhost:8080/convert \\
  -H "Authorization: Bearer <token>" \\
  -H "Content-Type: application/json" \\
  -d '{
    "data": "<base64-encoded-model-data>",
    "source_format": "safetensors",
    "target_format": "pytorch"
  }'`}</CodeBlock>
    </>
  );
}
