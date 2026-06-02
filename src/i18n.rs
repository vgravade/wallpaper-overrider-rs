//! Small static localization catalog used by the native UI and error messages.

use crate::wallpaper_style::WallpaperStyle;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    French,
    German,
    Spanish,
    Italian,
    Portuguese,
    Dutch,
    Polish,
    Russian,
    ChineseSimplified,
    Japanese,
    Korean,
}

struct Texts {
    app_title: &'static str,
    choose_picture: &'static str,
    empty_path: &'static str,
    empty_preview_title: &'static str,
    images_filter: &'static str,
    browse_button: &'static str,
    choose_fit: &'static str,
    wallpaper_styles: WallpaperStyleTexts,
    apply_button: &'static str,
    close_button: &'static str,
    applying_wallpaper: &'static str,
    no_changes_to_apply: &'static str,
    no_wallpaper_selected: &'static str,
    file_no_longer_exists: &'static str,
    failed_resolve_sid: &'static str,
    wallpaper_applied: &'static str,
    failed_to_apply: &'static str,
    elevated_broker_failed: CodeMessage,
    elevation_failed: &'static str,
}

struct WallpaperStyleTexts {
    center: &'static str,
    tile: &'static str,
    stretch: &'static str,
    fit: &'static str,
    fill: &'static str,
    span: &'static str,
}

impl WallpaperStyleTexts {
    fn label(&self, style: WallpaperStyle) -> &'static str {
        match style {
            WallpaperStyle::Center => self.center,
            WallpaperStyle::Tile => self.tile,
            WallpaperStyle::Stretch => self.stretch,
            WallpaperStyle::Fit => self.fit,
            WallpaperStyle::Fill => self.fill,
            WallpaperStyle::Span => self.span,
        }
    }
}

struct CodeMessage {
    before_code: &'static str,
    after_code: &'static str,
}

const EN: Texts = Texts {
    app_title: "Wallpaper Overrider",
    choose_picture: "Image:",
    empty_path: "No file selected",
    empty_preview_title: "No preview yet",
    images_filter: "Images",
    browse_button: "Browse...",
    choose_fit: "Style:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "Center",
        tile: "Tile",
        stretch: "Stretch",
        fit: "Fit",
        fill: "Fill",
        span: "Span",
    },
    apply_button: "Apply",
    close_button: "Close",
    applying_wallpaper: "Applying wallpaper...",
    no_changes_to_apply: "No changes to apply.",
    no_wallpaper_selected: "No wallpaper selected.",
    file_no_longer_exists: "File no longer exists.",
    failed_resolve_sid: "Failed to resolve current SID: ",
    wallpaper_applied: "Wallpaper applied successfully.",
    failed_to_apply: "Failed to apply: ",
    elevated_broker_failed: CodeMessage {
        before_code: "Elevated broker failed with exit code ",
        after_code: ".",
    },
    elevation_failed: "Elevation failed: ",
};

const FR: Texts = Texts {
    app_title: "Forçage du fond d'écran",
    choose_picture: "Image :",
    empty_path: "Aucun fichier sélectionné",
    empty_preview_title: "Aucun aperçu",
    images_filter: "Images",
    browse_button: "Parcourir...",
    choose_fit: "Style :",
    wallpaper_styles: WallpaperStyleTexts {
        center: "Centrer",
        tile: "Mosaïque",
        stretch: "Étirer",
        fit: "Ajuster",
        fill: "Remplir",
        span: "Étendre",
    },
    apply_button: "Appliquer",
    close_button: "Fermer",
    applying_wallpaper: "Application du fond d'écran...",
    no_changes_to_apply: "Aucun changement à appliquer.",
    no_wallpaper_selected: "Aucun fond d'écran sélectionné.",
    file_no_longer_exists: "Le fichier n'existe plus.",
    failed_resolve_sid: "Impossible de récupérer le SID courant : ",
    wallpaper_applied: "Fond d'écran appliqué avec succès.",
    failed_to_apply: "Impossible d'appliquer : ",
    elevated_broker_failed: CodeMessage {
        before_code: "Le broker élevé a échoué avec le code de sortie ",
        after_code: ".",
    },
    elevation_failed: "L'élévation a échoué : ",
};

