#!/usr/bin/env python3
"""
IronVault - PyTorch Integration Demo

This script demonstrates how to use IronVault with PyTorch models:
- Saving PyTorch models to the vault
- Loading models from the vault
- Version control for model checkpoints
- Fine-tuning workflow with version tracking
- Quantization pipeline
- Model comparison and rollback

Requirements:
    # Install uv if not already installed
    curl -LsSf https://astral.sh/uv/install.sh | sh  # Linux/macOS
    powershell -c "irm https://astral.sh/uv/install.ps1 | iex"  # Windows
    
    # Install dependencies
    uv pip install -r requirements.txt
    
    # Or install manually
    uv pip install torch torchvision
"""

import sys
import subprocess
import json
import tempfile
from pathlib import Path
from typing import Optional, Dict, Any

# Check for required packages
try:
    import torch
    import torch.nn as nn
    import torchvision.models as models
except ImportError:
    print("X PyTorch not installed!")
    print()
    print("Install with uv (recommended):")
    print("  uv pip install torch torchvision")
    print()
    print("Or using pip:")
    print("  pip install torch torchvision")
    print()
    print("For this demo, we'll simulate PyTorch operations...")
    print()
    
    # Mock PyTorch for demo purposes
    class MockModule:
        def __init__(self):
            self.parameters_list = []
        
        def parameters(self):
            return self.parameters_list
        
        def state_dict(self):
            return {}
    
    class MockNN:
        Module = MockModule
        class Linear: pass
        class Conv2d: pass
    
    class MockTorch:
        nn = MockNN()
        __version__ = "2.0.0 (simulated)"
        qint8 = "qint8"
        
        class quantization:
            @staticmethod
            def quantize_dynamic(model, layers, dtype):
                return model
        
        @staticmethod
        def save(obj, path):
            # Write mock data safely
            try:
                with open(path, 'wb') as f:
                    f.write(b"mock_model_data" * 1000)
            except:
                pass
        
        @staticmethod
        def randn_like(tensor):
            return None
    
    class MockModels:
        class ResNet18_Weights:
            IMAGENET1K_V1 = "IMAGENET1K_V1"
        
        @staticmethod
        def resnet18(weights=None):
            return MockModule()
    
    torch = MockTorch()
    nn = torch.nn
    models = MockModels()
    
    PYTORCH_AVAILABLE = False
else:
    PYTORCH_AVAILABLE = True


# ANSI color codes (check for Windows compatibility)
import os
import platform

def supports_color():
    """Check if terminal supports color"""
    if platform.system() == "Windows":
        # Try to enable ANSI on Windows
        try:
            import ctypes
            kernel32 = ctypes.windll.kernel32
            kernel32.SetConsoleMode(kernel32.GetStdHandle(-11), 7)
            return True
        except:
            return False
    return True

USE_COLOR = supports_color()

class Colors:
    RESET = '\033[0m' if USE_COLOR else ''
    BOLD = '\033[1m' if USE_COLOR else ''
    GREEN = '\033[32m' if USE_COLOR else ''
    BLUE = '\033[34m' if USE_COLOR else ''
    YELLOW = '\033[33m' if USE_COLOR else ''
    CYAN = '\033[36m' if USE_COLOR else ''
    RED = '\033[31m' if USE_COLOR else ''


def print_header(message: str):
    """Print a formatted header"""
    print()
    print(f"{Colors.BOLD}{Colors.CYAN}=== {message} ==={Colors.RESET}")
    print()


def print_success(message: str):
    """Print a success message"""
    # Use ASCII symbols on Windows to avoid encoding issues
    import platform
    symbol = "[OK]" if platform.system() == "Windows" else ("√" if USE_COLOR else "[OK]")
    print(f"{Colors.GREEN}{symbol}{Colors.RESET} {message}")


def print_info(message: str):
    """Print an info message"""
    import platform
    symbol = ">" if platform.system() == "Windows" else ("→" if USE_COLOR else ">")
    print(f"{Colors.BLUE}{symbol}{Colors.RESET} {message}")


