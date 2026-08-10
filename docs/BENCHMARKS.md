# Benchmark Metadata

Attach structured benchmark scores to model versions — track MMLU, HumanEval, perplexity, latency, and custom metrics with hardware context.

## Quick Start

```bash
# Add a benchmark result
iv benchmark add my-model --version 1 --benchmark mmlu --score 72.5 --unit percent

# Add with hardware and dataset context
iv benchmark add my-model --version 1 --benchmark humaneval --score 48.2 --unit percent \
    --higher-is-better --hardware "A100 80GB" --dataset "HumanEval"

# Show benchmarks for a model
iv benchmark show my-model

# Show specific version in JSON
iv benchmark show my-model --version 1 --format json
```

## CLI Reference

### benchmark add

```
iv benchmark add <NAME> [OPTIONS]

Arguments:
  <NAME>                    Model name in vault

Options:
  --version <V>             Model version (required)
  --benchmark <BENCH>       Benchmark name (e.g., mmlu, humaneval, perplexity)
  --score <N>               Numeric score
  --unit <UNIT>             Unit of measurement (e.g., percent, ms, tokens/s)
  --higher-is-better        Score direction (default: false)
  --hardware <HW>           Hardware description
  --dataset <DS>            Dataset name
```

### benchmark show

```
iv benchmark show <NAME> [OPTIONS]

Arguments:
  <NAME>                    Model name in vault

Options:
  --version <V>             Filter by version
  -f, --format <FMT>        Output format: text (default) or json
```

## Storage

Benchmark records are stored as JSON files in `<data_dir>/benchmarks/`:

```
benchmarks/
  my-model__v1.bench.json
  my-model__v2.bench.json
  gpt2-finetuned__v1.bench.json
```

### Record Format

```json
{
  "model_name": "my-model",
  "version": 1,
  "hardware": "NVIDIA A100 80GB",
  "environment": {},
  "results": [
    {
      "benchmark": "mmlu",
      "score": 72.5,
      "unit": "percent",
      "higher_is_better": true,
      "dataset": "MMLU",
      "metadata": {},
      "recorded_at": "2026-04-04T12:00:00Z"
    }
  ],
  "created_at": "2026-04-04T12:00:00Z",
  "updated_at": "2026-04-04T12:30:00Z"
}
```

## Rust API

```rust
use ironvault::benchmark::{BenchmarkStore, BenchmarkRecord};

let store = BenchmarkStore::new("./data")?;

// Create/load a record
let mut record = store.get_or_create("my-model", 1)?;
record.add_result("mmlu", 72.5, "percent", true);
record.add_detailed_result("humaneval", 48.2, "percent", true, Some("HumanEval"), HashMap::new());
store.save(&record)?;

// Query
let records = store.list_for_model("my-model")?;
for r in records {
    println!("{}", r.display());
}
```
