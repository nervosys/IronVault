// Model Card Demo - Complete demonstration of model card functionality
//
// This example shows:
// 1. Creating comprehensive model cards
// 2. Adding all sections (details, intended use, training, evaluation, ethics)
// 3. Exporting to JSON, YAML, and Markdown
// 4. Integration with vault storage
// 5. Real-world examples for different model types

use ironvault::model_card::*;
use ironvault::Result;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("=== IronVault - Model Card Demo ===\n");

    // Demo 1: LLM Model Card
    println!("📝 Demo 1: Large Language Model Card");
    println!("═══════════════════════════════════════\n");
    demo_llm_model_card()?;

    // Demo 2: Image Classifier Model Card
    println!("\n📝 Demo 2: Image Classification Model Card");
    println!("═══════════════════════════════════════════\n");
    demo_image_classifier_card()?;

    // Demo 3: Model Card with Environmental Impact
    println!("\n📝 Demo 3: Model Card with Environmental Impact");
    println!("═══════════════════════════════════════════════\n");
    demo_environmental_impact()?;

    // Demo 4: Model Card Export Formats
    println!("\n📝 Demo 4: Export to Different Formats");
    println!("═══════════════════════════════════════════\n");
    demo_export_formats()?;

    // Demo 5: Model Card with Fairness Metrics
    println!("\n📝 Demo 5: Model Card with Fairness Analysis");
    println!("═════════════════════════════════════════════\n");
    demo_fairness_metrics()?;

    println!("\n✅ All model card demos completed successfully!");

    Ok(())
}

