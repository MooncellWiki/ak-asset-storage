import path from "node:path";
import vue from "@vitejs/plugin-vue";
import UnoCSS from "unocss/vite";
import Icons from "unplugin-icons/vite";
import VueRouter from "unplugin-vue-router/vite";
import { defineConfig } from "vite";

export default defineConfig({
  resolve: {
    alias: {
      "~/": `${path.resolve(__dirname, "src")}/`,
    },
  },
  server: {
    proxy: {
      "/api": "http://localhost:25150",
      "/storaeg": "http://localhost:29000/arknights-assets/",
    },
    port: 25173,
  },
  plugins: [
    // https://github.com/posva/unplugin-vue-router
    VueRouter({
      exclude: "src/**/components/*",
    }),
    vue(),
    Icons({ compiler: "vue3" }),
    // https://github.com/antfu/unocss
    // see uno.config.ts for config
    UnoCSS(),
  ],
});
