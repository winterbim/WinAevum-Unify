//! agent-trust-bench — print JSON scorecard for Aevum trust gates.

use aevum_agent_trust_bench::run_all;
use std::process::ExitCode;

fn main() -> ExitCode {
    let results = run_all();
    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    let report = serde_json::json!({
        "bench": "AgentTrustBench",
        "version": "v0",
        "aevum_passed": passed,
        "aevum_total": total,
        // Not "PERFECT": this bench is self-run in-repo (not third-party verified).
        "verdict": if passed == total {
            "AEVUM_SELF_RUN_PASS"
        } else {
            "AEVUM_FAIL"
        },
        "cases": results,
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    eprintln!("AgentTrustBench: {passed}/{total}");
    if passed == total {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