fn demo_llm_model_card() -> Result<()> {
    // Create comprehensive model details
    let details = ModelDetails {
        name: "NervosysChat-7B".to_string(),
        version: "1.0.0".to_string(),
        description:
            "A 7 billion parameter large language model fine-tuned for conversational AI. \
                     Based on the LLaMA-2 architecture with custom instruction tuning for \
                     enterprise customer support applications."
                .to_string(),
        model_type: "Large Language Model (Causal Language Model)".to_string(),
        architecture: "Transformer (Decoder-only, 32 layers)".to_string(),
        size: "7.2B parameters (13.5 GB FP16)".to_string(),
        framework: "PyTorch 2.1".to_string(),
        format: "safetensors".to_string(),
        license: Some("MIT License".to_string()),
        citation: Some(
            r#"@misc{nervosyschat7b2024,
  title={NervosysChat-7B: Enterprise Conversational AI},
  author={NERVOSYS AI Team},
  year={2024},
  url={https://github.com/nervosys/nervosyschat}
}"#
            .to_string(),
        ),
        developers: vec![
            "NERVOSYS AI Team".to_string(),
            "Research Lab Contributors".to_string(),
        ],
        contact: Some("ai-team@nervosys.ai".to_string()),
        repository: Some("https://github.com/nervosys/nervosyschat".to_string()),
        paper: Some("https://arxiv.org/abs/2024.xxxxx".to_string()),
    };

    // Define intended use
    let intended_use = IntendedUse {
        primary_uses: vec![
            "Customer support chatbot".to_string(),
            "Technical documentation Q&A".to_string(),
            "Enterprise knowledge base queries".to_string(),
            "Task-oriented dialog systems".to_string(),
        ],
        primary_users: vec![
            "Enterprise software developers".to_string(),
            "Customer support teams".to_string(),
            "IT administrators".to_string(),
        ],
        out_of_scope_uses: vec![
            "Medical diagnosis or treatment recommendations".to_string(),
            "Legal advice or contract interpretation".to_string(),
            "Financial advice or investment decisions".to_string(),
            "Content that could cause harm or spread misinformation".to_string(),
            "Generation of personally identifiable information".to_string(),
        ],
        use_case_examples: Some(vec![
            "Answering software troubleshooting questions".to_string(),
            "Providing product documentation summaries".to_string(),
            "Routing customer inquiries to appropriate departments".to_string(),
        ]),
    };

    // Add training data information
    let training_data = TrainingData {
        datasets: vec![
            "Custom enterprise corpus (proprietary)".to_string(),
            "Stack Overflow Q&A (filtered)".to_string(),
            "Technical documentation corpus".to_string(),
            "Customer support transcripts (anonymized)".to_string(),
        ],
        sources: Some(vec![
            "Internal knowledge base".to_string(),
            "Public technical forums".to_string(),
            "Open-source documentation".to_string(),
        ]),
        collection_methods: Some(
            "Data collected from January 2023 to December 2023. \
                                 All customer data was anonymized and filtered for PII. \
                                 Stack Overflow data used under CC BY-SA license."
                .to_string(),
        ),
        preprocessing: Some(vec![
            "PII removal using named entity recognition".to_string(),
            "Profanity and offensive content filtering".to_string(),
            "Deduplication using MinHash LSH".to_string(),
            "Quality filtering (grammar, coherence)".to_string(),
        ]),
        size: Some("50 billion tokens (100 GB cleaned text)".to_string()),
        splits: Some({
            let mut splits = HashMap::new();
            splits.insert("train".to_string(), "48B tokens (96%)".to_string());
            splits.insert("validation".to_string(), "1B tokens (2%)".to_string());
            splits.insert("test".to_string(), "1B tokens (2%)".to_string());
            splits
        }),
        languages: Some(vec!["English".to_string()]),
        demographics: Some(
            "Data represents English-language technical support interactions. \
                           Primarily from North American and European regions."
                .to_string(),
        ),
    };

    // Add evaluation metrics
    let evaluation = Evaluation {
        datasets: vec![
            "Custom enterprise test set".to_string(),
            "MMLU (Massive Multitask Language Understanding)".to_string(),
            "HumanEval (code generation)".to_string(),
        ],
        metrics: vec![
            Metric {
                name: "Accuracy (Enterprise Test Set)".to_string(),
                value: 0.872,
                description: Some("Exact match accuracy on customer support queries".to_string()),
                threshold: Some(0.80),
            },
            Metric {
                name: "MMLU Score".to_string(),
                value: 0.652,
                description: Some("Average across 57 subjects".to_string()),
                threshold: None,
            },
            Metric {
                name: "HumanEval Pass@1".to_string(),
                value: 0.427,
                description: Some("Code generation correctness".to_string()),
                threshold: None,
            },
            Metric {
                name: "Response Relevance".to_string(),
                value: 0.91,
                description: Some(
                    "Human-rated relevance score (1-5 scale, normalized)".to_string(),
                ),
                threshold: Some(0.85),
            },
        ],
        benchmarks: Some({
            let mut benchmarks = HashMap::new();
            benchmarks.insert("MMLU".to_string(), 65.2);
            benchmarks.insert("HumanEval".to_string(), 42.7);
            benchmarks.insert("GSM8K (math)".to_string(), 58.3);
            benchmarks.insert("BBH (reasoning)".to_string(), 51.8);
            benchmarks
        }),
        performance_by_group: None,
        methodology: Some(
            "Models evaluated using greedy decoding with temperature=0. \
                          Enterprise test set consists of 10,000 real customer queries \
                          with expert-verified ground truth responses."
                .to_string(),
        ),
    };

    // Add ethical considerations
    let ethical = EthicalConsiderations {
        sensitive_data: Some(
            "Training data was screened for PII and sensitive information. \
                             All customer transcripts were anonymized before use."
                .to_string(),
        ),
        bias: Some(vec![
            "Model trained primarily on English technical content from Western sources".to_string(),
            "May have reduced performance on non-technical domains".to_string(),
            "Training data skews toward Stack Overflow demographics".to_string(),
        ]),
        fairness: Some(vec![
            "Performance evaluated across different query complexities".to_string(),
            "No significant performance differences found across technical domains".to_string(),
            "Further evaluation needed for non-English use cases".to_string(),
        ]),
        privacy: Some(
            "Model does not store or memorize user queries. No user data \
                      is retained after inference."
                .to_string(),
        ),
        environmental_impact: Some(EnvironmentalImpact {
            hardware: "8x NVIDIA A100 80GB GPUs".to_string(),
            hours: 240.0,
            cloud_provider: Some("AWS (us-east-1)".to_string()),
            carbon_emitted: Some(156.8),   // kg CO2e
            energy_consumed: Some(1920.0), // kWh
        }),
        human_oversight: Some(
            "Recommended for human-in-the-loop customer support. \
                              Not recommended for fully autonomous decision-making."
                .to_string(),
        ),
        risks: Some(vec![
            "May generate plausible-sounding but incorrect technical information".to_string(),
            "Could reflect biases present in training data".to_string(),
            "May not handle edge cases or novel scenarios appropriately".to_string(),
        ]),
        mitigations: Some(vec![
            "Implement confidence scoring for responses".to_string(),
            "Route low-confidence queries to human agents".to_string(),
            "Regular monitoring of model outputs for quality and safety".to_string(),
            "Continuous feedback collection from users".to_string(),
        ]),
    };

    // Add caveats and recommendations
    let caveats = CaveatsAndRecommendations {
        limitations: vec![
            "Limited to English language input and output".to_string(),
            "Knowledge cutoff date: December 2023".to_string(),
            "May struggle with highly specialized or domain-specific queries".to_string(),
            "Not suitable for real-time applications requiring <100ms latency".to_string(),
        ],
        known_issues: Some(vec![
            "Occasionally generates overly verbose responses".to_string(),
            "May repeat information when uncertain".to_string(),
            "Limited mathematical reasoning capabilities".to_string(),
        ]),
        recommendations: vec![
            "Deploy with human oversight for critical applications".to_string(),
            "Implement input validation and output filtering".to_string(),
            "Monitor for drift in production performance".to_string(),
            "Retrain or fine-tune regularly with new data".to_string(),
            "Use with explicit user guidelines about limitations".to_string(),
        ],
        testing_recommendations: Some(vec![
            "Test on domain-specific queries before deployment".to_string(),
            "Evaluate performance on edge cases and adversarial inputs".to_string(),
            "Conduct user studies with target end-users".to_string(),
        ]),
        tradeoffs: Some(vec![
            "Size vs. performance: 7B model balances quality and inference speed".to_string(),
            "Specialization vs. generalization: Optimized for support, less capable on general tasks".to_string(),
        ]),
    };

    // Create complete model card
    let card = ModelCard::new(details, intended_use)
        .with_training_data(training_data)
        .with_evaluation(evaluation)
        .with_ethical_considerations(ethical)
        .with_caveats_and_recommendations(caveats)
        .add_metadata("training_date".to_string(), "2024-01-15".to_string())
        .add_metadata("base_model".to_string(), "LLaMA-2-7B".to_string())
        .add_metadata(
            "fine_tuning_method".to_string(),
            "LoRA + Full Fine-tuning".to_string(),
        );

    println!("✅ Created model card: {}", card.model_details.name);
    println!("   Version: {}", card.model_details.version);
    println!("   Model Type: {}", card.model_details.model_type);
    println!("   Parameters: {}", card.model_details.size);
    println!("\n📊 Evaluation Metrics:");
    if let Some(eval) = &card.evaluation {
        for metric in &eval.metrics {
            println!("   • {}: {:.3}", metric.name, metric.value);
        }
    }

    println!("\n🌍 Environmental Impact:");
    if let Some(ethical) = &card.ethical_considerations {
        if let Some(impact) = &ethical.environmental_impact {
            println!("   • Hardware: {}", impact.hardware);
            println!("   • Training Hours: {:.1}", impact.hours);
            if let Some(carbon) = impact.carbon_emitted {
                println!("   • Carbon Emitted: {:.1} kg CO2e", carbon);
            }
        }
    }

    Ok(())
}

