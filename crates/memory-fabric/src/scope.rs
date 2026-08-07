//! Tenant / mission scope — enforced isolation for multi-tenant local memory (ADR-0019).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Bound for search, assemble, FTS, and multi-mission store rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantScope {
    pub tenant_id: String,
    pub mission_id: String,
}

impl TenantScope {
    pub fn new(tenant_id: impl Into<String>, mission_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            mission_id: mission_id.into(),
        }
    }

    pub fn local(mission_id: impl Into<String>) -> Self {
        Self::new("local", mission_id)
    }

    /// Canonical group id — always `mission:{mission_id}`.
    pub fn group_id(&self) -> String {
        format!("mission:{}", self.mission_id)
    }

    /// Resolve tenant from `AEVUM_TENANT_ID` (default `local`) + mission from metadata.json.
    pub fn from_mission_dir(mission_dir: impl AsRef<Path>) -> Self {
        let tenant = std::env::var("AEVUM_TENANT_ID").unwrap_or_else(|_| "local".into());
        let meta = mission_dir.as_ref().join("metadata.json");
        let mission_id = fs::read_to_string(&meta)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|v| {
                v.get("mission")
                    .and_then(|m| m.get("mission_id"))
                    .or_else(|| v.get("mission_id"))
                    .and_then(|x| x.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "mis_unknown".into());
        Self::new(tenant, mission_id)
    }
}
