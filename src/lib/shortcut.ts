import type { HotkeyBinding, Shortcut } from "./types";

export const MOD_LABEL: Record<string, string> = {
  Meta: "⌘",
  Control: "⌃",
  Alt: "⌥",
  Shift: "⇧",
};

export const KEY_LABEL: Record<string, string> = {
  AltRight: "Right ⌥",
  AltLeft: "Left ⌥",
  MetaRight: "Right ⌘",
  MetaLeft: "Left ⌘",
  ControlRight: "Right ⌃",
  ControlLeft: "Left ⌃",
  ShiftRight: "Right ⇧",
  ShiftLeft: "Left ⇧",
  Space: "Space",
  Escape: "Esc",
  Tab: "Tab",
  Enter: "Return",
  Backspace: "Del",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

const MODIFIER_CODES = new Set([
  "MetaLeft",
  "MetaRight",
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
]);

export function isModifierCode(code: string): boolean {
  return MODIFIER_CODES.has(code);
}

export function collectModifiers(e: KeyboardEvent): string[] {
  const mods: string[] = [];
  if (e.metaKey) mods.push("Meta");
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  return mods;
}

export function displayKey(code: string): string {
  if (KEY_LABEL[code]) return KEY_LABEL[code];
  const k = code.match(/^Key([A-Z])$/);
  if (k) return k[1];
  const d = code.match(/^Digit(\d)$/);
  if (d) return d[1];
  if (/^F\d{1,2}$/.test(code)) return code;
  return code;
}

export function shortcutKey(shortcut: Shortcut): string {
  return `${shortcut.key}|${shortcut.modifiers.join(",")}|${shortcut.is_double_tap ?? false}`;
}

export function shortcutsEqual(a: Shortcut, b: Shortcut): boolean {
  if (a.key !== b.key) return false;
  if ((a.is_double_tap ?? false) !== (b.is_double_tap ?? false)) return false;
  if (a.modifiers.length !== b.modifiers.length) return false;
  const aSet = new Set(a.modifiers);
  return b.modifiers.every((m) => aSet.has(m));
}

export function hasConflict(bindings: HotkeyBinding[], index: number): boolean {
  const key = shortcutKey(bindings[index].shortcut);
  return bindings.some(
    (other, i) => i !== index && shortcutKey(other.shortcut) === key,
  );
}

export function formatShortcut(s: Shortcut): string {
  const mods = s.modifiers.map((m) => MOD_LABEL[m] ?? m).join(" + ");
  const key = displayKey(s.key);
  const base = mods ? `${mods} + ${key}` : key;
  return s.is_double_tap ? `${base} (double-tap)` : base;
}
