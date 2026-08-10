"""
XDG Base Directory Specification compliant configuration.

Cross-platform support for Linux, macOS, and Windows following XDG standards.
"""

import os
from pathlib import Path
from typing import Optional, Dict, Any
import yaml
from platformdirs import user_data_dir, user_config_dir, user_cache_dir


class VaultConfig:
    """
    XDG-compliant vault configuration.
    
    Directory structure:
    - Config: ~/.config/ironvault/ (or platform equivalent)
    - Data: ~/.local/share/ironvault/ (or platform equivalent)
    - Cache: ~/.cache/ironvault/ (or platform equivalent)
    
    Compliance:
        - XDG Base Directory Specification
        - CMMC AC.3.014: Separate duties of individuals
    """
    
    APP_NAME = "ironvault"
    APP_AUTHOR = "nervosys"
    
    def __init__(self, config_override: Optional[Dict[str, Any]] = None) -> None:
        """
        Initialize vault configuration.
        
        Args:
            config_override: Optional configuration overrides
        """
        # XDG-compliant directories
        self.config_dir = Path(user_config_dir(self.APP_NAME, self.APP_AUTHOR))
        self.data_dir = Path(user_data_dir(self.APP_NAME, self.APP_AUTHOR))
        self.cache_dir = Path(user_cache_dir(self.APP_NAME, self.APP_AUTHOR))
        
        # Create directories with secure permissions
        self._ensure_directories()
        
        # Configuration file
        self.config_file = self.config_dir / "config.yaml"
        
        # Load or create configuration
        self.config = self._load_config()
        
        # Apply overrides
        if config_override:
            self.config.update(config_override)
        
        # Apply configuration
        self._apply_config()
    
    def _ensure_directories(self) -> None:
        """Create XDG directories with secure permissions."""
        for directory in [self.config_dir, self.data_dir, self.cache_dir]:
            directory.mkdir(parents=True, exist_ok=True)
            # Set secure permissions (owner only: rwx------)
            if os.name != 'nt':  # Unix-like systems
                os.chmod(directory, 0o700)
    
    def _load_config(self) -> Dict[str, Any]:
        """Load configuration from file or create default."""
        if self.config_file.exists():
            with open(self.config_file, 'r') as f:
                return yaml.safe_load(f) or {}
        else:
            return self._create_default_config()
    
    def _create_default_config(self) -> Dict[str, Any]:
        """Create default configuration."""
        default_config = {
            "version": "1.0",
            "vault": {
                "data_dir": str(self.data_dir / "vaults"),
                "default_vault": "default",
            },
            "crypto": {
                "algorithm": "aes-256-gcm",
                "kdf": "pbkdf2-hmac-sha256",
                "iterations": 600000,
            },
            "compression": {
                "algorithm": "gzip",
                "level": 6,
            },
            "storage": {
                "max_versions": 10,
                "auto_cleanup": True,
                "checkpoint_format": "v{version}_{timestamp}",
            },
            "security": {
                "require_passphrase": True,
                "session_timeout": 3600,  # 1 hour
                "audit_log": True,
            },
            "compliance": {
                "fips_mode": True,
                "cve_scanning": True,
                "audit_retention_days": 90,
            },
        }
        
        # Save default configuration
        self.save_config(default_config)
        return default_config
    
    def _apply_config(self) -> None:
        """Apply configuration to instance attributes."""
        # Vault settings
        vault_config = self.config.get("vault", {})
        self.vault_dir = Path(vault_config.get("data_dir", self.data_dir / "vaults"))
        self.default_vault = vault_config.get("default_vault", "default")
        
        # Crypto settings
        crypto_config = self.config.get("crypto", {})
        self.crypto_algorithm = crypto_config.get("algorithm", "aes-256-gcm")
        self.kdf = crypto_config.get("kdf", "pbkdf2-hmac-sha256")
        self.kdf_iterations = crypto_config.get("iterations", 600000)
        
        # Compression settings
        compression_config = self.config.get("compression", {})
        self.compression_algorithm = compression_config.get("algorithm", "gzip")
        self.compression_level = compression_config.get("level", 6)
        
        # Storage settings
        storage_config = self.config.get("storage", {})
        self.max_versions = storage_config.get("max_versions", 10)
        self.auto_cleanup = storage_config.get("auto_cleanup", True)
        self.checkpoint_format = storage_config.get("checkpoint_format", "v{version}_{timestamp}")
        
        # Security settings
        security_config = self.config.get("security", {})
        self.require_passphrase = security_config.get("require_passphrase", True)
        self.session_timeout = security_config.get("session_timeout", 3600)
        self.audit_log = security_config.get("audit_log", True)
        
        # Compliance settings
        compliance_config = self.config.get("compliance", {})
        self.fips_mode = compliance_config.get("fips_mode", True)
        self.cve_scanning = compliance_config.get("cve_scanning", True)
        self.audit_retention_days = compliance_config.get("audit_retention_days", 90)
        
        # Ensure vault directory exists
        self.vault_dir.mkdir(parents=True, exist_ok=True)
        if os.name != 'nt':
            os.chmod(self.vault_dir, 0o700)
    
    def save_config(self, config: Optional[Dict[str, Any]] = None) -> None:
        """
        Save configuration to file.
        
        Args:
            config: Configuration to save (uses self.config if not provided)
        """
        config_to_save = config if config is not None else self.config
        
        with open(self.config_file, 'w') as f:
            yaml.dump(config_to_save, f, default_flow_style=False)
        
        # Set secure permissions
        if os.name != 'nt':
            os.chmod(self.config_file, 0o600)
    
    def get_vault_path(self, vault_name: Optional[str] = None) -> Path:
        """
        Get path to specific vault.
        
        Args:
            vault_name: Vault name (uses default if not provided)
        
        Returns:
            Path to vault directory
        """
        name = vault_name or self.default_vault
        return self.vault_dir / name
    
    def get_audit_log_path(self) -> Path:
        """Get path to audit log file."""
        log_dir = self.data_dir / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        if os.name != 'nt':
            os.chmod(log_dir, 0o700)
        return log_dir / "audit.log"