const DE: Texts = Texts {
    app_title: "Wallpaper erzwingen",
    choose_picture: "Bild:",
    empty_path: "Keine Datei ausgewählt",
    empty_preview_title: "Noch keine Vorschau",
    images_filter: "Bilder",
    browse_button: "Durchsuchen...",
    choose_fit: "Stil:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "Zentrieren",
        tile: "Kacheln",
        stretch: "Strecken",
        fit: "Anpassen",
        fill: "Ausfüllen",
        span: "Über mehrere Monitore",
    },
    apply_button: "Anwenden",
    close_button: "Schließen",
    applying_wallpaper: "Hintergrundbild wird angewendet...",
    no_changes_to_apply: "Keine Änderungen anzuwenden.",
    no_wallpaper_selected: "Kein Hintergrundbild ausgewählt.",
    file_no_longer_exists: "Datei existiert nicht mehr.",
    failed_resolve_sid: "Aktuelle SID konnte nicht ermittelt werden: ",
    wallpaper_applied: "Hintergrundbild erfolgreich angewendet.",
    failed_to_apply: "Anwenden fehlgeschlagen: ",
    elevated_broker_failed: CodeMessage {
        before_code: "Erhöhter Broker ist mit Exit-Code ",
        after_code: " fehlgeschlagen.",
    },
    elevation_failed: "Erhöhung fehlgeschlagen: ",
};

const ES: Texts = Texts {
    app_title: "Forzador de fondo de pantalla",
    choose_picture: "Imagen:",
    empty_path: "Ningún archivo seleccionado",
    empty_preview_title: "Sin vista previa",
    images_filter: "Imágenes",
    browse_button: "Examinar...",
    choose_fit: "Estilo:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "Centrar",
        tile: "Mosaico",
        stretch: "Estirar",
        fit: "Ajustar",
        fill: "Rellenar",
        span: "Expandir",
    },
    apply_button: "Aplicar",
    close_button: "Cerrar",
    applying_wallpaper: "Aplicando fondo de pantalla...",
    no_changes_to_apply: "No hay cambios que aplicar.",
    no_wallpaper_selected: "No se ha seleccionado ningún fondo de pantalla.",
    file_no_longer_exists: "El archivo ya no existe.",
    failed_resolve_sid: "No se pudo resolver el SID actual: ",
    wallpaper_applied: "Fondo de pantalla aplicado correctamente.",
    failed_to_apply: "No se pudo aplicar: ",
    elevated_broker_failed: CodeMessage {
        before_code: "El broker elevado falló con el código de salida ",
        after_code: ".",
    },
    elevation_failed: "Error de elevación: ",
};

const IT: Texts = Texts {
    app_title: "Forzatura sfondo",
    choose_picture: "Immagine:",
    empty_path: "Nessun file selezionato",
    empty_preview_title: "Nessuna anteprima",
    images_filter: "Immagini",
    browse_button: "Sfoglia...",
    choose_fit: "Stile:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "Centra",
        tile: "Affianca",
        stretch: "Allunga",
        fit: "Adatta",
        fill: "Riempi",
        span: "Estendi",
    },
    apply_button: "Applica",
    close_button: "Chiudi",
    applying_wallpaper: "Applicazione dello sfondo...",
    no_changes_to_apply: "Nessuna modifica da applicare.",
    no_wallpaper_selected: "Nessuno sfondo selezionato.",
    file_no_longer_exists: "Il file non esiste più.",
    failed_resolve_sid: "Impossibile risolvere il SID corrente: ",
    wallpaper_applied: "Sfondo applicato correttamente.",
    failed_to_apply: "Impossibile applicare: ",
    elevated_broker_failed: CodeMessage {
        before_code: "Il broker elevato non è riuscito con codice di uscita ",
        after_code: ".",
    },
    elevation_failed: "Elevazione non riuscita: ",
};

