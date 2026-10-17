-- Account-level display-language preference. NULL means "no preference":
-- new plays then fall back to the pack's own default language.
ALTER TABLE players
ADD COLUMN IF NOT EXISTS preferred_locale VARCHAR(32);
