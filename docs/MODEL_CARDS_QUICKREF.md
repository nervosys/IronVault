# Model Cards - Quick Reference

⚡ Fast guide to creating model cards in IronVault (AIMV)

---

## Quick Start

```rust
use ironvault::model_card::*;

// 1. Create details
let details = ModelDetails {
    name: "my-model".to_string(),
    version: "1.0.0".to_string(),
    description: "What it does".to_string(),
    model_type: "Classifier".to_string(),
    architecture: "ResNet-50".to_string(),
    size: "25M params".to_string(),
    framework: "PyTorch".to_string(),
    format: "safetensors".to_string(),
    license: Some("MIT".to_string()),
    citation: None,
    developers: vec!["Team".to_string()],
    contact: Some("email@example.com".to_string()),
    repository: None,
    paper: None,
};

// 2. Define use
let intended_use = IntendedUse {
    primary_uses: vec!["Use case 1".to_string()],
    primary_users: vec!["User type".to_string()],
    out_of_scope_uses: vec!["DON'T use for...".to_string()],
    use_case_examples: None,
};

// 3. Create card
let card = ModelCard::new(details, intended_use);

// 4. Export
println!("{}", card.to_markdown());
```

---

## Core Sections

### 1. Model Details (Required)
```rust
ModelDetails {
    name: "model-name",          // Identifier
    version: "1.0.0",             // Semantic version
    description: "...",           // What it does
    model_type: "...",            // Type (LLM, Classifier, etc.)
    architecture: "...",          // Architecture
    size: "...",                  // Parameters/size
    framework: "...",             // Framework
    format: "...",                // File format
    license: Some("..."),         // License
    citation: Some("..."),        // BibTeX citation
    developers: vec![...],        // Authors
    contact: Some("..."),         // Contact
    repository: Some("..."),      // Code repo
    paper: Some("..."),           // Paper URL
}
```

### 2. Intended Use (Required)
```rust
IntendedUse {
    primary_uses: vec![          // What it's FOR
        "Customer support",
        "Q&A systems",
    ],
    primary_users: vec![         // Who should use it
        "Developers",
        "Enterprises",
    ],
    out_of_scope_uses: vec![     // What it's NOT FOR
        "Medical diagnosis",
        "Legal advice",
    ],
    use_case_examples: Some(vec![...]),
}
```

### 3. Training Data (Recommended)
```rust
TrainingData {
    datasets: vec!["Dataset1", "Dataset2"],
    sources: Some(vec!["Source1"]),
    collection_methods: Some("How data was collected"),
    preprocessing: Some(vec!["Step1", "Step2"]),
    size: Some("100GB, 50B tokens"),
    splits: Some(train/val/test splits),
    languages: Some(vec!["English"]),
    demographics: Some("..."),
}
```

### 4. Evaluation (Recommended)
```rust
Evaluation {
    datasets: vec!["Test set"],
    metrics: vec![
        Metric {
            name: "Accuracy".to_string(),
            value: 0.95,
            description: Some("..."),
            threshold: Some(0.90),
        },
    ],
    benchmarks: Some("Benchmark results"),
    performance_by_group: Some(fairness_metrics),
    methodology: Some("How evaluated"),
}
```

### 5. Ethical Considerations (Recommended)
```rust
EthicalConsiderations {
    sensitive_data: Some("PII handling"),
    bias: Some(vec!["Known biases"]),
    fairness: Some(vec!["Fairness analysis"]),
    privacy: Some("Privacy measures"),
    environmental_impact: Some(EnvironmentalImpact {
        hardware: "8x A100",
        hours: 240.0,
        carbon_emitted: Some(156.8),   // kg CO2e
        energy_consumed: Some(1920.0), // kWh
    }),
    human_oversight: Some("Required oversight"),
    risks: Some(vec!["Risk1", "Risk2"]),
    mitigations: Some(vec!["Mitigation1"]),
}
```

### 6. Caveats (Recommended)
```rust
CaveatsAndRecommendations {
    limitations: vec![
        "Limitation 1",
        "Limitation 2",
    ],
    known_issues: Some(vec!["Issue1"]),
    recommendations: vec![
        "Test on your data",
        "Use thresholds",
    ],
    testing_recommendations: Some(vec![...]),
    tradeoffs: Some(vec![...]),
}
```

---

## Builder Pattern

```rust
let card = ModelCard::new(details, intended_use)
    .with_training_data(training_data)
    .with_evaluation(evaluation)
    .with_ethical_considerations(ethical)
    .with_caveats_and_recommendations(caveats)
    .add_metadata("key".to_string(), "value".to_string());
```

---

## Export Formats

### JSON
```rust
let json = card.to_json()?;
std::fs::write("model_card.json", json)?;
```

### YAML
```rust
let yaml = card.to_yaml()?;
std::fs::write("model_card.yaml", yaml)?;
```

### Markdown (HuggingFace Style)
```rust
let markdown = card.to_markdown();
std::fs::write("README.md", markdown)?;
```

---

## Import/Parse

```rust
// From JSON
let card = ModelCard::from_json(&json_string)?;

// From YAML
let card = ModelCard::from_yaml(&yaml_string)?;
```

