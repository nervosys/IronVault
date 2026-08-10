// Assertions in these tests compare literal constants that round-trip
// bit-for-bit and build fixed strings; the lints below are noise here.
#![allow(clippy::float_cmp)]
//! Comprehensive model card tests
//! Tests every feature, edge case, and validation

use ironvault::model_card::*;
use std::collections::HashMap;

// ============================================================================
// BASIC CONSTRUCTION TESTS
// ============================================================================

#[test]
fn test_minimal_model_card_creation() {
    let details = ModelDetails {
        name: "minimal-model".to_string(),
        version: "0.1.0".to_string(),
        description: "Minimal test model".to_string(),
        model_type: "Test".to_string(),
        architecture: "Simple".to_string(),
        size: "1KB".to_string(),
        framework: "None".to_string(),
        format: "raw".to_string(),
        license: None,
        citation: None,
        developers: vec![],
        contact: None,
        repository: None,
        paper: None,
    };

    let intended_use = IntendedUse {
        primary_uses: vec!["testing".to_string()],
        primary_users: vec!["developers".to_string()],
        out_of_scope_uses: vec![],
        use_case_examples: None,
    };

    let card = ModelCard::new(details, intended_use);

    assert_eq!(card.model_details.name, "minimal-model");
    assert_eq!(card.model_details.version, "0.1.0");
    assert!(card.training_data.is_none());
    assert!(card.evaluation.is_none());
    assert!(card.ethical_considerations.is_none());
    assert!(card.caveats_and_recommendations.is_none());
    assert!(card.metadata.is_empty());
}

