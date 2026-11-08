import { createApp } from "vue";
// https://github.com/un-ts/eslint-plugin-import-x/issues/372
import { createRouter, createWebHistory } from "vue-router";
import { routes } from "vue-router/auto-routes";
import { DataLoaderPlugin } from "vue-router/experimental";
import App from "./App.vue";
import "@mooncellwiki/prts-design-css/standalone.css";

const app = createApp(App);
const router = createRouter({
  routes: [...routes, { path: "/file", redirect: "/diff" }],
  history: createWebHistory(import.meta.env.BASE_URL),
});
// A chunk can fail to load when the server has moved on to another release
// (self-hosted builds serve only their own chunks) or the CDN is unreachable.
// Load the page again from the server, at the route being navigated to, to
// pick up the current index.html; the timestamp keeps a chunk that stays
// unavailable from causing a reload loop.
const CHUNK_RELOAD_KEY = "akas-chunk-reload";
let navigatingTo: string | undefined;
router.beforeEach((to) => {
  navigatingTo = to.fullPath;
});
router.afterEach(() => {
  navigatingTo = undefined;
});
window.addEventListener("vite:preloadError", () => {
  try {
    const last = Number(sessionStorage.getItem(CHUNK_RELOAD_KEY));
    if (Date.now() - last < 10_000) return;
    sessionStorage.setItem(CHUNK_RELOAD_KEY, String(Date.now()));
  } catch {
    return;
  }
  if (navigatingTo === undefined) location.reload();
  else location.assign(navigatingTo);
});

app.use(DataLoaderPlugin, { router });
app.use(router).mount("#app");