---

## Common Patterns

### LLM Model Card
```rust
let details = ModelDetails {
    model_type: "Large Language Model".to_string(),
    architecture: "Transformer".to_string(),
    size: "7B parameters".to_string(),
    // ...
};

let training_data = TrainingData {
    size: Some("50B tokens".to_string()),
    preprocessing: Some(vec![
        "PII removal".to_string(),
        "Deduplication".to_string(),
    ]),
    // ...
};
```

### Image Classifier
```rust
let details = ModelDetails {
    model_type: "Image Classification".to_string(),
    architecture: "ResNet-50".to_string(),
    size: "25M parameters".to_string(),
    // ...
};

let evaluation = Evaluation {
    metrics: vec![
        Metric {
            name: "Top-1 Accuracy".to_string(),
            value: 0.76,
            // ...
        },
        Metric {
            name: "Top-5 Accuracy".to_string(),
            value: 0.93,
            // ...
        },
    ],
    // ...
};
```

### Medical Model (High-Risk)
```rust
let details = ModelDetails {
    license: Some("⚠️ RESEARCH ONLY - NOT FOR CLINICAL USE".to_string()),
    // ...
};

let intended_use = IntendedUse {
    out_of_scope_uses: vec![
        "❌ NOT for clinical diagnosis".to_string(),
        "❌ NOT FDA approved".to_string(),
    ],
    // ...
};

let ethical = EthicalConsiderations {
    human_oversight: Some(
        "MANDATORY: Board-certified physician review required".to_string()
    ),
    risks: Some(vec![
        "Misdiagnosis risk".to_string(),
        "Population bias".to_string(),
    ]),
    // ...
};
```

---

## Fairness Metrics

```rust
use std::collections::HashMap;

let mut performance_by_group = HashMap::new();

// By gender
let mut gender = HashMap::new();
gender.insert("male".to_string(), 0.831);
gender.insert("female".to_string(), 0.817);
gender.insert("non-binary".to_string(), 0.809);
performance_by_group.insert("gender".to_string(), gender);

// By age
let mut age = HashMap::new();
age.insert("18-30".to_string(), 0.92);
age.insert("31-50".to_string(), 0.90);
age.insert("51+".to_string(), 0.87);
performance_by_group.insert("age".to_string(), age);

let evaluation = Evaluation {
    performance_by_group: Some(performance_by_group),
    // ...
};
```

---

## Environmental Impact

```rust
let environmental = EnvironmentalImpact {
    hardware: "8x NVIDIA A100 80GB GPUs".to_string(),
    hours: 240.0,  // Training time
    cloud_provider: Some("AWS".to_string()),
    carbon_emitted: Some(156.8),   // kg CO2e
    energy_consumed: Some(1920.0), // kWh
};

let ethical = EthicalConsiderations {
    environmental_impact: Some(environmental),
    // ...
};
```

---

## Vault Integration

```rust
use ironvault::VaultConfig;

// Store card with model
let config = VaultConfig::new()?;
let mut vault = config.build()?;

let card_json = card.to_json()?;
let metadata = ModelMetadata::new("my-model".to_string(), format)
    .add_custom_field("model_card".to_string(), card_json);

vault.store_model("my-model", &model_data, &metadata, None)?;

// Retrieve card
let retrieved = vault.get_version("my-model", None).unwrap();
if let Some(card_json) = retrieved.metadata.get("model_card") {
    let card = ModelCard::from_json(card_json)?;
    println!("{}", card.to_markdown());
}
```

---

## Checklist

Before deployment:

**Required** ✅
- [ ] Model name & version
- [ ] Description
- [ ] Architecture & size
- [ ] Primary uses
- [ ] Out-of-scope uses
- [ ] Limitations

**Recommended** ⚠️
- [ ] Training data
- [ ] Evaluation metrics
- [ ] Fairness analysis
- [ ] Environmental impact
- [ ] Known issues
- [ ] Recommendations

**High-Risk** 🚨
- [ ] Extensive fairness evaluation
- [ ] Risk assessment
- [ ] Mitigation strategies
- [ ] Human oversight requirements
- [ ] Regulatory compliance

---

## Examples

Run demo:
```bash
cargo run --example model_card_demo --release
```

Demos include:
1. **LLM**: NervosysChat-7B with full documentation
2. **Medical**: Image classifier with fairness metrics
3. **Environmental**: Large model with carbon tracking
4. **Exports**: JSON/YAML/Markdown formats
5. **Fairness**: Hiring model with demographic analysis

---

## Best Practices

### ✅ Do
- Be comprehensive (include all relevant sections)
- Be specific about limitations
- Include fairness metrics
- Document environmental impact
- Update regularly
- Use semantic versioning

### ❌ Don't
- Create minimal cards
- Be vague about risks
- Skip fairness analysis
- Ignore environmental impact
- Let cards become stale

---

## Resources

📖 Full docs: `docs/MODEL_CARDS.md`  
🔬 Examples: `examples/model_card_demo.rs`  
📚 Standards: Mitchell et al. (2019), HuggingFace, Partnership on AI

---

**IronVault (AIMV)** - Responsible AI documentation made simple.
