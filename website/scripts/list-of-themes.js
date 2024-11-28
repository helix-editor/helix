/*
 * This script creates @/generated/list-of-themes/* files
 *
 * It automatically runs on `pnpm dev`, `pnpm build` and `pnpm preview` and can be manually run with `pnpm termshots`
 */

import toml from "@iarna/toml";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

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

const writeDirectory = path.join(
  scriptDirectory,
  "..",
  "src",
  "termshots",
  "generated",
  "list-of-themes",
);

if (!fs.existsSync(writeDirectory)) {
  fs.mkdirSync(writeDirectory, { recursive: true });
}

function factory(theme) {
  const backgroundWrapperCreator = (cursorline) => (children) =>
    `<span style="background-color:${theme[cursorline ? "ui.cursorline.primary" : "ui.background"].bg}">${children}</span>`;

  const withBackground = backgroundWrapperCreator(false);
  const withBackgroundCursorline = backgroundWrapperCreator(true);

  const raw = {
    operator: (background) => (operator) =>
      background(`<font color="${theme["operator"].fg}">${operator}</font>`),

    linenr: (background) => (lineNumber) =>
      background(
        `<font color="${theme["ui.linenr"].fg}">${" ".repeat(3 - String(lineNumber).length)}${lineNumber}</font>`,
      ),

    space:
      (background) =>
      (count = 1) =>
        background(
          `<font color="${theme["ui.background"].fg}">${" ".repeat(count)}</font>`,
        ),

    fn:
      (background, subtype = "") =>
      (func) =>
        background(
          `<font color="${theme[`function${subtype}`].fg}">${func}</font>`,
        ),

    keyword: (background) => (keyword) =>
      background(`<font color="${theme["keyword"].fg}">${keyword}</font>`),

    variable:
      (background, subtype = "") =>
      (variable) =>
        background(
          `<font color="${theme[`variable${subtype}`].fg}">${subtype === ".parameter" ? "<i>" : ""}${variable}${subtype === ".parameter" ? "</i>" : ""}</font>`,
        ),

    punctuation:
      (background, subtype = "") =>
      (punctuation) =>
        background(
          `<font color="${theme[`punctuation${subtype}`].fg}">${punctuation}</font>`,
        ),

    number:
      (background, subtype = ".numeric") =>
      (n) =>
        background(
          `<font color="${theme[`constant${subtype}`].fg}">${n}</font>`,
        ),

    comment:
      (background, subtype = "") =>
      (n) =>
        background(
          `<font color="${theme[`comment${subtype}`].fg}"><i>${n}</i></font>`,
        ),

    type:
      (background, subtype = "") =>
      (t) =>
        background(`<font color="${theme[`type${subtype}`].fg}">${t}</font>`),
  };

  const utils = {
    number: raw.number(withBackground),
    operator: raw.operator(withBackground),
    linenr: raw.linenr(withBackground),
    fn: raw.fn(withBackground),
    space: raw.space(withBackground),
    keyword: raw.keyword(withBackground),
    variable: raw.variable(withBackground),
    comment: raw.comment(withBackground),
    param: raw.variable(withBackground, ".parameter"),
    punctuation: raw.punctuation(withBackground),
    macro: raw.fn(withBackground, ".macro"),
    type: raw.type(withBackground),
    typeBuiltin: raw.type(withBackground, ".builtin"),
    punctuationDelimiter: raw.punctuation(withBackground, ".delimiter"),
    cursorline: {
      macro: raw.fn(withBackgroundCursorline, ".macro"),
      punctuationDelimiter: raw.punctuation(
        withBackgroundCursorline,
        ".delimiter",
      ),
      number: raw.number(withBackgroundCursorline),
      operator: raw.operator(withBackgroundCursorline),
      linenr: raw.linenr(withBackgroundCursorline),
      fn: raw.fn(withBackgroundCursorline),
      space: raw.space(withBackgroundCursorline),
      keyword: raw.keyword(withBackgroundCursorline),
      variable: raw.variable(withBackgroundCursorline),
      punctuation: raw.punctuation(withBackgroundCursorline),
    },
  };

  return utils;
}

