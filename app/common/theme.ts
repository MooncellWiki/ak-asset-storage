import { ref, watch } from "vue";

/** Appearance: follow the OS, or force the light / dark theme. */
export type ThemeMode = "os" | "light" | "dark";

const KEY = "akas-theme";

function read(): ThemeMode {
  try {
    const stored = localStorage.getItem(KEY);
    if (stored === "light" || stored === "dark") return stored;
  } catch {}
  return "os";
}

export const themeMode = ref<ThemeMode>(read());

// index.html applies the stored mode before the first paint; this keeps
// <html data-theme> and storage in sync afterwards.
watch(themeMode, (mode) => {
  const root = document.documentElement;
  if (mode === "os") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", mode);
  try {
    if (mode === "os") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, mode);
  } catch {}
});
