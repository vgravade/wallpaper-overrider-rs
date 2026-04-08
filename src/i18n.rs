use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    French,
}

impl Language {
    pub fn detect() -> Self {
        let Some(locale) = sys_locale::get_locale() else {
            return Self::English;
        };

        if locale.to_lowercase().starts_with("fr") {
            Self::French
        } else {
            Self::English
        }
    }

    pub fn app_title(self) -> &'static str {
        match self {
            Self::English => "Wallpaper Overrider",
            Self::French => "Forçage du fond d'écran",
        }
    }

    pub fn heading(self) -> &'static str {
        match self {
            Self::English => "Override Wallpaper",
            Self::French => "Forcer le fond d'écran",
        }
    }

    pub fn choose_picture(self) -> &'static str {
        match self {
            Self::English => "Choose your picture",
            Self::French => "Choisissez votre image",
        }
    }

    pub fn empty_path(self) -> &'static str {
        match self {
            Self::English => "(none)",
            Self::French => "(aucun)",
        }
    }

    pub fn images_filter(self) -> &'static str {
        match self {
            Self::English => "Images",
            Self::French => "Images",
        }
    }

    pub fn browse_button(self) -> &'static str {
        match self {
            Self::English => "Browse...",
            Self::French => "Parcourir...",
        }
    }

    pub fn choose_fit(self) -> &'static str {
        match self {
            Self::English => "Choose a fit",
            Self::French => "Choisissez un ajustement",
        }
    }

    pub fn preview_loading(self) -> &'static str {
        match self {
            Self::English => "Loading preview...",
            Self::French => "Chargement de l'aperçu...",
        }
    }

    pub fn apply_button(self) -> &'static str {
        match self {
            Self::English => "  Apply  ",
            Self::French => "  Appliquer  ",
        }
    }

    pub fn close_button(self) -> &'static str {
        match self {
            Self::English => "  Close  ",
            Self::French => "  Fermer  ",
        }
    }

    pub fn no_wallpaper_selected(self) -> &'static str {
        match self {
            Self::English => "No wallpaper selected.",
            Self::French => "Aucun fond d'écran sélectionné.",
        }
    }

    pub fn file_no_longer_exists(self) -> &'static str {
        match self {
            Self::English => "File no longer exists.",
            Self::French => "Le fichier n'existe plus.",
        }
    }

    pub fn failed_load_image(self, err: impl Display) -> String {
        match self {
            Self::English => format!("Failed to load image: {err}"),
            Self::French => format!("Impossible de charger l'image : {err}"),
        }
    }

    pub fn failed_resolve_sid(self, err: impl Display) -> String {
        match self {
            Self::English => format!("Failed to resolve current SID: {err}"),
            Self::French => format!("Impossible de récupérer le SID courant : {err}"),
        }
    }

    pub fn wallpaper_applied(self) -> &'static str {
        match self {
            Self::English => "Wallpaper applied successfully.",
            Self::French => "Fond d'écran appliqué avec succès.",
        }
    }

    pub fn failed_to_apply(self, err: impl Display) -> String {
        match self {
            Self::English => format!("Failed to apply: {err}"),
            Self::French => format!("Impossible d'appliquer : {err}"),
        }
    }

    pub fn elevated_broker_failed(self, code: u32) -> String {
        match self {
            Self::English => format!("Elevated broker failed with exit code {code}."),
            Self::French => {
                format!("Le broker élevé a échoué avec le code de sortie {code}.")
            }
        }
    }

    pub fn elevation_failed(self, err: impl Display) -> String {
        match self {
            Self::English => format!("Elevation failed: {err}"),
            Self::French => format!("L'élévation a échoué : {err}"),
        }
    }
}