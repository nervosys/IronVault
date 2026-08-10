# License Scanning

Detect licenses from model cards, GGUF metadata, config files, and LICENSE files with SPDX normalization and permissiveness classification.

## Quick Start

```bash
# Scan a directory (looks for README.md, LICENSE, config.json)
iv license-scan ./my-model/

# Scan a single file (GGUF metadata, LICENSE text, etc.)
iv license-scan model.gguf

# JSON output
iv license-scan ./my-model/ --format json
```

## CLI Reference

```
iv license-scan <PATH> [OPTIONS]

Arguments:
  <PATH>              File or directory to scan

Options:
  -f, --format <FMT>  Output format: text (default) or json
```

## Detection Sources

| Source | What It Scans |
|--------|---------------|
| Model Card | YAML frontmatter in README.md (`license:` field) |
| GGUF Metadata | License string in GGUF file header |
| LICENSE File | Full text matching of LICENSE/LICENSE.md files |
| Config File | `license` field in config.json |

## License Classification

Detected licenses are classified by permissiveness:

| Class | Examples |
|-------|----------|
| **Permissive** | MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause |
| **Copyleft** | GPL-2.0, GPL-3.0, AGPL-3.0, LGPL-3.0 |
| **Restricted** | CC-BY-NC-4.0, CC-BY-NC-SA-4.0, Llama-2, Llama-3 |
| **Proprietary** | Proprietary, custom license terms |
| **Unknown** | Unrecognized license identifiers |

## SPDX Normalization

Raw license strings are normalized to standard SPDX identifiers:

| Raw Input | SPDX Output |
|-----------|-------------|
| `mit` | `MIT` |
| `apache 2.0` | `Apache-2.0` |
| `gpl-3` | `GPL-3.0-only` |
| `cc-by-nc-4.0` | `CC-BY-NC-4.0` |
| `llama2` | `Llama-2` |

25 license families are recognized, covering open-source, Creative Commons, and model-specific licenses.

## Output Example

```
=== License Scan Report ===
Path: ./my-model/

Detected Licenses:
  [ModelCard]    Apache-2.0  (Permissive)
  [ConfigFile]   Apache-2.0  (Permissive)

Warnings: none
Has License: yes
```

## Rust API

```rust
use ironvault::license_scan::LicenseScanner;

// Scan a directory
let report = LicenseScanner::scan_directory(Path::new("./my-model/"))?;
println!("{}", report.display());

for license in &report.licenses {
    println!("{}: {} ({:?})", license.source, license.spdx_id, license.classification);
}

// Scan a single GGUF file
let report = LicenseScanner::scan_file(Path::new("model.gguf"))?;
println!("Has license: {}", report.has_license);
```
