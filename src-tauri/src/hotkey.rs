use crate::config::{HotkeyBinding, Shortcut};
use crate::state::ModifierState;
use std::time::{Duration, Instant};

pub const DOUBLE_TAP_THRESHOLD: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TapEvent {
    Down,
    Up,
    OtherKey,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dispatch {
    StartPtt,
    StopPtt,
    Nothing,
}

#[derive(Default)]
pub struct TapState {
    pub tap_count: u8,
    pub last_tap_up_time: Option<Instant>,
    /// Bumped on every state-mutating event. Pending coexistence timers capture
    /// the post-event value at schedule time and abort on wake if it no longer
    /// matches — wrapping is fine, u64 collisions are not realistic.
    pub generation: u64,
}

pub fn advance_tap_state(state: &mut TapState, event: TapEvent, now: Instant) -> Dispatch {
    let dispatch = match event {
        TapEvent::Down => {
            if state.tap_count == 1 {
                if let Some(t) = state.last_tap_up_time {
                    if now.duration_since(t) < DOUBLE_TAP_THRESHOLD {
                        state.tap_count = 2;
                        state.generation = state.generation.wrapping_add(1);
                        return Dispatch::StartPtt;
                    }
                }
            }
            state.tap_count = 1;
            state.last_tap_up_time = None;
            Dispatch::Nothing
        }
        TapEvent::Up => {
            if state.tap_count == 2 {
                state.tap_count = 0;
                state.last_tap_up_time = None;
                state.generation = state.generation.wrapping_add(1);
                return Dispatch::StopPtt;
            }
            if state.tap_count == 1 {
                state.last_tap_up_time = Some(now);
            }
            Dispatch::Nothing
        }
        TapEvent::OtherKey => {
            if state.tap_count == 1 {
                state.tap_count = 0;
                state.last_tap_up_time = None;
            }
            Dispatch::Nothing
        }
    };
    state.generation = state.generation.wrapping_add(1);
    dispatch
}

/// Outcome of a key-down on a key that has both a single-press and a double-tap
/// binding. State is mutated; the timer-arming variant carries the generation
/// the caller must capture so the timer can detect later events that invalidate it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoexDown {
    FireDoubleTap,
    ScheduleSinglePress { captured_generation: u64 },
}

pub fn coex_advance_down(state: &mut TapState, now: Instant) -> CoexDown {
    match advance_tap_state(state, TapEvent::Down, now) {
        Dispatch::StartPtt => CoexDown::FireDoubleTap,
        _ => CoexDown::ScheduleSinglePress {
            captured_generation: state.generation,
        },
    }
}

pub fn coex_timer_should_fire(state: &TapState, captured: u64) -> bool {
    state.generation == captured && state.tap_count == 1 && state.last_tap_up_time.is_none()
}

pub fn key_has_both_kinds(bindings: &[HotkeyBinding], shortcut: &Shortcut) -> bool {
    let mut has_single = false;
    let mut has_double = false;
    for b in bindings {
        if b.shortcut.key == shortcut.key && b.shortcut.modifiers == shortcut.modifiers {
            if b.shortcut.is_double_tap {
                has_double = true;
            } else {
                has_single = true;
            }
        }
    }
    has_single && has_double
}

pub fn shortcut_is_relevant(code: &str, shortcut: &Shortcut) -> bool {
    if code == shortcut.key {
        return true;
    }
    shortcut.modifiers.iter().any(|m| match m.as_str() {
        "Meta" => matches!(code, "MetaLeft" | "MetaRight"),
        "Control" => matches!(code, "ControlLeft" | "ControlRight"),
        "Alt" => matches!(code, "AltLeft" | "AltRight"),
        "Shift" => matches!(code, "ShiftLeft" | "ShiftRight"),
        _ => false,
    })
}

/// Modifier-only shortcuts (key is itself a modifier) skip the modifier
/// check because the FlagsChanged that fires the key also mutates the bitmask.
pub fn shortcut_matches(code: &str, shortcut: &Shortcut, mods: ModifierState) -> bool {
    code == shortcut.key && (is_modifier_code(&shortcut.key) || mods.matches(&shortcut.modifiers))
}

pub fn tap_state_key(shortcut: &Shortcut) -> (String, Vec<String>) {
    (shortcut.key.clone(), shortcut.modifiers.clone())
}

