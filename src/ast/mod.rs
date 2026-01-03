//! # AST (Abstract Syntax Tree)
//!
//! This module contains the logic for parsing raw ZPL strings into a structured
//! sequence of commands. It uses `nom` for efficient and robust parsing.

pub mod cmd;
pub mod commons;
mod parser;

pub use parser::parse_zpl;
