use std::collections::{BTreeMap, BTreeSet};

use crate::{GameAccountId, GameAccountProjection, GateError, IdentitySubject};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountId(String);

impl AccountId {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GateError::InvalidAccountId);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    Active,
    Locked,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub id: AccountId,
    pub subject: IdentitySubject,
    pub status: AccountStatus,
    pub game_accounts: Vec<GameAccountId>,
}

impl AccountRecord {
    pub fn ensure_login_allowed(&self) -> Result<(), GateError> {
        match self.status {
            AccountStatus::Active => Ok(()),
            AccountStatus::Locked => Err(GateError::AccountLocked),
            AccountStatus::Disabled => Err(GateError::AccountDisabled),
        }
    }

    pub fn game_account_projections(&self) -> Vec<GameAccountProjection> {
        self.game_accounts
            .iter()
            .cloned()
            .map(|game_account_id| GameAccountProjection {
                subject: self.subject.clone(),
                game_account_id,
            })
            .collect()
    }
}

/// Realmforge account lookup boundary.
///
/// Storage technology is deliberately outside this contract. A production
/// implementation may use SQL, an external identity service, or another
/// Realmforge component without changing Gate's account semantics.
pub trait AccountDirectory {
    fn find_by_subject(&self, subject: &IdentitySubject) -> Result<AccountRecord, GateError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryAccountDirectory {
    by_subject: BTreeMap<IdentitySubject, AccountRecord>,
}

impl MemoryAccountDirectory {
    pub fn new(records: impl IntoIterator<Item = AccountRecord>) -> Result<Self, GateError> {
        let mut by_subject = BTreeMap::new();
        let mut account_ids = BTreeSet::new();

        for record in records {
            if !account_ids.insert(record.id.clone()) {
                return Err(GateError::DuplicateAccountId);
            }
            if by_subject.insert(record.subject.clone(), record).is_some() {
                return Err(GateError::DuplicateIdentitySubject);
            }
        }

        Ok(Self { by_subject })
    }
}

impl AccountDirectory for MemoryAccountDirectory {
    fn find_by_subject(&self, subject: &IdentitySubject) -> Result<AccountRecord, GateError> {
        self.by_subject
            .get(subject)
            .cloned()
            .ok_or(GateError::AccountNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, subject: &str, status: AccountStatus) -> AccountRecord {
        AccountRecord {
            id: AccountId::new(id).unwrap(),
            subject: IdentitySubject::new(subject).unwrap(),
            status,
            game_accounts: vec![GameAccountId::new(format!("{id}-game")).unwrap()],
        }
    }

    #[test]
    fn directory_rejects_duplicate_account_ids() {
        let result = MemoryAccountDirectory::new([
            record("account-1", "subject-1", AccountStatus::Active),
            record("account-1", "subject-2", AccountStatus::Active),
        ]);
        assert_eq!(result.unwrap_err(), GateError::DuplicateAccountId);
    }

    #[test]
    fn directory_rejects_duplicate_subjects() {
        let result = MemoryAccountDirectory::new([
            record("account-1", "subject-1", AccountStatus::Active),
            record("account-2", "subject-1", AccountStatus::Active),
        ]);
        assert_eq!(result.unwrap_err(), GateError::DuplicateIdentitySubject);
    }

    #[test]
    fn locked_and_disabled_accounts_fail_closed() {
        assert_eq!(
            record("a", "s", AccountStatus::Locked)
                .ensure_login_allowed()
                .unwrap_err(),
            GateError::AccountLocked
        );
        assert_eq!(
            record("a", "s", AccountStatus::Disabled)
                .ensure_login_allowed()
                .unwrap_err(),
            GateError::AccountDisabled
        );
    }

    #[test]
    fn projections_keep_subject_and_game_account_identity_separate() {
        let account = record("account-1", "subject-1", AccountStatus::Active);
        let projection = &account.game_account_projections()[0];
        assert_eq!(projection.subject.as_str(), "subject-1");
        assert_eq!(projection.game_account_id.as_str(), "account-1-game");
    }
}