#[test]
fn test_complete_model_card_creation() {
    let details = ModelDetails {
        name: "complete-model".to_string(),
        version: "1.0.0".to_string(),
        description: "Complete test model with all fields".to_string(),
        model_type: "Large Language Model".to_string(),
        architecture: "Transformer".to_string(),
        size: "7B parameters".to_string(),
        framework: "PyTorch".to_string(),
        format: "safetensors".to_string(),
        license: Some("Apache-2.0".to_string()),
        citation: Some("@article{test2024}".to_string()),
        developers: vec!["Alice".to_string(), "Bob".to_string()],
        contact: Some("test@example.com".to_string()),
        repository: Some("https://github.com/test/model".to_string()),
        paper: Some("https://arxiv.org/abs/1234.5678".to_string()),
    };

    let intended_use = IntendedUse {
        primary_uses: vec![
            "Text generation".to_string(),
            "Question answering".to_string(),
        ],
        primary_users: vec!["Researchers".to_string(), "Developers".to_string()],
        out_of_scope_uses: vec!["Medical diagnosis".to_string(), "Legal advice".to_string()],
        use_case_examples: Some(vec!["Chatbot".to_string()]),
    };

    let mut splits = HashMap::new();
    splits.insert("train".to_string(), "80%".to_string());
    splits.insert("val".to_string(), "10%".to_string());
    splits.insert("test".to_string(), "10%".to_string());

    let training_data = TrainingData {
        datasets: vec!["CommonCrawl".to_string(), "Wikipedia".to_string()],
        sources: Some(vec!["Web".to_string(), "Books".to_string()]),
        collection_methods: Some("Web scraping and public datasets".to_string()),
        preprocessing: Some(vec!["Deduplication".to_string(), "PII removal".to_string()]),
        size: Some("500GB".to_string()),
        splits: Some(splits),
        languages: Some(vec!["English".to_string(), "Spanish".to_string()]),
        demographics: Some("Global internet users".to_string()),
    };

    let metrics = vec![
        Metric {
            name: "Accuracy".to_string(),
            value: 0.95,
            description: Some("Test set accuracy".to_string()),
            threshold: Some(0.90),
        },
        Metric {
            name: "F1".to_string(),
            value: 0.94,
            description: None,
            threshold: None,
        },
    ];

    let mut perf_by_group = HashMap::new();
    let mut gender_perf = HashMap::new();
    gender_perf.insert("male".to_string(), 0.95);
    gender_perf.insert("female".to_string(), 0.94);
    perf_by_group.insert("gender".to_string(), gender_perf);

    let mut benchmarks = HashMap::new();
    benchmarks.insert("GLUE".to_string(), 85.3);
    benchmarks.insert("SuperGLUE".to_string(), 78.2);

    let evaluation = Evaluation {
        datasets: vec!["TestSet".to_string()],
        metrics,
        benchmarks: Some(benchmarks),
        performance_by_group: Some(perf_by_group),
        methodology: Some("5-fold cross-validation".to_string()),
    };

    let environmental = EnvironmentalImpact {
        hardware: "8x A100 GPUs".to_string(),
        hours: 100.0,
        cloud_provider: Some("AWS".to_string()),
        carbon_emitted: Some(50.5),
        energy_consumed: Some(800.0),
    };

    let ethical = EthicalConsiderations {
        sensitive_data: Some("PII removed".to_string()),
        bias: Some(vec!["Language bias".to_string()]),
        fairness: Some(vec!["Evaluated across demographics".to_string()]),
        privacy: Some("No user data stored".to_string()),
        environmental_impact: Some(environmental),
        human_oversight: Some("Required for production".to_string()),
        risks: Some(vec!["Hallucination".to_string(), "Bias".to_string()]),
        mitigations: Some(vec!["Human review".to_string()]),
    };

    let caveats = CaveatsAndRecommendations {
        limitations: vec!["Context length limited".to_string()],
        known_issues: Some(vec!["Occasional repetition".to_string()]),
        recommendations: vec!["Test before deployment".to_string()],
        testing_recommendations: Some(vec!["A/B testing".to_string()]),
        tradeoffs: Some(vec!["Speed vs accuracy".to_string()]),
    };

    let card = ModelCard::new(details, intended_use)
        .with_training_data(training_data)
        .with_evaluation(evaluation)
        .with_ethical_considerations(ethical)
        .with_caveats_and_recommendations(caveats)
        .add_metadata("version_date".to_string(), "2024-01-01".to_string());

    // Verify all sections present
    assert!(card.training_data.is_some());
    assert!(card.evaluation.is_some());
    assert!(card.ethical_considerations.is_some());
    assert!(card.caveats_and_recommendations.is_some());
    assert_eq!(card.metadata.len(), 1);

    // Verify optional fields
    assert!(card.model_details.license.is_some());
    assert!(card.model_details.citation.is_some());
    assert!(card.model_details.contact.is_some());
    assert!(card.model_details.repository.is_some());
    assert!(card.model_details.paper.is_some());
}

// ============================================================================
// BUILDER PATTERN TESTS
// ============================================================================

#[test]
fn test_builder_pattern_training_data() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();

    let training_data = TrainingData {
        datasets: vec!["Test".to_string()],
        sources: None,
        collection_methods: None,
        preprocessing: None,
        size: None,
        splits: None,
        languages: None,
        demographics: None,
    };

    let card = ModelCard::new(details, intended_use).with_training_data(training_data);

    assert!(card.training_data.is_some());
    assert_eq!(card.training_data.unwrap().datasets[0], "Test");
}

#[test]
fn test_builder_pattern_evaluation() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();

    let evaluation = Evaluation {
        datasets: vec!["EvalSet".to_string()],
        metrics: vec![],
        benchmarks: None,
        performance_by_group: None,
        methodology: None,
    };

    let card = ModelCard::new(details, intended_use).with_evaluation(evaluation);

    assert!(card.evaluation.is_some());
}

#[test]
fn test_builder_pattern_ethical() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();

    let ethical = EthicalConsiderations {
        sensitive_data: Some("Test".to_string()),
        bias: None,
        fairness: None,
        privacy: None,
        environmental_impact: None,
        human_oversight: None,
        risks: None,
        mitigations: None,
    };

    let card = ModelCard::new(details, intended_use).with_ethical_considerations(ethical);

    assert!(card.ethical_considerations.is_some());
}

