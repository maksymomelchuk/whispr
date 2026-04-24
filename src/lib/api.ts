import { invoke } from '@tauri-apps/api/core'
import type {
  DeepgramSettings,
  Replacement,
  Settings,
  Shortcut,
} from './types'

export const getSettings = () => invoke<Settings>('get_settings')

export const setApiKey = (apiKey: string) =>
  invoke<void>('set_api_key', { apiKey })

export const setShortcut = (shortcut: Shortcut) =>
  invoke<void>('set_shortcut', { shortcut })

export const setReplacements = (replacements: Replacement[]) =>
  invoke<void>('set_replacements', { replacements })

export const setDeepgramSettings = (deepgram: DeepgramSettings) =>
  invoke<void>('set_deepgram_settings', { deepgram })

export const listInputDevices = () =>
  invoke<string[]>('list_input_devices')

export const setInputDevice = (device: string | null) =>
  invoke<void>('set_input_device', { device })

const MOD_MAP: Record<string, string> = {
  Meta: '⌘',
  Control: '⌃',
  Alt: '⌥',
  Shift: '⇧',
}

const KEY_MAP: Record<string, string> = {
  AltRight: 'Right ⌥',
  AltLeft: 'Left ⌥',
  MetaRight: 'Right ⌘',
  MetaLeft: 'Left ⌘',
  ControlRight: 'Right ⌃',
  ControlLeft: 'Left ⌃',
  ShiftRight: 'Right ⇧',
  ShiftLeft: 'Left ⇧',
  Space: 'Space',
  Escape: 'Esc',
  Tab: 'Tab',
  Enter: 'Return',
  Backspace: 'Del',
  ArrowUp: '↑',
  ArrowDown: '↓',
  ArrowLeft: '←',
  ArrowRight: '→',
}

const displayKey = (code: string): string => {
  if (KEY_MAP[code]) return KEY_MAP[code]
  // KeyA → A, KeyZ → Z
  const keyMatch = code.match(/^Key([A-Z])$/)
  if (keyMatch) return keyMatch[1]
  // Digit0 → 0, Digit9 → 9
  const digitMatch = code.match(/^Digit(\d)$/)
  if (digitMatch) return digitMatch[1]
  // F1..F20 → F1..F20
  if (/^F\d{1,2}$/.test(code)) return code
  return code
}

export const formatShortcut = (s: Shortcut): string => {
  const mods = s.modifiers.map((m) => MOD_MAP[m] ?? m).join(' + ')
  const key = displayKey(s.key)
  return mods ? `${mods} + ${key}` : key
}
