use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableScope {
    Game,
    Act,
    Stage,
}

impl Default for VariableScope {
    fn default() -> Self {
        Self::Game
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableType {
    String,
    Bool,
    Int,
}

impl Default for VariableType {
    fn default() -> Self {
        Self::String
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDeclaration {
    #[serde(default)]
    pub var_type: VariableType,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub scope: VariableScope,
}

#[derive(Debug, Clone, Serialize)]
pub struct VariableStore {
    values: BTreeMap<String, String>,
    #[serde(skip)]
    declarations: BTreeMap<String, VariableDeclaration>,
}

impl Default for VariableStore {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            declarations: BTreeMap::new(),
        }
    }
}

impl<'de> Deserialize<'de> for VariableStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VariableStoreVisitor;

        impl<'de> Visitor<'de> for VariableStoreVisitor {
            type Value = VariableStore;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a flat map or {\"values\": {...}}")
            }

            fn visit_map<M>(self, mut map: M) -> Result<VariableStore, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut values = BTreeMap::new();
                let mut wrapped_values = None;

                while let Some(key) = map.next_key::<String>()? {
                    if key == "values" {
                        wrapped_values = Some(map.next_value::<BTreeMap<String, String>>()?);
                    } else if wrapped_values.is_none() {
                        let val: String = map.next_value()?;
                        values.insert(key, val);
                    } else {
                        let _ = map.next_value::<serde_json::Value>()?;
                    }
                }

                if let Some(wrapped) = wrapped_values {
                    return Ok(VariableStore {
                        values: wrapped,
                        declarations: BTreeMap::new(),
                    });
                }

                Ok(VariableStore {
                    values,
                    declarations: BTreeMap::new(),
                })
            }
        }

        deserializer.deserialize_map(VariableStoreVisitor)
    }
}

#[derive(Debug, Clone)]
pub enum VariableError {
    Undeclared(String),
    TypeMismatch {
        key: String,
        expected: VariableType,
        actual: String,
    },
}

impl std::fmt::Display for VariableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableError::Undeclared(key) => write!(f, "undeclared variable '{key}'"),
            VariableError::TypeMismatch {
                key,
                expected,
                actual,
            } => write!(
                f,
                "variable '{key}' type mismatch: expected {expected:?}, got '{actual}'"
            ),
        }
    }
}

impl std::error::Error for VariableError {}

