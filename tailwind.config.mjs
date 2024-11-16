import starlightPlugin from "@astrojs/starlight-tailwind";

const colors = {
  white: "#ffffff",
  lilac: "#dbbfef",
  lavender: "#a4a0e8",
  comet: "#5a5977",
  bossanova: "#452859",
  midnight: "#3b224c",
  revolver: "#281733",
  silver: "#cccccc",
  sirocco: "#697C81",
  mint: "#9ff28f",
  almond: "#eccdba",
  chamois: "#E8DCA0",
  honey: "#efba5d",
  apricot: "#f47868",
  lightning: "#ffcd1c",
  delta: "#6F44F0",
  // these colors does not exist in the original theme
  haze: "#c7bdd5",
  purleLavender: "#9581ae",
};

const accent = {
  200: colors.lilac,
  600: colors.delta,
  900: "#ff00ff",
  950: colors.midnight,
};

const gray = {
  100: "#ff00ff",
  200: colors.almond, // interactive stuff
  300: colors.haze, // main text
  400: colors.lavender,
  400: colors.purleLavender,
  400: "#ff00ff",
  500: "#ff00ff",
  700: colors.bossanova, // border
  800: colors.midnight, // sidebar and navbar
  900: colors.revolver, // content background
};

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}"],
  theme: {
    colors: { accent, gray, white: colors.white },
  },
  plugins: [starlightPlugin()],
};
