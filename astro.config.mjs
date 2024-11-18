// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import tailwind from "@astrojs/tailwind";

import { ExpressiveCodeTheme } from "@astrojs/starlight/expressive-code";
import fs from "node:fs";

const jsoncString = fs.readFileSync(
  new URL(`./theme.json`, import.meta.url),
  "utf-8"
);
const theme = ExpressiveCodeTheme.fromJSONString(jsoncString);

// https://astro.build/config
export default defineConfig({
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
      expressiveCode: {
        themes: [theme],
      },
      components: {
        // HACK: override default components so user cannot use light theme
        ThemeProvider: "./src/components/ThemeProvider.astro",
        ThemeSelect: "./src/components/ThemeSelect.astro",
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
          ],
        },
      ],
      customCss: ["./src/tailwind.css"],
    }),
    tailwind({ applyBaseStyles: false }),
  ],
});