function themeToHtml([themeName, theme]) {
  const {
    operator,
    keyword,
    punctuation,
    comment,
    space,
    linenr,
    fn,
    param,
    macro,
    typeBuiltin,
    variable,
    punctuationDelimiter,
    cursorline,
    number,
  } = factory(theme);

  // prettier-ignore
  const lines = [
    [ `<pre aria-label="${themeName} theme preview" aria-role="img" style="background-color:${theme["ui.background"].bg}">`, space(2), linenr(1), space(2), keyword("fn"), space(), fn("main"), punctuation("()"), space(), punctuation("{"), space(14), ],
    [ space(2), linenr(2), space(4), keyword("let"), space(), variable("numbers"), space(), operator("="), space(10), ],
    [ cursorline.space(2), `<span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.linenr.selected"].fg}">  3</font></span>`, cursorline.space(2), `<span style="background-color:${theme["ui.linenr.selected"].fg}"><font color="${theme["ui.cursor"].fg}"> </font></span>`, cursorline.space(3), cursorline.macro("dbg!"), cursorline.punctuation("["), cursorline.number(1), cursorline.punctuation(","), cursorline.space(), cursorline.number(2), cursorline.punctuation(","), cursorline.space(), cursorline.number(3), cursorline.punctuation(","), cursorline.space(), cursorline.number(4), cursorline.punctuation("]"), cursorline.punctuationDelimiter(";"), cursorline.space(4), ],
    [ space(2), linenr(4), space(4), keyword("let"), space(), variable("doubled"), punctuation(":"), space(1), typeBuiltin("Vec"), punctuation("<"), typeBuiltin("i32"), punctuation(">"), ],
    [ space(2), linenr(6), space(10), punctuation("."), fn("iter"), punctuation("()"), space(9), ],
    [ space(2), linenr(7), space(10), punctuation("."), fn("map"), punctuation("(|"), param("n"), punctuation("|"), space(), param("n"), space(), operator("*"), space(), number(2), punctuation(")"), ],
    [ space(2), linenr(8), space(10), punctuation("."), fn("collect"), punctuation("()"), punctuationDelimiter(";"), ],
    [ space(2), linenr(9), space(4), comment("//"), space(), comment("[2,"), space(), comment("4,"), space(), comment("6,"), space(), comment("8]"), ],
    [ space(2), linenr(10), space(4), macro("dbg!"), punctuation("("), variable("doubled"), punctuation(")"), punctuationDelimiter(";"), ],
    [ space(2), linenr(11), space(2), punctuation("}") ],
    [ `<span style="background-color:${theme["ui.statusline"].bg}"><font color="${theme["ui.statusline"].fg}"> NOR   main.rs [+]   1 sel  3:1 </font></span>`, ],
    [space(32)],
  ];

  return [
    themeName,
    `\
<h3>
  <code>${themeName}</code>
</h3>
${lines.map((line) => line.join("")).join("\n")}`,
  ];
}

const indexPath = path.join(writeDirectory, "..", "ListOfThemes.astro");

if (fs.existsSync(indexPath)) {
  fs.rmSync(indexPath);
}

const indexStream = fs.createWriteStream(indexPath, { flags: "a" });

indexStream.write(`\
---
/*
 * This file has been automatically generated
 */

`);

