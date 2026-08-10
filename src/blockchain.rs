//! Blockchain-based immutable audit trail
//!
//! Provides cryptographic proof of audit log integrity using:
//! - Merkle tree structure for efficient verification
//! - Hash chain linking blocks together
//! - Digital signatures for non-repudiation
//!
//! ## Architecture
//!
//! ```text
//! Block N-1          Block N            Block N+1
//! ┌────────────┐    ┌────────────┐    ┌────────────┐
//! │ prev_hash  │◄───│ prev_hash  │◄───│ prev_hash  │
//! │ merkle_root│    │ merkle_root│    │ merkle_root│
//! │ timestamp  │    │ timestamp  │    │ timestamp  │
//! │ nonce      │    │ nonce      │    │ nonce      │
//! └────────────┘    └────────────┘    └────────────┘
//!       │                │                │
//!    entries          entries          entries
//! ```
//!
//! ## Security Properties
//!
//! - **Immutability**: Hash chains prevent modification of past entries
//! - **Tamper Evidence**: Any modification breaks the chain
//! - **Non-repudiation**: Optional signing proves authorship
//! - **Efficient Verification**: Merkle proofs for individual entries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::audit::{AuditEntry, AuditEventType};
use crate::error::{Result, VaultError};

/// SHA-256 hash as hex string
pub type Hash = String;

/// Block index
pub type BlockIndex = u64;

/// Compute SHA-256 hash of data
fn sha256(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Merkle tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Hash of this node
    pub hash: Hash,
    /// Left child hash (if internal node)
    pub left: Option<Hash>,
    /// Right child hash (if internal node)
    pub right: Option<Hash>,
}

/// Merkle tree for a set of entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Root hash
    pub root: Hash,
    /// All nodes indexed by hash
    pub nodes: HashMap<Hash, MerkleNode>,
    /// Leaf hashes (in order)
    pub leaves: Vec<Hash>,
}

impl MerkleTree {
    /// Build a Merkle tree from data
    pub fn build(data: &[Vec<u8>]) -> Self {
        if data.is_empty() {
            return Self {
                root: sha256(b""),
                nodes: HashMap::new(),
                leaves: Vec::new(),
            };
        }

        // Compute leaf hashes
        let leaves: Vec<Hash> = data.iter().map(|d| sha256(d)).collect();
        let mut nodes = HashMap::new();

        // Add leaf nodes
        for hash in &leaves {
            nodes.insert(
                hash.clone(),
                MerkleNode {
                    hash: hash.clone(),
                    left: None,
                    right: None,
                },
            );
        }

        // Build tree bottom-up
        let mut current_level = leaves.clone();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let left = &chunk[0];
                let right = if chunk.len() > 1 { &chunk[1] } else { left };

                let combined = format!("{}{}", left, right);
                let parent_hash = sha256(combined.as_bytes());

                nodes.insert(
                    parent_hash.clone(),
                    MerkleNode {
                        hash: parent_hash.clone(),
                        left: Some(left.clone()),
                        right: Some(right.clone()),
                    },
                );

                next_level.push(parent_hash);
            }

            current_level = next_level;
        }

        Self {
            root: current_level.into_iter().next().unwrap_or_default(),
            nodes,
            leaves,
        }
    }

    /// Generate proof for a leaf at given index
    pub fn generate_proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaves.len() {
            return None;
        }

        let mut proof = Vec::new();
        let mut current_idx = index;
        let mut current_level = self.leaves.clone();

        while current_level.len() > 1 {
            let sibling_idx = if current_idx.is_multiple_of(2) {
                current_idx + 1
            } else {
                current_idx - 1
            };

            let sibling = current_level
                .get(sibling_idx.min(current_level.len() - 1))
                .cloned();
            let is_left = current_idx % 2 == 1;

            if let Some(s) = sibling {
                proof.push(ProofElement { hash: s, is_left });
            }

            // Build next level
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let left = &chunk[0];
                let right = if chunk.len() > 1 { &chunk[1] } else { left };
                let combined = format!("{}{}", left, right);
                next_level.push(sha256(combined.as_bytes()));
            }

            current_idx /= 2;
            current_level = next_level;
        }

        Some(MerkleProof {
            leaf_hash: self.leaves[index].clone(),
            proof,
            root: self.root.clone(),
        })
    }

    /// Verify a proof
    pub fn verify_proof(proof: &MerkleProof) -> bool {
        let mut current = proof.leaf_hash.clone();

        for element in &proof.proof {
            let combined = if element.is_left {
                format!("{}{}", element.hash, current)
            } else {
                format!("{}{}", current, element.hash)
            };
            current = sha256(combined.as_bytes());
        }

        current == proof.root
    }
}

/// Merkle proof for a single entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Hash of the leaf being proven
    pub leaf_hash: Hash,
    /// Proof path (hashes of siblings from leaf to root)
    pub proof: Vec<ProofElement>,
    /// Expected root hash
    pub root: Hash,
}

/// Single element in a Merkle proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofElement {
    /// Hash of the sibling node
    pub hash: Hash,
    /// Whether this sibling is on the left
    pub is_left: bool,
}

