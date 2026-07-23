use std::fmt;

use crate::{config::WIKIDATA_CONFIG, Config, RestApiError};

#[derive(Debug, Clone, Default, PartialEq)]
pub enum EntityId {
    #[default]
    None,
    Item(String),
    Property(String),
}

impl EntityId {
    /// Returns the ID of the entity.
    pub const fn id(&self) -> Result<&String, RestApiError> {
        match self {
            EntityId::None => Err(RestApiError::IsNone),
            EntityId::Item(id) => Ok(id),
            EntityId::Property(id) => Ok(id),
        }
    }

    /// Returns the group of the entity.
    pub const fn group(&self) -> Result<&str, RestApiError> {
        match self {
            EntityId::Item(_) => Ok("items"),
            EntityId::Property(_) => Ok("properties"),
            _ => Err(RestApiError::IsNone),
        }
    }

    /// Returns the entity type of the entity.
    pub const fn entity_type(&self) -> Result<&str, RestApiError> {
        match self {
            EntityId::Item(_) => Ok("item"),
            EntityId::Property(_) => Ok("property"),
            _ => Err(RestApiError::IsNone),
        }
    }

    /// Creates a new entity ID from a string, using the default Wikidata configuration.
    pub fn new<S: Into<String>>(id: S) -> Result<EntityId, RestApiError> {
        Self::new_from_config(id, &WIKIDATA_CONFIG)
    }

    /// Creates a new entity ID from a string, using a bespoke configuration.
    ///
    /// The ID must be a type letter followed by digits (e.g. `Q42`). This validation rejects
    /// malformed input — including strings that could inject extra URL path segments.
    pub fn new_from_config<S: Into<String>>(
        id: S,
        config: &Config,
    ) -> Result<EntityId, RestApiError> {
        let id = id.into();
        let (variant, number): (fn(String) -> EntityId, &str) =
            if let Some(number) = id.strip_prefix(config.item_letter()) {
                (EntityId::Item, number)
            } else if let Some(number) = id.strip_prefix(config.property_letter()) {
                (EntityId::Property, number)
            } else {
                return Err(RestApiError::UnknownEntityLetter(id));
            };
        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
            return Err(RestApiError::InvalidEntityId(id));
        }
        Ok(variant(id))
    }

    /// Returns an unset (None) entity ID.
    pub const fn none() -> EntityId {
        EntityId::None
    }

    /// Returns a new entity ID for an item. The ID is **not** validated; prefer
    /// [`new`](Self::new) for untrusted input.
    pub fn item<S: Into<String>>(s: S) -> EntityId {
        EntityId::Item(s.into())
    }

    /// Returns a new entity ID for a property. The ID is **not** validated; prefer
    /// [`new`](Self::new) for untrusted input.
    pub fn property<S: Into<String>>(s: S) -> EntityId {
        EntityId::Property(s.into())
    }

    /// Returns the REST API path for this entity, e.g. `/entities/items/Q42`.
    pub fn entity_path(&self) -> Result<String, RestApiError> {
        Ok(format!("/entities/{}/{self}", self.group()?))
    }

    /// Returns true if the entity ID is an item or a property.
    pub fn is_some(&self) -> bool {
        *self != EntityId::None
    }

    /// Returns true if the entity ID is unset (None).
    pub fn is_none(&self) -> bool {
        *self == EntityId::None
    }
}

