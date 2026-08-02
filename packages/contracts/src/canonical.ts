// JCS-style canonical JSON + sha256 (M0+ helpers).
// Used by Constitution digest, Attestation canonicalisation (M3), Ledger hash chain (M10).
//
// We keep this dependency-free because the contracts package must work
// in any environment (browser, Node, future WASM). The hashing is
// implemented via Node's `crypto` on the server side and via
// `globalThis.crypto.subtle` in the browser; both yield identical
// hex sha256 digests.

export function canonicalJsonStringify(value: unknown): string {
  return JSON.stringify(sortDeep(value));
}

function sortDeep(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortDeep);
  if (value !== null && typeof value === "object") {
    const obj = value as Record<string, unknown>;
    const out: Record<string, unknown> = {};
    for (const k of Object.keys(obj).sort()) out[k] = sortDeep(obj[k]);
    return out;
  }
  return value;
}

export async function sha256Async(message: string): Promise<string> {
  const enc = new TextEncoder().encode(message);
  if (typeof globalThis !== "undefined" && globalThis.crypto && globalThis.crypto.subtle) {
    const digest = await globalThis.crypto.subtle.digest("SHA-256", enc);
    return Array.from(new Uint8Array(digest))
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
  }
  // Fallback: Node `crypto`.
  const { createHash } = await import("node:crypto");
  return createHash("sha256").update(enc).digest("hex");
}

let nodeCrypto: typeof import("node:crypto") | null = null;

function ensureNode(): typeof import("node:crypto") {
  if (!nodeCrypto) {
    // CommonJS require via createRequire keeps contracts package free of ESM
    // dependencies and works inside the Vitest sandbox at test time.
    // Using the dynamic import would create an async API; we keep sync.
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    nodeCrypto = eval("require")("node:crypto");
  }
  return nodeCrypto!;
}

export function sha256(message: string): string {
  const enc = new TextEncoder().encode(message);
  const crypto = ensureNode();
  return crypto.createHash("sha256").update(enc).digest("hex");
}
