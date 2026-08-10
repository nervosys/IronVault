import VideoCard from "@/components/VideoCard";
import Link from "next/link";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Demos — IronVault",
  description: "CLI animation demos showing IronVault workflows — vault init, model storage, versioning, format conversion, and security compliance.",
};

const demos = [
  {
    src: "/videos/CLIInit.mp4",
    title: "Initialize a Vault",
    description:
      "Create a new encrypted vault with AES-256-GCM encryption, unlock it with your passphrase, and inspect vault status — all from the CLI.",
    duration: "0:11",
    commands: ["iv init --encryption aes-256-gcm", "iv unlock", "iv status"],
  },
  {
    src: "/videos/CLIStore.mp4",
    title: "Store & List Models",
    description:
      "Store models in multiple formats with automatic detection — SafeTensors, GGUF, PyTorch, ONNX, and more. List everything in the vault with metadata.",
    duration: "0:14",
    commands: ["iv store model.safetensors", "iv store llama-7b.gguf", "iv list"],
  },
  {
    src: "/videos/CLIVersions.mp4",
    title: "Version Control & Lineage",
    description:
      "Track model version history, roll back to any previous version, and view the full lineage tree. Git-like semantics for model management.",
    duration: "0:16",
    commands: [
      "iv store model-v2.safetensors",
      "iv versions model.safetensors",
      "iv get model.safetensors --version 1",
      "iv lineage model.safetensors",
    ],
  },
  {
    src: "/videos/CLIConvert.mp4",
    title: "Format Conversion & Quantization",
    description:
      "Convert models between any of the 23+ supported formats. Apply quantization (Q4_K_M, Q8_0, etc.) during conversion for optimized deployment.",
    duration: "0:13",
    commands: [
      "iv convert model.safetensors --to gguf --quantize q4_k_m",
      "iv convert model.safetensors --to onnx",
      "iv list --format",
    ],
  },
  {
    src: "/videos/CLICompliance.mp4",
    title: "Security Compliance Audit",
    description:
      "Run a comprehensive security audit covering encryption, KDF, code safety, and threat models. 12 automated checks with a compliance score and audit log.",
    duration: "0:11",
    commands: ["iv compliance --verbose", "iv audit-log --last 3"],
  },
];

export default function DemosPage() {
  return (
    <div className="min-h-[calc(100vh-var(--header-height))]">
      {/* Hero */}
      <section className="relative overflow-hidden bg-gradient-to-b from-[#0d1117] via-[#161b22] to-[#0d1117] text-white">
        <div className="relative max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-16 sm:py-20 text-center">
          <h1 className="text-4xl sm:text-5xl font-bold tracking-tight mb-4">
            CLI Demos
          </h1>
          <p className="text-xl text-blue-200 max-w-2xl mx-auto">
            Animated walkthroughs of real IronVault workflows.
            Click any video to play.
          </p>
        </div>
      </section>

      {/* Videos */}
      <section className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
        <div className="space-y-16">
          {demos.map((demo, i) => (
            <div
              key={demo.src}
              className={`flex flex-col ${
                i % 2 === 0 ? "lg:flex-row" : "lg:flex-row-reverse"
              } gap-8 items-start`}
            >
              <div className="lg:w-3/5">
                <VideoCard
                  src={demo.src}
                  title={demo.title}
                  description={demo.description}
                  duration={demo.duration}
                />
              </div>
              <div className="lg:w-2/5 space-y-4">
                <h2 className="text-3xl font-bold">{demo.title}</h2>
                <p className="text-[var(--color-text-secondary)]">
                  {demo.description}
                </p>
                <div className="space-y-1.5">
                  <h4 className="text-sm font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">
                    Commands shown
                  </h4>
                  {demo.commands.map((cmd) => (
                    <div
                      key={cmd}
                      className="font-mono text-base bg-[var(--color-bg-code)] text-emerald-400 px-3 py-1.5 rounded border border-[var(--color-border)]"
                    >
                      <span className="text-gray-500">$ </span>
                      {cmd}
                    </div>
                  ))}
                </div>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* CTA */}
      <section className="bg-[var(--color-bg-secondary)] border-t border-[var(--color-border)]">
        <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-16 text-center">
          <h2 className="text-3xl font-bold mb-4">Ready to try it?</h2>
          <p className="text-lg text-[var(--color-text-secondary)] mb-6 max-w-xl mx-auto">
            Install IronVault and start managing your models in minutes.
          </p>
          <div className="flex flex-col sm:flex-row gap-4 justify-center">
            <Link
              href="/docs/quickstart"
              className="inline-flex items-center justify-center px-6 py-3 rounded bg-[var(--color-primary)] text-white font-semibold hover:bg-[var(--color-primary-dark)] transition-colors"
            >
              Get Started
            </Link>
            <Link
              href="/"
              className="inline-flex items-center justify-center px-6 py-3 rounded border border-[var(--color-border)] font-semibold hover:border-[var(--color-primary)]/50 transition-colors"
            >
              Back to Home
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
