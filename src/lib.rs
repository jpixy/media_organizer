//! Media Organizer Library
//!
//! A library for organizing video files (movies and TV shows) using AI and TMDB.

pub mod cli;
pub mod core;
pub mod error;
pub mod generators;
pub mod models;
pub mod preflight;
pub mod services;
pub mod utils;

pub use error::{Error, Result};
