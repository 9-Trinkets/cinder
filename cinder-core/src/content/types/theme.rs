use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDefinition {
    #[serde(default = "default_base")]
    pub base: String,
    #[serde(default = "default_surface")]
    pub surface: String,
    #[serde(default = "default_overlay")]
    pub overlay: String,
    #[serde(default = "default_muted")]
    pub muted: String,
    #[serde(default = "default_text")]
    pub text: String,
    #[serde(default = "default_love")]
    pub love: String,
    #[serde(default = "default_gold")]
    pub gold: String,
    #[serde(default = "default_rose")]
    pub rose: String,
    #[serde(default = "default_pine")]
    pub pine: String,
    #[serde(default = "default_foam")]
    pub foam: String,
    #[serde(default = "default_iris")]
    pub iris: String,
    #[serde(default = "default_highlight_high")]
    pub highlight_high: String,
    #[serde(default = "default_crt_glow")]
    pub crt_glow: String,
    #[serde(default = "default_crt_dim")]
    pub crt_dim: String,
    #[serde(default = "default_crt_bez")]
    pub crt_bez: String,
}

impl Default for ThemeDefinition {
    fn default() -> Self {
        Self {
            base: default_base(),
            surface: default_surface(),
            overlay: default_overlay(),
            muted: default_muted(),
            text: default_text(),
            love: default_love(),
            gold: default_gold(),
            rose: default_rose(),
            pine: default_pine(),
            foam: default_foam(),
            iris: default_iris(),
            highlight_high: default_highlight_high(),
            crt_glow: default_crt_glow(),
            crt_dim: default_crt_dim(),
            crt_bez: default_crt_bez(),
        }
    }
}

fn default_base() -> String {
    "#232136".into()
}
fn default_surface() -> String {
    "#2a273f".into()
}
fn default_overlay() -> String {
    "#393552".into()
}
fn default_muted() -> String {
    "#6e6a86".into()
}
fn default_text() -> String {
    "#e0def4".into()
}
fn default_love() -> String {
    "#eb6f92".into()
}
fn default_gold() -> String {
    "#f6c177".into()
}
fn default_rose() -> String {
    "#ea9a97".into()
}
fn default_pine() -> String {
    "#3e8fb0".into()
}
fn default_foam() -> String {
    "#9ccfd8".into()
}
fn default_iris() -> String {
    "#c4a7e7".into()
}
fn default_highlight_high() -> String {
    "#56526e".into()
}
fn default_crt_glow() -> String {
    "#a7f3d0".into()
}
fn default_crt_dim() -> String {
    "#3b8474".into()
}
fn default_crt_bez() -> String {
    "#12141a".into()
}
