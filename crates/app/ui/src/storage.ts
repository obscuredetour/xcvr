// Per-device conveniences only (sort order, column widths). Storage can be
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
  remove(key: string): void {
    try {
      localStorage.removeItem(key);
    } catch {
      // As above.
    }
  },
};