fn demo_image_classifier_card() -> Result<()> {
    let details = ModelDetails {
        name: "MedicalImageNet-ResNet50".to_string(),
        version: "2.1.3".to_string(),
        description: "ResNet-50 model fine-tuned for medical image classification. \
                     Trained to identify 10 common medical conditions from X-ray images."
            .to_string(),
        model_type: "Image Classification (Convolutional Neural Network)".to_string(),
        architecture: "ResNet-50 (50 layers, residual connections)".to_string(),
        size: "25.6M parameters (97 MB)".to_string(),
        framework: "PyTorch 2.0".to_string(),
        format: "safetensors".to_string(),
        license: Some("Research Use Only - Not for Clinical Use".to_string()),
        citation: None,
        developers: vec!["Medical AI Research Team".to_string()],
        contact: Some("research@medical-ai.org".to_string()),
        repository: Some("https://github.com/medical-ai/imagenet-resnet50".to_string()),
        paper: Some("https://arxiv.org/abs/2024.medical-vision".to_string()),
    };

    let intended_use = IntendedUse {
        primary_uses: vec![
            "Research into automated medical image analysis".to_string(),
            "Educational demonstrations of ML in healthcare".to_string(),
            "Benchmarking other medical image models".to_string(),
        ],
        primary_users: vec![
            "Medical AI researchers".to_string(),
            "Healthcare ML engineers (R&D only)".to_string(),
            "Academic institutions".to_string(),
        ],
        out_of_scope_uses: vec![
            "❌ CLINICAL DIAGNOSIS - Not FDA approved".to_string(),
            "❌ Patient treatment decisions".to_string(),
            "❌ Production healthcare systems".to_string(),
            "❌ Any use that could impact patient care".to_string(),
        ],
        use_case_examples: Some(vec![
            "Comparing model architectures for medical imaging".to_string(),
            "Teaching ML applications in healthcare".to_string(),
        ]),
    };

    let training_data = TrainingData {
        datasets: vec![
            "ChestX-ray14 (NIH)".to_string(),
            "MIMIC-CXR (subset)".to_string(),
        ],
        sources: Some(vec![
            "NIH Clinical Center".to_string(),
            "MIT MIMIC database".to_string(),
        ]),
        collection_methods: Some(
            "Retrospective collection from 2008-2015. \
                                 All patient identifiers removed."
                .to_string(),
        ),
        preprocessing: Some(vec![
            "Image resize to 224x224".to_string(),
            "Normalization (ImageNet stats)".to_string(),
            "DICOM metadata removal".to_string(),
            "Data augmentation (rotation, flip, crop)".to_string(),
        ]),
        size: Some("112,000 images (10 classes)".to_string()),
        splits: Some({
            let mut splits = HashMap::new();
            splits.insert("train".to_string(), "80,000 images".to_string());
            splits.insert("validation".to_string(), "16,000 images".to_string());
            splits.insert("test".to_string(), "16,000 images".to_string());
            splits
        }),
        languages: None,
        demographics: Some(
            "Patient demographics: Ages 18-90, mix of genders. \
                           Data primarily from North American hospitals. \
                           Demographic distribution may not represent global population."
                .to_string(),
        ),
    };

    let evaluation = Evaluation {
        datasets: vec!["MIMIC-CXR Test Set".to_string()],
        metrics: vec![
            Metric {
                name: "Top-1 Accuracy".to_string(),
                value: 0.834,
                description: Some("Exact class match".to_string()),
                threshold: None,
            },
            Metric {
                name: "Top-5 Accuracy".to_string(),
                value: 0.962,
                description: Some("Correct class in top 5 predictions".to_string()),
                threshold: None,
            },
            Metric {
                name: "F1 Score (Macro)".to_string(),
                value: 0.817,
                description: Some("Harmonic mean of precision and recall".to_string()),
                threshold: None,
            },
            Metric {
                name: "AUC-ROC".to_string(),
                value: 0.891,
                description: Some("Area under ROC curve".to_string()),
                threshold: Some(0.85),
            },
        ],
        benchmarks: None,
        performance_by_group: Some({
            let mut by_group = HashMap::new();

            let mut age_group = HashMap::new();
            age_group.insert("18-40".to_string(), 0.847);
            age_group.insert("41-60".to_string(), 0.839);
            age_group.insert("61+".to_string(), 0.816);
            by_group.insert("age_groups".to_string(), age_group);

            let mut gender_group = HashMap::new();
            gender_group.insert("male".to_string(), 0.838);
            gender_group.insert("female".to_string(), 0.829);
            by_group.insert("gender".to_string(), gender_group);

            by_group
        }),
        methodology: Some(
            "5-fold cross-validation on test set. \
                          Performance reported as mean across folds."
                .to_string(),
        ),
    };

    let ethical = EthicalConsiderations {
        sensitive_data: Some(
            "⚠️ CONTAINS MEDICAL DATA. All patient identifiers removed \
                             but images may still be sensitive."
                .to_string(),
        ),
        bias: Some(vec![
            "Training data from North American hospitals only".to_string(),
            "May not generalize to other populations or imaging equipment".to_string(),
            "Performance varies by patient age group (see fairness metrics)".to_string(),
        ]),
        fairness: Some(vec![
            "Evaluated across age and gender groups".to_string(),
            "Slight performance drop for 61+ age group".to_string(),
            "No evaluation on racial/ethnic groups due to data limitations".to_string(),
        ]),
        privacy: Some(
            "Patient data de-identified per HIPAA standards. \
                      Risk of re-identification is low but non-zero."
                .to_string(),
        ),
        environmental_impact: Some(EnvironmentalImpact {
            hardware: "4x NVIDIA V100 32GB GPUs".to_string(),
            hours: 72.0,
            cloud_provider: Some("On-premises cluster".to_string()),
            carbon_emitted: Some(28.4),
            energy_consumed: Some(576.0),
        }),
        human_oversight: Some(
            "⚠️ REQUIRES EXPERT OVERSIGHT. Should only be used as \
                              a decision support tool, never as sole diagnostic method."
                .to_string(),
        ),
        risks: Some(vec![
            "❌ False negatives could delay diagnosis".to_string(),
            "❌ False positives could cause unnecessary procedures".to_string(),
            "❌ Model may fail on novel pathologies not in training data".to_string(),
            "❌ Performance may degrade on different imaging equipment".to_string(),
        ]),
        mitigations: Some(vec![
            "Require radiologist review of all predictions".to_string(),
            "Display confidence scores with all predictions".to_string(),
            "Implement uncertainty quantification".to_string(),
            "Regular performance monitoring in deployment".to_string(),
        ]),
    };

    let caveats = CaveatsAndRecommendations {
        limitations: vec![
            "❌ NOT FDA APPROVED - Research use only".to_string(),
            "Limited to chest X-rays (frontal view)".to_string(),
            "Trained on specific imaging protocols and equipment".to_string(),
            "10 condition classes (many medical conditions not covered)".to_string(),
        ],
        known_issues: Some(vec![
            "Struggles with rare conditions (< 1% of training data)".to_string(),
            "May misclassify when multiple conditions present".to_string(),
            "Performance degrades on low-quality or corrupted images".to_string(),
        ]),
        recommendations: vec![
            "🏥 DO NOT USE FOR CLINICAL DIAGNOSIS".to_string(),
            "Always require board-certified radiologist review".to_string(),
            "Validate on your specific imaging equipment before research use".to_string(),
            "Monitor for distribution shift in production".to_string(),
            "Obtain appropriate ethics approval for research studies".to_string(),
        ],
        testing_recommendations: Some(vec![
            "Test on images from your institution's equipment".to_string(),
            "Evaluate on diverse patient populations".to_string(),
            "Compare against expert radiologist performance".to_string(),
        ]),
        tradeoffs: Some(vec![
            "ResNet-50 chosen for balance of accuracy and inference speed".to_string(),
            "Larger models (ResNet-101, EfficientNet) may provide better accuracy".to_string(),
        ]),
    };

    let card = ModelCard::new(details, intended_use)
        .with_training_data(training_data)
        .with_evaluation(evaluation)
        .with_ethical_considerations(ethical)
        .with_caveats_and_recommendations(caveats)
        .add_metadata(
            "regulatory_status".to_string(),
            "Research Only - Not Approved".to_string(),
        )
        .add_metadata("last_validation".to_string(), "2024-09-15".to_string());

    println!(
        "✅ Created medical imaging model card: {}",
        card.model_details.name
    );
    println!(
        "   ⚠️ License: {}",
        card.model_details.license.as_ref().unwrap()
    );
    println!("\n📊 Performance by Age Group:");
    if let Some(eval) = &card.evaluation {
        if let Some(by_group) = &eval.performance_by_group {
            if let Some(age_groups) = by_group.get("age_groups") {
                for (age, score) in age_groups {
                    println!("   • Age {}: {:.3}", age, score);
                }
            }
        }
    }

    Ok(())
}

