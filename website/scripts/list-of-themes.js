/*
 * This script creates @/generated/list-of-themes/* files
 *
 * It automatically runs on `pnpm dev`, `pnpm build` and `pnpm preview` and can be manually run with `pnpm termshots`
 */

import util from "node:util";
import toml from "@iarna/toml";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/* utilities */

function isObject(item) {
  return item && typeof item === "object" && !Array.isArray(item);
}

function mergeDeep(target, ...sources) {
  if (!sources.length) return target;
  const source = sources.shift();

  if (isObject(target) && isObject(source)) {
    for (const key in source) {
      if (isObject(source[key])) {
        if (!target[key]) {
          Object.assign(target, { [key]: {} });
          mergeDeep(target[key], source[key]);
        } else {
          Object.assign(target, { [key]: source[key] });
        }
      }
    }
  }

  return mergeDeep(target, ...sources);
}

/* main logic  */

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));

const helixDirectory = path.join(scriptDirectory, "..", "..");

const defaultHelixThemePath = path.join(helixDirectory, "theme.toml");

const themesDirectory = path.join(helixDirectory, "runtime", "themes");

const base16themes = new Set([
  "ttox.toml",
  "base16_theme.toml",
  "base16_default_dark.toml",
  "base16_default_light.toml",
  "base16_terminal.toml",
  "base16_transparent.toml",
  "term16_dark.toml",
  "term16_light.toml",
]);

const themesPaths = fs
  .readdirSync(themesDirectory)
  .filter((themePath) => {
    return themePath.endsWith(".toml") && !base16themes.has(themePath);
  })
  .map((themePath) => `${themesDirectory}/${themePath}`)
  .concat([defaultHelixThemePath]);

const unmergedThemes = themesPaths.map((themePath) => [
  themePath.split("/").pop().slice(0, -5),
  toml.parse(fs.readFileSync(themePath, "utf8")),
]);

const themeMap = new Map(unmergedThemes);

const resolveInheritedTheme = ([themeName, theme]) => {
  if ("inherits" in theme) {
    // theme A inherits theme B.
    // recursively override theme B's styles with theme A's styles
    const inheritedTheme = themeMap.get(theme.inherits);
    const merged = mergeDeep(inheritedTheme, theme);
    return [themeName, mergeDeep(inheritedTheme, theme)];
  } else {
    return [themeName, theme];
  }
};

const themes = unmergedThemes
  .map(resolveInheritedTheme)
  // some themes have A -> B -> C inheritance,
  // so A inherits from B which inherits from C
  //
  // We could make a more robust algorithm but in reality just calling the function twice is fine, since there isn't any chain of A -> B -> C -> D inheritance and this is a simple script
  .map(resolveInheritedTheme);

const setIntersection = (a, b) =>
  new Set([...a].filter((element) => b.has(element)));

// console.log(themesChanged);

/* normalization -- themes should have a specific structure */
const normalize = (theme, ...highlightNames) => {
  highlightNames.forEach((highlightName) => {
    if (!isObject(theme[highlightName])) {
      theme[highlightName] = { fg: theme[highlightName] };
    }
  });
};

const parsedThemes = themes.map(([themeName, theme], i) => {
  /* fallbacks -- if a key has not been specified, another key will be used for it */
  // theme["ui.cursorline.primary"] ??= { bg: theme["ui.background"].bg };
  // theme["ui.linenr"] ??= { fg: theme["ui.text"] };
  // theme["ui.linenr.selected"] ??= { fg: theme["ui.selection"].bg };

  // theme["function"] ?? { fg: theme["text"] };
  // theme["function.macro"] ?? { fg: theme["function"] };

  normalize(
    theme,
    "ui.linenr",
    "function",
    "ui.linenr.selected",
    "function.macro",
  );
  const obj = {
    "ui.background": {
      bg: theme["ui.background"].bg,
    },
    "ui.linenr": {
      fg: theme["ui.linenr"].fg ?? theme["ui.text"].fg ?? theme["ui.text"],
    },
  };

  Object.entries(obj).forEach(([highlightName, highlightValue]) => {
    if (
      typeof highlightValue.bg !== "string" &&
      typeof highlightValue.fg !== "string"
    ) {
      console.error(themeName);
      console.error(`${highlightName}: ${JSON.stringify(highlightValue)}`);
      throw new Error(`${highlightName}`);
    }
  });

  return obj;
});

// type Theme = {
//   "ui.linenr": Foreground;
//   keyword: Foreground;
//   function: Foreground;
//   punctuation: Foreground;
//   variable: Foreground;
//   operator: Foreground;
//   "ui.cursorline.primary": Background;
//   "ui.linenr.selected": Foreground;
//   "ui.virtual.ruler": Foreground;
//   "function.macro": Foreground;
//   constant: Foreground;
//   "type.builtin": Foreground;
//   "punctuation.delimiter": Foreground;
//   "variable.parameter": Foreground;
//   "constant.numeric": Foreground;
//   comment: Foreground;
//   "ui.statusline": Foreground & Background;
// };

function themeToHtml(theme) {
  return `\
<pre style="background-color:${theme["ui.background"]}" class="termshot-theme">
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}">  1</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.keyword}">fn</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.function.fg}">main</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">()</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">&lcub;</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}">  2</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.keyword}">let</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.variable}">numbers</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.operator}">=</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}"></span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme["ui.linenr.selected"].fg}">  3</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}"></span>
	<span style="background-color:${theme["ui.linenr"].fg}">
		<font color="${theme["ui.virtual.ruler"]}"></font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme["function.macro"].fg}">vec!</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.punctuation}">[</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.constant}">1</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.punctuation}">,</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.constant}">2</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.punctuation}">,</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.constant}">3</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.punctuation}">,</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.constant}">4</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme.punctuation}">];</font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.cursorline.primary"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}">  4</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.keyword}">let</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.variable}">doubled:</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["type.builtin"]}">Vec</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">&lt;</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["type.builtin"]}">i32</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">&gt;</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}">  5</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.operator}">=</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.variable}">numbers</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}">  6</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["punctuation.delimiter"]}">.</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.function.fg}">iter</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">()</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}">  7</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["punctuation.delimiter"]}">.</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.function.fg}">map</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">(|</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["variable.parameter"]}">n</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">|</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["variable.parameter"]}">n</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.operator}">*</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["constant.numeric"].fg}">2</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">)</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}">  8</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["punctuation.delimiter"]}">.</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.function.fg}">collect</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">()</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["punctuation.delimiter"]}">;</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}">  9</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.comment}">//</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.comment}">[2,</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.comment}">4,</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.comment}">6,</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.comment}">8]</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"> 10</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["function.macro"].fg}">dbg!</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["punctuation.delimiter"]}">(</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.variable}">doubled</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">)</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["punctuation.delimiter"]}">;</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"> 11</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme.punctuation}">&rcub;</font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}">
		<font color="${theme["ui.linenr"].fg}"></font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
	<span style="background-color:${theme["ui.statusline"].bg}">
		<font color="${theme["ui.statusline"].fg}"> NOR   main.rs [+]   1 sel  3:1 </font>
	</span>
	<span style="background-color:${theme["ui.background"].bg}"></span>
</pre>
`;
}
