/**
 * Typed capabilities (D14). A capability is granted explicitly, never inferred.
 */
export type NetworkMode = "none" | "allowlist";

export interface ResourceSelector {
  workspaceRoot?: string;
  branch?: string;
  paths?: string[];
  domains?: string[];
}

export interface CapabilityConstraints {
  paths?: string[];
  domains?: string[];
  maxBytes?: number;
  maxDurationSeconds?: number;
  maxMemoryMb?: number;
  maxCpuCores?: number;
  network?: NetworkMode;
  /** When true, the runtime MUST NOT accept free-form command strings. */
  shellInterpolation: false;
}

export interface CapabilityGrant {
  capability: string;             // e.g. "process.exec.argv", "git.branch.create"
  version: number;                // contract version
  resource: ResourceSelector;
  operations: string[];
  constraints: CapabilityConstraints;
}
