use std::fmt;

use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Serialize, Clone)]
pub struct SubscriberName(String);

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SubscriberName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SubscriberNameVisitor;

        impl<'de> Visitor<'de> for SubscriberNameVisitor {
            type Value = SubscriberName;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid subscriber name string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match SubscriberName::parse(value.to_string()) {
                    Ok(subscriber_name) => Ok(subscriber_name),
                    Err(err) => Err(E::custom(err)),
                }
            }
        }

        deserializer.deserialize_str(SubscriberNameVisitor)
    }
}

impl fmt::Display for SubscriberName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SubscriberName {
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty_or_white_space = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|c| forbidden_characters.contains(&c));

        if is_empty_or_white_space || is_too_long || contains_forbidden_characters {
            Err(format!("{s} is not a valid subscriber name."))
        } else {
            Ok(Self(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use claims::{assert_err, assert_ok};

    use crate::domain::SubscriberName;

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ё".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Pranitha".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
