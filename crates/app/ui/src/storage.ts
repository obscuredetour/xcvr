// Per-device conveniences only (last library, sort order). Storage can be
// unavailable or cleared, so every access is guarded.

export const storage = {
  get(key: string): string | null {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  set(key: string, value: string): void {
    try {
      localStorage.setItem(key, value);
    } catch {
      // Not worth surfacing; the app works without it.
    }
  },
};
