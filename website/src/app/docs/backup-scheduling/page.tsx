import CodeBlock from "@/components/DocElements";

export default function BackupSchedulingPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Backup Scheduling</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Configurable vault backup schedules with rotation limits and history tracking.
        Automate vault backups on hourly, daily, weekly, or monthly intervals.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Create a daily backup schedule
iv backup set nightly --frequency daily --max-backups 7 --output-dir /backups/vault

# List schedules
iv backup list

# View backup history
iv backup history

# Remove a schedule
iv backup remove nightly`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="frequencies">Backup Frequencies</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Frequency</th>
              <th className="text-left p-3 font-semibold">Aliases</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["hourly", "1h", "Every hour"],
              ["daily", "1d", "Every day"],
              ["weekly", "1w", "Every week"],
              ["monthly", "1m", "Every month"],
            ].map(([freq, alias, desc]) => (
              <tr key={freq} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{freq}</code></td>
                <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{alias}</code></td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cli">CLI Reference</h2>
      <CodeBlock language="bash">{`iv backup <COMMAND>

Commands:
  set      Create or update a backup schedule
  remove   Remove a backup schedule
  list     List backup schedules
  history  Show backup history`}</CodeBlock>

      <h3 className="text-xl font-bold mt-8 mb-3">Set Schedule</h3>
      <CodeBlock language="bash">{`iv backup set <NAME> --frequency <FREQ> --output-dir <PATH> [--max-backups <N>]

Options:
  -f, --frequency <FREQ>       Frequency: hourly, daily, weekly, monthly
  -m, --max-backups <N>        Maximum backups to retain (default: 7)
  -o, --output-dir <PATH>      Output directory for backup archives`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rotation">Rotation Policy</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        When the maximum number of backups is reached, older backups are automatically
        removed before creating new ones. This keeps disk usage bounded while maintaining
        recent backup availability.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="python">Python API</h2>
      <CodeBlock language="python">{`from ironvault import BackupManager

manager = BackupManager("/path/to/vault")

# Create schedule
manager.set_schedule("nightly", "daily", 7, "/backups/vault")

# List schedules
schedules = manager.list_schedules()

# Remove schedule
manager.remove_schedule("nightly")`}</CodeBlock>

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
              ["GET", "/api/v1/backups/schedules", "List all backup schedules"],
              ["POST", "/api/v1/backups/schedules", "Create/update a schedule"],
              ["GET", "/api/v1/backups/history", "Show backup history"],
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
    </>
  );
}