const htmlThemes = themes
  .flatMap(([themeName, theme]) => {
    const background = {
      bg: theme["ui.background"].bg,
      fg: theme["ui.background"].fg || "#ffaaaa",
    };

    const getColor = (key, fallback) => {
      const value = theme[key];
      if (typeof value === "string") {
        return { fg: value };
      } else if (!value) {
        return fallback();
      }
      return value;
    };

    const getFallbackColor = (primaryKey, fallbackFn) => {
      const value = theme[primaryKey];
      if (typeof value === "string") {
        return { fg: value };
      } else if (!value) {
        return fallbackFn();
      }
      return value;
    };

    const getUiBackground = () => background;
    const getVariable = () => getColor("variable", () => background.fg);
    const getFunction = () => getFallbackColor("function", getVariable);
    const getMacro = () => getFallbackColor("function.macro", getFunction);
    const getKeyword = () => getFallbackColor("keyword", getVariable);
    const getUiLinenr = () => getColor("ui.linenr", () => background.fg);
    const getUiLinenrSelected = () =>
      getFallbackColor("ui.linenr.selected", getHighlight);
    const getOperator = () => getColor("operator", () => background.fg);
    const getPunctuation = () => getColor("punctuation", () => background.fg);
    const getConstant = () => getFallbackColor("constant", getVariable);
    const getConstantNumeric = () =>
      getFallbackColor("constant.numeric", getConstant);
    const getPunctuationDelimiter = () =>
      getFallbackColor("punctuation.delimiter", getPunctuation);
    const getVariableParameter = () =>
      getFallbackColor("variable.parameter", getVariable);
    const getComment = () => getColor("comment", () => background.fg);
    const getType = () => getFallbackColor("type", getVariable);
    const getTypeBuiltin = () => getFallbackColor("type.builtin", getType);

    const getUiStatusline = () => {
      const uiStatusline = theme["ui.statusline"];
      if (!uiStatusline) return {};
      return {
        fg: uiStatusline.fg || background.fg,
        bg: uiStatusline.bg || background.bg,
      };
    };

    const getHighlight = () => getColor("ui.highlight", getUiBackground);
    const getUiCursorline = () =>
      getFallbackColor("ui.cursorline", getHighlight);
    const getUiCursorlinePrimary = () =>
      getFallbackColor("ui.cursorline.primary", getUiCursorline);

    const getUiVirtual = () =>
      getFallbackColor("ui.virtual", getUiVirtualInlayHint);

    const getUiCursor = () =>
      getFallbackColor("ui.cursor.primary", () => background.fg);

    const getUiVirtualInlayHint = () =>
      getColor("ui.virtual.inlay-hint", () => background.fg);
    const getUiVirtualIndentGuide = () =>
      getFallbackColor("ui.virtual.indent-guide", getUiVirtualInlayHint);
    const getUiVirtualRuler = () =>
      getFallbackColor("ui.virtual.ruler", getUiVirtualIndentGuide);

    return [
      [
        themeName,
        theme.palette,
        {
          "ui.background": getUiBackground(),
          keyword: getKeyword(),
          variable: getVariable(),
          "variable.parameter": getVariableParameter(),
          "ui.statusline": getUiStatusline(),
          "ui.linenr": getUiLinenr(),
          function: getFunction(),
          "function.macro": getMacro(),
          operator: getOperator(),
          punctuation: getPunctuation(),
          "punctuation.delimiter": getPunctuationDelimiter(),
          constant: getConstant(),
          "constant.numeric": getConstantNumeric(),
          comment: getComment(),
          "type.builtin": getTypeBuiltin(),
          "ui.cursorline.primary": getUiCursorlinePrimary(),
          "ui.linenr.selected": getUiLinenrSelected(),
          "ui.virtual.ruler": getUiVirtualRuler(),
          "ui.virtual": getUiVirtual(),
          "ui.cursor": getUiCursor(),
        },
      ],
    ];
  })
  .map(([name, palette, theme]) => {
    return [
      name,
      Object.fromEntries(
        Object.entries(theme).map(([scope, color]) => {
          const isHex = (s) => s.startsWith("#");
          if (typeof color.fg === "string") {
            if (!isHex(color.fg)) {
              color.fg = palette[color.fg];
            }
          }
          if (typeof color.bg === "string") {
            if (!isHex(color.bg)) {
              color.bg = palette[color.bg];
            }
          }
          return [scope, color];
        }),
      ),
    ];
  })
  .map(themeToHtml);

const themePascalNames = htmlThemes.map(([themeName, theme]) => {
  const themeNamePascalCase = themeName
    // create dash intermediary separators for proper PascalCase conversion
    .replaceAll("_", "-")
    .replace(/\w+/g, (w) => {
      return w[0].toUpperCase() + w.slice(1).toLowerCase();
    })
    // only keep alphanumeric chars
    .replace(/\W/g, "");

  const themeWritePath = path.join(
    writeDirectory,
    `${themeNamePascalCase}.astro`,
  );

  const comment = `\
---
/*
 * This file has been automatically generated by the script \`pnpm termshot-themes\`
 *
 * See helix-editor.vercel.app/contributing/this-site#termshots for more information
 */
---

`;
  const htmlTheme = `${comment}${theme}`;

  fs.writeFileSync(themeWritePath, htmlTheme);

  indexStream.write(
    `import ${themeNamePascalCase} from "./list-of-themes/${themeNamePascalCase}.astro"\n`,
  );

  return themeNamePascalCase;
});

indexStream.write("---\n\n");

themePascalNames.forEach((themeFilepath) => {
  indexStream.write(`<${themeFilepath} />\n\n`);
});
