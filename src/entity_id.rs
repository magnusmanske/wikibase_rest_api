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
    pub fn id(&self) -> Result<&String, RestApiError> {
        match self {
            EntityId::None => Err(RestApiError::IsNone),
            EntityId::Item(id) => Ok(id),
            EntityId::Property(id) => Ok(id),
        }
    }

    /// Returns the group of the entity.
    pub fn group(&self) -> Result<&str, RestApiError> {
        match self {
            EntityId::Item(_) => Ok("items"),
            EntityId::Property(_) => Ok("properties"),
            _ => Err(RestApiError::IsNone),
        }
    }

    /// Returns the entity type of the entity.
    pub fn entity_type(&self) -> Result<&str, RestApiError> {
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
    pub fn new_from_config<S: Into<String>>(id: S, config: &Config) -> Result<EntityId, RestApiError> {
        let id = id.into();
        if id.starts_with(config.item_letter()) {
            Ok(EntityId::Item(id.to_string()))
        } else if id.starts_with(config.property_letter()) {
            Ok(EntityId::Property(id.to_string()))
        } else {
            Err(RestApiError::UnknownEntityLetter(id))
        }
    }
    

    /// Returns an unset (None) entity ID.
    pub fn none() -> EntityId {
        EntityId::None
    }

    /// Returns a new entity ID for an item.
    pub fn item<S: Into<String>>(s: S) -> EntityId {
        EntityId::Item(s.into())
    }

    /// Returns a new entity ID for a property.
    pub fn property(s: &str) -> EntityId {
        EntityId::Property(s.to_string())
    }

    /// Returns true if the entity ID is an item or a property.
    pub fn is_some(&self) -> bool {
        *self!=EntityId::None
    }

    /// Returns true if the entity ID is unset (None).
    pub fn is_none(&self) -> bool {
        *self==EntityId::None
    }
}

impl Into<String> for EntityId {
    fn into(self) -> String {
        match self {
            EntityId::Item(id) => format!("{id}"),
            EntityId::Property(id) => format!("{id}"),
            EntityId::None => String::new(),
        }
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityId::Item(id) => write!(f, "{}", id),
            EntityId::Property(id) => write!(f, "{}", id),
            EntityId::None => Err(fmt::Error),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id() {
        let id = EntityId::item("Q123");
        assert_eq!(id, EntityId::item("Q123"));
        let id = EntityId::property("P123");
        assert_eq!(id, EntityId::property("P123"));
        let id = EntityId::none();
        assert_eq!(id, EntityId::None);
    }

    #[test]
    fn test_entity_id_is_some() {
        let id = EntityId::item("Q123");
        assert_eq!(id.is_some(), true);
        let id = EntityId::property("P123");
        assert_eq!(id.is_some(), true);
        let id = EntityId::none();
        assert_eq!(id.is_some(), false);
    }

    #[test]
    fn test_entity_id_is_none() {
        let id = EntityId::item("Q123");
        assert_eq!(id.is_none(), false);
        let id = EntityId::property("P123");
        assert_eq!(id.is_none(), false);
        let id = EntityId::none();
        assert_eq!(id.is_none(), true);
    }

    #[test]
    fn test_entity_id_default() {
        let id = EntityId::default();
        assert_eq!(id, EntityId::None);
    }

    #[test]
    fn test_entity_id_id() {
        let id = EntityId::item("Q123");
        assert_eq!(id.id().unwrap(), "Q123");
        let id = EntityId::property("P123");
        assert_eq!(id.id().unwrap(), "P123");
        let id = EntityId::none();
        assert!(id.id().is_err());
    }

    #[test]
    fn test_entity_id_group() {
        let id = EntityId::item("Q123");
        assert_eq!(id.group().unwrap(), "items");
        let id = EntityId::property("P123");
        assert_eq!(id.group().unwrap(), "properties");
        let id = EntityId::none();
        assert!(id.group().is_err());
    }

    #[test]
    fn test_entity_id_entity_type() {
        let id = EntityId::item("Q123");
        assert_eq!(id.entity_type().unwrap(), "item");
        let id = EntityId::property("P123");
        assert_eq!(id.entity_type().unwrap(), "property");
        let id = EntityId::none();
        assert!(id.entity_type().is_err());
    }

    #[test]
    fn test_entity_id_new() {
        let id = EntityId::new("Q123").unwrap();
        assert_eq!(id, EntityId::item("Q123"));
        let id = EntityId::new("P123").unwrap();
        assert_eq!(id, EntityId::property("P123"));
        let id = EntityId::new("X123");
        assert!(id.is_err());
    }

    #[test]
    fn test_entity_id_new_from_config() {
        let config = Config::new('A', 'B');
        let id = EntityId::new_from_config("A123", &config).unwrap();
        assert_eq!(id, EntityId::item("A123"));
        let id = EntityId::new_from_config("B123", &config).unwrap();
        assert_eq!(id, EntityId::property("B123"));
        let id = EntityId::new_from_config("X123", &config);
        assert!(id.is_err());
    }


}