const PT: Texts = Texts {
    app_title: "Forçador de papel de parede",
    choose_picture: "Imagem:",
    empty_path: "Nenhum arquivo selecionado",
    empty_preview_title: "Ainda sem pré-visualização",
    images_filter: "Imagens",
    browse_button: "Procurar...",
    choose_fit: "Estilo:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "Centralizar",
        tile: "Lado a lado",
        stretch: "Esticar",
        fit: "Ajustar",
        fill: "Preencher",
        span: "Estender",
    },
    apply_button: "Aplicar",
    close_button: "Fechar",
    applying_wallpaper: "Aplicando papel de parede...",
    no_changes_to_apply: "Nenhuma alteração para aplicar.",
    no_wallpaper_selected: "Nenhum papel de parede selecionado.",
    file_no_longer_exists: "O arquivo não existe mais.",
    failed_resolve_sid: "Falha ao resolver o SID atual: ",
    wallpaper_applied: "Papel de parede aplicado com sucesso.",
    failed_to_apply: "Falha ao aplicar: ",
    elevated_broker_failed: CodeMessage {
        before_code: "O broker elevado falhou com o código de saída ",
        after_code: ".",
    },
    elevation_failed: "Falha na elevação: ",
};

const NL: Texts = Texts {
    app_title: "Achtergrond afdwingen",
    choose_picture: "Afbeelding:",
    empty_path: "Geen bestand geselecteerd",
    empty_preview_title: "Nog geen voorbeeld",
    images_filter: "Afbeeldingen",
    browse_button: "Bladeren...",
    choose_fit: "Stijl:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "Centreren",
        tile: "Tegels",
        stretch: "Uitrekken",
        fit: "Passend",
        fill: "Vullen",
        span: "Spreiden",
    },
    apply_button: "Toepassen",
    close_button: "Sluiten",
    applying_wallpaper: "Achtergrond wordt toegepast...",
    no_changes_to_apply: "Geen wijzigingen om toe te passen.",
    no_wallpaper_selected: "Geen achtergrond geselecteerd.",
    file_no_longer_exists: "Bestand bestaat niet meer.",
    failed_resolve_sid: "Kan huidige SID niet bepalen: ",
    wallpaper_applied: "Achtergrond succesvol toegepast.",
    failed_to_apply: "Toepassen mislukt: ",
    elevated_broker_failed: CodeMessage {
        before_code: "Verhoogde broker is mislukt met afsluitcode ",
        after_code: ".",
    },
    elevation_failed: "Verhoging mislukt: ",
};

const PL: Texts = Texts {
    app_title: "Wymuszenie tapety",
    choose_picture: "Obraz:",
    empty_path: "Nie wybrano pliku",
    empty_preview_title: "Brak podglądu",
    images_filter: "Obrazy",
    browse_button: "Przeglądaj...",
    choose_fit: "Styl:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "Wyśrodkuj",
        tile: "Sąsiadująco",
        stretch: "Rozciągnij",
        fit: "Dopasuj",
        fill: "Wypełnij",
        span: "Rozciągnij na monitory",
    },
    apply_button: "Zastosuj",
    close_button: "Zamknij",
    applying_wallpaper: "Stosowanie tapety...",
    no_changes_to_apply: "Brak zmian do zastosowania.",
    no_wallpaper_selected: "Nie wybrano tapety.",
    file_no_longer_exists: "Plik już nie istnieje.",
    failed_resolve_sid: "Nie udało się ustalić bieżącego SID: ",
    wallpaper_applied: "Tapeta została zastosowana.",
    failed_to_apply: "Nie udało się zastosować: ",
    elevated_broker_failed: CodeMessage {
        before_code: "Broker z podwyższonymi uprawnieniami zakończył się kodem ",
        after_code: ".",
    },
    elevation_failed: "Podniesienie uprawnień nie powiodło się: ",
};

const RU: Texts = Texts {
    app_title: "Принудительная смена обоев",
    choose_picture: "Изображение:",
    empty_path: "Файл не выбран",
    empty_preview_title: "Предпросмотра пока нет",
    images_filter: "Изображения",
    browse_button: "Обзор...",
    choose_fit: "Стиль:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "По центру",
        tile: "Замостить",
        stretch: "Растянуть",
        fit: "Вписать",
        fill: "Заполнить",
        span: "На несколько мониторов",
    },
    apply_button: "Применить",
    close_button: "Закрыть",
    applying_wallpaper: "Применение обоев...",
    no_changes_to_apply: "Нет изменений для применения.",
    no_wallpaper_selected: "Обои не выбраны.",
    file_no_longer_exists: "Файл больше не существует.",
    failed_resolve_sid: "Не удалось определить текущий SID: ",
    wallpaper_applied: "Обои успешно применены.",
    failed_to_apply: "Не удалось применить: ",
    elevated_broker_failed: CodeMessage {
        before_code: "Повышенный брокер завершился с кодом ",
        after_code: ".",
    },
    elevation_failed: "Не удалось повысить права: ",
};

