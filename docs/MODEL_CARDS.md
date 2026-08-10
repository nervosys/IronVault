# Model Cards - Complete Documentation

## Overview

IronVault (AIMV) includes comprehensive **Model Card** support following industry standards. Model cards provide standardized documentation for AI models, including details about intended use, training data, evaluation metrics, ethical considerations, and limitations.

**Standards Supported**:
- Google's Model Cards for Model Reporting (Mitchell et al., 2019)
- HuggingFace Model Card specifications
- Partnership on AI Model Card standards

---

## Table of Contents

- [What are Model Cards?](#what-are-model-cards)
- [Why Use Model Cards?](#why-use-model-cards)
- [Model Card Structure](#model-card-structure)
- [Creating Model Cards](#creating-model-cards)
- [Export Formats](#export-formats)
- [Integration with Vault](#integration-with-vault)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [API Reference](#api-reference)

---

## What are Model Cards?

Model cards are structured documentation that accompanies trained ML models. They provide:

1. **Transparency**: Clear information about model capabilities and limitations
2. **Accountability**: Documentation of training data and evaluation
3. **Ethical Awareness**: Consideration of bias, fairness, and risks
4. **Reproducibility**: Details needed to understand model behavior

### Core Sections

| Section                    | Purpose                                         | Required      |
| -------------------------- | ----------------------------------------------- | ------------- |
| **Model Details**          | Basic information (name, version, architecture) | ✅ Yes         |
| **Intended Use**           | Primary uses, users, out-of-scope uses          | ✅ Yes         |
| **Training Data**          | Datasets, collection, preprocessing             | ⚠️ Recommended |
| **Evaluation**             | Metrics, benchmarks, fairness analysis          | ⚠️ Recommended |
| **Ethical Considerations** | Bias, fairness, privacy, environmental impact   | ⚠️ Recommended |
| **Caveats**                | Limitations, known issues, recommendations      | ⚠️ Recommended |

---

## Why Use Model Cards?

### For Developers
- **Documentation**: Single source of truth for model information
- **Compliance**: Meet regulatory requirements (AI Act, GDPR, etc.)
- **Debugging**: Track model behavior and issues
- **Versioning**: Document changes across model versions

### For Users
- **Understanding**: Know what the model can and cannot do
- **Trust**: Transparent information builds confidence
- **Safety**: Clear warnings about limitations and risks
- **Appropriate Use**: Guidance on proper use cases

### For Organizations
- **Risk Management**: Document potential issues before deployment
- **Governance**: Standardized model documentation process
- **Audit Trail**: Compliance with regulations and policies
- **Knowledge Transfer**: Easy onboarding for new team members

---

## Model Card Structure

### 1. Model Details

Basic information about the model:

```rust
ModelDetails {
    name: "model-name",           // Model identifier
    version: "1.0.0",              // Semantic versioning
    description: "...",            // Detailed description
    model_type: "...",             // e.g., "Large Language Model"
    architecture: "...",           // e.g., "Transformer"
    size: "...",                   // e.g., "7B parameters"
    framework: "...",              // e.g., "PyTorch"
    format: "...",                 // e.g., "safetensors"
    license: Some("..."),          // License info
    citation: Some("..."),         // BibTeX citation
    developers: vec![...],         // Author list
    contact: Some("..."),          // Contact info
    repository: Some("..."),       // Code repository
    paper: Some("..."),            // Research paper
}
```

### 2. Intended Use

Defines appropriate and inappropriate uses:

```rust
IntendedUse {
    primary_uses: vec![
        "Customer support chatbot",
        "Technical documentation Q&A",
    ],
    primary_users: vec![
        "Enterprise developers",
        "Support teams",
    ],
    out_of_scope_uses: vec![
        "Medical diagnosis",
        "Legal advice",
        "Financial decisions",
    ],
    use_case_examples: Some(vec![...]),
}
```

### 3. Training Data

Information about training datasets:

```rust
TrainingData {
    datasets: vec!["Dataset1", "Dataset2"],
    sources: Some(vec!["Source1", "Source2"]),
    collection_methods: Some("..."),
    preprocessing: Some(vec!["Step1", "Step2"]),
    size: Some("100GB, 50B tokens"),
    splits: Some(train/val/test split info),
    languages: Some(vec!["English"]),
    demographics: Some("..."),
}
```

### 4. Evaluation

Metrics and performance results:

```rust
Evaluation {
    datasets: vec!["Test Set"],
    metrics: vec![
        Metric {
            name: "Accuracy",
            value: 0.95,
            description: Some("..."),
            threshold: Some(0.90),
        },
    ],
    benchmarks: Some(benchmark results),
    performance_by_group: Some(fairness metrics),
    methodology: Some("..."),
}
```

### 5. Ethical Considerations

Critical ethical information:

```rust
EthicalConsiderations {
    sensitive_data: Some("..."),
    bias: Some(vec!["Bias1", "Bias2"]),
    fairness: Some(vec!["...analysis..."]),
    privacy: Some("..."),
    environmental_impact: Some(EnvironmentalImpact {
        hardware: "...",
        hours: 100.0,
        cloud_provider: Some("..."),
        carbon_emitted: Some(50.0),  // kg CO2e
        energy_consumed: Some(200.0), // kWh
    }),
    human_oversight: Some("..."),
    risks: Some(vec!["Risk1", "Risk2"]),
    mitigations: Some(vec!["Mitigation1", "Mitigation2"]),
}
```

### 6. Caveats and Recommendations

Limitations and guidance:

```rust
CaveatsAndRecommendations {
    limitations: vec!["Limitation1", "Limitation2"],
    known_issues: Some(vec!["Issue1"]),
    recommendations: vec!["Rec1", "Rec2"],
    testing_recommendations: Some(vec!["..."]),
    tradeoffs: Some(vec!["..."]),
}
```

---

## Creating Model Cards

### Basic Example

```rust
use ironvault::model_card::*;

// 1. Create model details
let details = ModelDetails {
    name: "my-classifier".to_string(),
    version: "1.0.0".to_string(),
    description: "Image classifier for cats vs dogs".to_string(),
    model_type: "Binary Classifier".to_string(),
    architecture: "ResNet-50".to_string(),
    size: "25M parameters".to_string(),
    framework: "PyTorch".to_string(),
    format: "safetensors".to_string(),
    license: Some("MIT".to_string()),
    citation: None,
    developers: vec!["ML Team".to_string()],
    contact: Some("ml@company.com".to_string()),
    repository: None,
    paper: None,
};

// 2. Define intended use
let intended_use = IntendedUse {
    primary_uses: vec![
        "Pet photo classification".to_string(),
    ],
    primary_users: vec![
        "Pet app developers".to_string(),
    ],
    out_of_scope_uses: vec![
        "Wildlife classification".to_string(),
    ],
    use_case_examples: None,
};

// 3. Create model card
let card = ModelCard::new(details, intended_use);

// 4. Export to markdown
let markdown = card.to_markdown();
println!("{}", markdown);
```

### Complete Example with All Sections

```rust
// Add training data
let training_data = TrainingData {
    datasets: vec!["ImageNet subset".to_string()],
    sources: Some(vec!["Kaggle".to_string()]),
    collection_methods: Some("Manual labeling".to_string()),
    preprocessing: Some(vec![
        "Resize to 224x224".to_string(),
        "Normalization".to_string(),
    ]),
    size: Some("10,000 images".to_string()),
    splits: Some({
        let mut splits = HashMap::new();
        splits.insert("train".to_string(), "8,000".to_string());
        splits.insert("val".to_string(), "1,000".to_string());
        splits.insert("test".to_string(), "1,000".to_string());
        splits
    }),
    languages: None,
    demographics: None,
};

// Add evaluation
let evaluation = Evaluation {
    datasets: vec!["Test set".to_string()],
    metrics: vec![
        Metric {
            name: "Accuracy".to_string(),
            value: 0.95,
            description: Some("Exact match".to_string()),
            threshold: Some(0.90),
        },
        Metric {
            name: "F1 Score".to_string(),
            value: 0.94,
            description: None,
            threshold: None,
        },
    ],
    benchmarks: None,
    performance_by_group: None,
    methodology: Some("5-fold cross-validation".to_string()),
};

// Add ethical considerations
let ethical = EthicalConsiderations {
    sensitive_data: None,
    bias: Some(vec![
        "May perform better on professional photos".to_string(),
    ]),
    fairness: None,
    privacy: Some("No user data stored".to_string()),
    environmental_impact: Some(EnvironmentalImpact {
        hardware: "1x NVIDIA V100".to_string(),
        hours: 12.0,
        cloud_provider: Some("AWS".to_string()),
        carbon_emitted: Some(5.2),
        energy_consumed: Some(96.0),
    }),
    human_oversight: None,
    risks: Some(vec![
        "May misclassify mixed-breed dogs".to_string(),
    ]),
    mitigations: Some(vec![
        "Human review for edge cases".to_string(),
    ]),
};

// Add caveats
let caveats = CaveatsAndRecommendations {
    limitations: vec![
        "Only works on cats and dogs".to_string(),
        "Requires clear, well-lit photos".to_string(),
    ],
    known_issues: Some(vec![
        "Struggles with black cats".to_string(),
    ]),
    recommendations: vec![
        "Test on your specific image types".to_string(),
        "Use confidence thresholds".to_string(),
    ],
    testing_recommendations: None,
    tradeoffs: None,
};

// Create complete card
let card = ModelCard::new(details, intended_use)
    .with_training_data(training_data)
    .with_evaluation(evaluation)
    .with_ethical_considerations(ethical)
    .with_caveats_and_recommendations(caveats)
    .add_metadata("training_date".to_string(), "2024-01-15".to_string());
```

---

## Export Formats

### JSON

```rust
let json = card.to_json()?;
std::fs::write("model_card.json", json)?;
```

**Output**:
```json
{
  "model_details": {
    "name": "my-classifier",
    "version": "1.0.0",
    ...
  },
  "intended_use": {
    ...
  },
  ...
}
```

### YAML

```rust
let yaml = card.to_yaml()?;
std::fs::write("model_card.yaml", yaml)?;
```

**Output**:
```yaml
model_details:
  name: my-classifier
  version: 1.0.0
  ...
intended_use:
  ...
```

### Markdown (HuggingFace Style)

```rust
let markdown = card.to_markdown();
std::fs::write("README.md", markdown)?;
```

**Output**:
```markdown
# Model Card: my-classifier

## Model Details

- **Name**: my-classifier
- **Version**: 1.0.0
- **Type**: Binary Classifier
...

## Intended Use

### Primary Uses
- Pet photo classification
...
```

### Parsing

```rust
// From JSON
let card = ModelCard::from_json(&json_string)?;

// From YAML
let card = ModelCard::from_yaml(&yaml_string)?;
```

---

## Integration with Vault

Model cards can be stored alongside models in the vault:

```rust
use ironvault::{VaultConfig, model_card::*};

let config = VaultConfig::new()?;
let mut vault = config.build()?;

// 1. Create model card
let card = ModelCard::new(details, intended_use);

// 2. Store as metadata with model
let metadata = ModelMetadata::new("my-model".to_string(), format)
    .add_custom_field("model_card".to_string(), card.to_json()?);

vault.store_model("my-model", &model_data, &metadata, None)?;

// 3. Later: retrieve and parse
let retrieved = vault.get_version("my-model", None).unwrap();
if let Some(card_json) = retrieved.metadata.get("model_card") {
    let card = ModelCard::from_json(card_json)?;
    println!("{}", card.to_markdown());
}
```

---

## Best Practices

### 1. Be Comprehensive

✅ **Do**: Include all relevant sections
```rust
let card = ModelCard::new(details, intended_use)
    .with_training_data(training_data)
    .with_evaluation(evaluation)
    .with_ethical_considerations(ethical)
    .with_caveats_and_recommendations(caveats);
```

❌ **Don't**: Create minimal cards
```rust
let card = ModelCard::new(details, intended_use);
// Missing important sections!
```

### 2. Be Specific About Limitations

✅ **Do**: Clearly state what the model cannot do
```rust
out_of_scope_uses: vec![
    "❌ NOT for medical diagnosis",
    "❌ NOT for financial advice",
    "❌ NOT for real-time safety systems",
]
```

❌ **Don't**: Be vague
```rust
out_of_scope_uses: vec!["Other uses"]
```

### 3. Include Fairness Metrics

✅ **Do**: Evaluate performance across groups
```rust
performance_by_group: Some({
    let mut groups = HashMap::new();
    // Age groups
    let mut age = HashMap::new();
    age.insert("18-30".to_string(), 0.92);
    age.insert("31-50".to_string(), 0.90);
    age.insert("51+".to_string(), 0.87);
    groups.insert("age".to_string(), age);
    groups
})
```

### 4. Document Environmental Impact

✅ **Do**: Report carbon emissions
```rust
environmental_impact: Some(EnvironmentalImpact {
    hardware: "8x A100 GPUs".to_string(),
    hours: 240.0,
    carbon_emitted: Some(156.8),  // kg CO2e
    energy_consumed: Some(1920.0), // kWh
})
```

### 5. Update Regularly

```rust
let mut card = load_existing_card()?;
card.touch(); // Updates timestamp
card.add_metadata("last_review".to_string(), "2024-11-06".to_string());
```

### 6. Use Semantic Versioning

```rust
version: "1.2.3"  // MAJOR.MINOR.PATCH
// MAJOR: Breaking changes (new architecture)
// MINOR: New features (improved accuracy)
// PATCH: Bug fixes (evaluation corrections)
```

---

## Examples

### LLM Model Card

```rust
let details = ModelDetails {
    name: "ChatBot-7B".to_string(),
    model_type: "Large Language Model".to_string(),
    architecture: "Transformer (32 layers)".to_string(),
    size: "7B parameters".to_string(),
    // ... full details
};

let intended_use = IntendedUse {
    primary_uses: vec![
        "Customer support".to_string(),
        "Technical Q&A".to_string(),
    ],
    out_of_scope_uses: vec![
        "Medical diagnosis".to_string(),
        "Legal advice".to_string(),
    ],
    // ...
};
```

See `examples/model_card_demo.rs` for complete examples.

### Image Classifier

```rust
let details = ModelDetails {
    name: "ImageNet-ResNet50".to_string(),
    model_type: "Image Classification".to_string(),
    architecture: "ResNet-50".to_string(),
    // ...
};

let ethical = EthicalConsiderations {
    bias: Some(vec![
        "Training data primarily from Western sources".to_string(),
    ]),
    fairness: Some(vec![
        "Performance evaluated across demographics".to_string(),
    ]),
    // ...
};
```

### Medical Model (High-Risk)

```rust
let details = ModelDetails {
    name: "MedicalDiag-BERT".to_string(),
    license: Some("⚠️ RESEARCH ONLY - NOT FOR CLINICAL USE".to_string()),
    // ...
};

let intended_use = IntendedUse {
    out_of_scope_uses: vec![
        "❌ CLINICAL DIAGNOSIS - Not FDA approved".to_string(),
        "❌ Patient treatment decisions".to_string(),
        "❌ ANY use affecting patient care".to_string(),
    ],
    // ...
};

let ethical = EthicalConsiderations {
    human_oversight: Some(
        "⚠️ MANDATORY: All outputs require board-certified physician review"
    ),
    // ...
};
```

---

## API Reference

### ModelCard

```rust
impl ModelCard {
    // Create new card
    pub fn new(details: ModelDetails, intended_use: IntendedUse) -> Self
    
    // Builder methods
    pub fn with_training_data(self, data: TrainingData) -> Self
    pub fn with_evaluation(self, eval: Evaluation) -> Self
    pub fn with_ethical_considerations(self, ethical: EthicalConsiderations) -> Self
    pub fn with_caveats_and_recommendations(self, caveats: CaveatsAndRecommendations) -> Self
    pub fn add_metadata(self, key: String, value: String) -> Self
    
    // Update timestamp
    pub fn touch(&mut self)
    
    // Export methods
    pub fn to_json(&self) -> Result<String>
    pub fn to_yaml(&self) -> Result<String>
    pub fn to_markdown(&self) -> String
    
    // Parse methods
    pub fn from_json(json: &str) -> Result<Self>
    pub fn from_yaml(yaml: &str) -> Result<Self>
}
```

### Full Struct Definitions

See `src/model_card.rs` for complete definitions of:
- `ModelDetails`
- `IntendedUse`
- `TrainingData`
- `Evaluation`
- `Metric`
- `EthicalConsiderations`
- `EnvironmentalImpact`
- `CaveatsAndRecommendations`

---

## Compliance

Model cards help meet regulatory requirements:

| Regulation        | Requirement                   | Model Card Section              |
| ----------------- | ----------------------------- | ------------------------------- |
| **EU AI Act**     | Risk assessment               | Ethical Considerations, Caveats |
|                   | Training data documentation   | Training Data                   |
|                   | Performance metrics           | Evaluation                      |
| **GDPR**          | Data processing documentation | Training Data, Privacy          |
| **FDA (Medical)** | Clinical validation           | Evaluation, Limitations         |
|                   | Risk analysis                 | Ethical Considerations          |
| **CCPA**          | Data usage transparency       | Training Data                   |

---

## Checklist

Before deploying a model, ensure your model card includes:

### Required ✅
- [ ] Model name and version
- [ ] Clear description
- [ ] Architecture and size
- [ ] Primary uses
- [ ] Out-of-scope uses (what NOT to use it for)
- [ ] Limitations

### Highly Recommended ⚠️
- [ ] Training data information
- [ ] Evaluation metrics
- [ ] Fairness/bias analysis
- [ ] Environmental impact
- [ ] Known issues
- [ ] Recommendations for use

### For High-Risk Applications 🚨
- [ ] Extensive fairness evaluation
- [ ] Risk assessment
- [ ] Mitigation strategies
- [ ] Human oversight requirements
- [ ] Legal/regulatory compliance notes
- [ ] Clinical validation (if medical)

---

## Resources

### Standards & Papers
- Mitchell et al. (2019): ["Model Cards for Model Reporting"](https://arxiv.org/abs/1810.03993)
- [HuggingFace Model Card Guidelines](https://huggingface.co/docs/hub/model-cards)
- [Partnership on AI Model Card Framework](https://partnershiponai.org/)

### Tools
- Model Card Demo: `cargo run --example model_card_demo`
- Model Card Generator: (coming soon)

### Further Reading
- EU AI Act requirements
- FDA guidance on AI/ML medical devices
- IEEE P7003 (Algorithmic Bias Considerations)

---

## Examples

Run the comprehensive demo:

```bash
cargo run --example model_card_demo --release
```

This demonstrates:
1. LLM model card with full sections
2. Medical imaging model with fairness metrics
3. Environmental impact reporting
4. Export to JSON/YAML/Markdown
5. Fairness analysis for hiring model

---

**IronVault (AIMV)** - Standardized model documentation for responsible AI.

*Last Updated: November 6, 2024*
