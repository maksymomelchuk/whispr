use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCategory {
    PersonalMessaging,
    WorkMessaging,
    Email,
    Code,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TonePreset {
    Casual,
    Formal,
    TechnicalCasing,
    Neutral,
}

pub fn preset_for_category(category: AppCategory) -> TonePreset {
    match category {
        AppCategory::PersonalMessaging => TonePreset::Casual,
        AppCategory::Email => TonePreset::Formal,
        AppCategory::Code => TonePreset::TechnicalCasing,
        AppCategory::WorkMessaging | AppCategory::Other => TonePreset::Neutral,
    }
}

pub const TONE_UNTOUCHABLE_CLAUSE: &str =
    "Grammar, phrasing, and word choice are untouchable — adjust punctuation, capitalization, and line breaks only.";

pub fn tone_directive(preset: TonePreset) -> Option<&'static str> {
    match preset {
        TonePreset::Casual => Some(
            "Tone: casual. Omit the terminal period on short messages. Do not capitalize sentence fragments. Contractions preferred. Grammar, phrasing, and word choice are untouchable — adjust punctuation, capitalization, and line breaks only.",
        ),
        TonePreset::Formal => Some(
            "Tone: formal. End each sentence with a period. Capitalize the first word of every sentence. Use complete sentences. Grammar, phrasing, and word choice are untouchable — adjust punctuation, capitalization, and line breaks only.",
        ),
        TonePreset::TechnicalCasing => Some(
            "Tone: technical. Preserve all technical identifiers exactly as spoken, including camelCase, snake_case, PascalCase, and ALL_CAPS. Do not add punctuation after identifiers or code tokens. Grammar, phrasing, and word choice are untouchable — adjust punctuation, capitalization, and line breaks only.",
        ),
        TonePreset::Neutral => None,
    }
}