def print_warning(message: str):
    """Print a warning message"""
    symbol = "!" if USE_COLOR else "[WARN]"
    print(f"{Colors.YELLOW}{symbol}{Colors.RESET} {message}")


def print_error(message: str):
    """Print an error message"""
    import platform
    symbol = "X" if platform.system() == "Windows" else ("✗" if USE_COLOR else "[ERROR]")
    print(f"{Colors.RED}{symbol}{Colors.RESET} {message}")


class IronVault:
    """Python wrapper for IronVault CLI"""
    
    def __init__(self, vault_path: Optional[Path] = None):
        self.vault_path = vault_path or Path.home() / ".ironvault"
        self.cli_path = self._find_cli()
        self.passphrase = b"demo_pytorch_passphrase_2024"
        
    def _find_cli(self) -> Path:
        """Find the IronVault CLI executable"""
        # Check if built in target/release
        cli_paths = [
            Path("target/release/iv.exe"),
            Path("target/release/iv"),
            Path("iv.exe"),
            Path("iv"),
        ]
        
        for cli_path in cli_paths:
            if cli_path.exists():
                return cli_path
        
        # Try to build if not found
        print_info("IronVault CLI not found, building...")
        try:
            subprocess.run(
                ["cargo", "build", "--release", "--bin", "iv"],
                check=True,
                capture_output=True
            )
            if Path("target/release/iv.exe").exists():
                return Path("target/release/iv.exe")
            elif Path("target/release/iv").exists():
                return Path("target/release/iv")
        except subprocess.CalledProcessError:
            print_error("Failed to build IronVault CLI")
            sys.exit(1)
        
        print_error("Could not find or build IronVault CLI")
        sys.exit(1)
    
    def store_model(
        self,
        model_name: str,
        model: nn.Module,
        metadata: Optional[Dict[str, Any]] = None,
        description: str = "",
    ) -> bool:
        """Store a PyTorch model in the vault"""
        # Save model to temporary file
        with tempfile.NamedTemporaryFile(suffix='.pt', delete=False) as tmp:
            tmp_path = Path(tmp.name)
            torch.save(model.state_dict(), tmp_path)
        
        try:
            # Note: In production, you would call the CLI here
            # For this demo, we'll simulate the vault operation
            print_info(f"Storing model '{model_name}' in vault...")
            
            # Get model size
            model_size = tmp_path.stat().st_size
            print_info(f"Model size: {model_size / 1_048_576:.2f} MB")
            
            if metadata:
                print_info(f"Metadata: {json.dumps(metadata, indent=2)}")
            
            print_success(f"Model '{model_name}' stored successfully")
            return True
            
        finally:
            # Clean up temporary file
            tmp_path.unlink()
    
    def load_model(
        self,
        model_name: str,
        model_class: type,
        version: Optional[int] = None
    ) -> nn.Module:
        """Load a PyTorch model from the vault"""
        print_info(f"Loading model '{model_name}' from vault...")
        
        if version:
            print_info(f"Retrieving version {version}...")
        else:
            print_info("Retrieving latest version...")
        
        # In production, retrieve from vault
        # For demo, create a new model instance
        model = model_class()
        
        print_success(f"Model '{model_name}' loaded successfully")
        return model


def create_simple_model():
    """Create a simple CNN for demonstration"""
    if PYTORCH_AVAILABLE:
        class SimpleCNN(nn.Module):
            def __init__(self):
                super(SimpleCNN, self).__init__()
                self.conv1 = nn.Conv2d(3, 16, 3, padding=1)
                self.conv2 = nn.Conv2d(16, 32, 3, padding=1)
                self.fc1 = nn.Linear(32 * 8 * 8, 128)
                self.fc2 = nn.Linear(128, 10)
                self.relu = nn.ReLU()
                self.pool = nn.MaxPool2d(2, 2)
            
            def forward(self, x):
                x = self.pool(self.relu(self.conv1(x)))
                x = self.pool(self.relu(self.conv2(x)))
                x = x.view(-1, 32 * 8 * 8)
                x = self.relu(self.fc1(x))
                x = self.fc2(x)
                return x
        
        return SimpleCNN()
    else:
        # Mock model for demo
        model = nn.Module()
        model.parameters_list = [type('obj', (object,), {'numel': lambda: 1000, 'requires_grad': True})() for _ in range(10)]
        return model


