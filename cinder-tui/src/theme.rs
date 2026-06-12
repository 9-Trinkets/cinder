use cinder_core::ThemeDefinition;
use ratatui::style::Color;

pub struct Theme {
    pub base: Color,
    pub surface: Color,
    pub overlay: Color,
    pub muted: Color,
    pub text: Color,
    pub love: Color,
    pub gold: Color,
    pub rose: Color,
    pub pine: Color,
    pub foam: Color,
    pub iris: Color,
    pub highlight_high: Color,
    pub crt_glow: Color,
    pub crt_dim: Color,
    pub crt_bez: Color,
}

impl From<&ThemeDefinition> for Theme {
    fn from(def: &ThemeDefinition) -> Self {
        Self {
            base: parse_hex(&def.base),
            surface: parse_hex(&def.surface),
            overlay: parse_hex(&def.overlay),
            muted: parse_hex(&def.muted),
            text: parse_hex(&def.text),
            love: parse_hex(&def.love),
            gold: parse_hex(&def.gold),
            rose: parse_hex(&def.rose),
            pine: parse_hex(&def.pine),
            foam: parse_hex(&def.foam),
            iris: parse_hex(&def.iris),
            highlight_high: parse_hex(&def.highlight_high),
            crt_glow: parse_hex(&def.crt_glow),
            crt_dim: parse_hex(&def.crt_dim),
            crt_bez: parse_hex(&def.crt_bez),
        }
    }
}

fn parse_hex(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let err = || format!("invalid theme color: {hex}");
    let r = u8::from_str_radix(hex.get(0..2).expect(&err()), 16).expect(&err());
    let g = u8::from_str_radix(hex.get(2..4).expect(&err()), 16).expect(&err());
    let b = u8::from_str_radix(hex.get(4..6).expect(&err()), 16).expect(&err());
    Color::Rgb(r, g, b)
}
