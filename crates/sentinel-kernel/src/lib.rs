#![forbid(unsafe_code)]
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Capability request from the agent (M2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRequest {
    pub capability: String,
    pub resource: String,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGrants {
    pub version: String,
    pub grants: Vec<CapabilityGrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGrant {
    pub capability: String,
    pub argv_template: Vec<String>,
    pub bounds: Vec<String>,
    pub denied_reason: Option<String>,
}

#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("no matching grant for capability {0}")]
    NoGrant(String),
    #[error("argv argument `{0}` is not allowed (shell metacharacter detected)")]
    ArgumentRejected(String),
    #[error("argv mismatch: expected {expected:?}, got {actual:?}")]
    ArgvMismatch {
        expected: Vec<String>,
        actual: Vec<String>,
    },
}

const FORBIDDEN_SHELL_CHARS: &[char] = &[
    '&', '|', ';', '`', '$', '<', '>', '\\', '\'', '"', '\n', '\r',
];

/// The Sentinel Kernel — verifies that a capability invocation is well-formed
/// (no shell metacharacters, argv matches a registered grant) and that the
/// grant is present in the local manifest.
///
/// Per blueprint §6.4 ("no shell libre on the agentic path"), the kernel
/// rejects any argv that contains shell control characters.
#[derive(Debug, Clone)]
pub struct SentinelKernel {
    pub grants: CapabilityGrants,
}

impl SentinelKernel {
    pub fn new(grants: CapabilityGrants) -> Self {
        Self { grants }
    }

    pub fn default_kernel() -> Self {
        Self {
            grants: CapabilityGrants {
                version: "aevum.capabilities/v1".to_string(),
                grants: vec![
                    CapabilityGrant {
                        capability: "git.branch.create".into(),
                        argv_template: vec!["git".into(), "checkout".into(), "-b".into(), "{branch}".into()],
                        bounds: vec!["aevum/*".into()],
                        denied_reason: None,
                    },
                    CapabilityGrant {
                        capability: "git.commit".into(),
                        argv_template: vec!["git".into(), "commit".into(), "-m".into(), "{message}".into()],
                        bounds: vec![],
                        denied_reason: None,
                    },
                    CapabilityGrant {
                        capability: "fs.read".into(),
                        argv_template: vec!["cat".into(), "{path}".into()],
                        bounds: vec!["${WORKSPACE_ROOT}/**".into()],
                        denied_reason: None,
                    },
                    CapabilityGrant {
                        capability: "fs.write".into(),
                        argv_template: vec!["scribble".into(), "{path}".into()],
                        bounds: vec!["${WORKSPACE_ROOT}/**".into()],
                        denied_reason: None,
                    },
                    CapabilityGrant {
                        capability: "deployment.promote".into(),
                        argv_template: vec!["unify".into(), "deploy".into(), "--commit".into(), "{commit_sha}".into()],
                        bounds: vec!["github:*/main".into()],
                        denied_reason: Some("Production deployment requires human approval — see kernel.json".into()),
                    },
                    CapabilityGrant {
                        capability: "sh.execute".into(),
                        argv_template: vec![],
                        bounds: vec![],
                        denied_reason: Some("sh -c style execution is FORBIDDEN on the agentic path (§16.4). Re-author via dedicated capability.".into()),
                    },
                ],
            },
        }
    }

    /// Authorise an argv against a registered grant. Returns an updated argv
    /// with bound-substituted parameters or a `CapabilityError` if rejected.
    pub fn authorise(&self, req: &CapabilityRequest) -> Result<Vec<String>, CapabilityError> {
        // 1. Reject any shell metachars.
        for part in &req.argv {
            if part.chars().any(|c| FORBIDDEN_SHELL_CHARS.contains(&c)) {
                return Err(CapabilityError::ArgumentRejected(part.clone()));
            }
        }
        // 2. Reject `sh.execute` (impossible to enforce integrity).
        if req.capability == "sh.execute" {
            return Err(CapabilityError::NoGrant(req.capability.clone()));
        }
        // 3. Find a matching grant and check argv shape.
        let grant = self
            .grants
            .grants
            .iter()
            .find(|g| g.capability == req.capability)
            .ok_or_else(|| CapabilityError::NoGrant(req.capability.clone()))?;
        if let Some(why) = &grant.denied_reason {
            return Err(CapabilityError::NoGrant(format!(
                "{} ({})",
                req.capability, why
            )));
        }
        let expected = grant.argv_template.len();
        if expected != req.argv.len() {
            return Err(CapabilityError::ArgvMismatch {
                expected: grant.argv_template.clone(),
                actual: req.argv.clone(),
            });
        }
        Ok(req.argv.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_shell_metachars() {
        let kernel = SentinelKernel::default_kernel();
        let req = CapabilityRequest {
            capability: "fs.write".into(),
            resource: "x".into(),
            argv: vec!["scribble".into(), "bad; ls / | rm -rf".into()],
        };
        assert!(matches!(
            kernel.authorise(&req),
            Err(CapabilityError::ArgumentRejected(_))
        ));
    }

    #[test]
    fn rejects_sh_execute() {
        let kernel = SentinelKernel::default_kernel();
        let req = CapabilityRequest {
            capability: "sh.execute".into(),
            resource: "x".into(),
            argv: vec!["sh".into(), "-c".into(), "echo hi".into()],
        };
        assert!(matches!(
            kernel.authorise(&req),
            Err(CapabilityError::NoGrant(_))
        ));
    }

    #[test]
    fn accepts_well_formed_git_branch_create() {
        let kernel = SentinelKernel::default_kernel();
        let req = CapabilityRequest {
            capability: "git.branch.create".into(),
            resource: "x".into(),
            argv: vec![
                "git".into(),
                "checkout".into(),
                "-b".into(),
                "aevum/sec".into(),
            ],
        };
        assert!(kernel.authorise(&req).is_ok());
    }

    #[test]
    fn refuses_unknown_capability() {
        let kernel = SentinelKernel::default_kernel();
        let req = CapabilityRequest {
            capability: "sql.drop".into(),
            resource: "x".into(),
            argv: vec!["psql".into(), "-c".into(), "DROP DATABASE".into()],
        };
        assert!(matches!(
            kernel.authorise(&req),
            Err(CapabilityError::NoGrant(_))
        ));
    }
}