/// Audit block in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditBlock {
    /// Block index (height)
    pub index: BlockIndex,
    /// Timestamp of block creation
    pub timestamp: DateTime<Utc>,
    /// Hash of the previous block
    pub prev_hash: Hash,
    /// Merkle root of entries in this block
    pub merkle_root: Hash,
    /// Entries in this block
    pub entries: Vec<BlockEntry>,
    /// Optional digital signature (base64-encoded)
    pub signature: Option<String>,
    /// Nonce (for proof-of-work, if enabled)
    pub nonce: u64,
    /// Block hash
    pub hash: Hash,
}

impl AuditBlock {
    /// Compute block hash
    pub fn compute_hash(&self) -> Hash {
        let header = format!(
            "{}:{}:{}:{}:{}",
            self.index, self.timestamp, self.prev_hash, self.merkle_root, self.nonce
        );
        sha256(header.as_bytes())
    }

    /// Verify block integrity
    pub fn verify(&self, prev_block: Option<&AuditBlock>) -> BlockVerification {
        let mut issues = Vec::new();

        // Check hash
        if self.hash != self.compute_hash() {
            issues.push("Block hash mismatch".into());
        }

        // Check previous hash
        if let Some(prev) = prev_block {
            if self.prev_hash != prev.hash {
                issues.push("Previous hash mismatch".into());
            }
            if self.index != prev.index + 1 {
                issues.push("Non-sequential index".into());
            }
            if self.timestamp < prev.timestamp {
                issues.push("Timestamp before previous block".into());
            }
        } else if self.index != 0 {
            issues.push("Non-genesis block without predecessor".into());
        }

        // Verify Merkle root
        let entry_data: Vec<Vec<u8>> = self
            .entries
            .iter()
            .map(|e| serde_json::to_vec(e).unwrap_or_default())
            .collect();
        let tree = MerkleTree::build(&entry_data);
        if tree.root != self.merkle_root {
            issues.push("Merkle root mismatch".into());
        }

        BlockVerification {
            valid: issues.is_empty(),
            issues,
        }
    }
}

/// Block verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockVerification {
    /// Whether the block is valid
    pub valid: bool,
    /// List of issues found
    pub issues: Vec<String>,
}

/// Entry within a block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEntry {
    /// Original audit entry
    pub audit: AuditEntry,
    /// Entry hash
    pub hash: Hash,
    /// Index within block
    pub index_in_block: usize,
}

/// Blockchain audit trail manager
pub struct BlockchainAudit {
    /// Chain storage directory
    chain_dir: PathBuf,
    /// Current block being built
    pending_entries: Vec<BlockEntry>,
    /// Block size threshold (entries per block)
    block_size: usize,
    /// Latest block (cached)
    latest_block: Option<AuditBlock>,
    /// Genesis block hash
    genesis_hash: Hash,
}

impl BlockchainAudit {
    /// Create new blockchain audit manager
    pub fn new(chain_dir: &Path, block_size: usize) -> Result<Self> {
        fs::create_dir_all(chain_dir)?;

        let mut manager = Self {
            chain_dir: chain_dir.to_path_buf(),
            pending_entries: Vec::new(),
            block_size,
            latest_block: None,
            genesis_hash: String::new(),
        };

        // Load latest block
        manager.load_latest_block()?;

        // Create genesis block if chain is empty
        if manager.latest_block.is_none() {
            manager.create_genesis_block()?;
        }

        Ok(manager)
    }

    /// Create genesis block
    fn create_genesis_block(&mut self) -> Result<()> {
        let genesis_entry = BlockEntry {
            audit: AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::VaultCreated,
                description: "Blockchain audit trail initialized".into(),
                model_name: None,
                version: None,
                success: true,
                metadata: None,
            },
            hash: sha256(b"genesis_entry"),
            index_in_block: 0,
        };

        // Compute Merkle root from entries
        let entry_data: Vec<Vec<u8>> = vec![serde_json::to_vec(&genesis_entry).unwrap_or_default()];
        let tree = MerkleTree::build(&entry_data);

        let genesis = AuditBlock {
            index: 0,
            timestamp: Utc::now(),
            prev_hash: "0".repeat(64), // Genesis has no predecessor
            merkle_root: tree.root,
            entries: vec![genesis_entry],
            signature: None,
            nonce: 0,
            hash: String::new(),
        };

        let mut genesis = genesis;
        genesis.hash = genesis.compute_hash();
        self.genesis_hash = genesis.hash.clone();

        self.save_block(&genesis)?;
        self.latest_block = Some(genesis);

