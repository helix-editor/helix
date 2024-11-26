import { defineCollection, z } from "astro:content";
import { docsSchema } from "@astrojs/starlight/schema";
import { blogSchema } from "starlight-blog/schema";

export const collections = {
  docs: defineCollection({
    schema: docsSchema({
      extend: (context) =>
        blogSchema(context).extend({
          // if more than 100 characters some of the text won't be visible as it will wrap to below the image
          description: z.string().max(100).optional(),
        }),
    }),
  }),
};
