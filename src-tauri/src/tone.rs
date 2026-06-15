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

macro_rules! tone_untouchable_clause {
    () => {
        "Grammar, phrasing, and word choice are untouchable — adjust punctuation, capitalization, and line breaks only."
    };
}

// Without this, a recognizer that already punctuates (e.g. Whisper) leaves the
// cleanup model nothing to "add", and the preservation rules keep it from
// stripping anything — so tone silently no-ops. The directive must claim
// precedence and explicitly license reformatting already-formatted input.
macro_rules! tone_override_clause {
    () => {
        "This overrides the punctuation and capitalization guidance in the rules above, and applies even when the transcript already contains punctuation."
    };
}

pub fn tone_directive(preset: TonePreset) -> Option<&'static str> {
    match preset {
        TonePreset::Casual => Some(concat!(
            "Tone: casual. ",
            tone_override_clause!(),
            " Drop a trailing period on short one- or two-sentence messages, even if one is already present — but keep question marks and exclamation marks; they carry meaning. Lowercase the first word — even if the input capitalizes it — unless it is a proper noun or \"I\". Always keep proper nouns (names, places, brands) and \"I\" capitalized wherever they appear; never lowercase them. Prefer contractions. ",
            tone_untouchable_clause!(),
        )),
        TonePreset::Formal => Some(concat!(
            "Tone: formal. ",
            tone_override_clause!(),
            " End every sentence with a period and capitalize its first word, adding them if missing. Prefer complete sentences over fragments. ",
            tone_untouchable_clause!(),
        )),
        TonePreset::TechnicalCasing => Some(concat!(
            "Tone: technical. ",
            tone_override_clause!(),
            " Interpret spoken casing cues literally inside identifiers: \"underscore\" becomes _ (\"user underscore id\" -> user_id), \"dot\" becomes . (\"config dot ts\" -> config.ts), and \"dash\" or \"hyphen\" becomes - inside a compound identifier (\"feature dash auth\" -> feature-auth). These spoken cues take priority over the default camelCase rule: \"user underscore id\" is user_id, never userId. Preserve any identifier already in camelCase, snake_case, PascalCase, ALL_CAPS, a dotted.path, or file/branch form exactly. Treat shell commands and code expressions as code, not prose: keep a leading command lowercase and add no trailing period (\"git commit -m ...\" stays \"git commit -m ...\", never \"Git commit ... .\"). This overrides the default sentence-capitalization and terminal-punctuation rules. Apply these edits to identifiers and code only — leave ordinary prose wording untouched.",
        )),
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

/// Wraps a user-authored per-app prompt with a precedence preamble so a weak
/// cleanup model still honors it over the base formatting rules. Unlike the
/// presets, a custom prompt carries no "untouchable" guardrail — the user
/// owns the risk of it rewriting content.
fn custom_directive(prompt: &str) -> String {
    format!(
        "Style instruction (this overrides the punctuation, capitalization, and formatting guidance in the rules above, and applies even when the transcript is already formatted): {}",
        prompt.trim()
    )
}

/// Resolves the effective tone directive for a bundle ID. A non-empty per-app
/// custom prompt wins; otherwise a per-app preset override; otherwise the
/// built-in taxonomy. Returns `None` when `bundle_id` is `None` or the resolved
/// preset has no directive (Neutral).
pub fn resolve_tone(
    bundle_id: Option<&str>,
    overrides: &BTreeMap<String, TonePreset>,
    custom_prompts: &BTreeMap<String, String>,
) -> Option<String> {
    let id = bundle_id?;
    if let Some(prompt) = custom_prompts.get(id) {
        if !prompt.trim().is_empty() {
            return Some(custom_directive(prompt));
        }
    }
    let preset = overrides
        .get(id)
        .copied()
        .unwrap_or_else(|| preset_for_category(categorize_app(id)));
    tone_directive(preset).map(|d| d.to_string())
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
        assert!(d.contains(tone_untouchable_clause!()));
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
        assert!(d.contains(tone_untouchable_clause!()));
    }

    #[test]
    fn casual_tone_drops_period_but_keeps_question_and_exclamation_marks() {
        let directive = tone_directive(TonePreset::Casual).unwrap();
        assert!(directive.contains("trailing period"));
        assert!(directive.contains("question marks and exclamation marks"));
    }

    #[test]
    fn casual_tone_keeps_proper_nouns_capitalized() {
        let directive = tone_directive(TonePreset::Casual).unwrap();
        assert!(directive.contains("Lowercase the first word"));
        assert!(directive.contains("keep proper nouns"));
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
    }

    #[test]
    fn technical_tone_interprets_spoken_casing_cues() {
        let d = tone_directive(TonePreset::TechnicalCasing).unwrap();
        assert!(d.contains("user_id"));
        assert!(d.contains("config.ts"));
        assert!(d.contains("leave ordinary prose wording untouched"));
    }

    #[test]
    fn technical_tone_drops_unreliable_all_caps_cue() {
        // The all-caps cue leaked literal "in all caps" into output and corrupted
        // adjacent cues across models; ALL_CAPS identifiers go through Vocabulary.
        let d = tone_directive(TonePreset::TechnicalCasing).unwrap();
        assert!(!d.contains("all caps"));
        assert!(!d.contains("uppercase"));
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
    fn casual_and_formal_directives_contain_untouchable_clause() {
        // Technical is excluded: it deliberately edits identifiers (e.g. "user
        // underscore id" -> user_id), which the untouchable clause forbids.
        for preset in [TonePreset::Casual, TonePreset::Formal] {
            let d = tone_directive(preset).expect("non-neutral preset should have a directive");
            assert!(
                d.contains(tone_untouchable_clause!()),
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

        let default_dir = resolve_tone(Some("com.apple.mail"), &BTreeMap::new(), &BTreeMap::new());
        let override_dir = resolve_tone(Some("com.apple.mail"), &overrides, &BTreeMap::new());

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

        assert!(resolve_tone(Some("com.apple.mail"), &overrides, &BTreeMap::new()).is_none());
    }

    #[test]
    fn resolve_tone_returns_none_for_missing_bundle_id() {
        assert!(resolve_tone(None, &BTreeMap::new(), &BTreeMap::new()).is_none());
    }

    #[test]
    fn custom_prompt_takes_precedence_over_preset_and_taxonomy() {
        let mut overrides = BTreeMap::new();
        overrides.insert("com.apple.mail".to_string(), TonePreset::Formal);
        let mut custom = BTreeMap::new();
        custom.insert(
            "com.apple.mail".to_string(),
            "write in pirate speak".to_string(),
        );

        let dir = resolve_tone(Some("com.apple.mail"), &overrides, &custom)
            .expect("custom prompt should produce a directive");
        assert!(dir.contains("write in pirate speak"));
        assert!(!dir.contains("Tone: formal"));
    }

    #[test]
    fn blank_custom_prompt_falls_back_to_preset() {
        let mut custom = BTreeMap::new();
        custom.insert("com.apple.mail".to_string(), "   ".to_string());

        let dir = resolve_tone(Some("com.apple.mail"), &BTreeMap::new(), &custom)
            .expect("blank custom falls back to taxonomy preset");
        assert!(dir.contains("formal"));
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
