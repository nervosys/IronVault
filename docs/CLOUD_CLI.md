# ☁️ Cloud Storage CLI Guide

Complete guide to using IronVault's cloud storage commands.

## 📋 Table of Contents

- [Overview](#overview)
- [Supported Providers](#supported-providers)
- [Configuration](#configuration)
- [Commands](#commands)
- [Examples](#examples)
- [Security Notes](#security-notes)
- [Troubleshooting](#troubleshooting)

---

## Overview

IronVault supports pushing and pulling models to/from cloud storage providers:
- **AWS S3** - Amazon Simple Storage Service
- **Azure Blob Storage** - Microsoft Azure cloud storage
- **Google Cloud Storage** - Google Cloud Platform storage (temporarily disabled)

Since 4.3.0, `iv cloud push` seals the payload with AES-256-GCM before it
leaves the process, so the bucket holds ciphertext. See
[Security Notes](#security-notes) before choosing a bucket.

---

## Supported Providers

### AWS S3 ✅
- **Status**: Fully Supported
- **Features**: Push, Pull, List, Config
- **Requirements**: AWS credentials (Access Key ID, Secret Access Key)
- **Optional**: AWS Region (defaults to us-east-1)

### Azure Blob Storage ✅
- **Status**: Fully Supported
- **Features**: Push, Pull, List, Config
- **Requirements**: Azure Storage Account, Storage Key
- **Optional**: Custom endpoints

### Google Cloud Storage ⚠️
- **Status**: Temporarily Disabled
- **Reason**: Security vulnerabilities in cloud-storage dependency
  - RUSTSEC-2025-0009: Unmaintained crate
  - RUSTSEC-2025-0010: Security issues
- **Alternative**: Use AWS S3 or Azure Blob Storage
- **Timeline**: Will be re-enabled when secure alternative is available

---

## Configuration

### AWS S3 Setup

```bash
# Set AWS credentials
export AWS_ACCESS_KEY_ID=your_access_key_here
export AWS_SECRET_ACCESS_KEY=your_secret_key_here

# Optional: Set region (defaults to us-east-1)
export AWS_REGION=us-west-2

# Verify configuration
iv cloud config --provider s3 --show
```

**Windows PowerShell**:
```powershell
$env:AWS_ACCESS_KEY_ID = "your_access_key_here"
$env:AWS_SECRET_ACCESS_KEY = "your_secret_key_here"
$env:AWS_REGION = "us-west-2"

iv cloud config --provider s3 --show
```

### Azure Blob Storage Setup

```bash
# Set Azure credentials
export AZURE_STORAGE_ACCOUNT=your_account_name
export AZURE_STORAGE_SAS_TOKEN=your_sas_token

# Verify configuration
iv cloud config --provider azure --show
```

**Windows PowerShell**:
```powershell
$env:AZURE_STORAGE_ACCOUNT = "your_account_name"
$env:AZURE_STORAGE_SAS_TOKEN = "your_sas_token"

iv cloud config --provider azure --show
```

---

## Commands

### `iv cloud push`

Upload a model from your local vault to cloud storage.

**Usage**:
```bash
iv cloud push <MODEL> --provider <PROVIDER> --bucket <BUCKET> [--version <VERSION>]
```

**Arguments**:
- `<MODEL>` - Model name in your vault
- `--provider, -p` - Cloud provider: `s3`, `azure`, or `gcs`
- `--bucket, -b` - Bucket/container name
- `--version, -v` - Optional: Version number (defaults to latest)

**Examples**:
```bash
# Push latest version to S3
iv cloud push gpt2-finetuned --provider s3 --bucket my-models

# Push specific version to Azure
iv cloud push bert-classifier --provider azure --bucket ml-models --version 3

# Push to S3 with short flags
iv cloud push llama-7b -p s3 -b production-models
```

**Output**:
```
☁️  Pushing model to cloud storage
   Model: gpt2-finetuned
   Provider: s3
   Bucket: my-models

📤 Uploading to S3...
   Region: us-east-1
   Path: gpt2-finetuned/safetensors/v2.vault
   Size: 548576768 bytes

✅ Model pushed successfully!
   Use 'iv cloud pull' to retrieve from cloud
```

### `iv cloud pull`

Download a model from cloud storage to your local vault.

**Usage**:
```bash
iv cloud pull <MODEL> --provider <PROVIDER> --bucket <BUCKET> --remote-path <PATH>
```

**Arguments**:
- `<MODEL>` - Model name to save as locally
- `--provider, -p` - Cloud provider: `s3`, `azure`, or `gcs`
- `--bucket, -b` - Bucket/container name
- `--remote-path, -r` - Path to model in cloud storage

**Examples**:
```bash
# Pull from S3
iv cloud pull gpt2-finetuned --provider s3 --bucket my-models --remote-path gpt2-finetuned/safetensors/v2.vault

# Pull from Azure
iv cloud pull bert-classifier --provider azure --bucket ml-models --remote-path models/bert/v1.vault
```

**Output**:
```
☁️  Pulling model from cloud storage
   Model: gpt2-finetuned
   Provider: s3
   Bucket: my-models
   Remote path: gpt2-finetuned/safetensors/v2.vault

📥 Downloading from S3...

⚠️  Note: Cloud pull functionality requires:
   1. AWS credentials configured (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
   2. Appropriate IAM permissions for S3 access
   3. The model to be stored in the specified bucket/path

💡 After downloading, use 'iv store' to import into vault
```

### `iv cloud list`

List models stored in cloud storage.

**Usage**:
```bash
iv cloud list --provider <PROVIDER> --bucket <BUCKET> [--prefix <PREFIX>]
```

**Arguments**:
- `--provider, -p` - Cloud provider: `s3`, `azure`, or `gcs`
- `--bucket, -b` - Bucket/container name
- `--prefix` - Optional: Filter by path prefix

**Examples**:
```bash
# List all models in S3 bucket
iv cloud list --provider s3 --bucket my-models

# List models with prefix
iv cloud list --provider azure --bucket ml-models --prefix production/

# List with short flags
iv cloud list -p s3 -b my-models
```

### `iv cloud config`

View or configure cloud storage credentials.

**Usage**:
```bash
iv cloud config --provider <PROVIDER> [--show]
```

**Arguments**:
- `--provider, -p` - Cloud provider: `s3`, `azure`, or `gcs`
- `--show` - Display current configuration status

**Examples**:
```bash
# Show S3 configuration
iv cloud config --provider s3 --show

# Show Azure configuration
iv cloud config --provider azure --show

# Show help for configuring
iv cloud config --provider s3
```

**Output (S3)**:
```
☁️  Cloud Storage Configuration
   Provider: s3

📝 AWS S3 Configuration:
   Required environment variables:
   - AWS_ACCESS_KEY_ID: ✅ Set
   - AWS_SECRET_ACCESS_KEY: ✅ Set
   - AWS_REGION (optional): us-west-2

💡 To configure:
   export AWS_ACCESS_KEY_ID=your_access_key
   export AWS_SECRET_ACCESS_KEY=your_secret_key
   export AWS_REGION=us-east-1  # optional
```

---

## Examples

### Complete Workflow: S3

```bash
# 1. Configure AWS credentials
export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
export AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
export AWS_REGION=us-west-2

# 2. Verify configuration
iv cloud config --provider s3 --show

# 3. Store a model locally first
iv store gpt2-finetuned pytorch_model.bin --format safetensors

# 4. Push to S3
iv cloud push gpt2-finetuned --provider s3 --bucket my-ai-models

# 5. List models in S3
iv cloud list --provider s3 --bucket my-ai-models

# 6. Pull model on another machine
iv cloud pull gpt2-finetuned --provider s3 --bucket my-ai-models --remote-path gpt2-finetuned/safetensors/v1.vault
```

### Complete Workflow: Azure

```bash
# 1. Configure Azure credentials
export AZURE_STORAGE_ACCOUNT=mystorageaccount
export AZURE_STORAGE_SAS_TOKEN="sv=2024-11-04&ss=b&srt=co&sp=rwdlac&sig=..."

# 2. Verify configuration
iv cloud config --provider azure --show

# 3. Store model locally
iv store bert-classifier model.safetensors --format safetensors

# 4. Push to Azure
iv cloud push bert-classifier --provider azure --bucket ml-models

# 5. List models
iv cloud list --provider azure --bucket ml-models

# 6. Pull on another machine
iv cloud pull bert-classifier --provider azure --bucket ml-models --remote-path bert-classifier/safetensors/v1.vault
```

### Multi-Version Workflow

```bash
# Push multiple versions
iv cloud push mymodel --provider s3 --bucket models --version 1
iv cloud push mymodel --provider s3 --bucket models --version 2
iv cloud push mymodel --provider s3 --bucket models --version 3

# List all versions
iv cloud list --provider s3 --bucket models --prefix mymodel/

# Pull specific version
iv cloud pull mymodel-v2 --provider s3 --bucket models --remote-path mymodel/safetensors/v2.vault
```

---

## Security Notes

### Encryption

**As of 4.3.0 `iv cloud push` encrypts before upload.** The payload is sealed
with AES-256-GCM under an Argon2id key derived from your vault passphrase,
with a fresh salt per object.

- In transit: TLS, enforced by both cloud SDKs
- At rest in the bucket: ciphertext — the provider never sees the model
- Portable: the salt travels with the object, so a peer who knows the
  passphrase can pull into a different vault
- The passphrase never leaves your machine, and is what protects the object

Server-side encryption (SSE-KMS on S3, CMK on Azure) is still worth enabling
as defence in depth.

Objects pushed **before** 4.3.0 are plaintext. `pull` still accepts them, so
nothing is stranded, but it warns — re-push to seal, then delete the old
object. See [CLOUD_STORAGE.md](CLOUD_STORAGE.md#security-model).

### Credentials
- **Never commit credentials** to version control
- Use environment variables or secure credential stores
- Rotate credentials regularly
- Use IAM roles on cloud platforms when possible

### Access Control
- Set bucket policies to restrict access
- Use private buckets only (not public)
- Enable versioning on buckets for rollback
- Enable audit logging for compliance

### Best Practices
1. **Use separate buckets** for dev/staging/production
2. **Enable bucket encryption** at rest (S3/Azure)
3. **Use VPC endpoints** (S3) or Private Endpoints (Azure) when possible
4. **Monitor access logs** for unusual activity
5. **Test disaster recovery** procedures regularly

---

## Troubleshooting

### Error: "Unsupported provider"
**Problem**: Provider name is incorrect or not supported.  
**Solution**: Use `s3`, `azure`, or `gcs` (case-insensitive).
```bash
# Wrong
iv cloud push model --provider aws --bucket mybucket

# Correct
iv cloud push model --provider s3 --bucket mybucket
```

### Error: "AWS credentials not set"
**Problem**: AWS environment variables missing.  
**Solution**: Set credentials and verify:
```bash
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
iv cloud config --provider s3 --show
```

### Error: "Azure credentials not set"
**Problem**: Azure environment variables missing.  
**Solution**: Set credentials and verify:
```bash
export AZURE_STORAGE_ACCOUNT=your_account
export AZURE_STORAGE_SAS_TOKEN=your_sas_token
iv cloud config --provider azure --show
```

### Error: "Model not found"
**Problem**: Model doesn't exist in local vault.  
**Solution**: List models and check name:
```bash
iv list
iv cloud push correct-model-name --provider s3 --bucket mybucket
```

### Error: "Bucket access denied"
**Problem**: Insufficient permissions or bucket doesn't exist.  
**Solution**: 
1. Verify bucket exists
2. Check IAM permissions (S3) or RBAC (Azure)
3. Test with AWS CLI or Azure CLI first

### GCS Security Warning
**Problem**: GCS commands show security warning.  
**Reason**: Temporary security hold on cloud-storage dependency.  
**Solution**: Use S3 or Azure instead:
```bash
# Instead of GCS
iv cloud push model --provider gcs --bucket mybucket

# Use S3 or Azure
iv cloud push model --provider s3 --bucket mybucket
iv cloud push model --provider azure --bucket mybucket
```

### Feature Flag: S3 Support
If you see "S3 support not enabled in this build":
```bash
# Rebuild with S3 feature
cargo build --release --features s3

# Or install pre-built binary with cloud support
```

---

## Performance Tips

### Large Models
For models >1GB:
1. Use regions close to your location
2. Consider multi-part upload (future feature)
3. Monitor network bandwidth
4. Use compression (already enabled in vault)

### Batch Operations
For multiple models:
```bash
#!/bin/bash
BUCKET="my-models"
PROVIDER="s3"

for model in model1 model2 model3; do
    iv cloud push $model --provider $PROVIDER --bucket $BUCKET
done
```

### Cost Optimization
- Use S3 Standard-IA or Azure Cool tier for infrequent access
- Enable lifecycle policies to archive old versions
- Monitor storage costs with cloud provider tools
- Delete unused models from cloud storage

---

## Integration Examples

### CI/CD Pipeline
```yaml
# GitHub Actions example
name: Deploy Model
on: [push]
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Setup credentials
        env:
          AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
        run: |
          iv cloud config --provider s3 --show
      - name: Push model
        run: |
          iv cloud push production-model --provider s3 --bucket prod-models
```

### Backup Script
```bash
#!/bin/bash
# backup-models.sh

BUCKET="model-backups"
PROVIDER="s3"
DATE=$(date +%Y%m%d)

# Get all models
for model in $(iv list | grep -v "^No models" | awk '{print $1}'); do
    echo "Backing up $model..."
    iv cloud push $model --provider $PROVIDER --bucket $BUCKET
done

echo "Backup complete: $DATE"
```

---

## Future Enhancements

Planned features for future releases:
- ✅ Multi-part upload for large models
- ✅ Resumable uploads
- ✅ Cloud-to-cloud copy
- ✅ Lifecycle management automation
- ✅ Cost estimation
- ✅ Batch operations
- ✅ Progress bars for uploads/downloads
- ✅ GCS re-enablement with secure dependency

---

## See Also

- [CLI Guide](CLI.md) - Complete CLI reference
- [Cloud Storage Implementation](https://github.com/nervosys/IronVault/blob/master/reports/CLOUD_STORAGE_COMPLETE.md) - Technical details
- [Security Guide](https://github.com/nervosys/IronVault/blob/master/SECURITY.md) - Security best practices
- [Quick Start](QUICKSTART.md) - Getting started guide

---

**Version**: 0.1.0  
**Status**: Production Ready ✅  
**Last Updated**: November 7, 2025
