//! Rollback data model.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Rollback file structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rollback {
    /// Rollback version.
    pub version: String,
    /// Reference to the original plan ID.
    pub plan_id: String,
    /// Execution timestamp.
    pub executed_at: String,
    /// Operations performed (in execution order).
    pub operations: Vec<RollbackOperation>,
}

/// A single rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackOperation {
    /// Sequence number.
    pub seq: u32,
    /// Operation type that was performed.
    pub op_type: RollbackOpType,
    /// Source path (original location).
    pub from: PathBuf,
    /// Destination path (new location).
    pub to: PathBuf,
    /// File checksum (for verification).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    /// Rollback operation to undo this.
    pub rollback: RollbackAction,
    /// Whether this operation was executed.
    pub executed: bool,
}

/// Operation type for rollback tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollbackOpType {
    Mkdir,
    Move,
    Create,
    Download,
}

/// Action to undo an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackAction {
    /// Rollback operation type.
    pub op: RollbackActionType,
    /// Path for the rollback action.
    pub path: PathBuf,
    /// Additional path (for move operations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<PathBuf>,
}

/// Rollback action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollbackActionType {
    /// Remove a directory.
    Rmdir,
    /// Move file back.
    Move,
    /// Delete a created file.
    Delete,
}
