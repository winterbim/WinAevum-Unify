# ADR-0010 — MCP for tools, A2A for agents

- **Status:** adopted (binding of blueprint ADR-0011)
- **Date:** 2026-08-02
- **Authority:** Winter Fernandes
- **Source:** AEVUM_UNIFY_MASTER_BLUEPRINT_V1.0.md §18

## Context

Two inter-agent and inter-system protocols are emerging with distinct scopes.

## Decision

- **MCP** is the discovery and invocation protocol for tools/resources used by Aevum agents.
- **A2A** is the protocol for inter-agent collaboration with independent trust domains.
- **Action Attestation** is the authority/proof protocol used by the Kernel.
- **Capability API** is the internal, typed executor protocol.

Aevum wraps A2A remote tasks inside a sub-mission; the remote result is a Claim until verified locally.

## Rationale

- MCP is for tools; A2A is for agents; Action Attestation is for authority.
- Authorization and authorization semantics remain anchored in Aevum's policy engine, regardless of protocol.

## Consequences

- Each MCP server receives only the explicit context it requested.
- An A2A Agent Card must include trust domain, evidence capabilities and max autonomy.

## Revisit criteria

None in M0. Re-evaluate when a remote Agent Card must carry executable authority.