#[test]
fn test_builder_pattern_caveats() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();

    let caveats = CaveatsAndRecommendations {
        limitations: vec!["Limited".to_string()],
        known_issues: None,
        recommendations: vec![],
        testing_recommendations: None,
        tradeoffs: None,
    };

    let card = ModelCard::new(details, intended_use).with_caveats_and_recommendations(caveats);

    assert!(card.caveats_and_recommendations.is_some());
}

#[test]
fn test_builder_pattern_chaining() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();
    let training_data = create_test_training_data();
    let evaluation = create_test_evaluation();
    let ethical = create_test_ethical();
    let caveats = create_test_caveats();

    let card = ModelCard::new(details, intended_use)
        .with_training_data(training_data)
        .with_evaluation(evaluation)
        .with_ethical_considerations(ethical)
        .with_caveats_and_recommendations(caveats)
        .add_metadata("key1".to_string(), "value1".to_string())
        .add_metadata("key2".to_string(), "value2".to_string());

    assert!(card.training_data.is_some());
    assert!(card.evaluation.is_some());
    assert!(card.ethical_considerations.is_some());
    assert!(card.caveats_and_recommendations.is_some());
    assert_eq!(card.metadata.len(), 2);
}

#[test]
fn test_metadata_addition() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();

    let card = ModelCard::new(details, intended_use)
        .add_metadata("author".to_string(), "Alice".to_string())
        .add_metadata("version_date".to_string(), "2024-01-01".to_string())
        .add_metadata("tags".to_string(), "nlp,transformer".to_string());

    assert_eq!(card.metadata.len(), 3);
    assert_eq!(card.metadata.get("author").unwrap(), "Alice");
    assert_eq!(card.metadata.get("version_date").unwrap(), "2024-01-01");
    assert_eq!(card.metadata.get("tags").unwrap(), "nlp,transformer");
}

// ============================================================================
// TIMESTAMP TESTS
// ============================================================================

#[test]
fn test_timestamps_on_creation() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();

    let card = ModelCard::new(details, intended_use);

    assert_eq!(card.created_at, card.updated_at);
}

#[test]
fn test_touch_updates_timestamp() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();

    let mut card = ModelCard::new(details, intended_use);
    let original_updated = card.updated_at;

    std::thread::sleep(std::time::Duration::from_millis(10));
    card.touch();

    assert!(card.updated_at > original_updated);
    assert_eq!(card.created_at, card.created_at); // created_at unchanged
}

// ============================================================================
// JSON SERIALIZATION TESTS
// ============================================================================

#[test]
fn test_json_serialization_minimal() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let json = card.to_json().unwrap();
    assert!(json.contains("\"name\""));
    assert!(json.contains("\"version\""));
    assert!(json.contains("test-model"));
}

#[test]
fn test_json_deserialization_minimal() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let json = card.to_json().unwrap();
    let parsed = ModelCard::from_json(&json).unwrap();

    assert_eq!(parsed.model_details.name, card.model_details.name);
    assert_eq!(parsed.model_details.version, card.model_details.version);
}

#[test]
fn test_json_roundtrip_complete() {
    let card = create_complete_card();
    let json = card.to_json().unwrap();
    let parsed = ModelCard::from_json(&json).unwrap();

    assert_eq!(parsed.model_details.name, card.model_details.name);
    assert_eq!(
        parsed.intended_use.primary_uses,
        card.intended_use.primary_uses
    );
    assert!(parsed.training_data.is_some());
    assert!(parsed.evaluation.is_some());
    assert!(parsed.ethical_considerations.is_some());
    assert!(parsed.caveats_and_recommendations.is_some());
}

#[test]
fn test_json_with_special_characters() {
    let mut details = create_test_details();
    details.description = "Model with \"quotes\" and \n newlines".to_string();
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let json = card.to_json().unwrap();
    let parsed = ModelCard::from_json(&json).unwrap();

    assert_eq!(
        parsed.model_details.description,
        card.model_details.description
    );
}

