// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLinksValidator from "starlight-links-validator";
import starlightBlog from "starlight-blog";
import starlightImageZoom from "starlight-image-zoom";
import { rehypeHeadingIds } from "@astrojs/markdown-remark";
import rehypeAutolinkHeadings from "rehype-autolink-headings";
import rehypeExternalLinks from "rehype-external-links";

import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
  site: "https://helix-editor.vercel.app",
  vite: {
    optimizeDeps: { include: ["asciinema-player"] },
  },
  markdown: {
    rehypePlugins: [
      rehypeHeadingIds,

      [
        rehypeExternalLinks,
        {
          content: {
            type: "text",
            value: " ↗",
          },
          properties: {
            target: "_blank",
          },
          rel: ["noopener"],
        },
      ],
      [rehypeAutolinkHeadings, { behavior: "wrap" }],
    ],
  },
  integrations: [
    starlight({
      tableOfContents: {
        maxHeadingLevel: 5,
      },
      head: [
        {
          tag: "link",
          attrs: {
            rel: "sitemap",
            href: "/sitemap-index.xml",
          },
        },
        {
          tag: "script",
          attrs: {
            src: "https://static.cloudflareinsights.com/beacon.min.js",
            "data-cf-beacon": '{"token": "1c63f959be334c96913d5c16499a3887"}',
            defer: true,
          },
        },
      ],
      plugins: [
        starlightImageZoom(),
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
        Head: "./src/components/Head.astro",
      },
      editLink: {
        baseUrl:
          "https://github.com/NikitaRevenco/helix/tree/new-website/website",
      },
      sidebar: [
        {
          label: "Start Here",
          items: ["start-here/installation", "start-here/basics"],
        },
        {
          label: "Usage",
          items: [
            "usage/multiple-cursors",
            "usage/macros",
            "usage/text-objects",
            "usage/surround",
            "usage/language-support",
            "usage/buffers",
            "usage/pickers",
          ],
        },
        {
          label: "Reference",
          items: [
            "reference/keymap",
            "reference/typed-commands",
            "reference/list-of-themes",
            "reference/custom-themes",
            "reference/registers",
            "reference/language-servers",
            "reference/formatters",
            "reference/debuggers",
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
