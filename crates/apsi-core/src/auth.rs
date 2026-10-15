//! Tenant-aware authorization ("Access control" feature family).
//!
//! A principal belongs to exactly one tenant and holds one or more roles. Every
//! engine operation is checked against the target tenant and a permission.

use crate::error::{ApsiError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Viewer,
    Editor,
    Auditor,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Read,
    Write,
    Archive,
    Audit,
    Administer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    User,
    Service,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub subject: String,
    pub tenant: String,
    pub roles: Vec<Role>,
    pub kind: ActorKind,
}

impl Role {
    pub fn grants(self, p: Permission) -> bool {
        use Permission::*;
        match self {
            Role::Viewer => matches!(p, Read),
            Role::Editor => matches!(p, Read | Write | Archive),
            Role::Auditor => matches!(p, Read | Audit),
            Role::Admin => true,
        }
    }

    pub fn parse(s: &str) -> Result<Role> {
        match s {
            "viewer" => Ok(Role::Viewer),
            "editor" => Ok(Role::Editor),
            "auditor" => Ok(Role::Auditor),
            "admin" => Ok(Role::Admin),
            other => Err(ApsiError::InvalidInput(format!("unknown role {other}"))),
        }
    }
}

impl Principal {
    pub fn new(subject: &str, tenant: &str, roles: &[Role], kind: ActorKind) -> Self {
        Self { subject: subject.into(), tenant: tenant.into(), roles: roles.to_vec(), kind }
    }

    /// Denies cross-tenant access before checking roles, so a role in tenant A
    /// can never be used against tenant B.
    pub fn authorize(&self, tenant: &str, permission: Permission) -> Result<()> {
        if self.tenant != tenant {
            return Err(ApsiError::Forbidden(format!("principal is not a member of tenant {tenant}")));
        }
        if self.roles.iter().any(|r| r.grants(permission)) { Ok(()) } else { Err(ApsiError::Forbidden(format!("missing permission {permission:?}"))) }
    }
}

/// API tokens are stored only as SHA-256 hashes; the plaintext never persists.
#[derive(Debug, Default, Clone)]
pub struct TokenRegistry {
    by_hash: HashMap<String, Principal>,
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

impl TokenRegistry {
    pub fn insert(&mut self, token: &str, principal: Principal) {
        self.by_hash.insert(hash_token(token), principal);
    }

    pub fn resolve(&self, token: &str) -> Result<Principal> {
        self.by_hash.get(&hash_token(token)).cloned().ok_or(ApsiError::Unauthenticated)
    }

    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    /// Parses `token:tenant:subject:role[+role]` entries separated by commas.
    /// Subjects starting with `svc-` are service accounts.
    pub fn from_spec(spec: &str) -> Result<TokenRegistry> {
        let mut reg = TokenRegistry::default();
        for entry in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let parts: Vec<&str> = entry.split(':').collect();
            let [token, tenant, subject, roles] = parts.as_slice() else {
                return Err(ApsiError::InvalidInput("token spec must be token:tenant:subject:roles".into()));
            };
            if token.len() < 16 {
                return Err(ApsiError::InvalidInput("tokens must be at least 16 characters".into()));
            }
            crate::ids::validate_slug("tenant", tenant)?;
            crate::ids::validate_slug("subject", subject)?;
            let roles = roles.split('+').map(Role::parse).collect::<Result<Vec<_>>>()?;
            let kind = if subject.starts_with("svc-") { ActorKind::Service } else { ActorKind::User };
            reg.insert(token, Principal::new(subject, tenant, &roles, kind));
        }
        Ok(reg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_matrix_allowed_and_denied() {
        let viewer = Principal::new("v", "t1", &[Role::Viewer], ActorKind::User);
        assert!(viewer.authorize("t1", Permission::Read).is_ok());
        assert!(viewer.authorize("t1", Permission::Write).is_err());
        let editor = Principal::new("e", "t1", &[Role::Editor], ActorKind::User);
        assert!(editor.authorize("t1", Permission::Archive).is_ok());
        assert!(editor.authorize("t1", Permission::Administer).is_err());
        let auditor = Principal::new("a", "t1", &[Role::Auditor], ActorKind::User);
        assert!(auditor.authorize("t1", Permission::Audit).is_ok());
        assert!(auditor.authorize("t1", Permission::Write).is_err());
    }

    #[test]
    fn cross_tenant_access_is_denied_even_for_admin() {
        let admin = Principal::new("root", "t1", &[Role::Admin], ActorKind::User);
        assert!(admin.authorize("t1", Permission::Administer).is_ok());
        assert!(matches!(admin.authorize("t2", Permission::Read), Err(ApsiError::Forbidden(_))));
    }

    #[test]
    fn token_registry_hashes_and_parses() {
        let reg = TokenRegistry::from_spec("aaaaaaaaaaaaaaaa:t1:alice:editor+auditor, bbbbbbbbbbbbbbbb:t2:svc-etl:viewer").unwrap();
        let p = reg.resolve("aaaaaaaaaaaaaaaa").unwrap();
        assert_eq!(p.tenant, "t1");
        assert_eq!(p.roles, vec![Role::Editor, Role::Auditor]);
        assert_eq!(reg.resolve("bbbbbbbbbbbbbbbb").unwrap().kind, ActorKind::Service);
        assert!(matches!(reg.resolve("nope"), Err(ApsiError::Unauthenticated)));
        assert!(TokenRegistry::from_spec("short:t1:a:viewer").is_err());
        assert!(TokenRegistry::from_spec("aaaaaaaaaaaaaaaa:T1:a:viewer").is_err());
        assert!(TokenRegistry::from_spec("aaaaaaaaaaaaaaaa:t1:a:root").is_err());
    }
}
