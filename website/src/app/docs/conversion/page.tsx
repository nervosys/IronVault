import CodeBlock from "@/components/DocElements";
import { Callout } from "@/components/DocElements";

export default function ConversionPage() {
  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Format Conversion</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        Pipeline-based format conversion with 10 built-in converters and BFS multi-step path finding.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="overview">Overview</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        The conversion engine uses a graph of registered converters. When a direct converter isn&apos;t available,
        it finds the shortest multi-step path using BFS. For example, converting PyTorch → TensorRT
        goes through PyTorch → ONNX → TensorRT automatically.
      </p>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="converters">Built-in Converters</h2>
      <div className="overflow-x-auto">
        <table className="w-full text-sm border-collapse">
          <thead>
            <tr className="border-b border-[var(--color-border)]">
              <th className="text-left p-3 font-semibold">From</th>
              <th className="text-left p-3 font-semibold">To</th>
              <th className="text-left p-3 font-semibold">Type</th>
            </tr>
          </thead>
          <tbody className="text-[var(--color-text-secondary)]">
            {[
              ["SafeTensors", "Raw", "Pure Rust (roundtrip)"],
              ["Raw", "SafeTensors", "Pure Rust (roundtrip)"],
              ["SafeTensors", "PyTorch", "Shim (conversion plan)"],
              ["PyTorch", "SafeTensors", "Shim (conversion plan)"],
              ["PyTorch", "ONNX", "Shim (conversion plan)"],
              ["ONNX", "TensorRT", "Shim (conversion plan)"],
              ["ONNX", "Core ML", "Shim (conversion plan)"],
              ["SafeTensors", "GGUF", "Shim (quantization-aware)"],
              ["GGUF", "(parser)", "Pure Rust (header extraction)"],
              ["ONNX", "(parser)", "Pure Rust (metadata extraction)"],
            ].map(([from, to, type_], i) => (
              <tr key={i} className="border-b border-[var(--color-border)]">
                <td className="p-3">{from}</td>
                <td className="p-3">{to}</td>
                <td className="p-3">{type_}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <Callout type="info" title="Shim converters">
        Shim converters produce a JSON conversion plan rather than performing the conversion directly.
        Use <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">--plan-only</code> to see
        the plan without executing it.
      </Callout>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="cli">CLI Usage</h2>
      <CodeBlock language="bash">{`# Direct conversion
iv convert model.safetensors --from safetensors --to pytorch --output model.pt

# Multi-step (auto-routed via ONNX)
iv convert model.pt --from pytorch --to tensorrt --output model.engine

# With options
iv convert model.safetensors --from safetensors --to gguf \\
  --opset 17 --validate

# See the plan without executing
iv convert model.pt --from pytorch --to coreml --plan-only

# List all available conversions
iv list-conversions`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="rust">Rust API</h2>
      <CodeBlock language="rust">{`use ironvault::conversion::{ConversionPipeline, ConversionOptions, ModelFormat};

// Create pipeline with all built-in converters
let pipeline = ConversionPipeline::with_builtins();

// Find conversion path
let path = pipeline.find_path(ModelFormat::PyTorch, ModelFormat::TensorRT);
// Returns: Some([PyTorch -> ONNX, ONNX -> TensorRT])

// Convert with options
let options = ConversionOptions {
    opset_version: Some(17),
    validate: true,
    ..Default::default()
};

let result = pipeline.convert(
    &input_data,
    ModelFormat::SafeTensors,
    ModelFormat::PyTorch,
    &options,
)?;

println!("Output: {} bytes", result.output.len());
println!("Path: {:?}", result.conversion_path);
println!("Ratio: {:.2}x", result.compression_ratio());`}</CodeBlock>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="validation">Validation</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        The <code className="text-sm px-1 bg-[var(--color-bg-secondary)] rounded">--validate</code> flag runs
        post-conversion checks:
      </p>
      <ul className="space-y-1 text-[var(--color-text-secondary)]">
        <li>• Magic bytes verification for the target format</li>
        <li>• Output size sanity check</li>
        <li>• Header structure validation</li>
        <li>• Checksum computation and comparison</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="custom">Custom Converters</h2>
      <CodeBlock language="rust">{`use ironvault::conversion::{Converter, ConversionOptions, ConversionResult, ModelFormat};

struct MyConverter;

impl Converter for MyConverter {
    fn name(&self) -> &str { "my-custom-converter" }
    fn source_format(&self) -> ModelFormat { ModelFormat::ONNX }
    fn target_format(&self) -> ModelFormat { ModelFormat::Custom("my-format".into()) }

    fn convert(&self, input: &[u8], options: &ConversionOptions) -> Result<ConversionResult> {
        // Your conversion logic
    }
}

// Register it
pipeline.register(Box::new(MyConverter));`}</CodeBlock>
    </>
  );
}
