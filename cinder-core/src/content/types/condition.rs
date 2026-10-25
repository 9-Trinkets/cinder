//! The one language for "when does this hold": a list of conditions that gates
//! an action, an exit, a description, a topic, a stage signal and so on.
//!
//! Most conditions are short text: `shaman_defeated` (a story variable is set),
//! `!zayd_rescued` (it is not), `phase=night` (it equals a value),
//! `item:chalk` (the player carries it), `here:lamp` (it lies in this room),
//! `defeated:warden`. A condition over any other part of the world is written as
//! an object with a `path`, an `operator` and a `value`.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A condition over a path into the world's state, such as
/// `actor_stats.wren.points` `gte` `8`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathCondition {
    pub path: String,
    pub operator: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// A story variable is set to a truthy value (not empty, `false` or `0`).
    StoryVar(String),
    /// A story variable equals this text.
    StoryVarIs { name: String, value: String },
    /// The player is carrying the item.
    Carries(String),
    /// The item lies in the room being considered.
    ItemHere(String),
    /// Some trace mark lies in the room being considered.
    TraceMarkHere,
    /// The actor has been defeated.
    Defeated(String),
    /// A feature is in a state. `feature` is a feature id in the room being
    /// considered, or `room/feature` for one somewhere else.
    FeatureIs { feature: String, state: String },
    /// The condition does not hold. Never wraps another `Not` or a `Path`.
    Not(Box<Condition>),
    /// At least one of the conditions holds. Written `{"any": [...]}`.
    Any(Vec<Condition>),
    /// Every one of the conditions holds. Written `{"all": [...]}`.
    All(Vec<Condition>),
    /// A condition over a path into the world's state.
    Path(PathCondition),
}

impl Condition {
    /// Reads the text form. Unknown shapes are an error so a typo in a pack is
    /// caught when it loads.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix('!') {
            let inner = Self::parse(rest)?;
            return Ok(match inner {
                Condition::Not(_) => return Err(format!("condition '{text}' negates twice")),
                other => Condition::Not(Box::new(other)),
            });
        }
        if let Some(item) = text.strip_prefix("item:") {
            return Self::named(item, text).map(|item| Condition::Carries(item.to_string()));
        }
        if let Some(item) = text.strip_prefix("here:") {
            return if item == "trace_mark" {
                Ok(Condition::TraceMarkHere)
            } else {
                Self::named(item, text).map(|item| Condition::ItemHere(item.to_string()))
            };
        }
        if let Some(rest) = text.strip_prefix("feature:") {
            let (feature, state) = rest
                .split_once('=')
                .ok_or_else(|| format!("'{text}' needs a state, as feature:lamp=dim"))?;
            let feature = feature.trim();
            let valid_feature = !feature.is_empty()
                && feature.split('/').count() <= 2
                && feature
                    .split('/')
                    .all(|part| Self::named(part, text).is_ok());
            if !valid_feature || Self::named(state, text).is_err() {
                return Err(format!("'{text}' is not a condition"));
            }
            return Ok(Condition::FeatureIs {
                feature: feature.to_string(),
                state: state.trim().to_string(),
            });
        }
        if let Some(actor) = text.strip_prefix("defeated:") {
            return Self::named(actor, text).map(|actor| Condition::Defeated(actor.to_string()));
        }
        if let Some((name, value)) = text.split_once('=') {
            return Self::named(name, text).map(|name| Condition::StoryVarIs {
                name: name.to_string(),
                value: value.trim().to_string(),
            });
        }
        Self::named(text, text).map(|name| Condition::StoryVar(name.to_string()))
    }

    fn named<'a>(name: &'a str, whole: &str) -> Result<&'a str, String> {
        let name = name.trim();
        let valid = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'));
        if valid {
            Ok(name)
        } else {
            Err(format!("'{whole}' is not a condition"))
        }
    }

    /// The text form, when the condition has one.
    pub fn to_text(&self) -> Option<String> {
        Some(match self {
            Condition::StoryVar(name) => name.clone(),
            Condition::StoryVarIs { name, value } => format!("{name}={value}"),
            Condition::Carries(item) => format!("item:{item}"),
            Condition::ItemHere(item) => format!("here:{item}"),
            Condition::TraceMarkHere => "here:trace_mark".to_string(),
            Condition::Defeated(actor) => format!("defeated:{actor}"),
            Condition::FeatureIs { feature, state } => format!("feature:{feature}={state}"),
            Condition::Not(inner) => format!("!{}", inner.to_text()?),
            Condition::Path(_) | Condition::Any(_) | Condition::All(_) => return None,
        })
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ConditionRepr {
    Text(String),
    Path(PathCondition),
    Any { any: Vec<Condition> },
    All { all: Vec<Condition> },
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match ConditionRepr::deserialize(deserializer)? {
            ConditionRepr::Text(text) => Condition::parse(&text).map_err(D::Error::custom),
            ConditionRepr::Path(path) => Ok(Condition::Path(path)),
            ConditionRepr::Any { any } => Ok(Condition::Any(any)),
            ConditionRepr::All { all } => Ok(Condition::All(all)),
        }
    }
}