impl From<EntityId> for String {
    fn from(val: EntityId) -> Self {
        match val {
            EntityId::Item(id) | EntityId::Property(id) => id,
            EntityId::None => String::new(),
        }
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `None` renders as empty rather than `Err`, so `format!("{id}")` can never panic.
        match self {
            EntityId::Item(id) | EntityId::Property(id) => write!(f, "{id}"),
            EntityId::None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_item() {
        let id = EntityId::item("Q123");
        assert_eq!(id, EntityId::item("Q123"));
    }

    #[test]
    fn test_entity_id_property() {
        let id = EntityId::property("P123");
        assert_eq!(id, EntityId::property("P123"));
    }

    #[test]
    fn test_entity_id_none() {
        let id = EntityId::none();
        assert_eq!(id, EntityId::None);
    }

    #[test]
    fn test_entity_id_item_is_some() {
        let id = EntityId::item("Q123");
        assert!(id.is_some());
    }

    #[test]
    fn test_entity_id_property_is_some() {
        let id = EntityId::property("P123");
        assert!(id.is_some());
    }

    #[test]
    fn test_entity_id_none_is_some() {
        let id = EntityId::none();
        assert!(!id.is_some());
    }

    #[test]
    fn test_entity_id_item_is_none() {
        let id = EntityId::item("Q123");
        assert!(!id.is_none());
    }

    #[test]
    fn test_entity_id_property_is_none() {
        let id = EntityId::property("P123");
        assert!(!id.is_none());
    }

    #[test]
    fn test_entity_id_none_is_none() {
        let id = EntityId::none();
        assert!(id.is_none());
    }

    #[test]
    fn test_entity_id_default() {
        let id = EntityId::default();
        assert_eq!(id, EntityId::None);
    }

    #[test]
    fn test_entity_id_item_id() {
        let id = EntityId::item("Q123");
        assert_eq!(id.id().unwrap(), "Q123");
    }

    #[test]
    fn test_entity_id_property_id() {
        let id = EntityId::property("P123");
        assert_eq!(id.id().unwrap(), "P123");
    }

    #[test]
    fn test_entity_id_none_id() {
        let id = EntityId::none();
        assert!(id.id().is_err());
    }

    #[test]
    fn test_entity_id_item_group() {
        let id = EntityId::item("Q123");
        assert_eq!(id.group().unwrap(), "items");
    }

    #[test]
    fn test_entity_id_property_group() {
        let id = EntityId::property("P123");
        assert_eq!(id.group().unwrap(), "properties");
    }

    #[test]
    fn test_entity_id_none_group() {
        let id = EntityId::none();
        assert!(id.group().is_err());
    }

    #[test]
    fn test_entity_id_entity_item_type() {
        let id = EntityId::item("Q123");
        assert_eq!(id.entity_type().unwrap(), "item");
    }

    #[test]
    fn test_entity_id_entity_property_type() {
        let id = EntityId::property("P123");
        assert_eq!(id.entity_type().unwrap(), "property");
    }

    #[test]
    fn test_entity_id_entity_none_type() {
        let id = EntityId::none();
        assert!(id.entity_type().is_err());
    }

    #[test]
    fn test_entity_id_item_new() {
        let id = EntityId::new("Q123").unwrap();
        assert_eq!(id, EntityId::item("Q123"));
    }

    #[test]
    fn test_entity_id_property_new() {
        let id = EntityId::new("P123").unwrap();
        assert_eq!(id, EntityId::property("P123"));
    }

    #[test]
    fn test_entity_id_none_new() {
        let id = EntityId::new("X123");
        assert!(id.is_err());
    }

    #[test]
    fn test_entity_id_into_string() {
        let item_id: String = EntityId::item("Q123").into();
        assert_eq!(item_id, "Q123");
        let property_id: String = EntityId::property("P123").into();
        assert_eq!(property_id, "P123");
        // The `None` arm renders as an empty string.
        let none_id: String = EntityId::none().into();
        assert_eq!(none_id, "");
    }

    #[test]
    fn test_entity_id_display() {
        assert_eq!(EntityId::item("Q123").to_string(), "Q123");
        assert_eq!(EntityId::property("P123").to_string(), "P123");
        // `None` renders as empty and must never panic.
        assert_eq!(EntityId::none().to_string(), "");
    }

    #[test]
    fn test_entity_id_new_from_config() {
        let config = Config::new('A', 'B');
        let id_a = EntityId::new_from_config("A123", &config).unwrap();
        assert_eq!(id_a, EntityId::item("A123"));
        let id_b = EntityId::new_from_config("B123", &config).unwrap();
        assert_eq!(id_b, EntityId::property("B123"));
        let id_x = EntityId::new_from_config("X123", &config);
        assert!(id_x.is_err());
    }

    #[test]
    fn test_entity_id_new_rejects_malformed() {
        // Correct letter but non-numeric body (would inject a URL path segment).
        assert!(matches!(
            EntityId::new("Q42/labels/en"),
            Err(RestApiError::InvalidEntityId(_))
        ));
        assert!(matches!(
            EntityId::new("QWERTY"),
            Err(RestApiError::InvalidEntityId(_))
        ));
        // Just the letter, no number.
        assert!(matches!(
            EntityId::new("Q"),
            Err(RestApiError::InvalidEntityId(_))
        ));
        // Unknown letter.
        assert!(matches!(
            EntityId::new("X1"),
            Err(RestApiError::UnknownEntityLetter(_))
        ));
        // Valid.
        assert_eq!(EntityId::new("Q42").unwrap(), EntityId::item("Q42"));
        assert_eq!(EntityId::new("P31").unwrap(), EntityId::property("P31"));
    }
}