const ZH_HANS: Texts = Texts {
    app_title: "壁纸覆盖器",
    choose_picture: "图片：",
    empty_path: "未选择文件",
    empty_preview_title: "暂无预览",
    images_filter: "图片",
    browse_button: "浏览...",
    choose_fit: "样式：",
    wallpaper_styles: WallpaperStyleTexts {
        center: "居中",
        tile: "平铺",
        stretch: "拉伸",
        fit: "适应",
        fill: "填充",
        span: "跨区",
    },
    apply_button: "应用",
    close_button: "关闭",
    applying_wallpaper: "正在应用壁纸...",
    no_changes_to_apply: "没有要应用的更改。",
    no_wallpaper_selected: "未选择壁纸。",
    file_no_longer_exists: "文件已不存在。",
    failed_resolve_sid: "无法解析当前 SID：",
    wallpaper_applied: "壁纸已成功应用。",
    failed_to_apply: "应用失败：",
    elevated_broker_failed: CodeMessage {
        before_code: "提权代理失败，退出代码 ",
        after_code: "。",
    },
    elevation_failed: "提权失败：",
};

const JA: Texts = Texts {
    app_title: "壁紙オーバーライダー",
    choose_picture: "画像：",
    empty_path: "ファイルが選択されていません",
    empty_preview_title: "プレビューはまだありません",
    images_filter: "画像",
    browse_button: "参照...",
    choose_fit: "スタイル:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "中央",
        tile: "並べて表示",
        stretch: "拡大して表示",
        fit: "合わせる",
        fill: "塗りつぶし",
        span: "スパン",
    },
    apply_button: "適用",
    close_button: "閉じる",
    applying_wallpaper: "壁紙を適用しています...",
    no_changes_to_apply: "適用する変更はありません。",
    no_wallpaper_selected: "壁紙が選択されていません。",
    file_no_longer_exists: "ファイルは存在しなくなりました。",
    failed_resolve_sid: "現在の SID を解決できませんでした: ",
    wallpaper_applied: "壁紙を適用しました。",
    failed_to_apply: "適用に失敗しました: ",
    elevated_broker_failed: CodeMessage {
        before_code: "昇格ブローカーが終了コード ",
        after_code: " で失敗しました。",
    },
    elevation_failed: "昇格に失敗しました: ",
};

const KO: Texts = Texts {
    app_title: "배경화면 오버라이더",
    choose_picture: "이미지:",
    empty_path: "선택한 파일 없음",
    empty_preview_title: "아직 미리 보기 없음",
    images_filter: "이미지",
    browse_button: "찾아보기...",
    choose_fit: "스타일:",
    wallpaper_styles: WallpaperStyleTexts {
        center: "가운데",
        tile: "바둑판식",
        stretch: "늘이기",
        fit: "맞춤",
        fill: "채우기",
        span: "스팬",
    },
    apply_button: "적용",
    close_button: "닫기",
    applying_wallpaper: "배경화면 적용 중...",
    no_changes_to_apply: "적용할 변경 사항이 없습니다.",
    no_wallpaper_selected: "선택한 배경화면이 없습니다.",
    file_no_longer_exists: "파일이 더 이상 존재하지 않습니다.",
    failed_resolve_sid: "현재 SID를 확인하지 못했습니다: ",
    wallpaper_applied: "배경화면이 적용되었습니다.",
    failed_to_apply: "적용 실패: ",
    elevated_broker_failed: CodeMessage {
        before_code: "상승된 브로커가 종료 코드 ",
        after_code: "(으)로 실패했습니다.",
    },
    elevation_failed: "권한 상승 실패: ",
};

impl Language {
    pub fn detect() -> Self {
        let Some(locale) = sys_locale::get_locale() else {
            return Self::English;
        };

        Self::from_locale(&locale)
    }