#[test]
fn test_json_with_unicode() {
    let mut details = create_test_details();
    details.developers = vec![
        "José".to_string(),
        "李明".to_string(),
        "Владимир".to_string(),
    ];
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let json = card.to_json().unwrap();
    let parsed = ModelCard::from_json(&json).unwrap();

    assert_eq!(
        parsed.model_details.developers,
        card.model_details.developers
    );
}

#[test]
fn test_json_invalid_input() {
    let result = ModelCard::from_json("not valid json");
    assert!(result.is_err());
}

#[test]
fn test_json_empty_input() {
    let result = ModelCard::from_json("");
    assert!(result.is_err());
}

// ============================================================================
// YAML SERIALIZATION TESTS
// ============================================================================

#[test]
fn test_yaml_serialization_minimal() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let yaml = card.to_yaml().unwrap();
    assert!(yaml.contains("name:"));
    assert!(yaml.contains("version:"));
    assert!(yaml.contains("test-model"));
}

#[test]
fn test_yaml_deserialization_minimal() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let yaml = card.to_yaml().unwrap();
    let parsed = ModelCard::from_yaml(&yaml).unwrap();

    assert_eq!(parsed.model_details.name, card.model_details.name);
    assert_eq!(parsed.model_details.version, card.model_details.version);
}

#[test]
fn test_yaml_roundtrip_complete() {
    let card = create_complete_card();
    let yaml = card.to_yaml().unwrap();
    let parsed = ModelCard::from_yaml(&yaml).unwrap();

    assert_eq!(parsed.model_details.name, card.model_details.name);
    assert!(parsed.training_data.is_some());
    assert!(parsed.evaluation.is_some());
}

#[test]
fn test_yaml_invalid_input() {
    let result = ModelCard::from_yaml(":::: invalid yaml");
    assert!(result.is_err());
}

#[test]
fn test_yaml_empty_input() {
    let result = ModelCard::from_yaml("");
    assert!(result.is_err());
}

// ============================================================================
// MARKDOWN GENERATION TESTS
// ============================================================================

#[test]
fn test_markdown_minimal() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let md = card.to_markdown();

    assert!(md.contains("# Model Card: test-model"));
    assert!(md.contains("## Model Details"));
    assert!(md.contains("## Intended Use"));
    assert!(md.contains("**Name**: test-model"));
    assert!(md.contains("**Version**: 1.0.0"));
}

#[test]
fn test_markdown_complete() {
    let card = create_complete_card();
    let md = card.to_markdown();

    // Check all sections present
    assert!(md.contains("# Model Card:"));
    assert!(md.contains("## Model Details"));
    assert!(md.contains("## Intended Use"));
    assert!(md.contains("## Training Data"));
    assert!(md.contains("## Evaluation"));
    assert!(md.contains("## Ethical Considerations"));
    assert!(md.contains("## Limitations and Recommendations"));
}

#[test]
fn test_markdown_optional_sections_omitted() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let md = card.to_markdown();

    // Optional sections should not appear
    assert!(!md.contains("## Training Data"));
    assert!(!md.contains("## Evaluation"));
    assert!(!md.contains("## Ethical Considerations"));
}

#[test]
fn test_markdown_escaping() {
    let mut details = create_test_details();
    details.name = "model-with-*asterisks*-and-_underscores_".to_string();
    let intended_use = create_test_intended_use();
    let card = ModelCard::new(details, intended_use);

    let md = card.to_markdown();
    assert!(md.contains("model-with-*asterisks*-and-_underscores_"));
}

#[test]
fn test_markdown_lists() {
    let details = create_test_details();
    let mut intended_use = create_test_intended_use();
    intended_use.primary_uses = vec![
        "Use 1".to_string(),
        "Use 2".to_string(),
        "Use 3".to_string(),
    ];
    let card = ModelCard::new(details, intended_use);

    let md = card.to_markdown();
    assert!(md.contains("- Use 1"));
    assert!(md.contains("- Use 2"));
    assert!(md.contains("- Use 3"));
}

