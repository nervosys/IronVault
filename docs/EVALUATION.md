# Evaluation Harness

Record, compare, and query model evaluation results across benchmark suites and metrics. Track performance across model versions with structured evaluation data.

## Quick Start

```bash
# Record an evaluation run
iv eval record my-model --version 1 --suite mmlu --metric accuracy=0.85 --metric f1=0.82

# List evaluations for a model
iv eval list my-model

# Compare two model versions
iv eval compare my-model@1 my-model@2 --suite mmlu

# List all known suites
iv eval suites
```

## CLI Reference

```
iv eval <COMMAND>

Commands:
  record   Record an evaluation run
  list     List evaluation runs for a model
  compare  Compare two model versions on a suite
  suites   List all known evaluation suites
```

### `iv eval record`

```
iv eval record <NAME> --version <V> --suite <SUITE> --metric <NAME=VALUE>... [OPTIONS]

Arguments:
  <NAME>              Model name

Options:
  -v, --version <V>           Model version
  -s, --suite <SUITE>         Evaluation suite name (e.g. mmlu, hellaswag)
  -m, --metric <NAME=VALUE>   Metric in name=value format (repeatable)
  -u, --unit <UNIT>           Unit for all metrics (default: score)
      --higher-is-better      Higher is better (default: true)
```

### `iv eval list`

```
iv eval list <NAME> [OPTIONS]

Arguments:
  <NAME>              Model name

Options:
  -v, --version <V>           Filter by version
  -f, --format <FMT>          Output format: text (default) or json
```

### `iv eval compare`

```
iv eval compare <A> <B> --suite <SUITE> [OPTIONS]

Arguments:
  <A>    First model (name@version)
  <B>    Second model (name@version)

Options:
  -s, --suite <SUITE>    Evaluation suite
  -f, --format <FMT>     Output format: text (default) or json
```

## Metric Format

Metrics are specified as `name=value` pairs:

```bash
iv eval record my-model --version 1 --suite mmlu \
  --metric accuracy=0.85 \
  --metric precision=0.87 \
  --metric recall=0.83 \
  --metric f1=0.85
```

## Comparison Output

When comparing two model versions, the harness shows metric deltas:

```
Comparison: my-model v1 vs v2 on mmlu
  accuracy: 0.8500 → 0.8900 (+0.0400) ↑
  f1:       0.8200 → 0.8700 (+0.0500) ↑
```

## Python API

```python
from ironvault import EvalStore

store = EvalStore("/path/to/vault")

# Record an evaluation
store.record("my-model", 1, "mmlu", {"accuracy": 0.85, "f1": 0.82}, "score", True)

# List runs
runs = store.get_runs("my-model", version=1)

# List suites
suites = store.suites()
```

## REST API

| Method | Path                         | Description                              |
| ------ | ---------------------------- | ---------------------------------------- |
| `GET`  | `/api/v1/evaluations`        | List evaluations (query: model, version) |
| `POST` | `/api/v1/evaluations`        | Record an evaluation run                 |
| `GET`  | `/api/v1/evaluations/suites` | List all known suites                    |

### Example: Record Evaluation

```bash
curl -X POST http://localhost:8080/api/v1/evaluations \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
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
  }'
```

## Library API

```rust
use ironvault::{EvalStore, MetricResult};

let store = EvalStore::new("/path/to/vault")?;

// Record a run
let metrics = vec![
    MetricResult { name: "accuracy".into(), value: 0.85, unit: "score".into() },
    MetricResult { name: "f1".into(), value: 0.82, unit: "score".into() },
];
store.record("my-model", 1, "mmlu", metrics, true)?;

// Query runs
let runs = store.get_runs("my-model", Some(1))?;

// Compare versions
let comparison = store.compare("my-model", 1, 2, "mmlu")?;
for delta in &comparison.deltas {
    println!("{}: {} → {} ({:+})", delta.metric, delta.old_value, delta.new_value, delta.delta);
}
```