impl Serialize for Condition {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Condition::Path(path) => path.serialize(serializer),
            Condition::Any(any) => serde_json::json!({ "any": any }).serialize(serializer),
            Condition::All(all) => serde_json::json!({ "all": all }).serialize(serializer),
            other => other
                .to_text()
                .expect("every non-path condition has a text form")
                .serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_forms_read_and_write_back_the_same() {
        for text in [
            "shaman_defeated",
            "!zayd_rescued",
            "phase=night",
            "!phase=night",
            "item:chalk",
            "!item:chalk",
            "here:lamp",
            "here:trace_mark",
            "defeated:warden",
            "!defeated:warden",
            "feature:lamp=dim",
            "feature:hall/white-noise=quiet",
            "!feature:lamp=off",
        ] {
            let condition = Condition::parse(text).expect(text);
            assert_eq!(condition.to_text().as_deref(), Some(text));
        }
    }

    #[test]
    fn a_path_condition_is_an_object() {
        let condition: Condition = serde_json::from_str(
            r#"{"path":"actor_stats.wren.points","operator":"gte","value":8}"#,
        )
        .expect("object form");
        assert!(matches!(condition, Condition::Path(_)));
        let again = serde_json::to_value(&condition).expect("serializes");
        assert_eq!(again["operator"], "gte");
    }

    #[test]
    fn text_and_object_both_load_from_json() {
        let list: Vec<Condition> =
            serde_json::from_str(r#"["door_open", {"path":"a.b","operator":"equal","value":1}]"#)
                .expect("a mixed list");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], Condition::StoryVar("door_open".to_string()));
    }

    #[test]
    fn any_and_all_group_other_conditions() {
        let any: Condition = serde_json::from_str(r#"{"any": ["a", "!b"]}"#).expect("any form");
        assert_eq!(
            any,
            Condition::Any(vec![
                Condition::StoryVar("a".to_string()),
                Condition::Not(Box::new(Condition::StoryVar("b".to_string()))),
            ])
        );
        let all: Condition = serde_json::from_str(r#"{"all": ["a", "b"]}"#).expect("all form");
        assert!(matches!(all, Condition::All(_)));
        let again = serde_json::to_value(&any).expect("serializes");
        assert_eq!(again["any"][1], "!b");
    }

    #[test]
    fn nonsense_is_an_error_when_it_loads() {
        assert!(Condition::parse("two words").is_err());
        assert!(Condition::parse("!!twice").is_err());
        assert!(Condition::parse("item:").is_err());
        assert!(Condition::parse("feature:lamp").is_err());
        assert!(Condition::parse("feature:a/b/c=on").is_err());
        assert!(serde_json::from_str::<Condition>("\"has space\"").is_err());
    }
}