        Ok(())
    }

    /// Load the latest block from disk
    fn load_latest_block(&mut self) -> Result<()> {
        let index_path = self.chain_dir.join("latest_index");
        if !index_path.exists() {
            return Ok(());
        }

        let latest_idx: BlockIndex = fs::read_to_string(&index_path)?
            .trim()
            .parse()
            .map_err(|_| VaultError::IoError(std::io::Error::other("Invalid block index")))?;

        let block_path = self.chain_dir.join(format!("block_{:08}.json", latest_idx));
        if block_path.exists() {
            let contents = fs::read_to_string(&block_path)?;
            let block: AuditBlock = serde_json::from_str(&contents)?;
            self.genesis_hash = if block.index == 0 {
                block.hash.clone()
            } else {
                // Load genesis to get its hash
                let genesis_path = self.chain_dir.join("block_00000000.json");
                if genesis_path.exists() {
                    let genesis_contents = fs::read_to_string(&genesis_path)?;
                    let genesis: AuditBlock = serde_json::from_str(&genesis_contents)?;
                    genesis.hash
                } else {
                    String::new()
                }
            };
            self.latest_block = Some(block);
        }

        Ok(())
    }

    /// Save a block to disk
    fn save_block(&self, block: &AuditBlock) -> Result<()> {
        let block_path = self
            .chain_dir
            .join(format!("block_{:08}.json", block.index));
        let json = serde_json::to_string_pretty(block)?;

        // Write with restrictive permissions
        {
            use std::io::Write;
            let mut opts = fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            crate::permissions::set_create_mode(&mut opts);
            let mut f = opts.open(&block_path)?;
            f.write_all(json.as_bytes())?;
        }
        crate::permissions::restrict_file(&block_path)?;

        // Update latest index
        let index_path = self.chain_dir.join("latest_index");
        {
            use std::io::Write;
            let mut opts = fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            crate::permissions::set_create_mode(&mut opts);
            let mut f = opts.open(&index_path)?;
            f.write_all(block.index.to_string().as_bytes())?;
        }
        crate::permissions::restrict_file(&index_path)?;

        Ok(())
    }

    /// Add an entry to the pending block
    pub fn add_entry(&mut self, entry: AuditEntry) -> Result<Hash> {
        let entry_json = serde_json::to_vec(&entry)?;
        let entry_hash = sha256(&entry_json);

        let block_entry = BlockEntry {
            audit: entry,
            hash: entry_hash.clone(),
            index_in_block: self.pending_entries.len(),
        };

        self.pending_entries.push(block_entry);

        // Create new block if threshold reached
        if self.pending_entries.len() >= self.block_size {
            self.finalize_block()?;
        }

        Ok(entry_hash)
    }

    /// Finalize current pending entries into a new block
    pub fn finalize_block(&mut self) -> Result<Option<BlockIndex>> {
        if self.pending_entries.is_empty() {
            return Ok(None);
        }

        let prev = self
            .latest_block
            .as_ref()
            .ok_or_else(|| VaultError::IoError(std::io::Error::other("No previous block")))?;

        // Build Merkle tree
        let entry_data: Vec<Vec<u8>> = self
            .pending_entries
            .iter()
            .map(|e| serde_json::to_vec(e).unwrap_or_default())
            .collect();
        let tree = MerkleTree::build(&entry_data);

        let mut block = AuditBlock {
            index: prev.index + 1,
            timestamp: Utc::now(),
            prev_hash: prev.hash.clone(),
            merkle_root: tree.root,
            entries: std::mem::take(&mut self.pending_entries),
            signature: None,
            nonce: 0,
            hash: String::new(),
        };

        block.hash = block.compute_hash();

        self.save_block(&block)?;
        let idx = block.index;
        self.latest_block = Some(block);

        Ok(Some(idx))
    }

    /// Get a specific block
    pub fn get_block(&self, index: BlockIndex) -> Result<Option<AuditBlock>> {
        let block_path = self.chain_dir.join(format!("block_{:08}.json", index));
        if !block_path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&block_path)?;
        let block: AuditBlock = serde_json::from_str(&contents)?;
        Ok(Some(block))
    }

    /// Get the latest block
    pub fn latest(&self) -> Option<&AuditBlock> {
        self.latest_block.as_ref()
    }

    /// Get chain height (number of blocks)
    pub fn height(&self) -> BlockIndex {
        self.latest_block.as_ref().map(|b| b.index + 1).unwrap_or(0)
    }

    /// Entries accepted but not yet written into a block.
    ///
    /// These live in memory only: they are not on disk and not covered by
    /// [`Self::verify_chain`] until [`Self::finalize_block`] runs. A non-zero
    /// count is exactly the amount of audit evidence a crash would lose.
    pub fn pending_count(&self) -> usize {
        self.pending_entries.len()
    }

    /// Entries per block for this chain.
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Verify entire chain integrity
    pub fn verify_chain(&self) -> ChainVerification {
        let mut result = ChainVerification {
            valid: true,
            blocks_verified: 0,
            blocks_total: self.height(),
            issues: Vec::new(),
        };

        if result.blocks_total == 0 {
            return result;
        }

        let mut prev_block: Option<AuditBlock> = None;

        for idx in 0..result.blocks_total {
            match self.get_block(idx) {
                Ok(Some(block)) => {
                    let verification = block.verify(prev_block.as_ref());
                    if !verification.valid {
                        result.valid = false;
                        for issue in verification.issues {
                            result.issues.push(format!("Block {}: {}", idx, issue));
                        }
                    }
                    result.blocks_verified += 1;
                    prev_block = Some(block);
                }
                Ok(None) => {
                    result.valid = false;
                    result.issues.push(format!("Block {} missing", idx));
                    break;
                }
                Err(e) => {
                    result.valid = false;
                    result.issues.push(format!("Block {} error: {}", idx, e));
                    break;
                }
            }
        }

        result
    }

    /// Generate proof for an entry
    pub fn generate_proof(&self, block_idx: BlockIndex, entry_idx: usize) -> Result<AuditProof> {
        let block = self
            .get_block(block_idx)?
            .ok_or_else(|| VaultError::IoError(std::io::Error::other("Block not found")))?;

        if entry_idx >= block.entries.len() {
            return Err(VaultError::IoError(std::io::Error::other(
                "Entry not found",
            )));
        }

        // Build Merkle tree and generate proof
        let entry_data: Vec<Vec<u8>> = block
            .entries
            .iter()
            .map(|e| serde_json::to_vec(e).unwrap_or_default())
            .collect();
        let tree = MerkleTree::build(&entry_data);

        let merkle_proof = tree.generate_proof(entry_idx).ok_or_else(|| {
            VaultError::IoError(std::io::Error::other("Failed to generate Merkle proof"))
        })?;

        // Build chain of block hashes to genesis
        let mut block_chain = Vec::new();
        let mut current_idx = block_idx;
        while current_idx > 0 {
            if let Some(b) = self.get_block(current_idx)? {
                block_chain.push(BlockHashLink {
                    index: b.index,
                    hash: b.hash,
                    prev_hash: b.prev_hash,
                });
            }
            current_idx -= 1;
        }
        // Add genesis
        if let Some(genesis) = self.get_block(0)? {
            block_chain.push(BlockHashLink {
                index: 0,
                hash: genesis.hash,
                prev_hash: genesis.prev_hash,
            });
        }

        Ok(AuditProof {
            entry: block.entries[entry_idx].clone(),
            block_index: block_idx,
            merkle_proof,
            block_chain,
            genesis_hash: self.genesis_hash.clone(),
        })
    }

    /// Verify a proof
    pub fn verify_proof(proof: &AuditProof) -> ProofVerification {
        let mut issues = Vec::new();

        // Bind the claimed entry to the leaf the Merkle path starts from.
        //
        // `MerkleTree::verify_proof` walks from `merkle_proof.leaf_hash` to the
        // root, but that leaf hash travels *inside* the proof. Without this
        // check the entry is unbound: swapping `proof.entry` for arbitrary
        // content leaves an otherwise-valid proof, so a tampered audit record
        // verifies clean. Recompute the leaf the same way `generate_proof`
        // built it -- sha256 of the serialized `BlockEntry`.
        match serde_json::to_vec(&proof.entry) {
            Ok(entry_json) => {
                let recomputed = sha256(&entry_json);
                if recomputed != proof.merkle_proof.leaf_hash {
                    issues.push(
                        "Entry does not match the proof's leaf hash (entry was altered)".into(),
                    );
                }
            }
            Err(e) => issues.push(format!("Entry could not be serialized for hashing: {e}")),
        }

        // Verify Merkle proof
        if !MerkleTree::verify_proof(&proof.merkle_proof) {
            issues.push("Merkle proof invalid".into());
        }

        // Verify block chain to genesis.
        //
        // `windows(2)` rather than `0..len() - 1`: this function parses a proof
        // file supplied by whoever runs `iv chain verify-proof`, and an empty
        // `block_chain` made that subtraction underflow and panic on untrusted
        // input.
        if proof.block_chain.is_empty() {
            issues.push("Proof carries no block chain".into());
        }
        for pair in proof.block_chain.windows(2) {
            let (current, prev) = (&pair[0], &pair[1]);
            if current.prev_hash != prev.hash {
                issues.push(format!("Block chain broken at index {}", current.index));
            }
        }

        // Verify ends at genesis
        if let Some(last) = proof.block_chain.last() {
            if last.index != 0 || last.hash != proof.genesis_hash {
                issues.push("Chain doesn't end at genesis".into());
            }
        }

        ProofVerification {
            valid: issues.is_empty(),
            issues,
        }
    }

    /// Search entries
    pub fn search(
        &self,
        model_name: Option<&str>,
        event_type: Option<AuditEventType>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Vec<(BlockIndex, usize, BlockEntry)>> {
        let mut results = Vec::new();
        let mut idx = self.height().saturating_sub(1);

        loop {
            if let Some(block) = self.get_block(idx)? {
                // Check time bounds
                if let Some(from_ts) = from {
                    if block.timestamp < from_ts {
                        break;
                    }
                }
                if let Some(to_ts) = to {
                    if block.timestamp > to_ts {
                        if idx == 0 {
                            break;
                        }
                        idx -= 1;
                        continue;
                    }
                }

                for (entry_idx, entry) in block.entries.iter().enumerate() {
                    let matches = model_name
                        .map(|m| entry.audit.model_name.as_deref() == Some(m))
                        .unwrap_or(true)
                        && event_type
                            .as_ref()
                            .map(|t| {
                                std::mem::discriminant(&entry.audit.event_type)
                                    == std::mem::discriminant(t)
                            })
                            .unwrap_or(true);

                    if matches {
                        results.push((idx, entry_idx, entry.clone()));
                        if results.len() >= limit {
                            return Ok(results);
                        }
                    }
                }
            }

            if idx == 0 {
                break;
            }
            idx -= 1;
        }

        Ok(results)
    }
}