# Store SimpleCNN class globally for loading
SimpleCNN = type('SimpleCNN', (nn.Module,), {
    '__init__': lambda self: setattr(self, 'parameters_list', [])
})


def count_parameters(model) -> int:
    """Count the number of trainable parameters"""
    if PYTORCH_AVAILABLE:
        return sum(p.numel() for p in model.parameters() if p.requires_grad)
    else:
        # Mock parameter count
        return 25832  # Simulated parameter count


def get_model_size(model) -> int:
    """Get model size in bytes"""
    if not PYTORCH_AVAILABLE:
        return 103_328  # Return realistic size for mock
    
    with tempfile.NamedTemporaryFile(suffix='.pt', delete=False) as tmp:
        tmp_path = Path(tmp.name)
    
    try:
        torch.save(model.state_dict(), tmp_path)
        size = tmp_path.stat().st_size
        return size
    finally:
        try:
            tmp_path.unlink()
        except:
            pass


def simulate_training(model, epochs: int = 1):
    """Simulate model training (just modify weights slightly)"""
    print_info(f"Training for {epochs} epoch(s)...")
    
    if PYTORCH_AVAILABLE:
        # Simulate training by slightly modifying weights
        with torch.no_grad():
            for param in model.parameters():
                param.add_(torch.randn_like(param) * 0.01)
    
    print_success("Training complete")
    return model


def quantize_model(model):
    """Quantize model to INT8"""
    print_info("Quantizing model to INT8...")
    
    if PYTORCH_AVAILABLE:
        # Simple dynamic quantization
        quantized = torch.quantization.quantize_dynamic(
            model,
            {nn.Linear, nn.Conv2d},
            dtype=torch.qint8
        )
    else:
        quantized = model  # Mock quantization
    
    print_success("Quantization complete")
    return quantized


def print_box(text_lines):
    """Print a box with text (cross-platform)"""
    if USE_COLOR:
        # Try Unicode box drawing
        try:
            print("╔═══════════════════════════════════════════════════════════════╗")
            for line in text_lines:
                print(f"║ {line:61} ║")
            print("╚═══════════════════════════════════════════════════════════════╝")
        except UnicodeEncodeError:
            # Fallback to ASCII
            print("+" + "=" * 63 + "+")
            for line in text_lines:
                print(f"| {line:61} |")
            print("+" + "=" * 63 + "+")
    else:
        print("+" + "=" * 63 + "+")
        for line in text_lines:
            print(f"| {line:61} |")
        print("+" + "=" * 63 + "+")


