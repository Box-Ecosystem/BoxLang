//! Integration modules for BoxLang
//!
//! This module provides integrations with external tools and formats,
//! including AppBox packaging format.

pub mod appbox;

pub use appbox::{AppBoxBuilder, AppBoxManifest, convert_box_toml_to_appbox, generate_default_main};
