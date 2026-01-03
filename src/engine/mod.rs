//! # Engine Layer
//!
//! The engine processes the linear sequence of ZPL commands into a set of
//! self-contained, renderable instructions. It manages the state machine
//! that tracks label configurations like origin, font settings, and barcode
//! defaults.
//!
//! ## Core Workflow
//! 1. Receives a stream of `Command` enums from the AST parser.
//! 2. Updates the internal state as modal commands are processed.
//! 3. Emits a `ZplInstruction` when a field separator (`^FS`) is encountered.
//! 4. Produces a final `Vec<ZplInstruction>` ready for the rendering backends.

mod backend;
mod common;
#[allow(clippy::module_inception)]
mod engine;
mod font;
mod intr;
mod state;

pub use backend::ZplForgeBackend;
pub use common::{Resolution, Unit, ZplInstruction};
pub use engine::ZplEngine;
pub use font::FontManager;
