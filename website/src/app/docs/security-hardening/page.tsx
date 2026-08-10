import { Callout } from "@/components/DocElements";

export default function SecurityHardeningPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Security Hardening</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Production deployment recommendations for maximum security posture.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="passphrase">Passphrase Policy</h2>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• <strong>Minimum length</strong>: 12 characters (8 enforced, 12+ recommended)</li>
        <li>• <strong>Entropy</strong>: Mix of uppercase, lowercase, numbers, and symbols</li>
        <li>• <strong>Generation</strong>: Use a password manager or <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">openssl rand -base64 32</code></li>
        <li>• <strong>Rotation</strong>: Change passphrases periodically (quarterly recommended)</li>
        <li>• <strong>Storage</strong>: Never store passphrases in plaintext, version control, or logs</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="filesystem">Filesystem Permissions</h2>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• Vault directories: <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">chmod 0700</code> (Unix) or restricted ACLs (Windows)</li>
        <li>• Config files: <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">chmod 0600</code></li>
        <li>• Run as a dedicated, unprivileged user</li>
        <li>• Use mandatory access control (SELinux/AppArmor) in production</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="api-hardening">API Server Hardening</h2>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• Always set <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">IRONVAULT_JWT_SECRET</code> to a strong random value</li>
        <li>• Use TLS termination (nginx, Traefik, or cloud load balancer) — the built-in server is HTTP only</li>
        <li>• Do <strong>not</strong> use <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">--cors-permissive</code> in production</li>
        <li>• Set appropriate <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">--token-expiry</code> (default 3600s = 1 hour)</li>
        <li>• Bind to <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">127.0.0.1</code> when behind a reverse proxy</li>
      </ul>

      <Callout type="warning" title="TLS Required">
        The built-in API server does <strong>not</strong> provide TLS. Always deploy behind
        a reverse proxy with TLS termination for production use.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="container-sec">Container Security</h2>
      <p className="text-[var(--color-text-secondary)] mb-2">
        There is no first-party image — the Dockerfile and Helm chart were
        removed in 4.5.0. If you build your own, keep the properties the
        removed image had:
      </p>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• Run as a non-root user</li>
        <li>• Read-only root filesystem, with the vault directory mounted as a volume</li>
        <li>• All Linux capabilities dropped</li>
        <li>• Inject secrets at runtime — never bake them into a layer</li>
        <li>• Scan images regularly with <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">trivy</code> or <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">grype</code></li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="dependencies">Dependency Auditing</h2>
      <ul className="space-y-2 text-[var(--color-text-secondary)]">
        <li>• Automated <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">cargo audit</code> and <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">cargo deny</code> in CI</li>
        <li>• Daily RUSTSEC advisory checks via GitHub Actions</li>
        <li>• No unsafe code in the codebase</li>
        <li>• All cryptography from audited RustCrypto crates</li>
      </ul>
    </>
  );
}
