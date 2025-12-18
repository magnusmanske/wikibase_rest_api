use crate::RestApiError;

#[derive(Debug, Clone, PartialEq, Default, Copy)]
pub enum StatementRank {
    #[default]
    Normal,
    Preferred,
    Deprecated,
}

impl StatementRank {
    /// Create a new `StatementRank` from a string
    pub fn new<S: Into<String>>(s: S) -> Result<Self, RestApiError> {
        let s = s.into();
        match s.to_lowercase().as_str() {
            "normal" => Ok(StatementRank::Normal),
            "preferred" => Ok(StatementRank::Preferred),
            "deprecated" => Ok(StatementRank::Deprecated),
            _ => Err(RestApiError::UnknownStatementRank(s)),
        }
    }

    /// Returns the `StatementRank` as a string
    pub const fn as_str(&self) -> &str {
        match self {
            StatementRank::Normal => "normal",
            StatementRank::Preferred => "preferred",
            StatementRank::Deprecated => "deprecated",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        assert_eq!(StatementRank::new("normal").unwrap(), StatementRank::Normal);
        assert_eq!(
            StatementRank::new("preferred").unwrap(),
            StatementRank::Preferred
        );
        assert_eq!(
            StatementRank::new("deprecated").unwrap(),
            StatementRank::Deprecated
        );
        assert!(StatementRank::new("unknown").is_err());
    }

    #[test]
    fn test_as_str() {
        assert_eq!(StatementRank::Normal.as_str(), "normal");
        assert_eq!(StatementRank::Preferred.as_str(), "preferred");
        assert_eq!(StatementRank::Deprecated.as_str(), "deprecated");
    }
}
