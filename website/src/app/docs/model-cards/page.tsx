import CodeBlock from "@/components/DocElements";

export default function ModelCardsPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Model Cards</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Standardized model documentation following Google, Hugging Face, and Partnership on AI best practices.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="overview">What are Model Cards?</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        Model Cards are structured documents that describe an ML model&apos;s intended use, training data,
        evaluation metrics, ethical considerations, and limitations. IronVault provides first-class
        support for creating, attaching, and exporting model cards in JSON, YAML, and Markdown formats.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cli">CLI Usage</h2>
      <CodeBlock language="bash">{`# Create a model card
iv card create my-model \\
  --author "ML Team" \\
  --task "text-generation" \\
  --description "Fine-tuned GPT-2 for code generation"

# Show card details
iv card show my-model

# Export as Markdown
iv card export my-model --format markdown --output card.md

# Export as JSON
iv card export my-model --format json --output card.json

# Export as YAML
iv card export my-model --format yaml --output card.yaml

# Attach an external card file
iv card attach my-model --file card.json`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="structure">Card Structure</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">Section</th>
              <th className="text-left p-3 font-semibold">Description</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["Model Details", "Name, version, authors, description, license, framework"],
              ["Intended Use", "Primary use cases, out-of-scope uses, target users"],
              ["Training Data", "Datasets, preprocessing, data splits"],
              ["Evaluation", "Metrics, benchmark results, performance by group"],
              ["Ethical Considerations", "Bias analysis, fairness metrics, risks"],
              ["Caveats & Recommendations", "Known limitations, deployment recommendations"],
              ["Environmental Impact", "Carbon emissions, compute resources, training time"],
            ].map(([section, desc]) => (
              <tr key={section} className="border-b border-[var(--color-border)]">
                <td className="p-3 font-medium text-[var(--color-text)]">{section}</td>
                <td className="p-3">{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rust">Rust API</h2>
      <CodeBlock language="rust">{`use ironvault::model_card::ModelCard;

// Build a card
let card = ModelCard::builder("my-model")
    .version("1.0")
    .authors(vec!["ML Team".to_string()])
    .description("Fine-tuned model for text generation")
    .license("MIT")
    .task("text-generation")
    .add_metric("accuracy", 0.95, None)
    .add_metric("f1", 0.92, Some("macro-averaged"))
    .training_data(vec!["dataset-a".to_string()])
    .build()?;

// Export
let markdown = card.to_markdown()?;
let json = card.to_json()?;
let yaml = card.to_yaml()?;`}</CodeBlock>
    </>
  );
}
