use aevum_memory_truth_bench::run_all;
use std::process::ExitCode;

fn main() -> ExitCode {
    let results = run_all();
    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    let report = serde_json::json!({
        "bench": "MemoryTruthBench",
        "version": "v0",
        "aevum_passed": passed,
        "aevum_total": total,
        "verdict": if passed == total { "AEVUM_MEMORY_PERFECT" } else { "AEVUM_MEMORY_FAIL" },
        "offline": true,
        "cases": results,
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    eprintln!("MemoryTruthBench: {passed}/{total} (offline)");
    if passed == total {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
