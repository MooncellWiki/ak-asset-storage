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
    assetsDir: "fe",
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
