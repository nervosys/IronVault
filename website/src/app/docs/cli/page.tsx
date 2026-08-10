import CodeBlock from "@/components/DocElements";

export default function CLIPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">CLI Reference</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Complete reference for the <code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-sm">iv</code> command-line tool.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="global">Global Options</h2>
      <CodeBlock language="bash">{`iv [OPTIONS] <COMMAND>

Options:
  -v, --vault <NAME>    Vault name (uses default if not specified)
  -c, --config <PATH>   Config file path
  -h, --help            Print help
  -V, --version         Print version`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="vault">Vault Management</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">init</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Initialize a new vault.</p>
      <CodeBlock language="bash">{`iv init <NAME>
iv init my-vault`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">store</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Store a model in the vault.</p>
      <CodeBlock language="bash">{`iv store <NAME> --format <FORMAT> --file <PATH> [--description <DESC>]
iv store gpt2 --format safetensors --file model.safetensors --description "Fine-tuned GPT-2"`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">get</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Retrieve a model from the vault.</p>
      <CodeBlock language="bash">{`iv get <NAME> [--version <N>] [--output <DIR>]
iv get gpt2 --version 2 --output ./models/`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">list</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">List all models in the vault.</p>
      <CodeBlock language="bash">{`iv list`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">delete</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Delete a model or specific version.</p>
      <CodeBlock language="bash">{`iv delete <NAME> [--version <N>]`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="versioning">Version Control</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">versions</h3>
      <CodeBlock language="bash">{`iv versions <NAME>`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">lineage</h3>
      <CodeBlock language="bash">{`iv lineage <NAME>`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">verify</h3>
      <CodeBlock language="bash">{`iv verify <NAME> [--version <N>]`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="conversion">Format Conversion</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">convert</h3>
      <CodeBlock language="bash">{`iv convert <FILE> --from <FORMAT> --to <FORMAT> [--output <FILE>] [--opset <N>] [--validate] [--plan-only]
iv convert model.safetensors --from safetensors --to pytorch --output model.pt`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">list-conversions</h3>
      <CodeBlock language="bash">{`iv list-conversions`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="model-cards">Model Cards</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">card create / show / export / attach</h3>
      <CodeBlock language="bash">{`iv card create <NAME> --author "Team" --task "text-generation"
iv card show <NAME>
iv card export <NAME> --format markdown --output card.md
iv card attach <NAME> --file card.json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="utilities">Utilities</h2>
      <CodeBlock language="bash">{`iv archive <NAME> --format tar --output backup.tar
iv extract backup.tar --output ./restored/
iv analyze <NAME>
iv info <NAME>`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cloud">Cloud Storage</h2>
      <CodeBlock language="bash">{`iv cloud config --provider s3 --show
iv cloud push <NAME> --provider s3 --bucket my-bucket
iv cloud pull <NAME> --provider s3 --bucket my-bucket
iv cloud list --provider s3 --bucket my-bucket`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="server">API Server</h2>
      <CodeBlock language="bash">{`iv serve [OPTIONS]

Options:
  --host <HOST>              Listen address (default: 127.0.0.1, env: IRONVAULT_HOST)
  --port <PORT>              Listen port (default: 8080, env: IRONVAULT_PORT)
  --jwt-secret <SECRET>      JWT signing key (env: IRONVAULT_JWT_SECRET)
  --token-expiry <SECONDS>   Token lifetime (default: 3600)
  --cors-permissive          Allow all CORS origins
  --no-dashboard             Disable web dashboard`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="download">Model Download</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">pull</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Download models from HuggingFace Hub, Ollama registry, or URLs.</p>
      <CodeBlock language="bash">{`iv pull <SOURCE> [-o DIR] [--sha256 HASH] [--token TOKEN] [--store] [--name NAME]

# HuggingFace
iv pull hf://TheBloke/Llama-2-7B-GGUF/llama-2-7b.Q4_K_M.gguf

# Ollama
iv pull ollama://llama2:7b

# URL with checksum verification
iv pull https://example.com/model.safetensors --sha256 abc123...

# Download and auto-store in vault
iv pull hf://user/repo/model.safetensors --store --name my-model`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="signing">Model Signing</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">sign</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Sign a model with HMAC-SHA256. Auto-generates key on first use.</p>
      <CodeBlock language="bash">{`iv sign <NAME> [-v VERSION] [-k KEY] [-i IDENTITY] [--file PATH]
iv sign my-model --identity "ML Team <ml@company.com>"`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">verify</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Verify a model signature.</p>
      <CodeBlock language="bash">{`iv verify <NAME> --signature <SIG> [-k KEY] [--file PATH]
iv verify my-model --signature my-model.sig`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="scanning">Safety Scanning</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">scan</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Scan PyTorch/pickle files for dangerous opcodes and malicious patterns.</p>
      <CodeBlock language="bash">{`iv scan [<NAME>] [--file PATH] [-v VERSION] [-f text|json]

# Scan a vault model
iv scan my-model

# Scan a file on disk
iv scan --file ./model.pt --format json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="diffing">Model Diffing</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">diff</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Compare two models at the tensor level (SafeTensors, GGUF).</p>
      <CodeBlock language="bash">{`iv diff <LEFT> <RIGHT> [-f text|json]

# Compare files
iv diff model_v1.safetensors model_v2.safetensors

# Compare vault versions
iv diff mymodel@v1 mymodel@v2`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="interop">Engine Interop</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">register</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Register a model with Ollama or LM Studio.</p>
      <CodeBlock language="bash">{`iv register <NAME> --engine <ollama|lm-studio> [-v VERSION] [--alias NAME] [--system-prompt TEXT]

iv register my-model --engine ollama --alias my-assistant --system-prompt "You are helpful."
iv register my-model --engine lm-studio`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="benchmarks">Benchmarks</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">benchmark add</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Attach benchmark scores to model versions.</p>
      <CodeBlock language="bash">{`iv benchmark add <NAME> --version V --benchmark <BENCH> --score <N> --unit <UNIT> [--higher-is-better]
iv benchmark add my-model --version 1 --benchmark mmlu --score 72.5 --unit percent --higher-is-better`}</CodeBlock>

      <h3 className="text-lg font-semibold mt-6 mb-2">benchmark show</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Display benchmark results for a model.</p>
      <CodeBlock language="bash">{`iv benchmark show <NAME> [--version V] [-f text|json]
iv benchmark show my-model --version 1 --format json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="license">License Scanning</h2>

      <h3 className="text-lg font-semibold mt-6 mb-2">license-scan</h3>
      <p className="text-[var(--color-text-secondary)] mb-2">Detect licenses from model cards, GGUF metadata, and config files.</p>
      <CodeBlock language="bash">{`iv license-scan <PATH> [-f text|json]

iv license-scan ./my-model/
iv license-scan model.gguf --format json`}</CodeBlock>
    </>
  );
}