fn taxonomy() -> &'static HashMap<&'static str, AppCategory> {
    static MAP: OnceLock<HashMap<&'static str, AppCategory>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();

        // Personal messaging — macOS bundle IDs
        m.insert("com.apple.MobileSMS", AppCategory::PersonalMessaging);
        m.insert("com.apple.iChat", AppCategory::PersonalMessaging);
        m.insert("com.facebook.archon", AppCategory::PersonalMessaging);
        m.insert("com.facebook.Messenger", AppCategory::PersonalMessaging);
        m.insert("com.whatsapp.WhatsApp", AppCategory::PersonalMessaging);
        m.insert("org.telegram.desktop", AppCategory::PersonalMessaging);
        m.insert("ru.keepcoder.Telegram", AppCategory::PersonalMessaging);
        m.insert("com.viber.desktop", AppCategory::PersonalMessaging);
        m.insert(
            "org.whispersystems.signal-desktop",
            AppCategory::PersonalMessaging,
        );

        // Personal messaging — Windows exe stems
        m.insert("WhatsApp", AppCategory::PersonalMessaging);
        m.insert("Telegram", AppCategory::PersonalMessaging);
        m.insert("Signal", AppCategory::PersonalMessaging);
        m.insert("Messenger", AppCategory::PersonalMessaging);

        // Work messaging — macOS bundle IDs
        m.insert("com.tinyspeck.slackmacgap", AppCategory::WorkMessaging);
        m.insert("com.microsoft.teams", AppCategory::WorkMessaging);
        m.insert("com.microsoft.teams2", AppCategory::WorkMessaging);
        m.insert("us.zoom.xos", AppCategory::WorkMessaging);
        m.insert("com.webex.meetingmanager", AppCategory::WorkMessaging);
        m.insert("com.discord.app", AppCategory::WorkMessaging);

        // Work messaging — Windows exe stems
        m.insert("Slack", AppCategory::WorkMessaging);
        m.insert("Teams", AppCategory::WorkMessaging);
        m.insert("Zoom", AppCategory::WorkMessaging);
        m.insert("Discord", AppCategory::WorkMessaging);

        // Email — macOS bundle IDs
        m.insert("com.apple.mail", AppCategory::Email);
        m.insert("com.microsoft.outlook", AppCategory::Email);
        m.insert("com.readitlater.Airmail5", AppCategory::Email);
        m.insert("com.mimestream.Mimestream", AppCategory::Email);
        m.insert("com.hnc.Spark", AppCategory::Email);
        m.insert("org.mozilla.thunderbird", AppCategory::Email);

        // Email — Windows exe stems
        m.insert("OUTLOOK", AppCategory::Email);
        m.insert("Outlook", AppCategory::Email);
        m.insert("Thunderbird", AppCategory::Email);

        // Code — macOS bundle IDs
        m.insert("com.apple.dt.Xcode", AppCategory::Code);
        m.insert("com.microsoft.VSCode", AppCategory::Code);
        m.insert("com.microsoft.VSCodeInsiders", AppCategory::Code);
        m.insert("com.todesktop.230313mzl4w4u92", AppCategory::Code); // Cursor
        m.insert("com.jetbrains.intellij", AppCategory::Code);
        m.insert("com.jetbrains.pycharm", AppCategory::Code);
        m.insert("com.jetbrains.webstorm", AppCategory::Code);
        m.insert("com.jetbrains.goland", AppCategory::Code);
        m.insert("com.jetbrains.clion", AppCategory::Code);
        m.insert("com.jetbrains.rider", AppCategory::Code);
        m.insert("com.jetbrains.rubymine", AppCategory::Code);
        m.insert("com.jetbrains.datagrip", AppCategory::Code);
        m.insert("com.apple.Terminal", AppCategory::Code);
        m.insert("com.googlecode.iterm2", AppCategory::Code);
        m.insert("dev.warp.Warp-Stable", AppCategory::Code);
        m.insert("dev.warp.Warp-Preview", AppCategory::Code);
        m.insert("com.github.GitHubClient", AppCategory::Code);
        m.insert("com.sublimetext.3", AppCategory::Code);
        m.insert("com.sublimetext.4", AppCategory::Code);
        m.insert("com.panic.Nova", AppCategory::Code);
        m.insert("org.gnu.emacs", AppCategory::Code);
        m.insert("com.vim.Vim", AppCategory::Code);
        m.insert("com.neovim.nvim", AppCategory::Code);

        // Code — Windows exe stems
        m.insert("Code", AppCategory::Code); // VS Code
        m.insert("cursor", AppCategory::Code);
        m.insert("WindowsTerminal", AppCategory::Code);
        m.insert("cmd", AppCategory::Code);
        m.insert("powershell", AppCategory::Code);
        m.insert("pwsh", AppCategory::Code);

        m
    })
}

/// Returns the `AppCategory` for a given app identifier string. The identifier
/// is either a macOS bundle ID (e.g. "com.apple.mail") or a Windows exe stem
/// (e.g. "Code"). Unknown identifiers map to `AppCategory::Other`.
pub fn categorize_app(identifier: &str) -> AppCategory {
    taxonomy()
        .get(identifier)
        .copied()
        .unwrap_or(AppCategory::Other)
}