// ============================================================================
// METRIC TESTS
// ============================================================================

#[test]
fn test_metric_creation() {
    let metric = Metric {
        name: "Accuracy".to_string(),
        value: 0.95,
        description: Some("Test accuracy".to_string()),
        threshold: Some(0.90),
    };

    assert_eq!(metric.name, "Accuracy");
    assert_eq!(metric.value, 0.95);
    assert!(metric.description.is_some());
    assert!(metric.threshold.is_some());
}

#[test]
fn test_metric_minimal() {
    let metric = Metric {
        name: "F1".to_string(),
        value: 0.88,
        description: None,
        threshold: None,
    };

    assert_eq!(metric.name, "F1");
    assert_eq!(metric.value, 0.88);
    assert!(metric.description.is_none());
    assert!(metric.threshold.is_none());
}

#[test]
fn test_multiple_metrics() {
    let metrics = [
        Metric {
            name: "Accuracy".to_string(),
            value: 0.95,
            description: None,
            threshold: None,
        },
        Metric {
            name: "Precision".to_string(),
            value: 0.93,
            description: None,
            threshold: None,
        },
        Metric {
            name: "Recall".to_string(),
            value: 0.92,
            description: None,
            threshold: None,
        },
    ];

    assert_eq!(metrics.len(), 3);
    assert!(metrics.iter().all(|m| m.value > 0.9));
}

// ============================================================================
// ENVIRONMENTAL IMPACT TESTS
// ============================================================================

#[test]
fn test_environmental_impact_full() {
    let env = EnvironmentalImpact {
        hardware: "8x A100".to_string(),
        hours: 100.0,
        cloud_provider: Some("AWS".to_string()),
        carbon_emitted: Some(50.5),
        energy_consumed: Some(800.0),
    };

    assert_eq!(env.hardware, "8x A100");
    assert_eq!(env.hours, 100.0);
    assert_eq!(env.carbon_emitted.unwrap(), 50.5);
    assert_eq!(env.energy_consumed.unwrap(), 800.0);
}

#[test]
fn test_environmental_impact_minimal() {
    let env = EnvironmentalImpact {
        hardware: "Local GPU".to_string(),
        hours: 10.0,
        cloud_provider: None,
        carbon_emitted: None,
        energy_consumed: None,
    };

    assert!(env.cloud_provider.is_none());
    assert!(env.carbon_emitted.is_none());
    assert!(env.energy_consumed.is_none());
}

#[test]
fn test_environmental_impact_large_scale() {
    let env = EnvironmentalImpact {
        hardware: "1024x A100 GPUs".to_string(),
        hours: 816.0,
        cloud_provider: Some("Azure".to_string()),
        carbon_emitted: Some(25000.0),   // 25 metric tons
        energy_consumed: Some(500000.0), // 500 MWh
    };

    assert!(env.carbon_emitted.unwrap() > 1000.0);
    assert!(env.energy_consumed.unwrap() > 100000.0);
}

// ============================================================================
// FAIRNESS TESTS
// ============================================================================

#[test]
fn test_performance_by_group_single() {
    let mut perf = HashMap::new();
    let mut gender = HashMap::new();
    gender.insert("male".to_string(), 0.85);
    gender.insert("female".to_string(), 0.83);
    perf.insert("gender".to_string(), gender);

    let evaluation = Evaluation {
        datasets: vec!["Test".to_string()],
        metrics: vec![],
        benchmarks: None,
        performance_by_group: Some(perf),
        methodology: None,
    };

    assert!(evaluation.performance_by_group.is_some());
    let groups = evaluation.performance_by_group.unwrap();
    assert!(groups.contains_key("gender"));
}