fn demo_environmental_impact() -> Result<()> {
    let details = ModelDetails {
        name: "GPT-Style-175B".to_string(),
        version: "1.0.0".to_string(),
        description: "Large-scale language model with 175 billion parameters".to_string(),
        model_type: "Large Language Model".to_string(),
        architecture: "Transformer (96 layers)".to_string(),
        size: "175B parameters (350 GB FP16)".to_string(),
        framework: "PyTorch 2.1".to_string(),
        format: "safetensors".to_string(),
        license: Some("Research License".to_string()),
        citation: None,
        developers: vec!["Research Lab".to_string()],
        contact: None,
        repository: None,
        paper: None,
    };

    let intended_use = IntendedUse {
        primary_uses: vec!["Language generation".to_string()],
        primary_users: vec!["Researchers".to_string()],
        out_of_scope_uses: vec!["Production use".to_string()],
        use_case_examples: None,
    };

    // Comprehensive environmental impact
    let ethical = EthicalConsiderations {
        sensitive_data: None,
        bias: None,
        fairness: None,
        privacy: None,
        environmental_impact: Some(EnvironmentalImpact {
            hardware: "1024x NVIDIA A100 80GB GPUs (HGX systems)".to_string(),
            hours: 34.0 * 24.0, // 34 days
            cloud_provider: Some("Azure ML (West US 2)".to_string()),
            carbon_emitted: Some(25_000.0),   // 25 metric tons CO2e
            energy_consumed: Some(500_000.0), // 500 MWh
        }),
        human_oversight: None,
        risks: Some(vec![
            "Significant environmental cost of training".to_string()
        ]),
        mitigations: Some(vec![
            "Used renewable energy data center".to_string(),
            "Purchased carbon offsets".to_string(),
            "Open-sourced model to prevent duplicate training".to_string(),
        ]),
    };

    let card = ModelCard::new(details, intended_use).with_ethical_considerations(ethical);

    println!("✅ Large-scale model environmental impact:");
    if let Some(ethical) = &card.ethical_considerations {
        if let Some(impact) = &ethical.environmental_impact {
            println!("   🖥️  Hardware: {}", impact.hardware);
            println!(
                "   ⏱️  Training Time: {:.0} hours ({:.0} days)",
                impact.hours,
                impact.hours / 24.0
            );
            if let Some(energy) = impact.energy_consumed {
                println!(
                    "   ⚡ Energy Consumed: {:.0} kWh ({:.0} MWh)",
                    energy,
                    energy / 1000.0
                );
            }
            if let Some(carbon) = impact.carbon_emitted {
                println!(
                    "   🌍 Carbon Emitted: {:.0} kg CO2e ({:.1} metric tons)",
                    carbon,
                    carbon / 1000.0
                );
                println!(
                    "      Equivalent to ~{:.0} transcontinental flights",
                    carbon / 2500.0
                );
            }
        }
        if let Some(mitigations) = &ethical.mitigations {
            println!("\n   ♻️  Mitigations:");
            for mitigation in mitigations {
                println!("      • {}", mitigation);
            }
        }
    }

    Ok(())
}

