# PyTorch Integration with uv

This guide shows how to use IronVault with PyTorch using [uv](https://astral.sh/uv/) for fast Python package management.

## Why uv?

- ⚡ **10-100x faster** than pip for installing packages
- 🎯 **Single binary** - no Python required to install
- 🔒 **Built-in virtual environments** - isolated dependencies
- 📦 **Better caching** - reuses downloads across projects
- 🚀 **Drop-in pip replacement** - same commands, faster execution

## Quick Start

### 1. Install uv (if not already installed)

**Windows (PowerShell)**:
```powershell
powershell -c "irm https://astral.sh/uv/install.ps1 | iex"
```

**Linux/macOS**:
```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

**Alternative**: Download from https://astral.sh/uv/

### 2. Run the PyTorch Demo

**Windows**:
```powershell
# Install PyTorch and run demo
.\setup_pytorch.ps1 -Install -Run

# Or step by step
.\setup_pytorch.ps1 -Install  # Install dependencies
.\setup_pytorch.ps1 -Run      # Run the demo
```

**Linux/macOS**:
```bash
# Install PyTorch and run demo
./setup_pytorch.sh --install --run

# Or step by step
./setup_pytorch.sh --install  # Install dependencies
./setup_pytorch.sh --run      # Run the demo
```

### 3. Manual Installation

If you prefer to install manually:

```bash
# Install dependencies with uv
uv pip install -r requirements.txt

# Or install specific packages
uv pip install torch torchvision

# Run the demo
uv run demo_pytorch.py
```

## What the Demo Shows

The PyTorch integration demo (`demo_pytorch.py`) demonstrates:

1. **Model Creation**: Create a simple CNN model for image classification
2. **Training Simulation**: Mock training with progress tracking
3. **Version Control**: Store model checkpoints with automatic versioning
4. **Fine-tuning**: Update model and track lineage
5. **Quantization**: Convert FP32 → INT8 and track compression
6. **Model Loading**: Retrieve any version from the vault
7. **Comparison**: Compare model sizes and versions
8. **Rollback**: Restore previous versions
9. **Lineage Tracking**: View complete model evolution
10. **Integrity Verification**: Verify model hasn't been tampered with
11. **Cleanup**: Automatic vault cleanup

## Works Without PyTorch

The demo includes mock PyTorch classes, so it works even if PyTorch isn't installed:

```bash
# Run without installing PyTorch
python demo_pytorch.py
```

You'll see a warning that PyTorch isn't installed, but the demo will simulate all operations.

## Requirements

- Python 3.8+
- uv 0.1.0+ (installed automatically by setup scripts)
- PyTorch 2.0+ (optional - demo has mocks)

## Project Files

- `requirements.txt` - Python dependencies (PyTorch, torchvision)
- `setup_pytorch.ps1` - Windows setup script
- `setup_pytorch.sh` - Linux/macOS setup script
- `demo_pytorch.py` - PyTorch integration demo

## Troubleshooting

### uv not found

If you get "uv: command not found":

1. Install uv following the instructions above
2. Restart your terminal to refresh PATH
3. Verify: `uv --version`

### PyTorch installation slow

uv is much faster than pip, but PyTorch is a large package (~2GB). First install might take 1-2 minutes.

### Permission errors on Windows

Run PowerShell as Administrator if you see permission errors.

### Unix permissions on Linux/macOS

Make sure scripts are executable:
```bash
chmod +x setup_pytorch.sh demo.sh
```

## Using with Your Models

To use IronVault with your own PyTorch models:

```python
import subprocess
import json

class IronVault:
    """Wrapper for IronVault CLI"""
    
    def __init__(self, vault_path="./model_vault"):
        self.vault_path = vault_path
        self.password = "your-secure-password"  # Use env vars in production
    
    def store_model(self, model_path, model_id, metadata=None):
        """Store a PyTorch model"""
        cmd = [
            "iv", "store",
            "--model", model_path,
            "--id", model_id,
            "--vault-path", self.vault_path,
            "--password", self.password
        ]
        
        if metadata:
            cmd.extend(["--metadata", json.dumps(metadata)])
        
        subprocess.run(cmd, check=True)
    
    def load_model(self, model_id, output_path, version=None):
        """Load a model from vault"""
        cmd = [
            "iv", "get",
            "--id", model_id,
            "--output", output_path,
            "--vault-path", self.vault_path,
            "--password", self.password
        ]
        
        if version:
            cmd.extend(["--version", version])
        
        subprocess.run(cmd, check=True)

# Example usage
vault = IronVault()

# Save your model
torch.save(model.state_dict(), "my_model.pt")
vault.store_model(
    "my_model.pt", 
    "image-classifier-v1",
    metadata={"accuracy": 0.95, "epoch": 50}
)

# Load it back
vault.load_model("image-classifier-v1", "restored_model.pt")
model.load_state_dict(torch.load("restored_model.pt"))
```

## Next Steps

- Read the [PyTorch demo source](demo_pytorch.py) for detailed examples
- Check [DEMO_GUIDE.md](DEMO_GUIDE.md) for more demo options
- See [UTILITIES.md](UTILITIES.md) for model utilities
- Read [FEATURES_DEMO.md](https://github.com/nervosys/IronVault/blob/master/reports/FEATURES_DEMO.md) for all features

## Learn More

- **uv documentation**: https://docs.astral.sh/uv/
- **IronVault docs**: [Quick Start](QUICKSTART.md)
- **PyTorch tutorials**: https://pytorch.org/tutorials/