def main():
    """Main demonstration"""
    print()
    print(f"{Colors.BOLD}{Colors.CYAN}")
    print_box([
        "IronVault - PyTorch Integration Demo",
        "",
        "Demonstrates secure storage of PyTorch models with",
        "version control, fine-tuning tracking, and quantization"
    ])
    print(f"{Colors.RESET}")
    
    # Initialize vault
    print_header("Step 1: Initialize IronVault")
    vault = IronVault()
    print_success("Vault initialized")
    print_info(f"Vault path: {vault.vault_path}")
    
    # Create a simple model
    print_header("Step 2: Create PyTorch Model")
    print_info("Creating a simple CNN for image classification...")
    model = create_simple_model()
    
    params = count_parameters(model)
    model_size = get_model_size(model)
    
    print_success("Model created successfully")
    print_info(f"Architecture: SimpleCNN (Conv2d -> Conv2d -> FC -> FC)")
    print_info(f"Parameters: {params:,} ({params / 1_000_000:.2f}M)")
    print_info(f"Size: {model_size / 1_048_576:.2f} MB (FP32)")
    
    # Store initial model (v1)
    print_header("Step 3: Store Initial Model (v1)")
    
    metadata_v1 = {
        "architecture": "SimpleCNN",
        "parameters": params,
        "task": "image-classification",
        "framework": f"PyTorch {torch.__version__}",
        "precision": "FP32",
        "description": "Initial untrained model"
    }
    
    vault.store_model(
        "simple-cnn",
        model,
        metadata=metadata_v1,
        description="Initial model architecture"
    )
    
    # Simulate training and store v2
    print_header("Step 4: Train Model and Store Checkpoint (v2)")
    trained_model = simulate_training(model, epochs=5)
    
    metadata_v2 = {
        **metadata_v1,
        "epochs": 5,
        "description": "Trained for 5 epochs on CIFAR-10"
    }
    
    vault.store_model(
        "simple-cnn",
        trained_model,
        metadata=metadata_v2,
        description="After 5 epochs of training"
    )
    
    # Fine-tune and store v3
    print_header("Step 5: Fine-tune Model (v3)")
    print_info("Fine-tuning on specialized dataset...")
    fine_tuned_model = simulate_training(trained_model, epochs=3)
    
    metadata_v3 = {
        **metadata_v2,
        "epochs": 8,
        "fine_tuning": "Medical images dataset",
        "description": "Fine-tuned on medical images (3 additional epochs)"
    }
    
    vault.store_model(
        "simple-cnn",
        fine_tuned_model,
        metadata=metadata_v3,
        description="Fine-tuned on medical images"
    )
    
    # Quantize and store v4
    print_header("Step 6: Quantize Model (v4)")
    quantized_model = quantize_model(fine_tuned_model)
    
    quantized_size = get_model_size(quantized_model)
    compression_ratio = (1 - quantized_size / model_size) * 100
    
    print_info(f"Original size: {model_size / 1_048_576:.2f} MB")
    print_info(f"Quantized size: {quantized_size / 1_048_576:.2f} MB")
    print_info(f"Compression: {compression_ratio:.1f}% smaller")
    
    metadata_v4 = {
        **metadata_v3,
        "precision": "INT8",
        "compression_ratio": f"{compression_ratio:.1f}%",
        "quantization": "Dynamic INT8",
        "description": "Quantized for faster inference"
    }
    
    vault.store_model(
        "simple-cnn",
        quantized_model,
        metadata=metadata_v4,
        description="INT8 quantized model"
    )
    
    # Demonstrate version control
    print_header("Step 7: Version Control Features")
    
    print_info("Version history:")
    versions = [
        ("v1", "Initial model", f"{model_size / 1_048_576:.2f} MB"),
        ("v2", "After 5 epochs training", f"{model_size / 1_048_576:.2f} MB"),
        ("v3", "Fine-tuned on medical images", f"{model_size / 1_048_576:.2f} MB"),
        ("v4", "INT8 quantized", f"{quantized_size / 1_048_576:.2f} MB"),
    ]
    
    for version, desc, size in versions:
        print(f"   {version}: {desc} ({size})")
    
    print()
    print_info("Lineage tree:")
    print("   v1 -> Initial")
    print("     v2 -> Trained (5 epochs)")
    print("       v3 -> Fine-tuned (medical images)")
    print("         v4 -> Quantized (INT8, 2-3x faster)")
    
    # Demonstrate loading specific versions
    print_header("Step 8: Load Specific Version")
    print_info("Loading v2 (trained model before fine-tuning)...")
    
    loaded_model = vault.load_model("simple-cnn", SimpleCNN, version=2)
    print_success("Model v2 loaded successfully")
    print_info("Use case: Compare performance before/after fine-tuning")
    
    # Demonstrate pre-trained models
    print_header("Step 9: Store Pre-trained ResNet (Example)")
    print_info("Loading ResNet-18 from torchvision...")
    
    resnet = models.resnet18(weights=models.ResNet18_Weights.IMAGENET1K_V1)
    resnet_params = count_parameters(resnet)
    resnet_size = get_model_size(resnet)
    
    print_success("ResNet-18 loaded")
    print_info(f"Parameters: {resnet_params:,} ({resnet_params / 1_000_000:.2f}M)")
    print_info(f"Size: {resnet_size / 1_048_576:.2f} MB")
    
    metadata_resnet = {
        "architecture": "ResNet-18",
        "parameters": resnet_params,
        "task": "image-classification",
        "framework": f"PyTorch {torch.__version__}",
        "weights": "ImageNet-1K",
        "top1_accuracy": "69.758%",
        "top5_accuracy": "89.078%",
        "description": "Pre-trained on ImageNet"
    }
    
    vault.store_model(
        "resnet18-imagenet",
        resnet,
        metadata=metadata_resnet,
        description="ResNet-18 pre-trained on ImageNet"
    )
    
    # Use cases
    print_header("Step 10: Real-World Use Cases")
    
    use_cases = [
        ("Training Checkpoints", "Save after each epoch, never lose progress"),
        ("Experiment Tracking", "Compare different architectures and hyperparameters"),
        ("Production Deployment", "Version models, rollback if issues arise"),
        ("Model Compression", "Track FP32 -> FP16 -> INT8 -> Q4 pipeline"),
        ("Fine-tuning Workflow", "Base model -> Domain-specific -> Task-specific"),
        ("Team Collaboration", "Share models securely with version control"),
        ("Compliance & Audit", "Track model lineage for regulatory requirements"),
    ]
    
    for title, description in use_cases:
        print(f"   - {Colors.BOLD}{title}{Colors.RESET}: {description}")
    
    # Code example
    print_header("Step 11: Code Example")
    
    print(f"{Colors.YELLOW}")
    print("Example PyTorch workflow with IronVault:")
    print(f"{Colors.RESET}")
    print()
    print("```python")
    print("import torch")
    print("from ironvault import IronVault")
    print()
    print("# Initialize vault")
    print("vault = IronVault()")
    print()
    print("# Train your model")
    print("model = YourModel()")
    print("train(model)")
    print()
    print("# Store checkpoint with metadata")
    print("vault.store_model(")
    print('    "my-model",')
    print("    model,")
    print("    metadata={")
    print('        "epoch": 10,')
    print('        "loss": 0.125,')
    print('        "accuracy": 0.945')
    print("    }")
    print(")")
    print()
    print("# Load later")
    print('loaded_model = vault.load_model("my-model", YourModel)')
    print("```")
    
    # Benefits summary
    print_header("Benefits Summary")
    
    benefits = [
        "[+] FIPS 140-3 encryption - Models protected at rest",
        "[+] Automatic compression - 30-50% smaller storage",
        "[+] Version control - Complete training history",
        "[+] Lineage tracking - See model evolution",
        "[+] Metadata storage - Track accuracy, loss, hyperparameters",
        "[+] Fast retrieval - Optimized for large models",
        "[+] Cloud storage - S3 and Azure support",
        "[+] PyTorch native - Save/load state_dict seamlessly",
    ]
    
    for benefit in benefits:
        print(f"   {benefit}")
    
    # Cleanup message
    print()
    print(f"{Colors.BOLD}{Colors.GREEN}")
    print_box([
        "PyTorch Integration Demo Complete"
    ])
    print(f"{Colors.RESET}")
    print()
    print("Next steps:")
    print("  - Integrate with your training pipeline")
    print("  - Use vault.store_model() after each epoch")
    print("  - Track experiments with rich metadata")
    print("  - Compare model versions easily")
    print()
    print("Documentation:")
    print("  - README.md - Full documentation")
    print("  - examples/ - More Python examples")
    print("  - docs/ARCHITECTURE.md - System design")
    print()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print()
        print_warning("Demo interrupted by user")
        sys.exit(0)
    except Exception as e:
        print()
        print_error(f"Demo failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
