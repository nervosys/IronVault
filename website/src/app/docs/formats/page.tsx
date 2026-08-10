export default function FormatsPage() {
  const formats = [
    { category: "LLM Formats", items: [
      { name: "SafeTensors", ext: ".safetensors", desc: "Hugging Face safe serialization — fast, memory-mapped, secure" },
      { name: "GGUF", ext: ".gguf", desc: "GPT-Generated Unified Format — quantized models for llama.cpp" },
      { name: "PyTorch", ext: ".pt, .pth, .bin", desc: "PyTorch native format — pickle-based serialization" },
      { name: "TensorRT", ext: ".engine, .plan", desc: "NVIDIA TensorRT optimized inference engines" },
      { name: "ONNX", ext: ".onnx", desc: "Open Neural Network Exchange — cross-framework interop" },
      { name: "MLX", ext: ".mlx", desc: "Apple MLX framework format for Apple Silicon" },
      { name: "Core ML", ext: ".mlmodel, .mlpackage", desc: "Apple Core ML for iOS/macOS deployment" },
      { name: "TorchScript", ext: ".pt", desc: "Serialized PyTorch JIT models" },
      { name: "TFLite", ext: ".tflite", desc: "TensorFlow Lite for mobile/edge deployment" },
    ]},
    { category: "General Deep Learning", items: [
      { name: "TensorFlow", ext: ".pb, .h5", desc: "TensorFlow SavedModel and frozen graph formats" },
      { name: "Keras", ext: ".keras, .h5", desc: "Keras native format" },
      { name: "OpenVINO", ext: ".xml, .bin", desc: "Intel OpenVINO inference engine" },
      { name: "TVM", ext: ".so, .tar", desc: "Apache TVM compiled models" },
      { name: "NCNN", ext: ".param, .bin", desc: "Tencent NCNN mobile inference" },
      { name: "MNN", ext: ".mnn", desc: "Alibaba MNN mobile neural network" },
      { name: "RKNN", ext: ".rknn", desc: "Rockchip NPU format" },
    ]},
    { category: "Data & Legacy", items: [
      { name: "HDF5", ext: ".h5, .hdf5", desc: "Hierarchical Data Format version 5" },
      { name: "NumPy", ext: ".npy, .npz", desc: "NumPy array serialization" },
      { name: "Pickle", ext: ".pkl", desc: "Python pickle serialization" },
      { name: "Caffe", ext: ".caffemodel", desc: "Caffe framework format" },
      { name: "MXNet", ext: ".params", desc: "Apache MXNet parameters" },
      { name: "Darknet", ext: ".weights", desc: "Darknet/YOLO weights" },
    ]},
  ];

  return (
    <>
      <h1 className="text-4xl font-bold mb-4">Format Support</h1>
      <p className="text-lg text-[var(--color-text-secondary)] mb-8">
        IronVault supports 23+ model formats with automatic detection via magic bytes and file extensions.
      </p>

      {formats.map((group) => (
        <div key={group.category}>
          <h2 className="text-2xl font-bold mt-10 mb-4">{group.category}</h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm border-collapse">
              <thead>
                <tr className="border-b border-[var(--color-border)]">
                  <th className="text-left p-3 font-semibold">Format</th>
                  <th className="text-left p-3 font-semibold">Extension(s)</th>
                  <th className="text-left p-3 font-semibold">Description</th>
                </tr>
              </thead>
              <tbody className="text-[var(--color-text-secondary)]">
                {group.items.map((fmt) => (
                  <tr key={fmt.name} className="border-b border-[var(--color-border)]">
                    <td className="p-3 font-medium text-[var(--color-text)]">{fmt.name}</td>
                    <td className="p-3"><code className="px-1.5 py-0.5 bg-[var(--color-bg-secondary)] rounded text-xs">{fmt.ext}</code></td>
                    <td className="p-3">{fmt.desc}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ))}

      <h2 className="text-2xl font-bold mt-10 mb-4" id="detection">Automatic Detection</h2>
      <p className="text-[var(--color-text-secondary)] mb-4">
        IronVault uses magic bytes (file signatures) to identify formats automatically:
      </p>
      <ul className="space-y-1 text-[var(--color-text-secondary)]">
        <li>• <strong>SafeTensors</strong> — JSON header with tensor metadata</li>
        <li>• <strong>GGUF</strong> — <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">GGUF</code> magic bytes (4 bytes)</li>
        <li>• <strong>PyTorch</strong> — ZIP archive or pickle header</li>
        <li>• <strong>ONNX</strong> — Protobuf header</li>
        <li>• <strong>TFLite</strong> — FlatBuffers identifier</li>
      </ul>

      <h2 className="text-2xl font-bold mt-10 mb-4" id="custom">Custom Formats</h2>
      <p className="text-[var(--color-text-secondary)]">
        Any binary file can be stored using the <code className="text-xs px-1 bg-[var(--color-bg-secondary)] rounded">custom</code> format type.
        Metadata is still captured and encrypted. Custom format names are preserved in version history.
      </p>
    </>
  );
}
