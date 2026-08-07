//! aevum-mcp — stdio MCP server bound to a mission directory.
//!
//! Usage:
//!   aevum-mcp --mission ./mission
//!
//! Cursor / Claude config:
//!   { "command": "aevum-mcp", "args": ["--mission", "/path/to/mission"] }

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();
    let mission = match parse_mission(&argv) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("aevum-mcp: {e}");
            eprintln!("usage: aevum-mcp --mission <dir>");
            return ExitCode::from(2);
        }
    };
    if !mission.join("metadata.json").exists() {
        eprintln!(
            "aevum-mcp: {} is not a mission (missing metadata.json) — run `unify new` first",
            mission.display()
        );
        return ExitCode::from(1);
    }
    let ctx = aevum_mcp::ToolCtx::new(mission);
    if let Err(e) = aevum_mcp::serve_stdio(ctx) {
        eprintln!("aevum-mcp: io error: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn parse_mission(argv: &[String]) -> Result<PathBuf, String> {
    let mut i = 1;
    while i < argv.len() {
        if argv[i] == "--mission" {
            return argv
                .get(i + 1)
                .map(PathBuf::from)
                .ok_or_else(|| "missing --mission path".into());
        }
        if argv[i] == "--help" || argv[i] == "-h" {
            return Err("help".into());
        }
        i += 1;
    }
    Err("missing --mission <dir>".into())
}
