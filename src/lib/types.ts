export interface Shortcut {
  key: string
  modifiers: string[]
}

export interface Settings {
  api_key: string | null
  shortcut: Shortcut
}
