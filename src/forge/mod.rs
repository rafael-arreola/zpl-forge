//! # Forge Layer
//!
//! The forge layer provides concrete implementations of rendering backends.
//! It translates the intermediate representation (`ZplInstruction`) into
//! specific output formats like images or documents.

pub mod pdf;
pub mod pdf_native;
pub mod png;
