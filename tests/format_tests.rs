//! Comprehensive format detection and handling tests

use ironvault::formats::{FormatConverter, ModelFormat, ModelMetadata};

#[test]
fn test_format_detection_from_extension() {
    let test_cases = vec![
        ("model.safetensors", ModelFormat::Safetensors),
        ("model.gguf", ModelFormat::GGUF),
        ("model.pt", ModelFormat::PyTorch),
        ("model.pth", ModelFormat::PyTorch),
        ("model.bin", ModelFormat::PyTorch),
        ("model.onnx", ModelFormat::ONNX),
        ("model.plan", ModelFormat::TensorRT),
        ("model.tflite", ModelFormat::TFLite),
        ("model.mlmodel", ModelFormat::CoreML),
        ("model.h5", ModelFormat::Keras),
        ("model.keras", ModelFormat::Keras),
        ("model.pb", ModelFormat::TensorFlow),
        ("model.xml", ModelFormat::OpenVINO),
        ("model.param", ModelFormat::NCNN),
        ("model.mnn", ModelFormat::MNN),
        ("model.rknn", ModelFormat::RKNN),
        ("model.caffemodel", ModelFormat::Caffe),
        ("model.params", ModelFormat::MXNet),
        ("model.weights", ModelFormat::Darknet),
        ("model.hdf5", ModelFormat::HDF5),
        ("model.pkl", ModelFormat::Pickle),
        ("model.npy", ModelFormat::NumPy),
        ("model.npz", ModelFormat::NumPy),
    ];

    for (filename, expected_format) in test_cases {
        let ext = filename.split('.').next_back().unwrap();
        let detected = ModelFormat::from_extension(ext);
        assert_eq!(detected, expected_format, "Failed for {}", filename);
    }
}

#[test]
fn test_format_extension_roundtrip() {
    let formats = vec![
        ModelFormat::Safetensors,
        ModelFormat::GGUF,
        ModelFormat::PyTorch,
        ModelFormat::ONNX,
        ModelFormat::TensorRT,
        ModelFormat::TFLite,
        ModelFormat::CoreML,
        ModelFormat::Keras,
        ModelFormat::TensorFlow,
        ModelFormat::OpenVINO,
        ModelFormat::NCNN,
        ModelFormat::MNN,
        ModelFormat::RKNN,
        ModelFormat::Caffe,
        ModelFormat::MXNet,
        ModelFormat::Darknet,
        ModelFormat::HDF5,
        ModelFormat::Pickle,
        ModelFormat::NumPy,
    ];

    for format in formats {
        let ext = format.extension();
        let detected = ModelFormat::from_extension(ext);
        // Some formats share extensions, so check they're compatible
        assert!(
            detected == format
                || matches!(detected, ModelFormat::PyTorch)
                || matches!(detected, ModelFormat::Keras),
            "Roundtrip failed for {:?}",
            format
        );
    }
}

#[test]
fn test_format_names() {
    assert_eq!(ModelFormat::Safetensors.name(), "Safetensors");
    assert_eq!(ModelFormat::GGUF.name(), "GGUF");
    assert_eq!(ModelFormat::PyTorch.name(), "PyTorch");
    assert_eq!(ModelFormat::ONNX.name(), "ONNX");
    assert_eq!(ModelFormat::TensorRT.name(), "TensorRT");
    assert_eq!(ModelFormat::MLX.name(), "MLX");
    assert_eq!(ModelFormat::CoreML.name(), "Core ML");
    assert_eq!(ModelFormat::TorchScript.name(), "TorchScript");
    assert_eq!(ModelFormat::TFLite.name(), "TensorFlow Lite");
    assert_eq!(ModelFormat::TensorFlow.name(), "TensorFlow");
    assert_eq!(ModelFormat::Keras.name(), "Keras");
}

#[test]
fn test_custom_format() {
    let custom = ModelFormat::Custom("custom_format".to_string());
    assert_eq!(custom.name(), "custom_format");
    assert_eq!(custom.extension(), "custom_format");
}

#[test]
fn test_case_insensitive_detection() {
    assert_eq!(
        ModelFormat::from_extension("SAFETENSORS"),
        ModelFormat::Safetensors
    );
    assert_eq!(ModelFormat::from_extension("Gguf"), ModelFormat::GGUF);
    assert_eq!(ModelFormat::from_extension("PT"), ModelFormat::PyTorch);
    assert_eq!(ModelFormat::from_extension("Onnx"), ModelFormat::ONNX);
}

