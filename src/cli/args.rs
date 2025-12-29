//! Command line argument definitions.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Media Organizer - Organize your video files with AI
#[derive(Parser, Debug)]
#[command(name = "media-organizer")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Skip preflight checks
    #[arg(long, global = true)]
    pub skip_preflight: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate an organization plan
    Plan {
        #[command(subcommand)]
        media_type: PlanType,
    },

    /// Execute a plan file
    Execute {
        /// Path to the plan.json file
        #[arg(value_name = "PLAN_FILE")]
        plan_file: PathBuf,

        /// Output path for rollback.json
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,
    },

    /// Rollback a previous execution
    Rollback {
        /// Path to the rollback.json file
        #[arg(value_name = "ROLLBACK_FILE")]
        rollback_file: PathBuf,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
    },

    /// Manage sessions
    Sessions {
        #[command(subcommand)]
        action: SessionsAction,
    },

    /// Verify video file integrity
    Verify {
        /// Path to verify
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum PlanType {
    /// Plan for movies
    Movies {
        /// Source directory containing movies
        #[arg(value_name = "SOURCE")]
        source: PathBuf,

        /// Target directory for organized movies
        #[arg(short, long, value_name = "TARGET")]
        target: Option<PathBuf>,

        /// Output path for plan.json
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,
    },

    /// Plan for TV shows
    Tvshows {
        /// Source directory containing TV shows
        #[arg(value_name = "SOURCE")]
        source: PathBuf,

        /// Target directory for organized TV shows
        #[arg(short, long, value_name = "TARGET")]
        target: Option<PathBuf>,

        /// Output path for plan.json
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum SessionsAction {
    /// List all sessions
    List,

    /// Show details of a specific session
    Show {
        /// Session ID
        #[arg(value_name = "SESSION_ID")]
        session_id: String,
    },
}



