//! Orchestrator contracts and role rotation helpers.

use anyhow::Result;
use async_trait::async_trait;

use crate::step::{Role, StepContext, StepResult};

/// Common behavior required from every LLM-backed agent.
#[async_trait]
pub trait Agent: Send + Sync {
    fn role(&self) -> Role;
    async fn plan(&self, ctx: &StepContext) -> Result<String>;
    async fn edit(&self, ctx: &StepContext) -> Result<StepResult>;
}

/// High-level control loop that coordinates agents and git operations.
#[async_trait]
pub trait Orchestrator {
    fn current_role(&self) -> Role;
    async fn next(&mut self) -> Result<()>;
}

/// Utility that encodes the Tester → Implementor → Refactorer rotation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleCycle {
    current: Role,
}

impl RoleCycle {
    /// Start from a specific role.
    pub const fn new(initial: Role) -> Self {
        Self { current: initial }
    }

    /// Determine the starting role based on repository history.
    ///
    /// When the repo is empty we always start with Tester. Otherwise we resume with the
    /// next role after the last successful one (defaults back to Tester when unknown).
    pub const fn from_history(last_role: Option<Role>, repo_is_empty: bool) -> Self {
        if repo_is_empty {
            return Self::new(Role::Tester);
        }

        let initial = match last_role {
            Some(role) => role.next(),
            None => Role::Tester,
        };
        Self::new(initial)
    }

    /// Return the current role in the cycle.
    pub const fn current(&self) -> Role {
        self.current
    }

    /// Advance to the next role and return it.
    pub fn advance(&mut self) -> Role {
        self.current = self.current.next();
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_cycle_advances_in_order() {
        let mut cycle = RoleCycle::new(Role::Tester);
        assert_eq!(cycle.current(), Role::Tester);
        assert_eq!(cycle.advance(), Role::Implementor);
        assert_eq!(cycle.advance(), Role::Refactorer);
        assert_eq!(cycle.advance(), Role::Tester);
    }

    #[test]
    fn role_cycle_respects_empty_repo_rule() {
        let cycle = RoleCycle::from_history(Some(Role::Refactorer), true);
        assert_eq!(cycle.current(), Role::Tester);
    }

    #[test]
    fn role_cycle_resumes_after_last_role() {
        let cycle = RoleCycle::from_history(Some(Role::Tester), false);
        assert_eq!(cycle.current(), Role::Implementor);

        let cycle = RoleCycle::from_history(None, false);
        assert_eq!(cycle.current(), Role::Tester);
    }
}