    fn from_locale(locale: &str) -> Self {
        let locale = locale.to_lowercase();
        let language = locale.split(['-', '_']).next().unwrap_or_default();

        match language {
            "fr" => Self::French,
            "de" => Self::German,
            "es" => Self::Spanish,
            "it" => Self::Italian,
            "pt" => Self::Portuguese,
            "nl" => Self::Dutch,
            "pl" => Self::Polish,
            "ru" => Self::Russian,
            "zh" => Self::ChineseSimplified,
            "ja" => Self::Japanese,
            "ko" => Self::Korean,
            _ => Self::English,
        }
    }

    fn texts(self) -> &'static Texts {
        match self {
            Self::English => &EN,
            Self::French => &FR,
            Self::German => &DE,
            Self::Spanish => &ES,
            Self::Italian => &IT,
            Self::Portuguese => &PT,
            Self::Dutch => &NL,
            Self::Polish => &PL,
            Self::Russian => &RU,
            Self::ChineseSimplified => &ZH_HANS,
            Self::Japanese => &JA,
            Self::Korean => &KO,
        }
    }

    pub fn app_title(self) -> &'static str {
        self.texts().app_title
    }

    pub fn choose_picture(self) -> &'static str {
        self.texts().choose_picture
    }

    pub fn empty_path(self) -> &'static str {
        self.texts().empty_path
    }

    pub fn empty_preview_title(self) -> &'static str {
        self.texts().empty_preview_title
    }

    pub fn images_filter(self) -> &'static str {
        self.texts().images_filter
    }

    pub fn browse_button(self) -> &'static str {
        self.texts().browse_button
    }

    pub fn choose_fit(self) -> &'static str {
        self.texts().choose_fit
    }

    pub fn wallpaper_style(self, style: WallpaperStyle) -> &'static str {
        self.texts().wallpaper_styles.label(style)
    }

    pub fn apply_button(self) -> &'static str {
        self.texts().apply_button
    }

    pub fn close_button(self) -> &'static str {
        self.texts().close_button
    }

    pub fn applying_wallpaper(self) -> &'static str {
        self.texts().applying_wallpaper
    }

    pub fn no_changes_to_apply(self) -> &'static str {
        self.texts().no_changes_to_apply
    }

    pub fn no_wallpaper_selected(self) -> &'static str {
        self.texts().no_wallpaper_selected
    }

    pub fn file_no_longer_exists(self) -> &'static str {
        self.texts().file_no_longer_exists
    }

    pub fn failed_resolve_sid(self, err: impl Display) -> String {
        format!("{}{err}", self.texts().failed_resolve_sid)
    }

    pub fn wallpaper_applied(self) -> &'static str {
        self.texts().wallpaper_applied
    }

    pub fn failed_to_apply(self, err: impl Display) -> String {
        format!("{}{err}", self.texts().failed_to_apply)
    }

    pub fn elevated_broker_failed(self, code: u32) -> String {
        let message = &self.texts().elevated_broker_failed;
        format!("{}{code}{}", message.before_code, message.after_code)
    }

    pub fn elevation_failed(self, err: impl Display) -> String {
        format!("{}{err}", self.texts().elevation_failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_locale_prefixes() {
        let cases = [
            ("en-US", Language::English),
            ("fr-FR", Language::French),
            ("de_DE", Language::German),
            ("es-ES", Language::Spanish),
            ("it-IT", Language::Italian),
            ("pt-BR", Language::Portuguese),
            ("nl-NL", Language::Dutch),
            ("pl-PL", Language::Polish),
            ("ru-RU", Language::Russian),
            ("zh-CN", Language::ChineseSimplified),
            ("ja-JP", Language::Japanese),
            ("ko-KR", Language::Korean),
        ];

        for (locale, language) in cases {
            assert_eq!(Language::from_locale(locale), language);
        }
    }

    #[test]
    fn falls_back_to_english_for_unknown_locale() {
        assert_eq!(Language::from_locale("sv-SE"), Language::English);
        assert_eq!(Language::from_locale(""), Language::English);
    }

    #[test]
    fn returns_style_labels_from_catalog() {
        assert_eq!(
            Language::French.wallpaper_style(WallpaperStyle::Tile),
            "Mosaïque"
        );
        assert_eq!(
            Language::German.wallpaper_style(WallpaperStyle::Span),
            "Über mehrere Monitore"
        );
    }
}