pub fn is_modifier_code(code: &str) -> bool {
    matches!(
        code,
        "AltLeft"
            | "AltRight"
            | "MetaLeft"
            | "MetaRight"
            | "ControlLeft"
            | "ControlRight"
            | "ShiftLeft"
            | "ShiftRight"
    )
}

/// Modifiers intentionally ignored so held-modifier PTT shortcuts (e.g. hold
/// Right-Alt) still allow cancellation without releasing PTT first.
pub fn is_cancel_event(code: &str, is_press: bool, ptt_active: bool) -> bool {
    code == "Escape" && is_press && ptt_active
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_tap_within_window_starts_on_second_down_stops_on_second_up() {
        let base = Instant::now();
        let mut state = TapState::default();

        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Down, base),
            Dispatch::Nothing
        );
        assert_eq!(state.tap_count, 1);

        let t1 = base + Duration::from_millis(50);
        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Up, t1),
            Dispatch::Nothing
        );
        assert!(state.last_tap_up_time.is_some());

        let t2 = base + Duration::from_millis(150);
        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Down, t2),
            Dispatch::StartPtt
        );
        assert_eq!(state.tap_count, 2);

        let t3 = base + Duration::from_millis(300);
        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Up, t3),
            Dispatch::StopPtt
        );
        assert_eq!(state.tap_count, 0);
        assert!(state.last_tap_up_time.is_none());
    }

    #[test]
    fn double_tap_expired_window_second_down_treated_as_new_first_tap() {
        let base = Instant::now();
        let mut state = TapState::default();

        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Down, base),
            Dispatch::Nothing
        );
        let t1 = base + Duration::from_millis(50);
        advance_tap_state(&mut state, TapEvent::Up, t1);

        let t2 = base + Duration::from_millis(500);
        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Down, t2),
            Dispatch::Nothing
        );
        assert_eq!(state.tap_count, 1);
        assert!(state.last_tap_up_time.is_none());
    }

    #[test]
    fn other_key_between_taps_resets_state() {
        let base = Instant::now();
        let mut state = TapState::default();

        advance_tap_state(&mut state, TapEvent::Down, base);
        let t1 = base + Duration::from_millis(50);
        advance_tap_state(&mut state, TapEvent::Up, t1);
        assert_eq!(state.tap_count, 1);

        let t2 = base + Duration::from_millis(100);
        assert_eq!(
            advance_tap_state(&mut state, TapEvent::OtherKey, t2),
            Dispatch::Nothing
        );
        assert_eq!(state.tap_count, 0);
        assert!(state.last_tap_up_time.is_none());

        let t3 = base + Duration::from_millis(150);
        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Down, t3),
            Dispatch::Nothing
        );
        assert_eq!(state.tap_count, 1);
    }

    #[test]
    fn fresh_state_up_event_is_noop() {
        let mut state = TapState::default();
        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Up, Instant::now()),
            Dispatch::Nothing
        );
        assert_eq!(state.tap_count, 0);
        assert!(state.last_tap_up_time.is_none());
    }

    fn sp_binding(key: &str) -> HotkeyBinding {
        HotkeyBinding::ptt(
            Shortcut {
                key: key.to_string(),
                modifiers: vec![],
                is_double_tap: false,
            },
            "mode-a".to_string(),
        )
    }

    fn dt_binding(key: &str) -> HotkeyBinding {
        HotkeyBinding::ptt(
            Shortcut {
                key: key.to_string(),
                modifiers: vec![],
                is_double_tap: true,
            },
            "mode-b".to_string(),
        )
    }

    #[test]
    fn key_has_both_kinds_detects_coexistence_pair() {
        let bindings = vec![sp_binding("AltRight"), dt_binding("AltRight")];
        assert!(key_has_both_kinds(&bindings, &bindings[0].shortcut));
        assert!(key_has_both_kinds(&bindings, &bindings[1].shortcut));
    }

    #[test]
    fn key_has_both_kinds_false_for_single_press_only() {
        let bindings = vec![sp_binding("AltRight")];
        assert!(!key_has_both_kinds(&bindings, &bindings[0].shortcut));
    }

    #[test]
    fn key_has_both_kinds_false_for_double_tap_only() {
        let bindings = vec![dt_binding("AltRight")];
        assert!(!key_has_both_kinds(&bindings, &bindings[0].shortcut));
    }

    #[test]
    fn key_has_both_kinds_distinguishes_by_modifiers() {
        let with_shift = HotkeyBinding::ptt(
            Shortcut {
                key: "AltRight".to_string(),
                modifiers: vec!["Shift".to_string()],
                is_double_tap: true,
            },
            "mode-x".to_string(),
        );
        let bindings = vec![sp_binding("AltRight"), with_shift];
        assert!(!key_has_both_kinds(&bindings, &bindings[0].shortcut));
        assert!(!key_has_both_kinds(&bindings, &bindings[1].shortcut));
    }

    #[test]
    fn advance_tap_state_bumps_generation_on_every_event() {
        let base = Instant::now();
        let mut state = TapState::default();
        let g0 = state.generation;

        advance_tap_state(&mut state, TapEvent::Down, base);
        let g1 = state.generation;
        assert_ne!(g0, g1, "Down must bump generation");

        advance_tap_state(&mut state, TapEvent::Up, base + Duration::from_millis(50));
        let g2 = state.generation;
        assert_ne!(g1, g2, "Up must bump generation");

        advance_tap_state(
            &mut state,
            TapEvent::OtherKey,
            base + Duration::from_millis(100),
        );
        let g3 = state.generation;
        assert_ne!(g2, g3, "OtherKey must bump generation");
    }

    #[test]
    fn coex_tap_and_hold_fires_single_press_at_threshold() {
        let base = Instant::now();
        let mut state = TapState::default();

        let outcome = coex_advance_down(&mut state, base);
        let CoexDown::ScheduleSinglePress {
            captured_generation,
        } = outcome
        else {
            panic!("first down should schedule single-press timer");
        };
        assert!(coex_timer_should_fire(&state, captured_generation));
    }

    #[test]
    fn coex_tap_tap_and_hold_fires_double_tap_on_second_down() {
        let base = Instant::now();
        let mut state = TapState::default();

        let CoexDown::ScheduleSinglePress {
            captured_generation: g1,
        } = coex_advance_down(&mut state, base)
        else {
            panic!();
        };

        advance_tap_state(&mut state, TapEvent::Up, base + Duration::from_millis(50));
        assert!(!coex_timer_should_fire(&state, g1));

        let outcome = coex_advance_down(&mut state, base + Duration::from_millis(150));
        assert_eq!(outcome, CoexDown::FireDoubleTap);

        let stop = advance_tap_state(&mut state, TapEvent::Up, base + Duration::from_millis(300));
        assert_eq!(stop, Dispatch::StopPtt);
    }

    #[test]
    fn coex_tap_release_within_window_then_silence_fires_nothing() {
        let base = Instant::now();
        let mut state = TapState::default();

        let CoexDown::ScheduleSinglePress {
            captured_generation,
        } = coex_advance_down(&mut state, base)
        else {
            panic!();
        };
        advance_tap_state(&mut state, TapEvent::Up, base + Duration::from_millis(100));
        assert!(!coex_timer_should_fire(&state, captured_generation));
    }

    #[test]
    fn coex_two_separated_taps_with_gap_over_window_fire_nothing() {
        let base = Instant::now();
        let mut state = TapState::default();

        let CoexDown::ScheduleSinglePress {
            captured_generation: g1,
        } = coex_advance_down(&mut state, base)
        else {
            panic!();
        };
        advance_tap_state(&mut state, TapEvent::Up, base + Duration::from_millis(100));
        assert!(!coex_timer_should_fire(&state, g1));

        let outcome2 = coex_advance_down(&mut state, base + Duration::from_millis(1100));
        let CoexDown::ScheduleSinglePress {
            captured_generation: g2,
        } = outcome2
        else {
            panic!("gap > window should schedule a fresh SP timer, not fire DT");
        };
        advance_tap_state(&mut state, TapEvent::Up, base + Duration::from_millis(1200));
        assert!(!coex_timer_should_fire(&state, g2));
    }

    #[test]
    fn coex_rapid_triple_tap_fires_double_tap_then_resets() {
        let base = Instant::now();
        let mut state = TapState::default();

        let _ = coex_advance_down(&mut state, base);
        advance_tap_state(&mut state, TapEvent::Up, base + Duration::from_millis(50));
        assert_eq!(
            coex_advance_down(&mut state, base + Duration::from_millis(150)),
            CoexDown::FireDoubleTap,
        );
        assert_eq!(
            advance_tap_state(&mut state, TapEvent::Up, base + Duration::from_millis(250)),
            Dispatch::StopPtt,
        );

        let outcome3 = coex_advance_down(&mut state, base + Duration::from_millis(350));
        assert!(matches!(outcome3, CoexDown::ScheduleSinglePress { .. }));
    }

    #[test]
    fn coex_hold_past_threshold_keeps_timer_eligible_until_event_arrives() {
        let base = Instant::now();
        let mut state = TapState::default();

        let CoexDown::ScheduleSinglePress {
            captured_generation,
        } = coex_advance_down(&mut state, base)
        else {
            panic!();
        };
        assert!(coex_timer_should_fire(&state, captured_generation));

        let outcome2 = coex_advance_down(&mut state, base + Duration::from_millis(800));
        assert!(matches!(outcome2, CoexDown::ScheduleSinglePress { .. }));
        assert_eq!(state.tap_count, 1);
    }

    #[test]
    fn coex_helper_says_no_for_single_press_only_key() {
        let bindings = vec![sp_binding("AltRight")];
        let other = Shortcut {
            key: "AltRight".to_string(),
            modifiers: vec![],
            is_double_tap: false,
        };
        assert!(!key_has_both_kinds(&bindings, &other));
    }

    #[test]
    fn coex_helper_says_no_for_double_tap_only_key() {
        let bindings = vec![dt_binding("AltRight")];
        let other = Shortcut {
            key: "AltRight".to_string(),
            modifiers: vec![],
            is_double_tap: true,
        };
        assert!(!key_has_both_kinds(&bindings, &other));
    }

    #[test]
    fn coex_timer_with_stale_generation_does_not_fire() {
        let base = Instant::now();
        let mut state = TapState::default();
        let CoexDown::ScheduleSinglePress {
            captured_generation,
        } = coex_advance_down(&mut state, base)
        else {
            panic!();
        };

        advance_tap_state(
            &mut state,
            TapEvent::OtherKey,
            base + Duration::from_millis(10),
        );
        assert!(!coex_timer_should_fire(&state, captured_generation));
    }

    #[test]
    fn cancel_predicate_fires_on_escape_press_during_active_session() {
        assert!(is_cancel_event("Escape", true, true));
    }

    #[test]
    fn cancel_predicate_inert_when_no_session_active() {
        assert!(!is_cancel_event("Escape", true, false));
    }

    #[test]
    fn cancel_predicate_ignores_escape_release() {
        assert!(!is_cancel_event("Escape", false, true));
    }

    #[test]
    fn cancel_predicate_ignores_non_escape_keys() {
        assert!(!is_cancel_event("Space", true, true));
        assert!(!is_cancel_event("KeyA", true, true));
    }

    #[test]
    fn cancel_predicate_ignores_escape_release_when_idle() {
        assert!(!is_cancel_event("Escape", false, false));
    }

    #[test]
    fn shortcut_is_relevant_matches_on_key() {
        let sc = Shortcut {
            key: "AltRight".to_string(),
            modifiers: vec![],
            is_double_tap: false,
        };
        assert!(shortcut_is_relevant("AltRight", &sc));
        assert!(!shortcut_is_relevant("AltLeft", &sc));
    }

    #[test]
    fn shortcut_is_relevant_matches_on_required_modifier() {
        let sc = Shortcut {
            key: "KeyA".to_string(),
            modifiers: vec!["Meta".to_string()],
            is_double_tap: false,
        };
        assert!(shortcut_is_relevant("MetaLeft", &sc));
        assert!(shortcut_is_relevant("MetaRight", &sc));
        assert!(shortcut_is_relevant("KeyA", &sc));
        assert!(!shortcut_is_relevant("AltRight", &sc));
    }

    #[test]
    fn shortcut_matches_requires_same_key_and_mods() {
        let sc = Shortcut {
            key: "KeyA".to_string(),
            modifiers: vec!["Meta".to_string()],
            is_double_tap: false,
        };
        let mods_with_meta = ModifierState {
            meta: true,
            control: false,
            alt: false,
            shift: false,
        };
        let mods_bare = ModifierState::default();

        assert!(shortcut_matches("KeyA", &sc, mods_with_meta));
        assert!(!shortcut_matches("KeyA", &sc, mods_bare));
        assert!(!shortcut_matches("KeyB", &sc, mods_with_meta));
    }

    #[test]
    fn shortcut_matches_modifier_only_shortcut_skips_mod_check() {
        let sc = Shortcut {
            key: "AltRight".to_string(),
            modifiers: vec![],
            is_double_tap: false,
        };
        let mods_bare = ModifierState::default();
        assert!(shortcut_matches("AltRight", &sc, mods_bare));
    }
}
