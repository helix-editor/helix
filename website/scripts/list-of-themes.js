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

const parsedThemes = themes
  .flatMap(([themeName, theme], i) => {
    if (themeName !== "catppuccin_mocha") {
      return [];
    }
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
  .map(themeToHtml)
  .join("");

console.log(parsedThemes);

function factory(theme) {
  const withBackground = (children) =>
    `<span style="background-color:${theme["ui.background"].bg}">${children}</span>`;

  return {
    operator: (operator) =>
      withBackground(
        `<font color="${theme["operator"].fg}">${operator}</font>`,
      ),

    linenr: (lineNumber) =>
      withBackground(
        `<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}">  ${lineNumber}</font></span>`,
      ),

    space: (count = 1) =>
      withBackground(
        `<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">${" ".repeat(count)}</font></span>`,
      ),

    fn: (func) =>
      withBackground(
        `<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["function"].fg}">${func}</font></span>`,
      ),

    keyword: (keyword) =>
      withBackground(`<font color="${theme["keyword"].fg}">${keyword}</font>`),

    variable: (variable) =>
      withBackground(
        `<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["variable"].fg}">${variable}</font></span>`,
      ),

    punctuation: (punctuation) =>
      withBackground(
        `<font color="${theme["punctuation"].fg}">${punctuation}</font>`,
      ),
  };
}

function themeToHtml([themeName, theme]) {
  const { operator, keyword, punctuation, space, linenr, fn, variable } =
    factory(theme);

  return `\
<h3>
  <code>${themeName}</code>
</h3>
<pre aria-label="${themeName} theme preview" aria-role="img" style="background-color:${theme["ui.background"].bg}">${space(2)}${linenr(1)}${space(2)}${keyword("fn")}${space()}${fn("main")}${punctuation("()")}${space()}${punctuation("{")}${space(14)}
${space(2)}${linenr(2)}${space(4)}${keyword("let")}${space()}${variable("numbers")}${space()}${operator("=")}${space(10)}
<span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.linenr.selected"].fg}">  3</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.linenr.selected"].fg}"><font color="${theme["ui.cursor"].fg}"> </font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.virtual"].fg}">   </font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["function.macro"].fg}">vec!</font></span><span style="background-color:${theme["ui.cursorline.primary"]}"><font color="${theme["punctuation"].fg}">[</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["constant.numeric"].fg}">1</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["punctuation"].fg}">,</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["constant.numeric"].fg}">2</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["punctuation"].fg}">,</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["constant.numeric"].fg}">3</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["punctuation"].fg}">,</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["constant.numeric"].fg}">4</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["punctuation.delimiter"].fg}">];</font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.cursorline.primary"].bg}"><font color="${theme["ui.background"].fg}">   </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}">  4</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}">  </font></span>${keyword("let")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">doubled:</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["type.builtin"].fg}">Vec</font></span>${punctuation("&lt;")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["type.builtin"].fg}">i32</font></span>${punctuation("&gt;")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}"> </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}">  5</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}">    </font></span>${operator("=")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">numbers</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">           </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}">  6</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}">        </font></span>${punctuation(".")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["function"].fg}">iter</font></span>${punctuation("()")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">         </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}">  7</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}">        </font></span>${punctuation(".")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["function"].fg}">map</font></span>${punctuation("(|")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["variable.parameter"].fg}"><i>n</i></font></span>${punctuation("|")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["variable.parameter"].fg}"><i>n</i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span>${operator("*")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["constant.numeric"].fg}">2</font></span>${punctuation(")")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}"> </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}">  8</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}">        </font></span>${punctuation(".")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["function"].fg}">collect</font></span>${punctuation("();")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">     </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}">  9</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["comment"].fg}"><i>//</i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"><i> </i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["comment"].fg}"><i>[2,</i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"><i> </i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["comment"].fg}"><i>4,</i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"><i> </i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["comment"].fg}"><i>6,</i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"><i> </i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["comment"].fg}"><i>8]</i></font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">       </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}"> 10</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["function.macro"].fg}">dbg!</font></span>${punctuation("(")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">doubled</font></span>${punctuation(");")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">        </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.linenr"].fg}"> 11</font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">  </font></span>${punctuation("}")}<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.virtual"].fg}"> </font></span><span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">                       </font></span>
<span style="background-color:${theme["ui.statusline"].bg}"><font color="${theme["ui.statusline"].fg}"> NOR   main.rs [+]   1 sel  3:1 </font></span>
<span style="background-color:${theme["ui.background"].bg}"><font color="${theme["ui.background"].fg}">                                </font></span>
</pre>`;
}
