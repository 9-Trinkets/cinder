use super::{ContentValidator, Diagnostics};
use crate::content::types::{ContentPack, MovieStyle};

const FONTS: &[&str] = &["", "prose", "mono", "hand"];
const ALIGNS: &[&str] = &["", "center", "left"];
const AMBIENTS: &[&str] = &["", "none", "drift", "snow", "rain", "leaves"];
const REVEALS: &[&str] = &["", "fade", "type"];

/// Checks movies and the stages that play them: a stage can only play a movie
/// that exists, and a card's settings must be ones the client knows.
pub(super) struct MoviesValidator;

impl ContentValidator for MoviesValidator {
    fn validate(&self, pack: &ContentPack, diagnostics: &mut Diagnostics) {
        for stage in &pack.beats.stages {
            if !stage.movie_id.is_empty() && !pack.movies.iter().any(|m| m.id == stage.movie_id) {
                diagnostics.push(format!(
                    "stage '{}' plays unknown movie '{}'",
                    stage.id, stage.movie_id
                ));
            }
        }
        for movie in &pack.movies {
            for frame in &movie.frames {
                if !frame.text_item.is_empty() && pack.item(&frame.text_item).is_none() {
                    diagnostics.push(format!(
                        "movie '{}' takes a frame's text from unknown item '{}'",
                        movie.id, frame.text_item
                    ));
                }
            }
            if movie.style != MovieStyle::Card {
                continue;
            }
            if movie.frames.is_empty() {
                diagnostics.push(format!("card movie '{}' has no frames", movie.id));
            }
            for (setting, value, allowed) in [
                ("font", movie.card.font.as_str(), FONTS),
                ("align", movie.card.align.as_str(), ALIGNS),
                ("ambient", movie.card.ambient.as_str(), AMBIENTS),
                ("reveal", movie.card.reveal.as_str(), REVEALS),
            ] {
                if !allowed.contains(&value) {
                    diagnostics.push(format!(
                        "card movie '{}' has unknown {setting} '{value}'",
                        movie.id
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{
        BeatDefinition, OpeningMovieDefinition, OpeningMovieFrameDefinition,
    };
    use crate::engine::test_fixtures::minimal_test_pack;

    fn card(id: &str, frames: usize) -> OpeningMovieDefinition {
        OpeningMovieDefinition {
            id: id.to_string(),
            style: MovieStyle::Card,
            frames: (0..frames)
                .map(|_| OpeningMovieFrameDefinition {
                    heading: "A date".to_string(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn a_stage_must_play_a_movie_that_exists() {
        let mut pack = minimal_test_pack();
        pack.beats.stages.push(BeatDefinition {
            id: "later".to_string(),
            movie_id: "missing".to_string(),
            ..Default::default()
        });
        let mut diagnostics = Diagnostics::default();
        MoviesValidator.validate(&pack, &mut diagnostics);
        let message = diagnostics.finish().expect_err("unknown movie").to_string();
        assert!(message.contains("unknown movie 'missing'"), "{message}");
    }

    #[test]
    fn card_settings_must_be_known_and_a_card_needs_frames() {
        let mut pack = minimal_test_pack();
        let mut odd = card("odd", 1);
        odd.card.ambient = "fireworks".to_string();
        pack.movies = vec![odd, card("empty", 0)];
        let mut diagnostics = Diagnostics::default();
        MoviesValidator.validate(&pack, &mut diagnostics);
        let message = diagnostics.finish().expect_err("two errors").to_string();
        assert!(message.contains("unknown ambient 'fireworks'"), "{message}");
        assert!(message.contains("'empty' has no frames"), "{message}");
    }

    #[test]
    fn a_frame_cannot_take_its_text_from_an_unknown_item() {
        let mut pack = minimal_test_pack();
        let mut movie = card("quote", 1);
        movie.frames[0].text_item = "missing-page".to_string();
        pack.movies = vec![movie];
        let mut diagnostics = Diagnostics::default();
        MoviesValidator.validate(&pack, &mut diagnostics);
        let message = diagnostics.finish().expect_err("unknown item").to_string();
        assert!(message.contains("unknown item 'missing-page'"), "{message}");
    }

    #[test]
    fn a_plain_card_is_fine() {
        let mut pack = minimal_test_pack();
        pack.movies = vec![card("plain", 2)];
        let mut diagnostics = Diagnostics::default();
        MoviesValidator.validate(&pack, &mut diagnostics);
        assert!(diagnostics.finish().is_ok());
    }
}
