/// Wallpaper display style — mirrors the registry codes used by Windows.
///
/// Registry value `WallpaperStyle` under
/// `Software\Microsoft\Windows\CurrentVersion\Policies\System`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallpaperStyle {
    Center = 0,
    Tile = 1,
    Stretch = 2,
    Fit = 3,
    #[default]
    Fill = 4,
    Span = 5,
}

impl WallpaperStyle {
    /// Numeric string stored in the registry.
    pub fn code(self) -> &'static str {
        match self {
            Self::Center => "0",
            Self::Tile => "1",
            Self::Stretch => "2",
            Self::Fit => "3",
            Self::Fill => "4",
            Self::Span => "5",
        }
    }

    pub fn all() -> &'static [WallpaperStyle] {
        &[
            Self::Center,
            Self::Tile,
            Self::Stretch,
            Self::Fit,
            Self::Fill,
            Self::Span,
        ]
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim() {
            "0" => Some(Self::Center),
            "1" => Some(Self::Tile),
            "2" => Some(Self::Stretch),
            "3" => Some(Self::Fit),
            "4" => Some(Self::Fill),
            "5" => Some(Self::Span),
            _ => None,
        }
    }
}

impl std::str::FromStr for WallpaperStyle {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        // Try numeric first, then enum name.
        if let Some(style) = Self::from_code(s) {
            return Ok(style);
        }
        match s.to_uppercase().as_str() {
            "CENTER" => Ok(Self::Center),
            "TILE" => Ok(Self::Tile),
            "STRETCH" => Ok(Self::Stretch),
            "FIT" => Ok(Self::Fit),
            "FILL" => Ok(Self::Fill),
            "SPAN" => Ok(Self::Span),
            _ => anyhow::bail!("Unknown wallpaper style: {s}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_codes_round_trip() {
        let cases = [
            ("0", WallpaperStyle::Center),
            ("1", WallpaperStyle::Tile),
            ("2", WallpaperStyle::Stretch),
            ("3", WallpaperStyle::Fit),
            ("4", WallpaperStyle::Fill),
            ("5", WallpaperStyle::Span),
        ];

        for (code, style) in cases {
            assert_eq!(style.code(), code);
            assert_eq!(WallpaperStyle::from_code(code), Some(style));
            assert_eq!(code.parse::<WallpaperStyle>().unwrap(), style);
        }
    }

    #[test]
    fn parses_style_names_case_insensitively() {
        let cases = [
            ("center", WallpaperStyle::Center),
            (" TILE ", WallpaperStyle::Tile),
            ("Stretch", WallpaperStyle::Stretch),
            ("fit", WallpaperStyle::Fit),
            ("FILL", WallpaperStyle::Fill),
            ("span", WallpaperStyle::Span),
        ];

        for (input, style) in cases {
            assert_eq!(input.parse::<WallpaperStyle>().unwrap(), style);
        }
    }

    #[test]
    fn rejects_unknown_styles() {
        assert!(WallpaperStyle::from_code("6").is_none());
        assert!("cover".parse::<WallpaperStyle>().is_err());
    }
}
