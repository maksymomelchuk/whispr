export type Platform = "macos" | "windows" | "linux";

export function currentPlatform(): Platform {
  if (typeof navigator === "undefined") return "macos";
  const ua = navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Linux|X11|CrOS/i.test(ua)) return "linux";
  return "macos";
}

export function isMacOS(): boolean {
  return currentPlatform() === "macos";
}
