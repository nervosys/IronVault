import CodeBlock from "@/components/DocElements";

export default function BenchmarksPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Benchmarks</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Attach structured benchmark scores to model versions — track MMLU, HumanEval, perplexity,
        latency, and custom metrics with hardware context.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Add a benchmark result
iv benchmark add my-model --version 1 --benchmark mmlu --score 72.5 --unit percent

# Add with hardware context
iv benchmark add my-model --version 1 --benchmark humaneval --score 48.2 --unit percent \\
    --higher-is-better --hardware "A100 80GB" --dataset "HumanEval"

# Show benchmarks
iv benchmark show my-model
iv benchmark show my-model --version 1 --format json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="add">benchmark add</h2>
      <CodeBlock language="bash">{`iv benchmark add <NAME> [OPTIONS]

Options:
  --version <V>             Model version (required)
  --benchmark <BENCH>       Benchmark name (e.g., mmlu, humaneval, perplexity)
  --score <N>               Numeric score
  --unit <UNIT>             Unit (e.g., percent, ms, tokens/s, ppl)
  --higher-is-better        Score direction flag
  --hardware <HW>           Hardware description
  --dataset <DS>            Dataset name`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="show">benchmark show</h2>
      <CodeBlock language="bash">{`iv benchmark show <NAME> [OPTIONS]

Options:
  --version <V>             Filter by version
  -f, --format <FMT>        Output format: text (default) or json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="storage">Storage</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Benchmark records are stored as JSON files in the data directory:
      </p>
      <CodeBlock language="bash">{`benchmarks/
  my-model__v1.bench.json
  my-model__v2.bench.json
  gpt2-finetuned__v1.bench.json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="format">Record Format</h2>
      <CodeBlock language="json">{`{
  "model_name": "my-model",
  "version": 1,
  "hardware": "NVIDIA A100 80GB",
  "results": [
    {
      "benchmark": "mmlu",
      "score": 72.5,
      "unit": "percent",
      "higher_is_better": true,
      "dataset": "MMLU",
      "recorded_at": "2026-04-04T12:00:00Z"
    }
  ],
  "created_at": "2026-04-04T12:00:00Z",
  "updated_at": "2026-04-04T12:30:00Z"
}`}</CodeBlock>
    </>
  );
}