fn demo_export_formats() -> Result<()> {
    let details = ModelDetails {
        name: "ExampleModel".to_string(),
        version: "1.0.0".to_string(),
        description: "Example model for format demonstration".to_string(),
        model_type: "Classifier".to_string(),
        architecture: "Transformer".to_string(),
        size: "100M parameters".to_string(),
        framework: "PyTorch".to_string(),
        format: "safetensors".to_string(),
        license: Some("MIT".to_string()),
        citation: None,
        developers: vec!["Team".to_string()],
        contact: None,
        repository: None,
        paper: None,
    };

    let intended_use = IntendedUse {
        primary_uses: vec!["Classification".to_string()],
        primary_users: vec!["Developers".to_string()],
        out_of_scope_uses: vec!["None specified".to_string()],
        use_case_examples: None,
    };

    let card = ModelCard::new(details, intended_use);

    // Export to JSON
    println!("📄 JSON Export (first 300 chars):");
    let json = card.to_json()?;
    println!("{}", &json[..json.len().min(300)]);
    println!("   ... ({} total chars)\n", json.len());

    // Export to YAML
    println!("📄 YAML Export (first 300 chars):");
    let yaml = card.to_yaml()?;
    println!("{}", &yaml[..yaml.len().min(300)]);
    println!("   ... ({} total chars)\n", yaml.len());

    // Export to Markdown
    println!("📄 Markdown Export (first 500 chars):");
    let markdown = card.to_markdown();
    println!("{}", &markdown[..markdown.len().min(500)]);
    println!("   ... ({} total chars)", markdown.len());

    Ok(())
}

