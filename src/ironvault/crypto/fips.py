"""
FIPS 140-3 Compliant Cryptographic Module

This module provides FIPS 140-3 compliant encryption/decryption for model storage.
Uses AES-256-GCM with PBKDF2 key derivation.

.. warning:: Crypto Compatibility

   This Python module uses **PBKDF2-HMAC-SHA256** for key derivation.
   The Rust implementation (``src/crypto/mod.rs``) uses **Argon2id**.
   Vaults created by the Rust ``iv`` CLI **cannot** be decrypted by this
   Python module and vice-versa. For interop, use the Rust binary via
   ``ironvault.core.vault.Vault`` (subprocess wrapper) or wait for
   PyO3 bindings (planned for v0.3.0).

Security Controls:
- NIST SP 800-38D (GCM mode)
- NIST SP 800-132 (PBKDF2)
- FIPS 197 (AES)
"""

import os
import secrets
from typing import Optional, Tuple

from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.backends import default_backend


class FIPSCrypto:
    """
    FIPS 140-3 compliant cryptographic operations.
    
    Compliance Mappings:
    - CMMC 2.0: SC.3.177 (Employ FIPS-validated cryptography)
    - MITRE ATT&CK: T1486 mitigation (Data Encrypted for Impact)
    """
    
    # FIPS 140-3 approved parameters
    KEY_SIZE = 32  # 256 bits for AES-256
    SALT_SIZE = 32  # 256 bits
    NONCE_SIZE = 12  # 96 bits (recommended for GCM)
    TAG_SIZE = 16  # 128 bits authentication tag
    ITERATIONS = 600000  # OWASP recommendation for PBKDF2-HMAC-SHA256
    
    def __init__(self) -> None:
        """Initialize FIPS crypto module."""
        self.backend = default_backend()
    
    def generate_key(self, passphrase: bytes, salt: Optional[bytes] = None) -> Tuple[bytes, bytes]:
        """
        Derive encryption key from passphrase using PBKDF2-HMAC-SHA256.
        
        Args:
            passphrase: User passphrase
            salt: Optional salt (generated if not provided)
        
        Returns:
            Tuple of (encryption_key, salt)
        
        Compliance:
            - FIPS 140-3: Approved key derivation
            - NIST SP 800-132: Password-based key derivation
        """
        if salt is None:
            salt = secrets.token_bytes(self.SALT_SIZE)
        
        kdf = PBKDF2HMAC(
            algorithm=hashes.SHA256(),
            length=self.KEY_SIZE,
            salt=salt,
            iterations=self.ITERATIONS,
            backend=self.backend
        )
        
        key = kdf.derive(passphrase)
        return key, salt
    
    def encrypt(self, data: bytes, key: bytes) -> bytes:
        """
        Encrypt data using AES-256-GCM.
        
        Args:
            data: Plaintext data to encrypt
            key: 256-bit encryption key
        
        Returns:
            Encrypted data with format: nonce || ciphertext || tag
        
        Compliance:
            - FIPS 197: AES encryption
            - NIST SP 800-38D: GCM mode
            - CMMC SC.3.191: Protection of CUI at rest
        """
        if len(key) != self.KEY_SIZE:
            raise ValueError(f"Key must be {self.KEY_SIZE} bytes")
        
        # Generate cryptographically secure random nonce
        nonce = secrets.token_bytes(self.NONCE_SIZE)
        
        # Create AESGCM cipher
        aesgcm = AESGCM(key)
        
        # Encrypt and authenticate
        ciphertext = aesgcm.encrypt(nonce, data, None)
        
        # Format: nonce || ciphertext (includes auth tag)
        return nonce + ciphertext
    
    def decrypt(self, encrypted_data: bytes, key: bytes) -> bytes:
        """
        Decrypt data using AES-256-GCM.
        
        Args:
            encrypted_data: Encrypted data (nonce || ciphertext || tag)
            key: 256-bit encryption key
        
        Returns:
            Decrypted plaintext data
        
        Raises:
            InvalidTag: If authentication fails (tampering detected)
        """
        if len(key) != self.KEY_SIZE:
            raise ValueError(f"Key must be {self.KEY_SIZE} bytes")
        
        # Extract nonce and ciphertext
        nonce = encrypted_data[:self.NONCE_SIZE]
        ciphertext = encrypted_data[self.NONCE_SIZE:]
        
        # Create AESGCM cipher
        aesgcm = AESGCM(key)
        
        # Decrypt and verify authentication tag
        plaintext = aesgcm.decrypt(nonce, ciphertext, None)
        
        return plaintext
    
    @staticmethod
    def generate_passphrase(length: int = 32) -> str:
        """
        Generate cryptographically secure random passphrase.
        
        Args:
            length: Length of passphrase in bytes
        
        Returns:
            Hex-encoded passphrase
        
        Compliance:
            - FIPS 140-3: Approved random number generation
        """
        return secrets.token_hex(length)
    
    @staticmethod
    def secure_compare(a: bytes, b: bytes) -> bool:
        """
        Constant-time comparison to prevent timing attacks.
        
        Args:
            a: First value
            b: Second value
        
        Returns:
            True if equal, False otherwise
        
        Compliance:
            - MITRE ATT&CK: T1552.004 mitigation (timing attacks)
        """
        return secrets.compare_digest(a, b)


class KeyManager:
    """
    Secure key management system.
    
    Compliance:
        - CMMC AC.3.018: Control connection of mobile devices
        - CMMC IA.3.080: Protect authenticators
    """
    
    def __init__(self, key_storage_path: Optional[str] = None) -> None:
        """
        Initialize key manager.
        
        Args:
            key_storage_path: Optional path for encrypted key storage
        """
        self.crypto = FIPSCrypto()
        self.key_storage_path = key_storage_path
    
    def store_key(self, key: bytes, filename: str, master_passphrase: bytes) -> None:
        """
        Store encryption key securely using key encryption key (KEK).
        
        Args:
            key: Key to store
            filename: Storage filename
            master_passphrase: Master passphrase for KEK
        """
        if not self.key_storage_path:
            raise ValueError("Key storage path not configured")
        
        # Generate KEK from master passphrase
        kek, salt = self.crypto.generate_key(master_passphrase)
        
        # Encrypt the key
        encrypted_key = self.crypto.encrypt(key, kek)
        
        # Store salt || encrypted_key
        key_path = os.path.join(self.key_storage_path, filename)
        with open(key_path, 'wb') as f:
            f.write(salt + encrypted_key)
        
        # Set restrictive permissions (owner read/write only)
        os.chmod(key_path, 0o600)
    
    def load_key(self, filename: str, master_passphrase: bytes) -> bytes:
        """
        Load and decrypt stored encryption key.
        
        Args:
            filename: Storage filename
            master_passphrase: Master passphrase for KEK
        
        Returns:
            Decrypted key
        """
        if not self.key_storage_path:
            raise ValueError("Key storage path not configured")
        
        key_path = os.path.join(self.key_storage_path, filename)
        
        with open(key_path, 'rb') as f:
            data = f.read()
        
        # Extract salt and encrypted key
        salt = data[:FIPSCrypto.SALT_SIZE]
        encrypted_key = data[FIPSCrypto.SALT_SIZE:]
        
        # Derive KEK from master passphrase
        kek, _ = self.crypto.generate_key(master_passphrase, salt)
        
        # Decrypt the key
        key = self.crypto.decrypt(encrypted_key, kek)
        
        return key
