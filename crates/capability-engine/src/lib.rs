#![forbid(unsafe_code)]
#![allow(missing_docs)]

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("capability {0} is not in the registry")]
    Unknown(String),
    #[error("caller lacks required role {0}")]
    Forbidden(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    pub capability: String,
    pub argv_template: Vec<String>,
    pub denied_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registry(pub Vec<Grant>);

pub struct CapabilityEngine {
    pub registry: Registry,
    pub allowed_roles: HashSet<String>,
}

impl CapabilityEngine {
    pub fn new(registry: Registry, allowed_roles: HashSet<String>) -> Self {
        Self {
            registry,
            allowed_roles,
        }
    }

    pub fn from_grants<I: IntoIterator<Item = Grant>>(grants: I) -> Self {
        let grants: Vec<_> = grants.into_iter().collect();
        let roles: HashSet<String> = ["producer", "verifier", "operator", "human-admin"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        Self::new(Registry(grants), roles)
    }

    pub fn allow(&mut self, role: impl Into<String>) {
        self.allowed_roles.insert(role.into());
    }
    pub fn deny(&mut self, role: impl Into<String>) {
        self.allowed_roles.remove(&role.into());
    }

    pub fn can(&self, role: &str, capability: &str) -> Result<&Grant, EngineError> {
        if !self.allowed_roles.contains(role) {
            return Err(EngineError::Forbidden(role.to_string()));
        }
        self.registry
            .0
            .iter()
            .find(|g| g.capability == capability)
            .ok_or_else(|| EngineError::Unknown(capability.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry(vec![
            Grant {
                capability: "git.branch.create".into(),
                argv_template: vec!["git".into(), "checkout".into(), "-b".into()],
                denied_reason: None,
            },
            Grant {
                capability: "git.push".into(),
                argv_template: vec!["git".into(), "push".into()],
                denied_reason: None,
            },
            Grant {
                capability: "fs.read".into(),
                argv_template: vec!["cat".into()],
                denied_reason: None,
            },
        ])
    }

    #[test]
    fn known_capability_can_be_queried() {
        let eng = CapabilityEngine::from_grants(registry().0);
        assert!(eng.can("producer", "git.branch.create").is_ok());
    }

    #[test]
    fn unknown_role_forbidden() {
        let eng = CapabilityEngine::from_grants(registry().0);
        assert!(matches!(
            eng.can("ghost", "git.branch.create"),
            Err(EngineError::Forbidden(_))
        ));
    }

    #[test]
    fn unknown_capability_reported() {
        let eng = CapabilityEngine::from_grants(registry().0);
        assert!(matches!(
            eng.can("producer", "shell.exec"),
            Err(EngineError::Unknown(_))
        ));
    }

    #[test]
    fn allow_then_deny_role_works() {
        let mut eng = CapabilityEngine::from_grants(registry().0);
        eng.allow("tester");
        assert!(eng.can("tester", "git.branch.create").is_ok());
        eng.deny("tester");
        assert!(matches!(
            eng.can("tester", "git.branch.create"),
            Err(EngineError::Forbidden(_))
        ));
    }
}
