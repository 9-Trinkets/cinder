//! Content-pack validation.
//!
//! Each [`ContentValidator`] owns one independent concern and reports every
//! problem it finds into a shared [`Diagnostics`]. The loader runs every
//! validator and aggregates the results, so a malformed pack surfaces its whole
//! error list in a single load instead of one failure at a time.

mod movies;
mod protection;
mod rooms;
mod skills;

use crate::content::types::ContentPack;
use std::error::Error;
use std::fmt;

/// A single content-validation failure, rendered verbatim for pack authors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ContentError {
    message: String,
}

impl ContentError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Every error one validation pass collected, in the order they were found.
///
/// A single failure renders exactly like the historic first-error string so
/// callers that match on it keep working; multiple failures are listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ContentValidationError {
    errors: Vec<ContentError>,
}

impl fmt::Display for ContentValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.errors.as_slice() {
            [only] => write!(f, "{only}"),
            errors => {
                writeln!(f, "content validation failed with {} errors:", errors.len())?;
                for (index, error) in errors.iter().enumerate() {
                    let separator = if index + 1 < errors.len() { "\n" } else { "" };
                    write!(f, "  - {error}{separator}")?;
                }
                Ok(())
            }
        }
    }
}

impl Error for ContentValidationError {}

/// Collects errors across validators instead of short-circuiting on the first.
#[derive(Debug, Default)]
pub(super) struct Diagnostics {
    errors: Vec<ContentError>,
}

impl Diagnostics {
    pub(super) fn push(&mut self, message: impl Into<String>) {
        self.errors.push(ContentError::new(message));
    }

    fn finish(self) -> Result<(), ContentValidationError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(ContentValidationError {
                errors: self.errors,
            })
        }
    }
}

/// One independent validation concern over a fully-loaded [`ContentPack`].
pub(super) trait ContentValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics);
}

fn validators() -> &'static [&'static dyn ContentValidator] {
    &[
        &skills::SkillDeclarationsValidator,
        &skills::ActorSkillAssignmentsValidator,
        &skills::SkillReferencesValidator,
        &skills::CombatActorSkillsValidator,
        &protection::ProtectionRulesValidator,
        &rooms::RoomItemsValidator,
        &movies::MoviesValidator,
    ]
}

/// Runs every registered validator and returns an aggregate error if any failed.
pub(super) fn validate(pack: &ContentPack) -> Result<(), Box<dyn Error>> {
    let mut diagnostics = Diagnostics::default();
    for validator in validators() {
        validator.validate(pack, &mut diagnostics);
    }
    diagnostics.finish().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::SkillDefinition;

    #[test]
    fn single_error_displays_verbatim() {
        let error = ContentValidationError {
            errors: vec![ContentError::new("boom")],
        };
        assert_eq!(error.to_string(), "boom");
    }

    #[test]
    fn multiple_errors_are_all_reported() {
        let error = ContentValidationError {
            errors: vec![ContentError::new("first"), ContentError::new("second")],
        };
        let rendered = error.to_string();
        assert!(rendered.contains("2 errors"));
        assert!(rendered.contains("first"));
        assert!(rendered.contains("second"));
    }

    #[test]
    fn loader_reports_every_skill_error_in_one_pass() {
        let mut pack =
            crate::content::loader::load_named_pack("layla", Some("en")).expect("layla loads");
        pack.skills.skills.push(SkillDefinition::default()); // empty id
        pack.skills.skills.push(SkillDefinition {
            id: "dup".to_string(),
            ..SkillDefinition::default()
        });
        pack.skills.skills.push(SkillDefinition {
            id: "dup".to_string(),
            ..SkillDefinition::default()
        });

        let rendered = validate(&pack).expect_err("pack is invalid").to_string();
        assert!(rendered.contains("empty id"), "{rendered}");
        assert!(
            rendered.contains("declares duplicate skill 'dup'"),
            "{rendered}"
        );
        assert_eq!(rendered.matches("  - ").count(), 2, "{rendered}");
    }
}