/// Resolves the effective tone directive for a bundle ID, consulting per-app
/// overrides before falling back to the built-in taxonomy.
/// Returns `None` when `bundle_id` is `None` or the resolved preset has no directive.
pub fn resolve_tone(
    bundle_id: Option<&str>,
    overrides: &BTreeMap<String, TonePreset>,
) -> Option<&'static str> {
    let id = bundle_id?;
    let preset = overrides
        .get(id)
        .copied()
        .unwrap_or_else(|| preset_for_category(categorize_app(id)));
    tone_directive(preset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_app_maps_to_formal_tone() {
        let category = categorize_app("com.apple.mail");
        assert_eq!(category, AppCategory::Email);
        let preset = preset_for_category(category);
        assert_eq!(preset, TonePreset::Formal);
        let directive = tone_directive(preset);
        assert!(directive.is_some());
        let d = directive.unwrap();
        assert!(d.contains("formal") || d.contains("Tone: formal"));
        assert!(d.contains(TONE_UNTOUCHABLE_CLAUSE));
    }

    #[test]
    fn personal_messaging_app_maps_to_casual_tone() {
        let category = categorize_app("com.apple.MobileSMS");
        assert_eq!(category, AppCategory::PersonalMessaging);
        let preset = preset_for_category(category);
        assert_eq!(preset, TonePreset::Casual);
        let directive = tone_directive(preset);
        assert!(directive.is_some());
        let d = directive.unwrap();
        assert!(d.contains("casual") || d.contains("Tone: casual"));
        assert!(d.contains(TONE_UNTOUCHABLE_CLAUSE));
    }

    #[test]
    fn code_editor_maps_to_technical_casing_tone() {
        let category = categorize_app("com.microsoft.VSCode");
        assert_eq!(category, AppCategory::Code);
        let preset = preset_for_category(category);
        assert_eq!(preset, TonePreset::TechnicalCasing);
        let directive = tone_directive(preset);
        assert!(directive.is_some());
        let d = directive.unwrap();
        assert!(d.contains("technical") || d.contains("Tone: technical"));
        assert!(d.contains(TONE_UNTOUCHABLE_CLAUSE));
    }

    #[test]
    fn work_messaging_app_maps_to_neutral_tone() {
        let category = categorize_app("com.tinyspeck.slackmacgap");
        assert_eq!(category, AppCategory::WorkMessaging);
        let preset = preset_for_category(category);
        assert_eq!(preset, TonePreset::Neutral);
        assert!(tone_directive(preset).is_none());
    }

    #[test]
    fn unknown_app_maps_to_other_and_neutral_tone() {
        let category = categorize_app("com.unknown.someapp");
        assert_eq!(category, AppCategory::Other);
        let preset = preset_for_category(category);
        assert_eq!(preset, TonePreset::Neutral);
        assert!(tone_directive(preset).is_none());
    }

    #[test]
    fn neutral_preset_has_no_directive() {
        assert!(tone_directive(TonePreset::Neutral).is_none());
    }

    #[test]
    fn non_neutral_directives_all_contain_untouchable_clause() {
        for preset in [
            TonePreset::Casual,
            TonePreset::Formal,
            TonePreset::TechnicalCasing,
        ] {
            let d = tone_directive(preset).expect("non-neutral preset should have a directive");
            assert!(
                d.contains(TONE_UNTOUCHABLE_CLAUSE),
                "{preset:?} directive missing untouchable clause"
            );
        }
    }

    #[test]
    fn telegram_is_personal_messaging() {
        assert_eq!(
            categorize_app("org.telegram.desktop"),
            AppCategory::PersonalMessaging
        );
    }

    #[test]
    fn zoom_is_work_messaging() {
        assert_eq!(categorize_app("us.zoom.xos"), AppCategory::WorkMessaging);
    }

    #[test]
    fn windows_vscode_exe_is_code() {
        assert_eq!(categorize_app("Code"), AppCategory::Code);
    }

    #[test]
    fn windows_outlook_exe_is_email() {
        assert_eq!(categorize_app("Outlook"), AppCategory::Email);
    }

    #[test]
    fn override_preset_takes_precedence_over_taxonomy() {
        let mut overrides = BTreeMap::new();
        // email normally maps to Formal — override to Casual
        overrides.insert("com.apple.mail".to_string(), TonePreset::Casual);

        let default_dir = resolve_tone(Some("com.apple.mail"), &BTreeMap::new());
        let override_dir = resolve_tone(Some("com.apple.mail"), &overrides);

        assert!(
            default_dir.unwrap().contains("formal"),
            "taxonomy: email → formal"
        );
        assert!(
            override_dir.unwrap().contains("casual"),
            "override: email → casual"
        );
    }

    #[test]
    fn override_to_neutral_suppresses_directive() {
        let mut overrides = BTreeMap::new();
        overrides.insert("com.apple.mail".to_string(), TonePreset::Neutral);

        assert!(resolve_tone(Some("com.apple.mail"), &overrides).is_none());
    }

    #[test]
    fn resolve_tone_returns_none_for_missing_bundle_id() {
        assert!(resolve_tone(None, &BTreeMap::new()).is_none());
    }

    #[test]
    fn tone_preset_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&TonePreset::TechnicalCasing).unwrap(),
            "\"technical_casing\""
        );
        assert_eq!(
            serde_json::to_string(&TonePreset::Casual).unwrap(),
            "\"casual\""
        );
    }
}
