use crate::i18n::Language;

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

    pub fn label(self, lang: Language) -> &'static str {
        match lang {
            Language::English => match self {
                Self::Center => "Center",
                Self::Tile => "Tile",
                Self::Stretch => "Stretch",
                Self::Fit => "Fit",
                Self::Fill => "Fill",
                Self::Span => "Span",
            },
            Language::French => match self {
                Self::Center => "Centrer",
                Self::Tile => "Mosaïque",
                Self::Stretch => "Étirer",
                Self::Fit => "Ajuster",
                Self::Fill => "Remplir",
                Self::Span => "Étendre",
            },
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
