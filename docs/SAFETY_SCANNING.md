# Safety Scanning

Static analysis of PyTorch/pickle files for dangerous opcodes and malicious patterns. Detects common supply-chain attack vectors in serialized Python objects.

## Quick Start

```bash
# Scan a vault model
iv scan my-model

# Scan a specific version
iv scan my-model --version 2

# Scan a file on disk
iv scan --file ./model.pt

# JSON output for CI/CD integration
iv scan --file ./model.pt --format json
```

## CLI Reference

```
iv scan [<NAME>] [OPTIONS]

Arguments:
  [NAME]              Model name in vault (optional)

Options:
  --file <PATH>       Scan a file on disk
  -v, --version <V>   Model version
  -f, --format <FMT>  Output format: text (default) or json
```

## What It Detects

### Dangerous Opcodes

The scanner checks for 7 pickle opcodes that allow arbitrary code execution:

| Opcode | Risk | Description |
|--------|------|-------------|
| `REDUCE` | Critical | Calls arbitrary callable with args |
| `GLOBAL` | Critical | Imports any module/attribute |
| `BUILD` | Warning | Calls `__setstate__` on objects |
| `INST` | Critical | Instantiates arbitrary classes |
| `NEWOBJ` | Critical | Creates new objects via `__new__` |
| `NEWOBJ_EX` | Critical | Extended object creation |
| `STACK_GLOBAL` | Critical | Dynamic module import from stack |

### Dangerous Patterns

12 string patterns associated with malicious pickle payloads:

- `os`, `subprocess`, `__builtin__` — system command execution
- `exec`, `eval`, `__import__` — dynamic code execution
- `socket`, `http`, `urllib` — network access
- `shutil`, `tempfile`, `ctypes` — file system and memory manipulation

## Scan Report

```
=== Pickle Safety Scan ===
File: model.pt
Size: 456,789 bytes
Format: Pickle (ZIP archive)

Findings:
  [CRITICAL] REDUCE opcode found (3 occurrences)
  [CRITICAL] GLOBAL opcode found (2 occurrences)
  [WARNING]  BUILD opcode found (1 occurrence)

Verdict: UNSAFE
Recommendation: Do not load this file with torch.load(). Consider converting to SafeTensors format.
```

## Rust API

```rust
use ironvault::scanning::PickleScanner;

// Scan a file
let report = PickleScanner::scan(Path::new("model.pt"))?;
println!("Safe: {}", report.safe);
for finding in &report.findings {
    println!("[{:?}] {}", finding.severity, finding.description);
}

// Scan bytes in memory
let data = std::fs::read("model.pt")?;
let report = PickleScanner::scan_bytes(&data, "model.pt");
```

## CI/CD Integration

```bash
# Fail pipeline if model is unsafe
iv scan --file model.pt --format json | jq -e '.safe == true'
```
