# Spell checking

Helix can spell-check documents and surface misspellings as diagnostics.
Corrections and "add to dictionary" are offered as [code
actions](./commands.md), alongside any from language servers.

Spell checking is opt-in. Helix will use tree-sitter to check only requested
regions of a language to avoid noisy diagnostics for keywords. Multiple
dictionaries may be configured at once. A word is accepted if any configured
dictionary allows it.

## Enabling

There are several ways to turn spell checking on:

1. Per buffer, with the command:

   ```
   :set-spelling-language <language>
   ```

   Pass several languages to check against all of them (`:set-spelling-language
   en_US en_GB`), or `off` to disable checking for the buffer. With no argument
   it reports the current language(s). This choice overrides the settings below
   and persists across config reloads for that buffer.

2. Per path, with an [`.editorconfig`](https://editorconfig.org/)
   `spelling_language` key:

   ```ini
   [*.md]
   spelling_language = en_US
   ```

3. Per language, in `languages.toml` (see [Languages](./languages.md)):

   ```toml
   [[language]]
   name = "markdown"
   spelling.languages = ["en_US"]
   ```

4. Globally, in `config.toml` under [`[editor.spelling]`](./editor.md#editorspelling-section):

   ```toml
   [editor.spelling]
   languages = ["en_US"]
   ```

## Scope

What gets checked is controlled per language by a tree-sitter `spellcheck.scm`
query: a node captured `@spell` is checked, and `@nospell` excludes part of one.
Most languages inject a shared `comment` grammar, so their comments are checked
with no language-specific query, and Markdown also checks its prose. Which
regions are checked therefore depends on the queries present in your runtime
directories, and grows as more are added. Strings and identifiers are checked
only where a language's query opts them in. A file with no query at all is
checked in full, like plain text.

See [Adding spellcheck queries](./guides/spellcheck.md) to extend coverage to a
new language.

## Dictionaries

Dictionaries are Hunspell `.aff`/`.dic` pairs loaded from
`dictionaries/<language>/<language>.{aff,dic}` in the [runtime
directories](./install.md#configuring-helixs-runtime-files). `hx --health`
lists those directories.

Helix bundles only `en_US`. To add another language, drop its Hunspell files
into a runtime directory, named after the language code you reference in the
config. For example, for `de_DE`:

```
~/.config/helix/runtime/dictionaries/de_DE/de_DE.aff
~/.config/helix/runtime/dictionaries/de_DE/de_DE.dic
```

Hunspell dictionaries for most languages are distributed with LibreOffice and
by the various aspell/Hunspell projects.

### Personal dictionary

The "Add to dictionary" code action accepts a word permanently. It is appended
to a personal dictionary, one word per line, under Helix's state directory:

```
<state>/dictionaries/<language>.txt
```

(`<state>` is e.g. `~/.local/state/helix` on Linux.) The file is namespaced per
language, so a word added for one language is not accepted in another. You can
edit it by hand; entries are loaded the next time that language's dictionary is
read.

## Tuning

Spell checking produces false positives on names, jargon, and code-like tokens.
URLs and email addresses are skipped automatically. Beyond that, three knobs
(settable globally under `[editor.spelling]` and per language in
`languages.toml`) reduce noise:

| Key               | Description
| ---               | ---
| `words`           | Extra accepted words, matched case-insensitively.
| `ignore-regexes`  | Tokens matching any of these are not checked (e.g. `"^[A-Z0-9_]+$"`).
| `min-word-length` | Tokens shorter than this are not checked.

When both global and per-language settings are present, `languages` and
`min-word-length` are replaced by the language's value, while `words` and
`ignore-regexes` are added to the global lists. See
[`[editor.spelling]`](./editor.md#editorspelling-section) for the full
reference.
