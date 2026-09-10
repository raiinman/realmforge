use crate::GateError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentitySubject(String);

impl IdentitySubject {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GateError::InvalidIdentitySubject);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameAccountId(String);

impl GameAccountId {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GateError::InvalidGameAccountId);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameAccountProjection {
    pub subject: IdentitySubject,
    pub game_account_id: GameAccountId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_subject_rejects_blank_values() {
        assert_eq!(
            IdentitySubject::new("   ").unwrap_err(),
            GateError::InvalidIdentitySubject
        );
    }

    #[test]
    fn game_account_id_rejects_blank_values() {
        assert_eq!(
            GameAccountId::new("").unwrap_err(),
            GateError::InvalidGameAccountId
        );
    }
}
