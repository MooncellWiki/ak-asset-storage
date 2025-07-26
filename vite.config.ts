import path from "node:path";
import vue from "@vitejs/plugin-vue";
import UnoCSS from "unocss/vite";
import Icons from "unplugin-icons/vite";
import { NaiveUiResolver } from "unplugin-vue-components/resolvers";
import Components from "unplugin-vue-components/vite";
import VueRouter from "unplugin-vue-router/vite";
import { defineConfig } from "vite";

export default defineConfig({
  resolve: {
    alias: {
      "~/": `${path.resolve(__dirname, "app")}/`,
    },
  },
  server: {
    proxy: {
      "/api": "http://localhost:25150",
      "/storage": "http://localhost:29000/arknights-assets/",
      "/gamedata": "http://localhost:25150",
      "/assets": "http://localhost:25150",
    },
    port: 25173,
  },
  plugins: [
    // https://github.com/posva/unplugin-vue-router
    VueRouter({
      routesFolder: "app/pages",
      exclude: "app/**/components/*",
    }),
    vue(),
    Components({
      dts: true,
      resolvers: [NaiveUiResolver()],
    }),
    Icons({ compiler: "vue3" }),
    // https://github.com/antfu/unocss
    // see uno.config.ts for config
    UnoCSS(),
  ],
});
