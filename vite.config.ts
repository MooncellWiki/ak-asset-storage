import { readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import vue from "@vitejs/plugin-vue";
import { parse } from "smol-toml";
import { defineConfig } from "vite";
import VueRouter from "vue-router/vite";
import type { Plugin, ProxyOptions } from "vite";

// The footer shows the server version; Cargo.toml is its single source.
const cargo = parse(
  readFileSync(path.resolve(import.meta.dirname, "Cargo.toml"), "utf8"),
) as {
  workspace?: { package?: { version?: string } };
};
const cargoVersion = cargo.workspace?.package?.version;

// AK_DEV_BACKEND points every proxied path at one deployed instance (e.g.
// https://torappu.prts.wiki) to develop the frontend without local services.
const remote = process.env.AK_DEV_BACKEND;
const local = "http://localhost:5150";
function proxy(target: string): ProxyOptions {
  return { target: remote ?? target, changeOrigin: Boolean(remote) };
}

// Release builds set AK_FE_CDN (e.g. https://static.prts.wiki/torappu-fe/) and
// upload the build assets there, so tabs opened on an earlier release can still
// load their chunks after a deploy replaces the embedded copy. Only asset URLs
// move: `base` stays "/" since the router uses it as the history base.
const ASSETS_DIR = "fe";
// Docker passes an unset build arg as an empty string.
const feCdn = process.env.AK_FE_CDN
  ? process.env.AK_FE_CDN.replace(/\/?$/, "/")
  : undefined;
function assetUrl(
  filename: string,
  { type }: { type: "asset" | "public" },
): string | undefined {
  if (feCdn && type === "asset" && filename.startsWith(`${ASSETS_DIR}/`))
    return feCdn + filename.slice(ASSETS_DIR.length + 1);
  return undefined;
}

// Web fonts are not in @mooncellwiki/prts-design-css; prts.wiki serves the
// Arknights skin's set and allows *.prts.wiki origins (CORS). The dev server
// proxies it, since localhost is not allowed.
const SKIN = "https://prts.wiki";
const SKIN_FONTS = "/skins/Arknights/resources/design-system/fonts.css";
function skinFonts(): Plugin {
  return {
    name: "prts-skin-fonts",
    transformIndexHtml: (_, ctx) => [
      ...(ctx.server
        ? []
        : [
            {
              tag: "link",
              attrs: { rel: "preconnect", href: SKIN, crossorigin: "" },
            },
          ]),
      {
        tag: "link",
        attrs: {
          rel: "stylesheet",
          href: ctx.server ? SKIN_FONTS : SKIN + SKIN_FONTS,
        },
      },
    ],
  };
}

export default defineConfig({
  build: {
    assetsDir: ASSETS_DIR,
  },
  experimental: {
    renderBuiltUrl: assetUrl,
  },
  define: {
    __APP_VERSION__: JSON.stringify(cargoVersion ?? "dev"),
  },
  resolve: {
    alias: {
      "~/": `${path.resolve(import.meta.dirname, "app")}/`,
    },
  },
  server: {
    proxy: {
      "/api": proxy(local),
      "/storage": remote
        ? proxy(local)
        : proxy("http://localhost:31000/arknights-assets/"),
      "/gamedata": proxy(local),
      "/assets": proxy(local),
      "/skins/Arknights/resources/design-system": {
        target: SKIN,
        changeOrigin: true,
      },
    },
    port: 25173,
  },
  plugins: [
    VueRouter({
      routesFolder: "app/pages",
      exclude: "app/**/components/*",
    }),
    vue(),
    skinFonts(),
  ],
});
