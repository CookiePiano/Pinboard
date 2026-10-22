import { writable } from "svelte/store";
import type { SelectionTools, Settings, Theme } from "./ipc";

export const selectionTools = writable<SelectionTools>({
  sizeStep: 1, highlightColor: "#ffe066",
});
export const theme = writable<Theme>("system");
export const darkMode = writable(false);

export function applyAppearance(settings: Settings) {
  theme.set(settings.theme);
  selectionTools.set(settings.selectionTools);
}

export function watchTheme() {
  const media = matchMedia("(prefers-color-scheme: dark)");
  let current: Theme = "system";
  function apply() {
    const dark = current === "dark" || (current === "system" && media.matches);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    darkMode.set(dark);
  }
  const stop = theme.subscribe((value) => { current = value; apply(); });
  media.addEventListener("change", apply);
  return () => { stop(); media.removeEventListener("change", apply); };
}
