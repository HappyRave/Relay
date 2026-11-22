import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://happyrave.github.io",
  base: "/Relay",
  integrations: [
    starlight({
      title: "Relay",
      description: "A free macro recorder for Windows: record what you do, edit it as steps, play it back on cue.",
      logo: { src: "./src/assets/logo.svg", alt: "" },
      favicon: "/favicon.svg",
      social: [{ icon: "github", label: "GitHub", href: "https://github.com/HappyRave/Relay" }],
      editLink: { baseUrl: "https://github.com/HappyRave/Relay/edit/main/docs/user-guide/" },
      sidebar: [{ autogenerate: { directory: "guide" } }],
      expressiveCode: { shiki: { langAlias: { autohotkey: "ahk2" } } },
      customCss: ["@fontsource-variable/archivo/wdth.css", "./src/styles/relay.css"],
    }),
  ],
});
