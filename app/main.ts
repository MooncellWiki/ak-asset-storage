import { DataLoaderPlugin } from "unplugin-vue-router/data-loaders";
import { createApp } from "vue";
// https://github.com/un-ts/eslint-plugin-import-x/issues/372
import { createRouter, createWebHistory } from "vue-router";
import { routes } from "vue-router/auto-routes";
import App from "./App.vue";
import "./styles/main.css";
import "virtual:uno.css";
// import "@unocss/reset/tailwind.css";

const app = createApp(App);
const router = createRouter({
  routes,
  history: createWebHistory(import.meta.env.BASE_URL),
});
app.use(DataLoaderPlugin, { router });
app.use(router).mount("#app");