fn demo_fairness_metrics() -> Result<()> {
    let details = ModelDetails {
        name: "ResumeScreener-BERT".to_string(),
        version: "1.2.1".to_string(),
        description: "BERT-based model for automated resume screening".to_string(),
        model_type: "Binary Classification".to_string(),
        architecture: "BERT-base".to_string(),
        size: "110M parameters".to_string(),
        framework: "PyTorch".to_string(),
        format: "safetensors".to_string(),
        license: Some("Internal Use Only".to_string()),
        citation: None,
        developers: vec!["HR Tech Team".to_string()],
        contact: Some("hr-tech@company.com".to_string()),
        repository: None,
        paper: None,
    };

    let intended_use = IntendedUse {
        primary_uses: vec![
            "Initial resume screening for technical positions".to_string(),
            "Filtering qualified candidates for review".to_string(),
        ],
        primary_users: vec!["HR recruiters".to_string()],
        out_of_scope_uses: vec![
            "Final hiring decisions (requires human review)".to_string(),
            "Roles outside software engineering".to_string(),
        ],
        use_case_examples: Some(vec![
            "Screen 1000+ applications for senior engineer role".to_string()
        ]),
    };

    // Detailed fairness evaluation
    let evaluation = Evaluation {
        datasets: vec!["Internal candidate database (2020-2023)".to_string()],
        metrics: vec![Metric {
            name: "Overall Accuracy".to_string(),
            value: 0.823,
            description: Some("Accuracy across all demographics".to_string()),
            threshold: Some(0.80),
        }],
        benchmarks: None,
        performance_by_group: Some({
            let mut by_group = HashMap::new();

            // Gender performance
            let mut gender = HashMap::new();
            gender.insert("male".to_string(), 0.831);
            gender.insert("female".to_string(), 0.817);
            gender.insert("non-binary".to_string(), 0.809);
            by_group.insert("gender_inference".to_string(), gender);

            // Years of experience
            let mut experience = HashMap::new();
            experience.insert("0-2 years".to_string(), 0.789);
            experience.insert("3-5 years".to_string(), 0.831);
            experience.insert("6-10 years".to_string(), 0.845);
            experience.insert("10+ years".to_string(), 0.828);
            by_group.insert("experience".to_string(), experience);

            // Education level
            let mut education = HashMap::new();
            education.insert("bootcamp".to_string(), 0.792);
            education.insert("bachelors".to_string(), 0.834);
            education.insert("masters".to_string(), 0.841);
            education.insert("phd".to_string(), 0.819);
            by_group.insert("education".to_string(), education);

            by_group
        }),
        methodology: Some(
            "Fairness evaluated using demographic proxy inference. \
                          Gender inferred from name analysis (imperfect proxy). \
                          Disparate impact ratios calculated for all groups."
                .to_string(),
        ),
    };

    let ethical = EthicalConsiderations {
        sensitive_data: Some(
            "Resume data contains names, education, work history. \
                             Potential for demographic inference."
                .to_string(),
        ),
        bias: Some(vec![
            "Model may reflect biases in historical hiring data".to_string(),
            "Performance varies by years of experience".to_string(),
            "Bootcamp graduates have lower pass rates".to_string(),
            "Name-based gender inference may be inaccurate".to_string(),
        ]),
        fairness: Some(vec![
            "⚠️ Gender performance gap: 2.4% (male vs non-binary)".to_string(),
            "⚠️ Education disparity: Bootcamp vs. Bachelors (5.3% gap)".to_string(),
            "✅ Within legal threshold (<80% rule) but monitoring required".to_string(),
            "Disparate impact analysis conducted quarterly".to_string(),
        ]),
        privacy: Some("Resumes stored encrypted. Access logged and audited.".to_string()),
        environmental_impact: None,
        human_oversight: Some(
            "⚠️ REQUIRED HUMAN REVIEW. All candidates flagged by model \
                              must be reviewed by HR recruiter before rejection."
                .to_string(),
        ),
        risks: Some(vec![
            "May perpetuate historical hiring biases".to_string(),
            "Could discriminate against non-traditional backgrounds".to_string(),
            "Risk of disparate impact on protected classes".to_string(),
        ]),
        mitigations: Some(vec![
            "Quarterly fairness audits by external consultants".to_string(),
            "Manual review of all rejections".to_string(),
            "Blind review process (names/demographics removed)".to_string(),
            "Continuous monitoring of acceptance rates by group".to_string(),
            "Regular retraining with diverse candidate pool".to_string(),
        ]),
    };

    let caveats = CaveatsAndRecommendations {
        limitations: vec![
            "Training data from 2020-2023 only".to_string(),
            "May not reflect current job market trends".to_string(),
            "Limited to software engineering positions".to_string(),
        ],
        known_issues: Some(vec![
            "Lower accuracy for candidates with career gaps".to_string(),
            "May undervalue non-traditional education paths".to_string(),
        ]),
        recommendations: vec![
            "Use only as initial screening, not final decision".to_string(),
            "Regularly audit for fairness and bias".to_string(),
            "Provide appeal process for rejected candidates".to_string(),
            "Document all automated decisions for compliance".to_string(),
        ],
        testing_recommendations: Some(vec![
            "A/B test against human screeners".to_string(),
            "Measure long-term hiring outcomes by group".to_string(),
        ]),
        tradeoffs: Some(vec![
            "Efficiency vs. Fairness: Faster screening but requires vigilance".to_string(),
        ]),
    };

    let card = ModelCard::new(details, intended_use)
        .with_evaluation(evaluation)
        .with_ethical_considerations(ethical)
        .with_caveats_and_recommendations(caveats)
        .add_metadata("legal_review_date".to_string(), "2024-08-01".to_string())
        .add_metadata("fairness_audit_date".to_string(), "2024-10-15".to_string());

    println!("✅ Created hiring model card with fairness analysis");
    println!("   Model: {}", card.model_details.name);
    println!("\n⚖️ Fairness Metrics:");

    if let Some(eval) = &card.evaluation {
        if let Some(by_group) = &eval.performance_by_group {
            println!("\n   📊 Performance by Gender (inferred):");
            if let Some(gender) = by_group.get("gender_inference") {
                for (group, score) in gender {
                    println!("      • {}: {:.3}", group, score);
                }
            }

            println!("\n   📊 Performance by Experience:");
            if let Some(exp) = by_group.get("experience") {
                for (group, score) in exp {
                    println!("      • {}: {:.3}", group, score);
                }
            }

            println!("\n   📊 Performance by Education:");
            if let Some(edu) = by_group.get("education") {
                for (group, score) in edu {
                    println!("      • {}: {:.3}", group, score);
                }
            }
        }
    }

    if let Some(ethical) = &card.ethical_considerations {
        if let Some(fairness) = &ethical.fairness {
            println!("\n   🔍 Fairness Considerations:");
            for consideration in fairness {
                println!("      {}", consideration);
            }
        }
    }

    Ok(())
}
