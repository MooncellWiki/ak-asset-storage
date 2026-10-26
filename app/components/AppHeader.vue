<template>
  <header class="ak-header header">
    <div class="header__inner">
      <RouterLink class="header__logo" to="/">
        <img v-if="!logoFailed" :src="LOGO" alt="" @error="logoFailed = true" />
        <span class="header__wordmark">AK ASSETS</span>
      </RouterLink>
      <nav class="header__nav" aria-label="主导航">
        <RouterLink
          v-for="item in NAV"
          :key="item.to"
          :to="item.to"
          :class="{ 'is-active': item === active }"
          :aria-current="item === active ? 'page' : undefined"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
      <button
        type="button"
        class="search-trigger"
        aria-haspopup="dialog"
        :aria-expanded="paletteOpen"
        aria-keyshortcuts="Control+K Meta+K /"
        @click="paletteOpen = true"
      >
        <span class="search-trigger__icon"
          ><AsIcon name="search" :size="16"
        /></span>
        <span class="search-trigger__label">搜索文件、资源、bundle</span>
        <kbd class="ak-kbd">{{ isMac ? "⌘K" : "Ctrl K" }}</kbd>
      </button>
      <div class="header__tools">
        <a
          class="ak-btn ak-btn--ghost ak-btn--sm header__api"
          href="/api/v1/scalar"
          >API</a
        >
        <div class="theme-toggle" role="group" aria-label="外观">
          <button
            v-for="t in THEMES"
            :key="t.mode"
            type="button"
            :title="t.label"
            :aria-label="t.label"
            :aria-pressed="themeMode === t.mode"
            @click="themeMode = t.mode"
          >
            <AsIcon :name="t.icon" :size="15" />
          </button>
        </div>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useRoute } from "vue-router";
import { paletteOpen } from "~/common/palette";
import { themeMode } from "~/common/theme";
import AsIcon from "./AsIcon.vue";
import type { ThemeMode } from "~/common/theme";

const route = useRoute();

const NAV = [
  { to: "/", label: "版本", match: (p: string) => p === "/" },
  { to: "/diff", label: "变更", match: (p: string) => p.startsWith("/diff") },
  {
    to: "/manifest",
    label: "清单",
    match: (p: string) => p.startsWith("/manifest"),
  },
  { to: "/asset", label: "文件", match: (p: string) => p.startsWith("/asset") },
  {
    to: "/story",
    label: "剧情资源",
    match: (p: string) => p.startsWith("/story"),
  },
];
const active = computed(() => NAV.find((item) => item.match(route.path)));

const THEMES: {
  mode: ThemeMode;
  label: string;
  icon: "monitor" | "sun" | "moon";
}[] = [
  { mode: "os", label: "跟随系统", icon: "monitor" },
  { mode: "light", label: "亮", icon: "sun" },
  { mode: "dark", label: "暗", icon: "moon" },
];

const isMac = /Mac|iPhone|iPad/u.test(navigator.platform);
// Served by this instance from the extracted game assets, not bundled.
const LOGO = "/assets/camplogo/logo_rhodes.png";
const logoFailed = ref(false);
</script>

<style scoped lang="scss">
@use "~/styles/shared" as *;

