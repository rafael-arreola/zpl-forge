//! # ZPL-Forge
//!
//! `zpl-forge` is a high-performance engine for parsing, processing, and rendering Zebra Programming Language (ZPL) labels.
//! It transforms raw ZPL strings into an optimized Intermediate Representation (IR), which can then be exported to various
//! formats like PNG images or PDF documents.
//!
//! ## Core Features
//! - **Robust Parsing**: Uses `nom` for fast and safe ZPL command parsing.
//! - **State Machine**: Maintains label state (fonts, positions, barcodes) across commands.
//! - **Multiple Backends**: Native support for PNG (via `imageproc`) and PDF (via `printpdf`).
//! - **Extensible**: Includes custom commands for color support and Base64 image loading.
//! - **Security**: Built-in OOM protection and safe arithmetic for coordinate calculations.
//!
//! ## Quick Start
//!
//! Rendering a simple label to a PNG image:
//!
//! ```rust
//! use zpl_forge::{ZplEngine, Unit, Resolution};
//! use zpl_forge::forge::png::PngBackend;
//! use std::collections::HashMap;
//!
//! # fn main() -> zpl_forge::ZplResult<()> {
//! let zpl_input = "^XA^FO50,50^A0N,50,50^FDZPL Forge^FS^XZ";
//!
//! // Create the engine by parsing ZPL data
//! let engine = ZplEngine::new(
//!     zpl_input,
//!     Unit::Inches(4.0),
//!     Unit::Inches(2.0),
//!     Resolution::Dpi203
//! )?;
//!
//! // Render using the PNG backend
//! let backend = PngBackend::new();
//! let png_bytes = engine.render(backend, &HashMap::new())?;
//!
//! // png_bytes now contains the raw PNG data
//! # Ok(())
//! # }
//! ```
//!
//! ## Security and Limits
//!
//! To ensure stability and prevent Denial of Service (DoS) attacks via malformed input, `zpl-forge` implements the following restrictions:
//! - **Canvas Size**: Rendering is limited to a maximum of **8192 x 8192 pixels**.
//! - **Image Data**: Decoded bitmap data (`^GF`) cannot exceed **10 MB** per command.
//! - **Safe Calculations**: Saturating arithmetic is used for all coordinate and dimension calculations to prevent integer overflows.
//! - **Unit Normalization**: Input values for physical dimensions are validated to prevent negative sizes.

mod ast;
mod engine;
pub mod error;
pub mod forge;
pub mod tools;

pub use engine::*;
pub use error::{ZplError, ZplResult};

pub(crate) const TARGET: &str = "zpl-forge";
