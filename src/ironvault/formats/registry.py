"""
Model format registry for IronVault.

Enumerates supported model formats and provides detection/conversion guidance.
Mirrors the Rust `ModelFormat` enum in src/formats.rs.

NOTE: This enum is kept in sync with the Rust enum. If you add a variant here,
add the corresponding entry in src/formats.rs (and vice-versa).
"""

from enum import Enum
from pathlib import Path
from typing import Optional


class ModelFormat(Enum):
    """
    Supported AI model formats.

    Each variant corresponds to a file format that IronVault
    can store, retrieve, and (where applicable) convert between.

    This enum mirrors the Rust ``ModelFormat`` in ``src/formats.rs``.
    """

    # ----- LLM-centric formats -----
    SAFETENSORS = "safetensors"
    GGUF = "gguf"
    PYTORCH = "pytorch"
    TENSORRT = "tensorrt"
    ONNX = "onnx"
    MLX = "mlx"
    COREML = "coreml"
    TORCHSCRIPT = "torchscript"
    TFLITE = "tflite"

    # ----- General DL formats -----
    TENSORFLOW = "tensorflow"
    KERAS = "keras"
    OPENVINO = "openvino"
    TVM = "tvm"
    NCNN = "ncnn"
    MNN = "mnn"
    RKNN = "rknn"

    # ----- Legacy formats -----
    CAFFE = "caffe"
    MXNET = "mxnet"
    DARKNET = "darknet"

    # ----- Data formats -----
    HDF5 = "hdf5"
    PICKLE = "pickle"
    NUMPY = "numpy"

    # ----- Special -----
    CUSTOM = "custom"

    @classmethod
    def detect(cls, path: str) -> "ModelFormat":
        """
        Detect model format from file extension.

        Args:
            path: Path to the model file.

        Returns:
            Detected ModelFormat variant.
        """
        ext = Path(path).suffix.lower()
        extension_map = {
            # Safetensors
            ".safetensors": cls.SAFETENSORS,
            # GGUF
            ".gguf": cls.GGUF,
            # PyTorch
            ".pt": cls.PYTORCH,
            ".pth": cls.PYTORCH,
            ".bin": cls.PYTORCH,
            # TensorRT
            ".plan": cls.TENSORRT,
            ".engine": cls.TENSORRT,
            ".trt": cls.TENSORRT,
            # ONNX
            ".onnx": cls.ONNX,
            # MLX
            ".npz": cls.MLX,
            # Core ML
            ".mlmodel": cls.COREML,
            ".mlpackage": cls.COREML,
            # TorchScript
            ".torchscript": cls.TORCHSCRIPT,
            # TFLite
            ".tflite": cls.TFLITE,
            # TensorFlow
            ".pb": cls.TENSORFLOW,
            ".savedmodel": cls.TENSORFLOW,
            # Keras
            ".h5": cls.KERAS,
            ".keras": cls.KERAS,
            # OpenVINO
            ".xml": cls.OPENVINO,
            # NCNN
            ".param": cls.NCNN,
            # MNN
            ".mnn": cls.MNN,
            # RKNN
            ".rknn": cls.RKNN,
            # Caffe
            ".caffemodel": cls.CAFFE,
            # MXNet
            ".params": cls.MXNET,
            # Darknet
            ".weights": cls.DARKNET,
            # HDF5
            ".hdf5": cls.HDF5,
            # Pickle
            ".pkl": cls.PICKLE,
            ".pickle": cls.PICKLE,
            # NumPy
            ".npy": cls.NUMPY,
        }
        return extension_map.get(ext, cls.CUSTOM)

    @property
    def file_extensions(self) -> list:
        """Return common file extensions for this format."""
        ext_map = {
            self.SAFETENSORS: [".safetensors"],
            self.GGUF: [".gguf"],
            self.PYTORCH: [".pt", ".pth", ".bin"],
            self.TENSORRT: [".plan", ".engine", ".trt"],
            self.ONNX: [".onnx"],
            self.MLX: [".npz"],
            self.COREML: [".mlmodel", ".mlpackage"],
            self.TORCHSCRIPT: [".torchscript"],
            self.TFLITE: [".tflite"],
            self.TENSORFLOW: [".pb", ".savedmodel"],
            self.KERAS: [".h5", ".keras"],
            self.OPENVINO: [".xml"],
            self.TVM: [],
            self.NCNN: [".param"],
            self.MNN: [".mnn"],
            self.RKNN: [".rknn"],
            self.CAFFE: [".caffemodel"],
            self.MXNET: [".params"],
            self.DARKNET: [".weights"],
            self.HDF5: [".hdf5"],
            self.PICKLE: [".pkl", ".pickle"],
            self.NUMPY: [".npy", ".npz"],
        }
        return ext_map.get(self, [])

    def __str__(self) -> str:
        return self.value
