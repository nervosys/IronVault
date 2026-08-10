"""
IronVault Core Module

Provides vault management and configuration.
"""

from ironvault.core.config import VaultConfig
from ironvault.core.vault import Vault

__all__ = ["Vault", "VaultConfig"]
