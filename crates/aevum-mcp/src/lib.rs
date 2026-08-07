//! `aevum-mcp` — Model Context Protocol server for the Aevum trust path.

pub mod protocol;
pub mod silence;
pub mod tools;

pub use protocol::serve_stdio;
pub use tools::{list_tools_value, ToolCtx};
