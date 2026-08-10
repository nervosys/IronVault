//! Model card command handlers (create, show, validate, convert, template, attach, extract, generate).

use ironvault::model_card::*;
use ironvault::{Result, VaultConfig, VaultError};
use std::io::{self, Write};

use crate::cli::args::CardCommands;
use crate::cli::helpers::build_vault;

pub fn handle_card(command: CardCommands, config: VaultConfig, use_sqlite: bool) -> Result<()> {
    match command {
        CardCommands::Create {
            name,
            version,
            description,
            model_type,
            architecture,
            output,
            interactive,
        } => {
            println!("📝 Creating model card: {}", name);

            let mut details = ModelDetails {
                name: name.clone(),
                version,
                description: description.clone(),
                model_type,
                architecture,
                size: String::new(),
                framework: String::new(),
                format: String::new(),
                license: None,
                citation: None,
                developers: vec![],
                contact: None,
                repository: None,
                paper: None,
            };

            let mut intended_use = IntendedUse {
                primary_uses: vec![],
                primary_users: vec![],
                out_of_scope_uses: vec![],
                use_case_examples: None,
            };

            if interactive {
                println!("\n🔧 Interactive mode - Fill in additional details");
                println!("(Press Enter to skip optional fields)\n");

                // Size
                print!("Model size (e.g., '7B parameters', '125M'): ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                details.size = input.trim().to_string();

                // Framework
                print!("Framework (e.g., 'PyTorch', 'TensorFlow'): ");
                io::stdout().flush()?;
                input.clear();
                io::stdin().read_line(&mut input)?;
                details.framework = input.trim().to_string();

                // Format
                print!("Model format (e.g., 'safetensors', 'onnx'): ");
                io::stdout().flush()?;
                input.clear();
                io::stdin().read_line(&mut input)?;
                details.format = input.trim().to_string();

                // License
                print!("License (e.g., 'MIT', 'Apache-2.0'): ");
                io::stdout().flush()?;
                input.clear();
                io::stdin().read_line(&mut input)?;
                let license_str = input.trim().to_string();
                if !license_str.is_empty() {
                    details.license = Some(license_str);
                }

                // Primary uses
                println!("\nPrimary uses (one per line, empty line to finish):");
                loop {
                    print!("  > ");
                    io::stdout().flush()?;
                    input.clear();
                    io::stdin().read_line(&mut input)?;
                    let use_case = input.trim().to_string();
                    if use_case.is_empty() {
                        break;
                    }
                    intended_use.primary_uses.push(use_case);
                }

                // Out-of-scope uses
                println!("\nOut-of-scope uses (one per line, empty line to finish):");
                loop {
                    print!("  > ");
                    io::stdout().flush()?;
                    input.clear();
                    io::stdin().read_line(&mut input)?;
                    let use_case = input.trim().to_string();
                    if use_case.is_empty() {
                        break;
                    }
                    intended_use.out_of_scope_uses.push(use_case);
                }
            }

            let card = ModelCard::new(details, intended_use);

            // Determine output format from extension
            let ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("json");
            let content = match ext {
                "yaml" | "yml" => card.to_yaml()?,
                "md" | "markdown" => card.to_markdown(),
                _ => card.to_json()?,
            };

            std::fs::write(&output, content)?;
            println!("✅ Model card created: {}", output.display());
            println!("   Format: {}", ext);
        }

        CardCommands::Show { path, format } => {
            println!("📖 Loading model card: {}", path.display());

            let content = std::fs::read_to_string(&path)?;
            let card = if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                ModelCard::from_yaml(&content)?
            } else {
                ModelCard::from_json(&content)?
            };

            let output = match format.as_str() {
                "yaml" | "yml" => card.to_yaml()?,
                "markdown" | "md" => card.to_markdown(),
                _ => card.to_json()?,
            };

            println!("\n{}", output);
        }

        CardCommands::Validate { path, strict } => {
            println!("🔍 Validating model card: {}", path.display());

            let content = std::fs::read_to_string(&path)?;
            let card = if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                ModelCard::from_yaml(&content)?
            } else {
                ModelCard::from_json(&content)?
            };

            let mut issues = Vec::new();

            // Check required fields
            if card.model_details.name.is_empty() {
                issues.push("❌ Model name is empty");
            }
            if card.model_details.version.is_empty() {
                issues.push("❌ Model version is empty");
            }
            if card.intended_use.primary_uses.is_empty() {
                issues.push("⚠️  No primary uses specified");
            }

            if strict {
                if card.model_details.size.is_empty() {
                    issues.push("⚠️  Model size not specified");
                }
                if card.model_details.framework.is_empty() {
                    issues.push("⚠️  Framework not specified");
                }
                if card.training_data.is_none() {
                    issues.push("⚠️  Training data section missing");
                }
                if card.evaluation.is_none() {
                    issues.push("⚠️  Evaluation section missing");
                }
                if card.ethical_considerations.is_none() {
                    issues.push("⚠️  Ethical considerations section missing");
                }
            }

            if issues.is_empty() {
                println!("✅ Model card is valid!");
                println!(
                    "   Model: {} v{}",
                    card.model_details.name, card.model_details.version
                );
                if card.training_data.is_some() {
                    println!("   ✓ Has training data");
                }
                if card.evaluation.is_some() {
                    println!("   ✓ Has evaluation");
                }
                if card.ethical_considerations.is_some() {
                    println!("   ✓ Has ethical considerations");
                }
            } else {
                println!("⚠️  Validation issues found:");
                for issue in issues {
                    println!("   {}", issue);
                }
            }
        }

        CardCommands::Convert { input, output } => {
            println!("🔄 Converting model card");
            println!("   From: {}", input.display());
            println!("   To: {}", output.display());

            let content = std::fs::read_to_string(&input)?;
            let card = if input.extension().and_then(|e| e.to_str()) == Some("yaml")
                || input.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                ModelCard::from_yaml(&content)?
            } else {
                ModelCard::from_json(&content)?
            };

            let ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("json");
            let output_content = match ext {
                "yaml" | "yml" => card.to_yaml()?,
                "md" | "markdown" => card.to_markdown(),
                _ => card.to_json()?,
            };

            std::fs::write(&output, output_content)?;
            println!("✅ Conversion complete!");
        }

        CardCommands::Template {
            template_type,
            output,
        } => {
            println!("📋 Generating {} template", template_type);

            let (details, intended_use) = match template_type.as_str() {
                "llm" => {
                    let details = ModelDetails {
                        name: "my-llm-model".to_string(),
                        version: "1.0.0".to_string(),
                        description: "Large Language Model for text generation".to_string(),
                        model_type: "Large Language Model".to_string(),
                        architecture: "Transformer".to_string(),
                        size: "7B parameters".to_string(),
                        framework: "PyTorch".to_string(),
                        format: "safetensors".to_string(),
                        license: Some("Apache-2.0".to_string()),
                        citation: None,
                        developers: vec!["Your Team".to_string()],
                        contact: Some("team@example.com".to_string()),
                        repository: None,
                        paper: None,
                    };
                    let intended_use = IntendedUse {
                        primary_uses: vec![
                            "Text generation".to_string(),
                            "Question answering".to_string(),
                        ],
                        primary_users: vec!["Researchers".to_string(), "Developers".to_string()],
                        out_of_scope_uses: vec![
                            "Medical diagnosis".to_string(),
                            "Legal advice".to_string(),
                        ],
                        use_case_examples: None,
                    };
                    (details, intended_use)
                }
                "classifier" => {
                    let details = ModelDetails {
                        name: "my-classifier".to_string(),
                        version: "1.0.0".to_string(),
                        description: "Image classification model".to_string(),
                        model_type: "Image Classifier".to_string(),
                        architecture: "ResNet-50".to_string(),
                        size: "25M parameters".to_string(),
                        framework: "PyTorch".to_string(),
                        format: "onnx".to_string(),
                        license: Some("MIT".to_string()),
                        citation: None,
                        developers: vec!["Your Team".to_string()],
                        contact: None,
                        repository: None,
                        paper: None,
                    };
                    let intended_use = IntendedUse {
                        primary_uses: vec!["Image classification".to_string()],
                        primary_users: vec!["Developers".to_string()],
                        out_of_scope_uses: vec!["Medical diagnosis".to_string()],
                        use_case_examples: None,
                    };
                    (details, intended_use)
                }
                "medical" => {
                    let details = ModelDetails {
                        name: "medical-model".to_string(),
                        version: "1.0.0".to_string(),
                        description: "Medical imaging analysis model".to_string(),
                        model_type: "Medical Image Classifier".to_string(),
                        architecture: "ResNet-50".to_string(),
                        size: "25M parameters".to_string(),
                        framework: "PyTorch".to_string(),
                        format: "onnx".to_string(),
                        license: Some("⚠️ RESEARCH USE ONLY - NOT FOR CLINICAL USE".to_string()),
                        citation: None,
                        developers: vec!["Medical AI Team".to_string()],
                        contact: Some("medical-ai@example.com".to_string()),
                        repository: None,
                        paper: None,
                    };
                    let intended_use = IntendedUse {
                        primary_uses: vec!["Research".to_string()],
                        primary_users: vec!["Researchers".to_string()],
                        out_of_scope_uses: vec![
                            "❌ Clinical diagnosis".to_string(),
                            "❌ Patient treatment decisions".to_string(),
                            "❌ ANY clinical use without FDA approval".to_string(),
                        ],
                        use_case_examples: None,
                    };
                    (details, intended_use)
                }
                "hiring" => {
                    let details = ModelDetails {
                        name: "hiring-model".to_string(),
                        version: "1.0.0".to_string(),
                        description: "Resume screening model".to_string(),
                        model_type: "Text Classifier".to_string(),
                        architecture: "BERT".to_string(),
                        size: "110M parameters".to_string(),
                        framework: "PyTorch".to_string(),
                        format: "safetensors".to_string(),
                        license: Some("Proprietary".to_string()),
                        citation: None,
                        developers: vec!["HR AI Team".to_string()],
                        contact: Some("hr-ai@example.com".to_string()),
                        repository: None,
                        paper: None,
                    };
                    let intended_use = IntendedUse {
                        primary_uses: vec!["Resume screening".to_string()],
                        primary_users: vec!["HR teams".to_string()],
                        out_of_scope_uses: vec![
                            "Final hiring decisions without human review".to_string()
                        ],
                        use_case_examples: Some(vec!["Initial candidate screening".to_string()]),
                    };
                    (details, intended_use)
                }
                _ => {
                    // Basic template
                    let details = ModelDetails {
                        name: "my-model".to_string(),
                        version: "1.0.0".to_string(),
                        description: "Model description".to_string(),
                        model_type: "Model Type".to_string(),
                        architecture: "Architecture".to_string(),
                        size: "Size".to_string(),
                        framework: "Framework".to_string(),
                        format: "Format".to_string(),
                        license: Some("License".to_string()),
                        citation: None,
                        developers: vec!["Team".to_string()],
                        contact: None,
                        repository: None,
                        paper: None,
                    };
                    let intended_use = IntendedUse {
                        primary_uses: vec!["Primary use".to_string()],
                        primary_users: vec!["Primary users".to_string()],
                        out_of_scope_uses: vec!["Out of scope use".to_string()],
                        use_case_examples: None,
                    };
                    (details, intended_use)
                }
            };

            let card = ModelCard::new(details, intended_use);

            let ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("json");
            let content = match ext {
                "yaml" | "yml" => card.to_yaml()?,
                "md" | "markdown" => card.to_markdown(),
                _ => card.to_json()?,
            };

            std::fs::write(&output, content)?;
            println!("✅ Template created: {}", output.display());
            println!("   Type: {}", template_type);
            println!("\n💡 Edit the file to customize your model card");
        }

        CardCommands::Attach {
            model,
            version,
            card,
        } => {
            println!("📎 Attaching model card to vault model");
            println!("   Model: {}", model);
            println!("   Card: {}", card.display());

            // Read card
            let card_content = std::fs::read_to_string(&card)?;
            let model_card = if card.extension().and_then(|e| e.to_str()) == Some("yaml")
                || card.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                ModelCard::from_yaml(&card_content)?
            } else {
                ModelCard::from_json(&card_content)?
            };

            // Convert to JSON for storage
            let card_json = model_card.to_json()?;

            // Open vault
            let mut vault = build_vault(config.clone(), use_sqlite)?;

            // Get the specified version or latest
            let version_num = if let Some(v) = version {
                v
            } else {
                vault
                    .list_versions(&model)
                    .last()
                    .map(|mv| mv.version)
                    .ok_or_else(|| {
                        VaultError::ModelNotFound(format!(
                            "Model '{}' not found or has no versions",
                            model
                        ))
                    })?
            };

            // Update metadata with model card
            vault.update_version_metadata(&model, version_num, "model_card", card_json)?;

            println!("✅ Model card attached successfully!");
            println!("   Model: {} v{}", model, version_num);
        }

        CardCommands::Extract {
            model,
            version,
            output,
        } => {
            println!("📤 Extracting model card from vault model");
            println!("   Model: {}", model);
            println!("   Output: {}", output.display());

            // Open vault
            let vault = build_vault(config.clone(), use_sqlite)?;

            // Get the specified version or latest
            let version_num = if let Some(v) = version {
                v
            } else {
                vault
                    .list_versions(&model)
                    .last()
                    .map(|mv| mv.version)
                    .ok_or_else(|| {
                        VaultError::ModelNotFound(format!(
                            "Model '{}' not found or has no versions",
                            model
                        ))
                    })?
            };

            // Get model card from metadata
            let card_json = vault
                .get_version_metadata(&model, version_num, "model_card")
                .ok_or_else(|| {
                    VaultError::ModelNotFound(format!(
                        "Model '{}' v{} does not have an attached model card",
                        model, version_num
                    ))
                })?;

            // Parse and convert to desired format
            let model_card = ModelCard::from_json(&card_json)?;

            let ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("json");
            let output_content = match ext {
                "yaml" | "yml" => model_card.to_yaml()?,
                "md" | "markdown" => model_card.to_markdown(),
                _ => model_card.to_json()?,
            };

            std::fs::write(&output, output_content)?;
            println!("✅ Model card extracted successfully!");
            println!("   Model: {} v{}", model, version_num);
            println!("   Output: {}", output.display());
        }

        CardCommands::Generate {
            model,
            version,
            output,
            include_training,
            include_evaluation,
        } => {
            println!("🤖 Generating model card from metadata");
            println!("   Model: {}", model);
            println!("   Output: {}", output.display());

            // Open vault
            let vault = build_vault(config.clone(), use_sqlite)?;

            // Get the specified version or latest
            let version_num = if let Some(v) = version {
                v
            } else {
                vault
                    .list_versions(&model)
                    .last()
                    .map(|mv| mv.version)
                    .ok_or_else(|| {
                        VaultError::ModelNotFound(format!(
                            "Model '{}' not found or has no versions",
                            model
                        ))
                    })?
            };

            // Get model version info
            let versions = vault.list_versions(&model);
            let model_version = versions
                .iter()
                .find(|v| v.version == version_num)
                .ok_or_else(|| VaultError::VersionNotFound(version_num, model.to_string()))?;

            // Extract metadata
            let description = model_version
                .metadata
                .get("description")
                .cloned()
                .unwrap_or_else(|| format!("Model {}", model));

            let framework = model_version
                .metadata
                .get("framework")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let task = model_version
                .metadata
                .get("task")
                .cloned()
                .unwrap_or_else(|| "General".to_string());

            // Format size
            let size_str = if model_version.size_bytes > 1_000_000_000 {
                format!(
                    "{:.2} GB",
                    model_version.size_bytes as f64 / 1_000_000_000.0
                )
            } else if model_version.size_bytes > 1_000_000 {
                format!("{:.2} MB", model_version.size_bytes as f64 / 1_000_000.0)
            } else {
                format!("{:.2} KB", model_version.size_bytes as f64 / 1_000.0)
            };

            // Create model details
            let details = ModelDetails {
                name: model.clone(),
                version: version_num.to_string(),
                description,
                model_type: task.clone(),
                architecture: model_version
                    .metadata
                    .get("architecture")
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string()),
                size: size_str,
                framework,
                format: model_version.format.clone(),
                license: model_version.metadata.get("license").cloned(),
                citation: model_version.metadata.get("citation").cloned(),
                developers: vec!["Vault User".to_string()],
                contact: model_version.metadata.get("contact").cloned(),
                repository: model_version.metadata.get("repository").cloned(),
                paper: model_version.metadata.get("paper").cloned(),
            };

            // Create intended use
            let intended_use = IntendedUse {
                primary_uses: vec![task],
                primary_users: vec!["Researchers".to_string(), "Developers".to_string()],
                out_of_scope_uses: vec!["Production use without validation".to_string()],
                use_case_examples: None,
            };

            // Create basic model card
            let mut card = ModelCard::new(details, intended_use);

            // Add training data if requested
            if include_training {
                let training = TrainingData {
                    datasets: vec!["Unknown - please update".to_string()],
                    sources: None,
                    collection_methods: None,
                    preprocessing: None,
                    size: None,
                    splits: None,
                    languages: None,
                    demographics: None,
                };
                card = card.with_training_data(training);
            }

            // Add evaluation if requested
            if include_evaluation {
                let evaluation = Evaluation {
                    datasets: vec!["Unknown - please update".to_string()],
                    metrics: vec![],
                    benchmarks: None,
                    performance_by_group: None,
                    methodology: None,
                };
                card = card.with_evaluation(evaluation);
            }

            // Add vault-specific metadata as custom field
            card.metadata.insert(
                "vault_info".to_string(),
                format!(
                    "Generated from vault model '{}' v{} on {}",
                    model,
                    version_num,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                ),
            );
            card.metadata.insert(
                "original_size".to_string(),
                model_version.size_bytes.to_string(),
            );
            card.metadata.insert(
                "compressed_size".to_string(),
                model_version.compressed_size_bytes.to_string(),
            );
            card.metadata.insert(
                "checksum_sha256".to_string(),
                model_version.checksum_sha256.clone(),
            );

            // Convert to desired format
            let ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("json");
            let output_content = match ext {
                "yaml" | "yml" => card.to_yaml()?,
                "md" | "markdown" => card.to_markdown(),
                _ => card.to_json()?,
            };

            std::fs::write(&output, output_content)?;
            println!("✅ Model card generated successfully!");
            println!("   Model: {} v{}", model, version_num);
            println!("   Output: {}", output.display());
            println!("\n💡 Edit the file to add more details:");
            println!("   - Training data and evaluation metrics");
            println!("   - Ethical considerations");
            println!("   - Environmental impact");
        }
    }

    Ok(())
}
