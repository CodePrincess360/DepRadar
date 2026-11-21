/**
 * Colour theme: "system" follows the OS (no attribute), "light" and "dark" set
 * `data-theme` on <html>. The choice is kept in localStorage when available.
 */
export type ThemeChoice = "system" | "light" | "dark";

const STORAGE_KEY = "depradar-theme";

export function getStoredTheme(): ThemeChoice {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === "light" || value === "dark" ? value : "system";
  } catch {
    return "system";
  }
}

export function applyTheme(choice: ThemeChoice): void {
  const root = document.documentElement;
  if (choice === "system") delete root.dataset.theme;
  else root.dataset.theme = choice;
}

export function setTheme(choice: ThemeChoice): void {
  applyTheme(choice);
  try {
    if (choice === "system") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, choice);
  } catch {
    // Storage can be unavailable (private mode, blocked site data); the choice still applies for this visit.
  }
}
