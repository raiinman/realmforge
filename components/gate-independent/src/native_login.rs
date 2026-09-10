use std::collections::BTreeMap;
use std::fmt;

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

use crate::{AccountDirectory, GateError, IdentitySubject, MemoryAccountDirectory};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoginName(String);

impl LoginName {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.len() > 254
            || trimmed.chars().any(|ch| ch.is_control())
        {
            return Err(GateError::InvalidLoginName);
        }
        Ok(Self(trimmed.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct NativeCredential {
    pub login: LoginName,
    pub subject: IdentitySubject,
    password_hash: String,
}

impl NativeCredential {
    pub fn from_password(
        login: LoginName,
        subject: IdentitySubject,
        password: &str,
    ) -> Result<Self, GateError> {
        Ok(Self {
            login,
            subject,
            password_hash: hash_native_password(password)?,
        })
    }

    pub fn from_phc(
        login: LoginName,
        subject: IdentitySubject,
        password_hash: impl Into<String>,
    ) -> Result<Self, GateError> {
        let password_hash = password_hash.into();
        PasswordHash::new(&password_hash).map_err(|_| GateError::InvalidPasswordHash)?;
        Ok(Self {
            login,
            subject,
            password_hash,
        })
    }

    fn verify(&self, password: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(&self.password_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    }

    pub fn password_hash_phc(&self) -> &str {
        &self.password_hash
    }
}

impl fmt::Debug for NativeCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeCredential")
            .field("login", &self.login)
            .field("subject", &self.subject)
            .field("password_hash", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemoryNativeCredentialStore {
    credentials: BTreeMap<LoginName, NativeCredential>,
}

impl MemoryNativeCredentialStore {
    pub fn new(
        credentials: impl IntoIterator<Item = NativeCredential>,
    ) -> Result<Self, GateError> {
        let mut store = Self::default();
        for credential in credentials {
            store.register(credential)?;
        }
        Ok(store)
    }

    pub fn register(&mut self, credential: NativeCredential) -> Result<(), GateError> {
        if self.credentials.contains_key(&credential.login) {
            return Err(GateError::DuplicateLoginName);
        }
        self.credentials
            .insert(credential.login.clone(), credential);
        Ok(())
    }

    fn authenticate(
        &self,
        login: &LoginName,
        password: &str,
    ) -> Result<IdentitySubject, GateError> {
        let credential = self
            .credentials
            .get(login)
            .ok_or(GateError::InvalidCredentials)?;
        if !credential.verify(password) {
            return Err(GateError::InvalidCredentials);
        }
        Ok(credential.subject.clone())
    }
}

#[derive(Debug, Clone)]
pub struct NativeLoginService {
    credentials: MemoryNativeCredentialStore,
    accounts: MemoryAccountDirectory,
}

impl NativeLoginService {
    pub fn new(
        credentials: MemoryNativeCredentialStore,
        accounts: MemoryAccountDirectory,
    ) -> Self {
        Self {
            credentials,
            accounts,
        }
    }

    pub fn authenticate(
        &self,
        login: &LoginName,
        password: &str,
    ) -> Result<IdentitySubject, GateError> {
        let subject = self.credentials.authenticate(login, password)?;
        let account = self.accounts.find_by_subject(&subject)?;
        account.ensure_login_allowed()?;
        Ok(subject)
    }
}

pub fn hash_native_password(password: &str) -> Result<String, GateError> {
    if password.len() < 12 || password.len() > 1024 {
        return Err(GateError::InvalidPassword);
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| GateError::PasswordHashingFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountId, AccountRecord, AccountStatus};

    const GOOD_PASSWORD: &str = "correct horse battery staple";

    fn account(subject: &str, status: AccountStatus) -> AccountRecord {
        AccountRecord {
            id: AccountId::new(format!("account-{subject}")).unwrap(),
            subject: IdentitySubject::new(subject).unwrap(),
            status,
            game_accounts: vec![],
        }
    }

    fn service(status: AccountStatus) -> NativeLoginService {
        let subject = IdentitySubject::new("subject-1").unwrap();
        let credential = NativeCredential::from_password(
            LoginName::new("Admin@Realmforge.Local").unwrap(),
            subject,
            GOOD_PASSWORD,
        )
        .unwrap();
        NativeLoginService::new(
            MemoryNativeCredentialStore::new([credential]).unwrap(),
            MemoryAccountDirectory::new([account("subject-1", status)]).unwrap(),
        )
    }

    #[test]
    fn login_names_are_trimmed_and_ascii_case_insensitive() {
        assert_eq!(
            LoginName::new("  Admin@Realmforge.Local  ")
                .unwrap()
                .as_str(),
            "admin@realmforge.local"
        );
    }

    #[test]
    fn argon2id_hash_round_trip_authenticates() {
        let service = service(AccountStatus::Active);
        assert_eq!(
            service
                .authenticate(
                    &LoginName::new("ADMIN@REALMFORGE.LOCAL").unwrap(),
                    GOOD_PASSWORD,
                )
                .unwrap()
                .as_str(),
            "subject-1"
        );
    }

    #[test]
    fn wrong_or_unknown_credentials_share_one_public_error() {
        let service = service(AccountStatus::Active);
        assert_eq!(
            service
                .authenticate(
                    &LoginName::new("admin@realmforge.local").unwrap(),
                    "wrong password here",
                )
                .unwrap_err(),
            GateError::InvalidCredentials
        );
        assert_eq!(
            service
                .authenticate(
                    &LoginName::new("missing@realmforge.local").unwrap(),
                    GOOD_PASSWORD,
                )
                .unwrap_err(),
            GateError::InvalidCredentials
        );
    }

    #[test]
    fn account_state_still_fails_closed_after_valid_password() {
        assert_eq!(
            service(AccountStatus::Locked)
                .authenticate(
                    &LoginName::new("admin@realmforge.local").unwrap(),
                    GOOD_PASSWORD,
                )
                .unwrap_err(),
            GateError::AccountLocked
        );
    }

    #[test]
    fn native_credential_debug_redacts_hash() {
        let credential = NativeCredential::from_password(
            LoginName::new("admin@realmforge.local").unwrap(),
            IdentitySubject::new("subject-1").unwrap(),
            GOOD_PASSWORD,
        )
        .unwrap();
        let debug = format!("{credential:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(credential.password_hash_phc()));
    }

    #[test]
    fn weak_or_absurd_password_length_is_rejected() {
        assert_eq!(
            hash_native_password("too short").unwrap_err(),
            GateError::InvalidPassword
        );
        assert_eq!(
            hash_native_password(&"x".repeat(1025)).unwrap_err(),
            GateError::InvalidPassword
        );
    }
}
