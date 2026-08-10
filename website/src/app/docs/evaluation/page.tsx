import CodeBlock from "@/components/DocElements";

export default function EvaluationPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Evaluation Harness</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Record, compare, and query model evaluation results across benchmark suites and metrics.
        Track performance across model versions with structured evaluation data.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="quick-start">Quick Start</h2>
      <CodeBlock language="bash">{`# Record an evaluation run
iv eval record my-model --version 1 --suite mmlu --metric accuracy=0.85 --metric f1=0.82

# List evaluations for a model
iv eval list my-model

# Compare two model versions
iv eval compare my-model@1 my-model@2 --suite mmlu

# List all known suites
iv eval suites`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cli">CLI Reference</h2>
      <CodeBlock language="bash">{`iv eval <COMMAND>

Commands:
  record   Record an evaluation run
  list     List evaluation runs for a model
  compare  Compare two model versions on a suite
  suites   List all known evaluation suites`}</CodeBlock>

      <h3 className="text-xl font-bold mt-8 mb-3">Record Evaluation</h3>
      <CodeBlock language="bash">{`iv eval record <NAME> --version <V> --suite <SUITE> --metric <NAME=VALUE>...

Options:
  -v, --version <V>           Model version
  -s, --suite <SUITE>         Evaluation suite (e.g. mmlu, hellaswag)
  -m, --metric <NAME=VALUE>   Metric in name=value format (repeatable)
  -u, --unit <UNIT>           Unit for all metrics (default: score)
      --higher-is-better      Higher is better (default: true)`}</CodeBlock>

      <h3 className="text-xl font-bold mt-8 mb-3">Compare Versions</h3>
      <CodeBlock language="bash">{`iv eval compare <A> <B> --suite <SUITE>

Arguments:
  <A>    First model (name@version)
  <B>    Second model (name@version)`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="comparison">Comparison Output</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        When comparing two model versions, the harness shows metric deltas with direction indicators:
      </p>
      <CodeBlock language="text">{`Comparison: my-model v1 vs v2 on mmlu
  accuracy: 0.8500 → 0.8900 (+0.0400) ↑
  f1:       0.8200 → 0.8700 (+0.0500) ↑`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="python">Python API</h2>
      <CodeBlock language="python">{`from ironvault import EvalStore

store = EvalStore("/path/to/vault")

# Record an evaluation
store.record("my-model", 1, "mmlu", {"accuracy": 0.85, "f1": 0.82}, "score", True)

# List runs
runs = store.get_runs("my-model", version=1)

# List suites
suites = store.suites()`}</CodeBlock>

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
              ["GET", "/api/v1/evaluations", "List evaluations (query: model, version)"],
              ["POST", "/api/v1/evaluations", "Record an evaluation run"],
              ["GET", "/api/v1/evaluations/suites", "List all known suites"],
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

      <h3 className="text-xl font-bold mt-8 mb-3">Example: Record Evaluation</h3>
      <CodeBlock language="bash">{`curl -X POST http://localhost:8080/api/v1/evaluations \\
  -H "Authorization: Bearer $TOKEN" \\
  -H "Content-Type: application/json" \\
  -d '{
    "name": "my-model",
    "version": 1,
    "suite": "mmlu",
    "metrics": [
      {"name": "accuracy", "value": 0.85},
      {"name": "f1", "value": 0.82}
    ],
    "unit": "score",
    "higher_is_better": true
  }'`}</CodeBlock>
    </>
  );
}