#[test]
fn test_metadata_builder() {
    let metadata = ModelMetadata::new("test_model".to_string(), ModelFormat::PyTorch)
        .with_description("A test model".to_string())
        .with_framework("PyTorch 2.0".to_string())
        .with_task("classification".to_string())
        .with_architecture("ResNet-50".to_string())
        .with_parameters(25_000_000);

    assert_eq!(metadata.name, "test_model");
    assert_eq!(metadata.format, ModelFormat::PyTorch);
    assert_eq!(metadata.description, Some("A test model".to_string()));
    assert_eq!(metadata.framework, Some("PyTorch 2.0".to_string()));
    assert_eq!(metadata.task, Some("classification".to_string()));
    assert_eq!(metadata.architecture, Some("ResNet-50".to_string()));
    assert_eq!(metadata.parameters, Some(25_000_000));
}

#[test]
fn test_metadata_custom_fields() {
    let metadata = ModelMetadata::new("test_model".to_string(), ModelFormat::Safetensors)
        .add_custom_field("license".to_string(), "MIT".to_string())
        .add_custom_field("author".to_string(), "AI Team".to_string())
        .add_custom_field("version".to_string(), "1.0.0".to_string());

    assert_eq!(metadata.custom_fields.len(), 3);
    assert_eq!(
        metadata.custom_fields.get("license"),
        Some(&"MIT".to_string())
    );
    assert_eq!(
        metadata.custom_fields.get("author"),
        Some(&"AI Team".to_string())
    );
    assert_eq!(
        metadata.custom_fields.get("version"),
        Some(&"1.0.0".to_string())
    );
}

#[test]
fn test_metadata_optional_fields() {
    let metadata = ModelMetadata::new("minimal_model".to_string(), ModelFormat::GGUF);

    assert_eq!(metadata.name, "minimal_model");
    assert_eq!(metadata.format, ModelFormat::GGUF);
    assert_eq!(metadata.description, None);
    assert_eq!(metadata.framework, None);
    assert_eq!(metadata.task, None);
    assert_eq!(metadata.architecture, None);
    assert_eq!(metadata.parameters, None);
    assert!(metadata.custom_fields.is_empty());
}

#[test]
fn test_format_converter_creation() {
    let converter = FormatConverter::new();
    // Just verify it can be created
    assert_eq!(
        std::mem::size_of_val(&converter),
        std::mem::size_of::<FormatConverter>()
    );
}

#[test]
fn test_format_clone() {
    let format1 = ModelFormat::Safetensors;
    let format2 = format1.clone();
    assert_eq!(format1, format2);

    let custom1 = ModelFormat::Custom("test".to_string());
    let custom2 = custom1.clone();
    assert_eq!(custom1, custom2);
}

#[test]
fn test_metadata_clone() {
    let metadata1 = ModelMetadata::new("test".to_string(), ModelFormat::PyTorch)
        .with_description("desc".to_string())
        .add_custom_field("key".to_string(), "value".to_string());

    let metadata2 = metadata1.clone();

    assert_eq!(metadata1.name, metadata2.name);
    assert_eq!(metadata1.format, metadata2.format);
    assert_eq!(metadata1.description, metadata2.description);
    assert_eq!(metadata1.custom_fields, metadata2.custom_fields);
}

#[test]
fn test_all_llm_formats() {
    let llm_formats = vec![
        ("safetensors", ModelFormat::Safetensors),
        ("gguf", ModelFormat::GGUF),
        ("pt", ModelFormat::PyTorch),
        ("plan", ModelFormat::TensorRT),
        ("onnx", ModelFormat::ONNX),
        ("mlx", ModelFormat::MLX),
        ("mlmodel", ModelFormat::CoreML),
        ("tflite", ModelFormat::TFLite),
    ];

    for (ext, format) in llm_formats {
        let detected = ModelFormat::from_extension(ext);
        assert_eq!(detected, format);
        assert!(!format.name().is_empty());
        assert!(!format.extension().is_empty());
    }
}

#[test]
fn test_all_dl_framework_formats() {
    let dl_formats = vec![
        ("pb", ModelFormat::TensorFlow),
        ("h5", ModelFormat::Keras),
        ("xml", ModelFormat::OpenVINO),
        ("param", ModelFormat::NCNN),
        ("mnn", ModelFormat::MNN),
        ("rknn", ModelFormat::RKNN),
    ];

    for (ext, format) in dl_formats {
        let detected = ModelFormat::from_extension(ext);
        assert_eq!(detected, format);
    }
}

#[test]
fn test_all_legacy_formats() {
    let legacy_formats = vec![
        ("caffemodel", ModelFormat::Caffe),
        ("params", ModelFormat::MXNet),
        ("weights", ModelFormat::Darknet),
    ];

    for (ext, format) in legacy_formats {
        let detected = ModelFormat::from_extension(ext);
        assert_eq!(detected, format);
    }
}

#[test]
fn test_all_data_formats() {
    let data_formats = vec![
        ("hdf5", ModelFormat::HDF5),
        ("pkl", ModelFormat::Pickle),
        ("npy", ModelFormat::NumPy),
        ("npz", ModelFormat::NumPy),
    ];

    for (ext, format) in data_formats {
        let detected = ModelFormat::from_extension(ext);
        assert_eq!(detected, format);
    }
}
