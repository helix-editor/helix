// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import tailwind from "@astrojs/tailwind";

// https://astro.build/config
export default defineConfig({
  vite: {
    optimizeDeps: { include: ["asciinema-player"] },
  },
  integrations: [
    starlight({
      title: "Helix Editor",
      logo: {
        src: "./public/favicon.svg",
      },
      social: {
        github: "https://github.com/helix-editor/helix",
        matrix: "https://matrix.to/#/#helix-community:matrix.org",
      },
      components: {
        // HACK: override default components so user cannot use light theme
        ThemeProvider: "./src/components/ThemeProvider.astro",
        ThemeSelect: "./src/components/ThemeSelect.astro",
      },
      editLink: {
        baseUrl:
          "https://github.com/nikitarevenco/helix-better-docs/edit/main/",
      },
      sidebar: [
        {
          label: "Getting Started",
          items: [
            "getting-started/introduction",
            "getting-started/installation",
            "getting-started/basics",
          ],
        },
        {
          label: "Usage",
          items: [
            "usage/overview",
            "usage/multiple-cursors",
            "usage/select-mode",
            "usage/recipes",
            "usage/languages",
            "usage/configuration", // mention themes and config
            "usage/text-manipulation", // surround + textobjects
          ],
        },
        {
          label: "Reference",
          items: [
            "reference/themes",
            "reference/formatters",
            "reference/keymap",
            "reference/commands",
            "reference/debuggers",
          ],
        },
        {
          label: "Help",
          items: [
            "help/terminal-support",
            "help/faq",
            "help/from-vim",
            "help/troubleshooting",
          ],
        },

        {
          label: "Contributing to Helix",
          items: [
            "contributing/languages",
            "contributing/textobject-queries",
            "contributing/indent-queries",
            "contributing/injection-queries",
            "contributing/architecture",
            "contributing/releases",
          ],
        },
      ],
      customCss: ["./src/tailwind.css"],
    }),
    tailwind({ applyBaseStyles: false }),
  ],
});