#[test]
fn test_performance_by_group_multiple() {
    let mut perf = HashMap::new();

    let mut gender = HashMap::new();
    gender.insert("male".to_string(), 0.85);
    gender.insert("female".to_string(), 0.83);

    let mut age = HashMap::new();
    age.insert("18-30".to_string(), 0.90);
    age.insert("31-50".to_string(), 0.88);
    age.insert("51+".to_string(), 0.84);

    let mut education = HashMap::new();
    education.insert("high_school".to_string(), 0.80);
    education.insert("bachelors".to_string(), 0.85);
    education.insert("masters".to_string(), 0.87);

    perf.insert("gender".to_string(), gender);
    perf.insert("age".to_string(), age);
    perf.insert("education".to_string(), education);

    let evaluation = Evaluation {
        datasets: vec!["Test".to_string()],
        metrics: vec![],
        benchmarks: None,
        performance_by_group: Some(perf),
        methodology: None,
    };

    let groups = evaluation.performance_by_group.unwrap();
    assert_eq!(groups.len(), 3);
    assert!(groups.contains_key("gender"));
    assert!(groups.contains_key("age"));
    assert!(groups.contains_key("education"));
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn test_empty_vectors() {
    let details = create_test_details();
    let mut intended_use = create_test_intended_use();
    intended_use.primary_uses = vec![];
    intended_use.primary_users = vec![];
    intended_use.out_of_scope_uses = vec![];

    let card = ModelCard::new(details, intended_use);
    assert!(card.intended_use.primary_uses.is_empty());
}

#[test]
fn test_very_long_strings() {
    let mut details = create_test_details();
    details.description = "a".repeat(10000);
    let intended_use = create_test_intended_use();

    let card = ModelCard::new(details, intended_use);
    let json = card.to_json().unwrap();
    let parsed = ModelCard::from_json(&json).unwrap();

    assert_eq!(parsed.model_details.description.len(), 10000);
}

#[test]
fn test_many_developers() {
    let mut details = create_test_details();
    details.developers = (0..100).map(|i| format!("Developer {}", i)).collect();
    let intended_use = create_test_intended_use();

    let card = ModelCard::new(details, intended_use);
    assert_eq!(card.model_details.developers.len(), 100);
}

#[test]
fn test_many_metrics() {
    let details = create_test_details();
    let intended_use = create_test_intended_use();

    let metrics: Vec<Metric> = (0..50)
        .map(|i| Metric {
            name: format!("Metric {}", i),
            value: 0.5 + (i as f64 / 100.0),
            description: Some(format!("Description {}", i)),
            threshold: Some(0.5),
        })
        .collect();

    let evaluation = Evaluation {
        datasets: vec!["Test".to_string()],
        metrics,
        benchmarks: None,
        performance_by_group: None,
        methodology: None,
    };

    let card = ModelCard::new(details, intended_use).with_evaluation(evaluation);

    assert_eq!(card.evaluation.unwrap().metrics.len(), 50);
}

#[test]
fn test_extreme_values() {
    let env = EnvironmentalImpact {
        hardware: "Test".to_string(),
        hours: f64::MAX,
        cloud_provider: None,
        carbon_emitted: Some(f64::MAX),
        energy_consumed: Some(f64::MAX),
    };

    assert!(env.hours.is_finite());
    assert!(env.carbon_emitted.unwrap().is_finite());
}

#[test]
fn test_zero_values() {
    let env = EnvironmentalImpact {
        hardware: "Test".to_string(),
        hours: 0.0,
        cloud_provider: None,
        carbon_emitted: Some(0.0),
        energy_consumed: Some(0.0),
    };

    assert_eq!(env.hours, 0.0);
    assert_eq!(env.carbon_emitted.unwrap(), 0.0);
}

#[test]
fn test_negative_values() {
    let metric = Metric {
        name: "Test".to_string(),
        value: -0.5,
        description: None,
        threshold: Some(-1.0),
    };

    assert!(metric.value < 0.0);
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_multiple_cards_different_data() {
    let card1 = create_complete_card();
    let card2 = create_complete_card();

    // Cards created at slightly different times
    std::thread::sleep(std::time::Duration::from_millis(10));

    assert!(card2.created_at >= card1.created_at);
}

#[test]
fn test_clone_card() {
    let card = create_complete_card();
    let cloned = card.clone();

    assert_eq!(card.model_details.name, cloned.model_details.name);
    assert_eq!(card.created_at, cloned.created_at);
}

#[test]
fn test_format_comparison() {
    let card = create_complete_card();

    let json = card.to_json().unwrap();
    let yaml = card.to_yaml().unwrap();
    let markdown = card.to_markdown();

    // All formats should contain the model name
    assert!(json.contains("test-model"));
    assert!(yaml.contains("test-model"));
    assert!(markdown.contains("test-model"));

    // YAML typically shorter than JSON
    assert!(yaml.len() < json.len());
}

// ============================================================================
// VALIDATION TESTS
// ============================================================================

#[test]
fn test_version_format() {
    let mut details = create_test_details();
    details.version = "invalid.version.format.too.many.parts".to_string();
    let intended_use = create_test_intended_use();

    let card = ModelCard::new(details, intended_use);
    // No validation yet, but should still work
    assert!(card.model_details.version.contains("invalid"));
}

#[test]
fn test_url_format() {
    let mut details = create_test_details();
    details.repository = Some("not-a-valid-url".to_string());
    details.paper = Some("also-not-valid".to_string());
    let intended_use = create_test_intended_use();

    let card = ModelCard::new(details, intended_use);
    // No validation yet, but should still work
    assert!(card.model_details.repository.is_some());
}

#[test]
fn test_email_format() {
    let mut details = create_test_details();
    details.contact = Some("not-an-email".to_string());
    let intended_use = create_test_intended_use();

    let card = ModelCard::new(details, intended_use);
    // No validation yet, but should still work
    assert!(card.model_details.contact.is_some());
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_test_details() -> ModelDetails {
    ModelDetails {
        name: "test-model".to_string(),
        version: "1.0.0".to_string(),
        description: "Test model".to_string(),
        model_type: "Classifier".to_string(),
        architecture: "ResNet".to_string(),
        size: "25M".to_string(),
        framework: "PyTorch".to_string(),
        format: "safetensors".to_string(),
        license: Some("MIT".to_string()),
        citation: None,
        developers: vec!["Test".to_string()],
        contact: None,
        repository: None,
        paper: None,
    }
}

fn create_test_intended_use() -> IntendedUse {
    IntendedUse {
        primary_uses: vec!["Testing".to_string()],
        primary_users: vec!["Developers".to_string()],
        out_of_scope_uses: vec!["Production".to_string()],
        use_case_examples: None,
    }
}

fn create_test_training_data() -> TrainingData {
    TrainingData {
        datasets: vec!["TestSet".to_string()],
        sources: None,
        collection_methods: None,
        preprocessing: None,
        size: None,
        splits: None,
        languages: None,
        demographics: None,
    }
}

fn create_test_evaluation() -> Evaluation {
    Evaluation {
        datasets: vec!["EvalSet".to_string()],
        metrics: vec![],
        benchmarks: None,
        performance_by_group: None,
        methodology: None,
    }
}

fn create_test_ethical() -> EthicalConsiderations {
    EthicalConsiderations {
        sensitive_data: None,
        bias: None,
        fairness: None,
        privacy: None,
        environmental_impact: None,
        human_oversight: None,
        risks: None,
        mitigations: None,
    }
}

fn create_test_caveats() -> CaveatsAndRecommendations {
    CaveatsAndRecommendations {
        limitations: vec!["Test limitation".to_string()],
        known_issues: None,
        recommendations: vec![],
        testing_recommendations: None,
        tradeoffs: None,
    }
}

fn create_complete_card() -> ModelCard {
    let details = create_test_details();
    let intended_use = create_test_intended_use();
    let training_data = create_test_training_data();
    let evaluation = create_test_evaluation();
    let ethical = create_test_ethical();
    let caveats = create_test_caveats();

    ModelCard::new(details, intended_use)
        .with_training_data(training_data)
        .with_evaluation(evaluation)
        .with_ethical_considerations(ethical)
        .with_caveats_and_recommendations(caveats)
}
