/*
 * This file creates open graph images, which are preview images that get displayed when the website is linked on social media
 *
 * Sourced from https://hideoo.dev/notes/starlight-og-images
 */

import { getCollection } from "astro:content";
import { OGImageRoute } from "astro-og-canvas";
import type { CollectionEntry } from "astro:content";

const entries = await getCollection("docs");

const pages = Object.fromEntries(
  entries.map(({ data, id }: CollectionEntry<"docs">) => [id, { data }]),
);

export const { getStaticPaths, GET } = OGImageRoute({
  pages,
  /* the "slug" must be the same as the slug in the filename, e.g. [...slug].ts for this one */
  param: "slug",
  // Function called for each page to customize the generated image.
  getImageOptions: (_path, page: (typeof pages)[number]) => {
    return {
      title: page.data.title,
      description: page.data.description,
      logo: {
        path: "./src/assets/logo.png",
        size: [58],
      },
      /* styles to use for the image */
      bgGradient: [[69, 40, 88]],
      border: { color: [41, 24, 53], width: 20 },
      padding: 120,
    };
  },
});
