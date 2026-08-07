# ADR-0014 — P0 Memory Fabric, MCP choke-point, AgentTrustBench

**Status:** adopted  
**Date:** 2026-08-07  
**Authority:** Winter Fernandes / agent session  

## Context

To become #1 in Trusted Autonomy (not agent memory alone), Aevum must:
1. support recall without letting recall authorize,
2. expose a real MCP choke-point,
3. publish an adversarial scoreboard for trust gates.

## Decision

- `crates/memory-fabric`: `MemoryBackend` trait; `NativeBackend` / later `SqliteBackend`;
  remote/untrusted facts enter only as `Inference`.
- Epistemic **promotion protocol**: remote facts ingest as `Inference`; only
  attested promote creates `Authorizes`.
- `assemble()`: retrieval ∩ epistemic weight ∩ capability binding.
- `crates/aevum-mcp`: JSON-RPC MCP 2024-11-05 over stdio (tools that hit real
  unify/memory-fabric paths; stdout silenced during CLI helpers).
- `crates/agent-trust-bench`: AgentTrustBench v0 — real adversarial cases.
- CI runs AgentTrustBench + `scripts/aevum-on-aevum.sh` dogfood.

## Consequences

- Memory plane never is the trust plane.
- No mock remote recall responses.
- MCP is the distribution surface for Cursor/Claude.

## References

- Strategy canvas path-to-number-one; ADR-0010; ADR-0013
