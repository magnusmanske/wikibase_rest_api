#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    item_letter: char,
    property_letter: char,
}

impl Config {
    /// Constructs a new `Config` object from item and property letters.
    pub const fn new(item_letter: char, property_letter: char) -> Config {
        Config {
            item_letter,
            property_letter,
        }
    }

    /// Returns the letter used for items.
    pub const fn item_letter(&self) -> char {
        self.item_letter
    }

    /// Returns the letter used for properties.
    pub const fn property_letter(&self) -> char {
        self.property_letter
    }
}

pub const WIKIDATA_CONFIG: Config = Config {
    item_letter: 'Q',
    property_letter: 'P',
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = Config::new('Q', 'P');
        assert_eq!(config.item_letter(), 'Q');
        assert_eq!(config.property_letter(), 'P');
    }
}
