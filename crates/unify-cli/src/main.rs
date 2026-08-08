//! `unify` — Aevum Unify CLI (M10).
//!
//! This binary is a thin wrapper around `aevum_unify::cmd_*`. All the
//! business logic lives in `lib.rs` so it can be integration-tested
//! without spawning a subprocess.

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();
    if argv.len() < 2 {
        aevum_unify::print_help();
        return ExitCode::from(2);
    }
    let cmd = &argv[1];
    let result: Result<(), aevum_unify::CliError> = match cmd.as_str() {
        "new" => aevum_unify::cmd_new(&argv[2..]),
        "run" => aevum_unify::cmd_run(&argv[2..]),
        "verify" => aevum_unify::cmd_verify(&argv[2..]),
        "package" => aevum_unify::cmd_package(&argv[2..]),
        "verify-package" => aevum_unify::cmd_verify_package(&argv[2..]),
        "exec" => aevum_unify::cmd_exec(&argv[2..]),
        "graph" => aevum_unify::graph_cmd::cmd_graph(&argv[2..]),
        "context" => aevum_unify::graph_cmd::cmd_context(&argv[2..]),
        "mcp" => aevum_unify::graph_cmd::cmd_mcp_hint(&argv[2..]),
        "falsify" => aevum_unify::graph_cmd::cmd_falsify(&argv[2..]),
        "approve" => aevum_unify::graph_cmd::cmd_approve(&argv[2..]),
        "golden" => aevum_unify::golden::cmd_golden(&argv[2..]),
        "slop" => aevum_unify::slop::cmd_slop(&argv[2..]),
        "rules" => {
            let rest = &argv[2..];
            if rest.first().map(|s| s.as_str()) == Some("scan") {
                aevum_unify::rules::cmd_rules_scan(&rest[1..])
            } else {
                aevum_unify::rules::cmd_rules_scan(rest)
            }
        }
        "parallel" => {
            if argv.get(2).map(|s| s.as_str()) == Some("worktrees") {
                aevum_unify::parallel::cmd_parallel_worktrees(&argv[3..])
            } else {
                aevum_unify::parallel::cmd_parallel(&argv[2..])
            }
        }
        "--help" | "-h" | "help" => {
            aevum_unify::print_help();
            return ExitCode::SUCCESS;
        }
        "--version" | "-V" => {
            println!("unify v{} (aevum-unify)", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("unknown subcommand: {cmd}");
            aevum_unify::print_help();
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("unify {cmd}: {e}");
            ExitCode::from(1)
        }
    }
}
