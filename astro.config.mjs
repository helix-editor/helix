// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLinksValidator from "starlight-links-validator";
import starlightBlog from "starlight-blog";

import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
  site: "https://helix.vercel.app",
  vite: {
    optimizeDeps: { include: ["asciinema-player"] },
  },
  integrations: [
    starlight({
      head: [
        {
          tag: "link",
          attrs: {
            rel: "sitemap",
            href: "/sitemap-index.xml",
          },
        },
      ],
      plugins: [
        starlightLinksValidator(),
        starlightBlog({ title: "News", prefix: "news" }),
      ],
      title: "Helix",
      logo: {
        src: "./public/favicon.svg",
      },
      social: {
        github: "https://github.com/helix-editor/helix",
        matrix: "https://matrix.to/#/#helix-community:matrix.org",
      },
      components: {
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
          items: ["getting-started/installation", "getting-started/basics"],
        },
        {
          label: "Usage",
          items: [
            "usage/multiple-cursors",
            "usage/text-objects",
            "usage/surround",
            "usage/language-support",
            "usage/pickers",
            "usage/registers",
          ],
        },
        {
          label: "Configuration",
          items: [
            "configuration/editor",
            "configuration/languages",
            "configuration/remapping",
          ],
        },
        {
          label: "Reference",
          items: [
            "reference/keymap",
            "reference/typed-commands",
            "reference/list-of-themes",
            "reference/custom-themes",
            "reference/language-servers",
            "reference/formatters",
            "reference/debuggers",
          ],
        },
        {
          label: "Help",
          items: [
            "help/recipes",
            "help/faq",
            "help/troubleshooting",
            "help/terminal-support",
            "help/language-defaults",
            "help/refactor-examples",
          ],
        },
        {
          label: "Contributing to Helix",
          items: [
            "contributing/vision",
            "contributing/this-site",
            "contributing/releases",
            "contributing/architecture",
            "contributing/languages",
            "contributing/textobject-queries",
            "contributing/indent-queries",
            "contributing/injection-queries",
          ],
        },
      ],
      customCss: ["./src/globals.css"],
    }),
    sitemap(),
  ],
});
