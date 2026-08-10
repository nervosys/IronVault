"""
Version control system for model checkpoints.

Maintains complete history of model versions with metadata and generations.
"""

import json
import hashlib
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, asdict


@dataclass
class ModelVersion:
    """Represents a single model version/checkpoint."""
    
    version: int
    checkpoint_id: str
    timestamp: str
    parent_version: Optional[int]
    format: str
    size_bytes: int
    compressed_size_bytes: int
    checksum_sha256: str
    metadata: Dict[str, Any]
    file_path: str
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'ModelVersion':
        """Create from dictionary."""
        return cls(**data)


class VersionControl:
    """
    Version control system for model checkpoints.
    
    Features:
    - Complete version history
    - Parent-child relationships (branching)
    - Metadata tracking
    - Checksum verification
    - Generation/lineage tracking
    
    Compliance:
        - CMMC AU.3.046: Alert in the event of an audit logging process failure
        - CMMC AU.3.049: Protect audit information and tools from unauthorized access
    """
    
    VERSION_FILE = "versions.json"
    
    def __init__(self, vault_path: Path) -> None:
        """
        Initialize version control.
        
        Args:
            vault_path: Path to vault directory
        """
        self.vault_path = vault_path
        self.version_file = vault_path / self.VERSION_FILE
        self.versions: Dict[str, List[ModelVersion]] = {}
        self._load_versions()
    
    def _load_versions(self) -> None:
        """Load version history from file."""
        if self.version_file.exists():
            with open(self.version_file, 'r') as f:
                data = json.load(f)
                self.versions = {
                    model_name: [ModelVersion.from_dict(v) for v in versions]
                    for model_name, versions in data.items()
                }
        else:
            self.versions = {}
    
    def _save_versions(self) -> None:
        """Save version history to file."""
        data = {
            model_name: [v.to_dict() for v in versions]
            for model_name, versions in self.versions.items()
        }
        
        with open(self.version_file, 'w') as f:
            json.dump(data, f, indent=2)
    
    def add_version(
        self,
        model_name: str,
        file_path: str,
        format: str,
        size_bytes: int,
        compressed_size_bytes: int,
        checksum: str,
        metadata: Optional[Dict[str, Any]] = None,
        parent_version: Optional[int] = None,
    ) -> ModelVersion:
        """
        Add new model version.
        
        Args:
            model_name: Model identifier
            file_path: Path to encrypted/compressed model file
            format: Model format
            size_bytes: Original size in bytes
            compressed_size_bytes: Compressed size in bytes
            checksum: SHA-256 checksum of original data
            metadata: Optional metadata
            parent_version: Parent version number for branching
        
        Returns:
            Created ModelVersion
        """
        if model_name not in self.versions:
            self.versions[model_name] = []
        
        # Determine next version number
        if self.versions[model_name]:
            version = max(v.version for v in self.versions[model_name]) + 1
        else:
            version = 1
        
        # Generate checkpoint ID
        timestamp = datetime.utcnow().isoformat() + 'Z'
        checkpoint_id = self._generate_checkpoint_id(model_name, version, timestamp)
        
        # Create version
        model_version = ModelVersion(
            version=version,
            checkpoint_id=checkpoint_id,
            timestamp=timestamp,
            parent_version=parent_version,
            format=format,
            size_bytes=size_bytes,
            compressed_size_bytes=compressed_size_bytes,
            checksum_sha256=checksum,
            metadata=metadata or {},
            file_path=file_path,
        )
        
        # Add to history
        self.versions[model_name].append(model_version)
        self._save_versions()
        
        return model_version
    
    def get_version(self, model_name: str, version: Optional[int] = None) -> Optional[ModelVersion]:
        """
        Get specific model version.
        
        Args:
            model_name: Model identifier
            version: Version number (latest if not provided)
        
        Returns:
            ModelVersion or None if not found
        """
        if model_name not in self.versions or not self.versions[model_name]:
            return None
        
        if version is None:
            # Return latest version
            return max(self.versions[model_name], key=lambda v: v.version)
        
        # Find specific version
        for v in self.versions[model_name]:
            if v.version == version:
                return v
        
        return None
    
    def list_versions(self, model_name: str) -> List[ModelVersion]:
        """
        List all versions of a model.
        
        Args:
            model_name: Model identifier
        
        Returns:
            List of ModelVersion objects, sorted by version
        """
        if model_name not in self.versions:
            return []
        
        return sorted(self.versions[model_name], key=lambda v: v.version)
    
    def get_lineage(self, model_name: str, version: int) -> List[ModelVersion]:
        """
        Get complete lineage/generation history for a version.
        
        Args:
            model_name: Model identifier
            version: Version number
        
        Returns:
            List of ModelVersion objects from root to specified version
        """
        target = self.get_version(model_name, version)
        if not target:
            return []
        
        lineage = [target]
        current = target
        
        # Walk back through parent versions
        while current.parent_version is not None:
            parent = self.get_version(model_name, current.parent_version)
            if parent:
                lineage.insert(0, parent)
                current = parent
            else:
                break
        
        return lineage
    
    def delete_version(self, model_name: str, version: int) -> bool:
        """
        Delete a specific version.
        
        Args:
            model_name: Model identifier
            version: Version number to delete
        
        Returns:
            True if deleted, False if not found
        """
        if model_name not in self.versions:
            return False
        
        original_count = len(self.versions[model_name])
        self.versions[model_name] = [
            v for v in self.versions[model_name] if v.version != version
        ]
        
        if len(self.versions[model_name]) < original_count:
            self._save_versions()
            return True
        
        return False
    
    def cleanup_old_versions(self, model_name: str, keep_count: int = 10) -> List[int]:
        """
        Clean up old versions, keeping only the most recent.
        
        Args:
            model_name: Model identifier
            keep_count: Number of versions to keep
        
        Returns:
            List of deleted version numbers
        """
        if model_name not in self.versions:
            return []
        
        versions = sorted(self.versions[model_name], key=lambda v: v.version, reverse=True)
        
        if len(versions) <= keep_count:
            return []
        
        # Keep the most recent versions
        to_keep = versions[:keep_count]
        to_delete = versions[keep_count:]
        
        deleted_versions = [v.version for v in to_delete]
        
        self.versions[model_name] = to_keep
        self._save_versions()
        
        return deleted_versions
    
    def verify_checksum(self, model_name: str, version: int, data: bytes) -> bool:
        """
        Verify data integrity using stored checksum.
        
        Args:
            model_name: Model identifier
            version: Version number
            data: Original (decrypted, decompressed) data
        
        Returns:
            True if checksum matches, False otherwise
        """
        model_version = self.get_version(model_name, version)
        if not model_version:
            return False
        
        checksum = hashlib.sha256(data).hexdigest()
        return checksum == model_version.checksum_sha256
    
    @staticmethod
    def _generate_checkpoint_id(model_name: str, version: int, timestamp: str) -> str:
        """Generate unique checkpoint identifier."""
        data = f"{model_name}:{version}:{timestamp}"
        return hashlib.sha256(data.encode()).hexdigest()[:16]
    
    @staticmethod
    def compute_checksum(data: bytes) -> str:
        """
        Compute SHA-256 checksum of data.
        
        Args:
            data: Data to checksum
        
        Returns:
            Hex-encoded SHA-256 checksum
        """
        return hashlib.sha256(data).hexdigest()