// The header is black in both themes (the official site's nav bar). tokens.css
// already gives .ak-header the dark token set; semantic tokens are remapped to
// the --ak-chrome-* ones here so controls inside pick up header colours.
// Adapted from prts-design packages/css/src/chrome/header.css, which the npm
// package does not export.
.header {
  --ak-fg: var(--ak-chrome-fg);
  --ak-fg-secondary: var(--ak-chrome-fg-2);
  --ak-fg-muted: var(--ak-chrome-fg-muted);
  --ak-fg-subtle: var(--ak-chrome-fg-muted);
  --ak-bg-surface: var(--ak-chrome-bg-solid);
  --ak-bg-hover: var(--ak-chrome-hover);
  --ak-border: var(--ak-chrome-line);
  --ak-border-strong: var(--ak-chrome-line-strong);
  --ak-accent: var(--ak-theme-accent);
  --ak-accent-fg: var(--ak-theme-accent-fg);
  --ak-focus: var(--ak-theme-accent);

  position: sticky;
  top: 0;
  z-index: var(--ak-z-header);
  color-scheme: dark;
  color: var(--ak-fg);
  background: var(--ak-chrome-bg);
  border-bottom: 1px solid var(--ak-chrome-line);

  // Halftone dots fading in towards the tools on the right.
  &::before {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    opacity: var(--ak-chrome-texture);
    background-image: radial-gradient(
      color-mix(in srgb, var(--ak-gray-0) 34%, transparent) 1px,
      transparent 1.4px
    );
    background-size: 6px 6px;
    background-position: 0 1px;
    mask-image: linear-gradient(90deg, transparent 52%, #000 100%);
  }

  &__inner {
    position: relative;
    height: var(--ak-header-h);
    max-width: 1680px;
    margin: 0 auto;
    padding: 0 var(--ak-gutter);
    box-sizing: border-box;
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--ak-space-6);
  }

  &__logo {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    height: 100%;
    color: inherit;
    text-decoration: none;

    &:hover,
    &:visited {
      color: inherit;
      text-decoration: none;
    }

    > img {
      width: auto;
      height: 28px;
      max-width: 48px;
      object-fit: contain;
    }
  }

  &__wordmark {
    font: 800 18px/1 var(--ak-font-display);
    font-variant-numeric: lining-nums;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  // The navigation lives in the header: this tool has no sidebar.
  &__nav {
    display: flex;
    align-self: stretch;
    gap: 2px;

    > a {
      position: relative;
      display: inline-flex;
      align-items: center;
      padding: 0 12px;
      font: 600 var(--ak-fs-sm) / 1 var(--ak-font-body);
      color: var(--ak-chrome-fg-2);
      text-decoration: none;
      white-space: nowrap;
      transition:
        color var(--ak-dur-fast),
        background var(--ak-dur-fast);

      &:hover {
        color: var(--ak-chrome-fg);
        background: var(--ak-chrome-hover);
      }

      &.is-active {
        color: var(--ak-theme-accent);

        &::after {
          content: "";
          position: absolute;
          left: 12px;
          right: 12px;
          bottom: -1px;
          height: 3px;
          background: var(--ak-theme-accent);
        }
      }
    }
  }

  &__tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  &__api {
    font-family: var(--ak-font-label);
    letter-spacing: 0.06em;

    &:visited {
      color: var(--ak-fg);
    }
  }

  .ak-kbd {
    margin-left: auto;
    background: color-mix(in srgb, var(--ak-gray-1000) 40%, transparent);
    border-color: var(--ak-chrome-line-strong);
    box-shadow: inset 0 -1px 0 var(--ak-chrome-line-strong);
    color: var(--ak-fg-muted);
  }
}

// Looks like an input; opens the search palette.
.search-trigger {
  justify-self: end;
  display: inline-flex;
  align-items: center;
  gap: 10px;
  width: min(100%, 360px);
  height: 34px;
  padding: 0 8px 0 0;
  box-sizing: border-box;
  border: 1px solid var(--ak-chrome-line-strong);
  background: var(--ak-chrome-field);
  color: var(--ak-fg-secondary);
  font: inherit;
  font-size: var(--ak-fs-sm);
  text-align: left;
  cursor: text;
  transition:
    background var(--ak-dur-fast),
    border-color var(--ak-dur-fast),
    color var(--ak-dur-fast);

  &:hover {
    border-color: var(--ak-chrome-fg-muted);
    color: var(--ak-fg);
  }

  &:focus-visible {
    outline: 2px solid var(--ak-focus);
    outline-offset: 1px;
  }

  &[aria-expanded="true"] {
    border-color: var(--ak-theme-accent);
    background: color-mix(in srgb, var(--ak-gray-1000) 35%, transparent);
    color: var(--ak-fg);
  }

  // The one solid accent block in the header.
  &__icon {
    flex: none;
    align-self: stretch;
    display: grid;
    place-items: center;
    width: 32px;
    background: var(--ak-theme-accent);
    color: var(--ak-theme-accent-fg);
  }

  &__label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.theme-toggle {
  display: inline-flex;
  border: 1px solid var(--ak-border-strong);

  > button {
    all: unset;
    box-sizing: border-box;
    display: grid;
    place-items: center;
    width: 30px;
    height: 28px;
    color: var(--ak-fg-muted);
    cursor: pointer;
    transition:
      background var(--ak-dur-fast),
      color var(--ak-dur-fast);

    & + button {
      border-left: 1px solid var(--ak-border);
    }

    &:hover {
      color: var(--ak-fg);
      background: var(--ak-bg-hover);
    }

    &:focus-visible {
      outline: 2px solid var(--ak-focus);
      outline-offset: -2px;
    }

    &[aria-pressed="true"] {
      background: var(--ak-accent);
      color: var(--ak-accent-fg);
    }
  }
}

// Narrow: logo · search · tools on one line, the navigation scrolls on a second.
@media (max-width: $bp-narrow) {
  .header {
    display: flex;
    flex-wrap: wrap;
    padding: 0 var(--ak-gutter);

    &__inner {
      display: contents;
    }

    &__logo {
      order: -2;
      margin-right: auto;
      height: var(--ak-header-h);
    }

    &__tools {
      order: 0;
    }

    &__nav {
      order: 10;
      flex: 1 0 100%;
      height: 40px;
      margin: 0 calc(var(--ak-gutter) * -1);
      padding: 0 var(--ak-gutter);
      overflow-x: auto;
      scrollbar-width: none;
      border-top: 1px solid var(--ak-chrome-line);
    }
  }

  .search-trigger {
    order: -1;
    width: min(50vw, 320px);
    margin: 0 var(--ak-space-3);
  }
}

@media (max-width: $bp-phone) {
  .header {
    &__api {
      display: none;
    }

    &__wordmark {
      font-size: 15px;
    }

    &__logo > img {
      height: 22px;
    }
  }

  .search-trigger {
    width: 40px;
    padding: 0;
    border-color: transparent;
    background: transparent;

    &__label,
    .ak-kbd {
      display: none;
    }

    &__icon {
      width: 40px;
      background: transparent;
      color: var(--ak-chrome-fg-2);
    }
  }

  .theme-toggle > button {
    width: 34px;
    height: 30px;
  }
}

@media (prefers-reduced-motion: reduce) {
  .theme-toggle > button {
    transition: none;
  }
}
</style>