impl VariableStore {
    pub fn new(declarations: BTreeMap<String, VariableDeclaration>) -> Self {
        let mut store = Self {
            values: BTreeMap::new(),
            declarations,
        };
        for (key, decl) in &store.declarations {
            if let Some(default) = &decl.default {
                store.values.insert(key.clone(), default.clone());
            }
        }
        store
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    pub fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), VariableError> {
        if let Some(decl) = self.declarations.get(key) {
            validate_value(key, value, &decl.var_type)?;
        }
        self.values.insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub fn set_unchecked(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    pub fn clear_scoped(&mut self, scope: VariableScope) {
        let keys_to_remove: Vec<String> = self
            .declarations
            .iter()
            .filter(|(_, decl)| decl.scope == scope)
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys_to_remove {
            self.values.remove(&key);
        }
    }

    pub fn values(&self) -> &BTreeMap<String, String> {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut BTreeMap<String, String> {
        &mut self.values
    }

    pub fn declarations(&self) -> &BTreeMap<String, VariableDeclaration> {
        &self.declarations
    }

    pub fn render_template(&self, template: &str) -> String {
        let mut rendered = template.to_string();
        for (key, value) in &self.values {
            let curly = format!("{{{key}}}");
            if rendered.contains(&curly) {
                rendered = rendered.replace(&curly, value);
            }
            let dollar = format!("${key}");
            if rendered.contains(&dollar) {
                rendered = rendered.replace(&dollar, value);
            }
        }
        rendered
    }
}

fn validate_value(key: &str, value: &str, expected: &VariableType) -> Result<(), VariableError> {
    match expected {
        VariableType::String => Ok(()),
        VariableType::Bool => {
            if value == "true" || value == "false" {
                Ok(())
            } else {
                Err(VariableError::TypeMismatch {
                    key: key.to_string(),
                    expected: *expected,
                    actual: value.to_string(),
                })
            }
        }
        VariableType::Int => {
            if value.parse::<i64>().is_ok() {
                Ok(())
            } else {
                Err(VariableError::TypeMismatch {
                    key: key.to_string(),
                    expected: *expected,
                    actual: value.to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(scope: VariableScope) -> VariableDeclaration {
        VariableDeclaration {
            var_type: VariableType::String,
            default: None,
            scope,
        }
    }

    fn bool_decl(scope: VariableScope) -> VariableDeclaration {
        VariableDeclaration {
            var_type: VariableType::Bool,
            default: Some("false".to_string()),
            scope,
        }
    }

    fn int_decl(default: &str) -> VariableDeclaration {
        VariableDeclaration {
            var_type: VariableType::Int,
            default: Some(default.to_string()),
            scope: VariableScope::Game,
        }
    }

    #[test]
    fn get_returns_none_for_missing() {
        let store = VariableStore::default();
        assert_eq!(store.get("missing"), None);
    }

    #[test]
    fn set_and_get() {
        let mut store = VariableStore::default();
        store.set_unchecked("foo", "bar");
        assert_eq!(store.get("foo"), Some("bar"));
    }

    #[test]
    fn has_key() {
        let mut store = VariableStore::default();
        assert!(!store.has("foo"));
        store.set_unchecked("foo", "bar");
        assert!(store.has("foo"));
    }

    #[test]
    fn declarations_populate_defaults() {
        let mut decls = BTreeMap::new();
        decls.insert(
            "greeting".to_string(),
            VariableDeclaration {
                var_type: VariableType::String,
                default: Some("hello".to_string()),
                scope: VariableScope::Game,
            },
        );
        let store = VariableStore::new(decls);
        assert_eq!(store.get("greeting"), Some("hello"));
    }

    #[test]
    fn clear_scoped_removes_only_matching_scope() {
        let mut decls = BTreeMap::new();
        decls.insert("a".to_string(), decl(VariableScope::Act));
        decls.insert("b".to_string(), decl(VariableScope::Game));
        let mut store = VariableStore::new(decls);
        store.set_unchecked("a", "1");
        store.set_unchecked("b", "2");
        store.clear_scoped(VariableScope::Act);
        assert!(!store.has("a"));
        assert!(store.has("b"));
    }

    #[test]
    fn bool_validation() {
        let mut decls = BTreeMap::new();
        decls.insert("flag".to_string(), bool_decl(VariableScope::Game));
        let mut store = VariableStore::new(decls);
        assert!(store.set("flag", "true").is_ok());
        assert!(store.set("flag", "false").is_ok());
        assert!(store.set("flag", "maybe").is_err());
    }

    #[test]
    fn int_validation() {
        let mut decls = BTreeMap::new();
        decls.insert("count".to_string(), int_decl("0"));
        let mut store = VariableStore::new(decls);
        assert!(store.set("count", "42").is_ok());
        assert!(store.set("count", "-1").is_ok());
        assert!(store.set("count", "abc").is_err());
    }

    #[test]
    fn render_template_curly() {
        let mut store = VariableStore::default();
        store.set_unchecked("name", "Alice");
        assert_eq!(store.render_template("Hello {name}!"), "Hello Alice!");
    }

    #[test]
    fn render_template_dollar() {
        let mut store = VariableStore::default();
        store.set_unchecked("name", "Alice");
        assert_eq!(store.render_template("Hello $name!"), "Hello Alice!");
    }

    #[test]
    fn render_template_mixed() {
        let mut store = VariableStore::default();
        store.set_unchecked("a", "1");
        store.set_unchecked("b", "2");
        assert_eq!(store.render_template("{a} and $b"), "1 and 2");
    }

    #[test]
    fn undeclared_var_allows_set() {
        let store = VariableStore::default();
        let mut store = store;
        assert!(store.set("unknown", "value").is_ok());
    }
}