/// Complete audit proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditProof {
    /// The entry being proven
    pub entry: BlockEntry,
    /// Block index containing the entry
    pub block_index: BlockIndex,
    /// Merkle proof within the block
    pub merkle_proof: MerkleProof,
    /// Chain of block hashes from entry's block to genesis
    pub block_chain: Vec<BlockHashLink>,
    /// Genesis block hash
    pub genesis_hash: Hash,
}

/// Link in the block hash chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHashLink {
    /// Block index
    pub index: BlockIndex,
    /// Block hash
    pub hash: Hash,
    /// Previous block hash
    pub prev_hash: Hash,
}

/// Chain verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    /// Whether the entire chain is valid
    pub valid: bool,
    /// Number of blocks verified
    pub blocks_verified: u64,
    /// Total blocks in chain
    pub blocks_total: u64,
    /// Issues found
    pub issues: Vec<String>,
}

/// Proof verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofVerification {
    /// Whether the proof is valid
    pub valid: bool,
    /// Issues found
    pub issues: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hash() {
        let hash = sha256(b"hello");
        assert_eq!(hash.len(), 64); // 256 bits = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_merkle_tree_single() {
        let data = vec![b"hello".to_vec()];
        let tree = MerkleTree::build(&data);

        assert_eq!(tree.leaves.len(), 1);
        assert!(!tree.root.is_empty());
    }

    #[test]
    fn test_merkle_tree_multiple() {
        let data = vec![
            b"entry1".to_vec(),
            b"entry2".to_vec(),
            b"entry3".to_vec(),
            b"entry4".to_vec(),
        ];
        let tree = MerkleTree::build(&data);

        assert_eq!(tree.leaves.len(), 4);

        // Generate and verify proofs
        for i in 0..4 {
            let proof = tree.generate_proof(i).unwrap();
            assert!(MerkleTree::verify_proof(&proof));
        }
    }

    #[test]
    fn test_merkle_proof_verification() {
        let data = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()];
        let tree = MerkleTree::build(&data);

        let proof = tree.generate_proof(1).unwrap();
        assert!(MerkleTree::verify_proof(&proof));

        // Tamper with proof
        let mut tampered = proof.clone();
        tampered.leaf_hash = sha256(b"tampered");
        assert!(!MerkleTree::verify_proof(&tampered));
    }

    #[test]
    fn test_block_verification() {
        let block = AuditBlock {
            index: 1,
            timestamp: Utc::now(),
            prev_hash: "0".repeat(64),
            merkle_root: sha256(b"root"),
            entries: vec![],
            signature: None,
            nonce: 0,
            hash: String::new(),
        };

        let mut block = block;
        block.hash = block.compute_hash();

        // Valid block (as genesis-like)
        let result = block.verify(None);
        // Will fail because index != 0 and no predecessor
        assert!(!result.valid);
    }

    #[test]
    fn test_blockchain_audit_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        assert!(audit.latest().is_some());
        assert_eq!(audit.height(), 1); // Genesis block
    }

    #[test]
    fn test_blockchain_add_entry() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 2).unwrap();

        let entry = AuditEntry {
            timestamp: Utc::now(),
            event_type: AuditEventType::ModelStored,
            description: "Test entry".into(),
            model_name: Some("test_model".into()),
            version: Some(1),
            success: true,
            metadata: None,
        };

        let hash = audit.add_entry(entry).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_blockchain_verify_chain() {
        let temp_dir = tempfile::tempdir().unwrap();
        let audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        let result = audit.verify_chain();
        assert!(result.valid);
        assert_eq!(result.blocks_verified, 1);
    }

    #[test]
    fn test_blockchain_reopen_existing_chain() {
        // Covers lines 351, 392, 401, 408, 412 — load_latest_block on reopen
        let temp_dir = tempfile::tempdir().unwrap();
        {
            let mut audit = BlockchainAudit::new(temp_dir.path(), 2).unwrap();
            let entry = AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "First entry".into(),
                model_name: Some("model_a".into()),
                version: Some(1),
                success: true,
                metadata: None,
            };
            audit.add_entry(entry).unwrap();
            // Force a new block by adding more entries
            for i in 0..3 {
                let e = AuditEntry {
                    timestamp: Utc::now(),
                    event_type: AuditEventType::ModelRetrieved,
                    description: format!("Entry {}", i),
                    model_name: Some("model_a".into()),
                    version: Some(1),
                    success: true,
                    metadata: None,
                };
                audit.add_entry(e).unwrap();
            }
        }
        // Reopen — this triggers load_latest_block path
        let audit2 = BlockchainAudit::new(temp_dir.path(), 2).unwrap();
        assert!(audit2.height() >= 2);
        let verification = audit2.verify_chain();
        assert!(verification.valid);
    }

    #[test]
    fn test_blockchain_generate_and_verify_proof() {
        // Covers lines 533, 543-545, 552-558 — generate_proof
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        let entry = AuditEntry {
            timestamp: Utc::now(),
            event_type: AuditEventType::ModelStored,
            description: "Provable entry".into(),
            model_name: Some("prove_me".into()),
            version: Some(1),
            success: true,
            metadata: None,
        };
        audit.add_entry(entry).unwrap();

        // Genesis is block 0 with 1 entry
        let proof = audit.generate_proof(0, 0).unwrap();
        assert_eq!(proof.block_index, 0);
        assert!(!proof.genesis_hash.is_empty());

        // Verify the proof
        let verification = BlockchainAudit::verify_proof(&proof);
        assert!(verification.valid);
    }

    /// A proof must bind its entry, not just carry one.
    ///
    /// Regression: `verify_proof` walked the Merkle path from the proof's own
    /// `leaf_hash` without checking that hash came from `proof.entry`, so
    /// rewriting the entry left a proof that still verified clean -- a tamper
    /// check that accepted tampering.
    #[test]
    fn test_verify_proof_rejects_an_altered_entry() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 1).unwrap();

        audit
            .add_entry(AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "Model 'demo' version 1 stored".into(),
                model_name: Some("demo".into()),
                version: Some(1),
                success: true,
                metadata: None,
            })
            .unwrap();

        let proof = audit.generate_proof(1, 0).unwrap();
        assert!(
            BlockchainAudit::verify_proof(&proof).valid,
            "untouched proof should verify"
        );

        // Rewrite history: same shape, different claim.
        let mut forged = proof.clone();
        forged.entry.audit.description = "Model 'demo' version 9 stored".into();
        forged.entry.audit.version = Some(9);

        let verification = BlockchainAudit::verify_proof(&forged);
        assert!(
            !verification.valid,
            "an altered entry must not verify: {:?}",
            verification.issues
        );

        // Flipping `success` is the subtler forgery -- it turns a failed
        // operation into a successful one without changing any text.
        let mut flipped = proof.clone();
        flipped.entry.audit.success = !proof.entry.audit.success;
        assert!(
            !BlockchainAudit::verify_proof(&flipped).valid,
            "flipping the success flag must not verify"
        );
    }

    /// `verify_proof` parses attacker-supplied JSON; it must not panic on it.
    ///
    /// Regression: the block-chain walk used `0..len() - 1`, which underflows
    /// on an empty `block_chain` and panicked instead of reporting invalid.
    #[test]
    fn test_verify_proof_rejects_empty_block_chain_without_panicking() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 1).unwrap();

        audit
            .add_entry(AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "Entry".into(),
                model_name: Some("m".into()),
                version: Some(1),
                success: true,
                metadata: None,
            })
            .unwrap();

        let mut proof = audit.generate_proof(1, 0).unwrap();
        proof.block_chain.clear();

        let verification = BlockchainAudit::verify_proof(&proof);
        assert!(!verification.valid);
        assert!(
            verification
                .issues
                .iter()
                .any(|i| i.contains("no block chain")),
            "expected an explicit complaint, got {:?}",
            verification.issues
        );
    }

    #[test]
    fn test_blockchain_generate_proof_multi_block() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 2).unwrap();

        // Add enough entries to create a second block
        for i in 0..3 {
            let e = AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: format!("Entry {}", i),
                model_name: Some("test".into()),
                version: Some(i as u32),
                success: true,
                metadata: None,
            };
            audit.add_entry(e).unwrap();
        }

        // Generate proof for a block > 0 (covers block_chain traversal back to genesis)
        if audit.height() > 1 {
            let proof = audit.generate_proof(1, 0).unwrap();
            assert!(proof.block_chain.len() >= 2); // Should include block 1 and genesis
            let v = BlockchainAudit::verify_proof(&proof);
            assert!(v.valid);
        }
    }

    #[test]
    fn test_blockchain_search_by_model_name() {
        // Covers line 637+ — search results
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        let entry = AuditEntry {
            timestamp: Utc::now(),
            event_type: AuditEventType::ModelStored,
            description: "Searchable entry".into(),
            model_name: Some("searchable_model".into()),
            version: Some(1),
            success: true,
            metadata: None,
        };
        audit.add_entry(entry).unwrap();
        audit.finalize_block().unwrap();

        let results = audit
            .search(Some("searchable_model"), None, None, None, 10)
            .unwrap();
        assert!(!results.is_empty());
        assert_eq!(
            results[0].2.audit.model_name.as_deref(),
            Some("searchable_model")
        );
    }

    #[test]
    fn test_blockchain_search_by_event_type() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        let entry = AuditEntry {
            timestamp: Utc::now(),
            event_type: AuditEventType::ModelDeleted,
            description: "Deleted model".into(),
            model_name: Some("del_model".into()),
            version: Some(1),
            success: true,
            metadata: None,
        };
        audit.add_entry(entry).unwrap();
        audit.finalize_block().unwrap();

        let results = audit
            .search(None, Some(AuditEventType::ModelDeleted), None, None, 10)
            .unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_blockchain_search_with_limit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 100).unwrap();

        for _ in 0..5 {
            let e = AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "entry".into(),
                model_name: Some("m".into()),
                version: Some(1),
                success: true,
                metadata: None,
            };
            audit.add_entry(e).unwrap();
        }
        audit.finalize_block().unwrap();

        let results = audit.search(None, None, None, None, 2).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_verify_chain_with_missing_block() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 1).unwrap();

        // Add entries and finalize multiple blocks
        for i in 0..3 {
            let entry = AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: format!("Entry {}", i),
                model_name: Some("m".into()),
                version: Some(1),
                success: true,
                metadata: None,
            };
            audit.add_entry(entry).unwrap();
            audit.finalize_block().unwrap();
        }

        // Verify chain is valid before corruption
        let result = audit.verify_chain();
        assert!(result.valid);
        assert!(result.blocks_total >= 3);

        // Delete a block file to simulate corruption
        let block_path = temp_dir.path().join("block_00000001.json");
        if block_path.exists() {
            std::fs::remove_file(&block_path).unwrap();
        }

        // Verify chain should detect the missing block
        let result = audit.verify_chain();
        assert!(!result.valid);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_verify_chain_with_tampered_block() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 1).unwrap();

        // Create 2 real blocks
        for _ in 0..2 {
            let entry = AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "entry".into(),
                model_name: Some("m".into()),
                version: Some(1),
                success: true,
                metadata: None,
            };
            audit.add_entry(entry).unwrap();
            audit.finalize_block().unwrap();
        }

        // Tamper with block 1's hash
        let block_path = temp_dir.path().join("block_00000001.json");
        if block_path.exists() {
            let contents = std::fs::read_to_string(&block_path).unwrap();
            let mut block: serde_json::Value = serde_json::from_str(&contents).unwrap();
            block["hash"] = serde_json::json!("tampered_hash_value");
            std::fs::write(&block_path, serde_json::to_string_pretty(&block).unwrap()).unwrap();
        }

        let result = audit.verify_chain();
        assert!(!result.valid);
    }

    #[test]
    fn test_blockchain_reload_and_verify() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create blockchain and add data
        {
            let mut audit = BlockchainAudit::new(temp_dir.path(), 2).unwrap();
            for _ in 0..4 {
                let entry = AuditEntry {
                    timestamp: Utc::now(),
                    event_type: AuditEventType::ModelStored,
                    description: "test entry".into(),
                    model_name: Some("reload_model".into()),
                    version: Some(1),
                    success: true,
                    metadata: None,
                };
                audit.add_entry(entry).unwrap();
            }
            audit.finalize_block().unwrap();
        }

        // Reopen and verify — exercises load_latest_block
        let audit2 = BlockchainAudit::new(temp_dir.path(), 2).unwrap();
        let result = audit2.verify_chain();
        assert!(result.valid);
        assert!(audit2.height() > 0);
    }

    #[test]
    fn test_merkle_tree_generate_proof_out_of_range() {
        // Covers L139 — index >= leaves.len()
        let data = vec![b"a".to_vec(), b"b".to_vec()];
        let tree = MerkleTree::build(&data);
        assert!(tree.generate_proof(10).is_none());
    }

    #[test]
    fn test_verify_chain_fresh() {
        // Covers L533 — verify_chain on fresh chain (genesis only)
        let temp_dir = tempfile::tempdir().unwrap();
        let audit = BlockchainAudit::new(temp_dir.path(), 5).unwrap();
        let result = audit.verify_chain();
        assert!(result.valid);
        assert_eq!(result.blocks_total, 1); // genesis block
        assert_eq!(result.blocks_verified, 1);
    }

    #[test]
    fn test_search_with_event_type_filter() {
        // Covers L670, L675-676, L679 — search with event_type filter
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        audit
            .add_entry(AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "stored".into(),
                model_name: Some("m1".into()),
                version: Some(1),
                success: true,
                metadata: None,
            })
            .unwrap();
        audit
            .add_entry(AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelRetrieved,
                description: "retrieved".into(),
                model_name: Some("m1".into()),
                version: Some(1),
                success: true,
                metadata: None,
            })
            .unwrap();
        audit.finalize_block().unwrap();

        // Search by event type
        let results = audit
            .search(None, Some(AuditEventType::ModelStored), None, None, 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].2.audit.description, "stored");

        // Search by model name
        let results = audit.search(Some("m1"), None, None, None, 10).unwrap();
        assert_eq!(results.len(), 2);

        // Search with limit
        let results = audit.search(None, None, None, None, 1).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_with_time_bounds() {
        // Covers L644 — search with from/to time bounds
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        audit
            .add_entry(AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "entry1".into(),
                model_name: Some("m1".into()),
                version: Some(1),
                success: true,
                metadata: None,
            })
            .unwrap();
        audit.finalize_block().unwrap();

        // Far future from — all blocks are before it, so search breaks immediately
        let far_future = Utc::now() + chrono::Duration::days(365);
        let results = audit
            .search(None, None, Some(far_future), None, 10)
            .unwrap();
        assert!(
            results.is_empty(),
            "Far future from should return no results"
        );

        // Far past to — all blocks are after it, so they are all skipped
        let far_past = Utc::now() - chrono::Duration::days(365);
        let results = audit.search(None, None, None, Some(far_past), 10).unwrap();
        assert!(results.is_empty(), "Far past to should return no results");

        // Wide range — should find results
        let results = audit
            .search(None, None, Some(far_past), Some(far_future), 10)
            .unwrap();
        assert!(!results.is_empty(), "Wide range should return results");
    }
    #[test]
    fn test_generate_and_verify_proof() {
        // Covers L574, L588, L628, L637, L644 — proof generation + verification
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        audit
            .add_entry(AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "test_proof".into(),
                model_name: Some("m1".into()),
                version: Some(1),
                success: true,
                metadata: None,
            })
            .unwrap();
        audit.finalize_block().unwrap();

        // Generate valid proof
        let proof = audit.generate_proof(0, 0).unwrap();
        let verification = BlockchainAudit::verify_proof(&proof);
        assert!(
            verification.valid,
            "Valid proof should verify: {:?}",
            verification.issues
        );

        // Generate proof for invalid entry index
        let result = audit.generate_proof(0, 99);
        assert!(result.is_err());

        // Generate proof for invalid block index
        let result = audit.generate_proof(99, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_proof_chain_broken() {
        // Covers L628 — "Block chain broken at index" in verify_proof
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 2).unwrap();

        // Add entries to create block 1
        for i in 0..3 {
            audit
                .add_entry(AuditEntry {
                    timestamp: Utc::now(),
                    event_type: AuditEventType::ModelStored,
                    description: format!("Entry {}", i),
                    model_name: Some("broken_chain".into()),
                    version: Some(i as u32),
                    success: true,
                    metadata: None,
                })
                .unwrap();
        }

        // Generate a valid proof from block 1
        let mut proof = audit.generate_proof(1, 0).unwrap();
        assert!(proof.block_chain.len() >= 2);

        // Tamper with the chain: corrupt prev_hash of first link
        proof.block_chain[0].prev_hash = "tampered_hash".to_string();

        let verification = BlockchainAudit::verify_proof(&proof);
        assert!(!verification.valid);
        assert!(verification
            .issues
            .iter()
            .any(|i| i.contains("chain broken")));
    }

    #[test]
    fn test_verify_proof_genesis_mismatch() {
        // Covers L637, L644 — genesis hash check in verify_proof
        let temp_dir = tempfile::tempdir().unwrap();
        let mut audit = BlockchainAudit::new(temp_dir.path(), 10).unwrap();

        audit
            .add_entry(AuditEntry {
                timestamp: Utc::now(),
                event_type: AuditEventType::ModelStored,
                description: "genesis test".into(),
                model_name: Some("gen_test".into()),
                version: Some(1),
                success: true,
                metadata: None,
            })
            .unwrap();
        audit.finalize_block().unwrap();

        let mut proof = audit.generate_proof(0, 0).unwrap();

        // Tamper genesis hash so it doesn't match the last block's hash
        proof.genesis_hash = "wrong_genesis_hash".to_string();

        let verification = BlockchainAudit::verify_proof(&proof);
        assert!(!verification.valid);
        assert!(verification.issues.iter().any(|i| i.contains("genesis")));
    }

    #[test]
    fn test_audit_block_verify_timestamp_and_index() {
        // Covers L265 (timestamp before previous) and L268 (non-sequential index)
        use chrono::Duration;

        // Create two blocks
        let now = Utc::now();
        let prev_block = AuditBlock {
            index: 0,
            timestamp: now,
            entries: vec![],
            prev_hash: String::new(),
            hash: "prev_hash_123".to_string(),
            merkle_root: String::new(),
            signature: None,
            nonce: 0,
        };

        // Block with timestamp BEFORE previous
        let mut bad_time_block = AuditBlock {
            index: 1,
            timestamp: now - Duration::hours(1),
            entries: vec![],
            prev_hash: "prev_hash_123".to_string(),
            hash: String::new(),
            merkle_root: String::new(),
            signature: None,
            nonce: 0,
        };
        bad_time_block.hash = bad_time_block.compute_hash();

        let verification = bad_time_block.verify(Some(&prev_block));
        assert!(!verification.valid);
        assert!(verification.issues.iter().any(|i| i.contains("Timestamp")));

        // Block with non-sequential index
        let mut bad_idx_block = AuditBlock {
            index: 5, // Should be 1
            timestamp: now + Duration::hours(1),
            entries: vec![],
            prev_hash: "prev_hash_123".to_string(),
            hash: String::new(),
            merkle_root: String::new(),
            signature: None,
            nonce: 0,
        };
        bad_idx_block.hash = bad_idx_block.compute_hash();

        let verification = bad_idx_block.verify(Some(&prev_block));
        assert!(!verification.valid);
        assert!(verification
            .issues
            .iter()
            .any(|i| i.contains("Non-sequential")));
    }

    #[test]
    fn test_blockchain_load_chain_multi_block() {
        // Covers L408, L417 — load_chain path for non-genesis latest block
        let temp_dir = tempfile::tempdir().unwrap();

        {
            // Create a chain with multiple blocks, then drop it
            let mut audit = BlockchainAudit::new(temp_dir.path(), 2).unwrap();
            for i in 0..5 {
                audit
                    .add_entry(AuditEntry {
                        timestamp: Utc::now(),
                        event_type: AuditEventType::ModelStored,
                        description: format!("Persist entry {}", i),
                        model_name: Some("persist_model".into()),
                        version: Some(i as u32),
                        success: true,
                        metadata: None,
                    })
                    .unwrap();
            }
            // Finalize to ensure blocks are written
            audit.finalize_block().unwrap();
            assert!(audit.height() > 1, "Should have multiple blocks");
        }

        // Re-open the chain — this triggers load_chain with latest_block > 0
        // which loads genesis hash from block_00000000.json (L408, L417)
        let audit2 = BlockchainAudit::new(temp_dir.path(), 2).unwrap();
        assert!(audit2.height() > 1);

        // Verify the reloaded chain
        let result = audit2.verify_chain();
        assert!(
            result.valid,
            "Reloaded chain should be valid: {:?}",
            result.issues
        );
    }
}
