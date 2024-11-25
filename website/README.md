## New pages

I've put in some time to write step-by-step guides for newcomers to Helix who may be interested. These pages have been written from scratch:

- Basics
- Multiple Cursors
- Text Objects
- Surround
- Language Support -- Guide for installing language servers and formatters
- List of Themes
- Recipes -- Integrations with tools such as Lazygit
- Refactor Examples -- Using helix editor to become efficient
- This Site -- Documentation for the site itself, e.g. how to run and maintain it. Project structure

### Migrated Pages

I've also migrated some pages from the Wiki to make them more discoverable:

- Vision
- Creating a Release
- Architecture
- Debuggers
- Formatters
- Language Servers

## Old docs -> New docs

Map of old docs to new docs, this can be used to setup redirects

| Old page                                                                           | New page                                           |
| ---------------------------------------------------------------------------------- | -------------------------------------------------- |
| [Installation](https://docs.helix-editor.com/install.html)                         | /getting-started/installation                      |
| [Package Managers](https://docs.helix-editor.com/package-managers.html)            | /getting-started/installation#package-managers     |
| [Building from Source](https://docs.helix-editor.com/building-from-source.html)    | /getting-started/installation#building-from-source |
| [Usage](https://docs.helix-editor.com/usage.html)                                  | /getting-started/basics                            |
| [Registers](https://docs.helix-editor.com/registers.html)                          | /usage/registers                                   |
| [Surround](https://docs.helix-editor.com/surround.html)                            | /usage/surround                                    |
| [Text Objects](https://docs.helix-editor.com/textobjects.html)                     | /usage/text-objects                                |
| [Syntax aware motions](https://docs.helix-editor.com/syntax-aware-motions.html)    | /usage/text-objects#syntax-aware-motions           |
| [Keymap](https://docs.helix-editor.com/keymap.html)                                | /reference/keymap                                  |
| [Commands](https://docs.helix-editor.com/commands.html)                            | /reference/typed-commands                          |
| [Language Support](https://docs.helix-editor.com/lang-support.html)                | /help/language-support                             |
| [Migrating from Vim](https://docs.helix-editor.com/from-vim.html)                  | Probably not needed anymore[^1]                    |
| [Configurartion](https://docs.helix-editor.com/configuration.html)                 | /configuration/editor                              |
| [Themes](https://docs.helix-editor.com/themes.html)                                | /reference/custom-themes                           |
| [Key Remapping](https://docs.helix-editor.com/remapping.html)                      | /configuration/remapping                           |
| [Languages](https://docs.helix-editor.com/languages.html)                          | /configuration/languages                           |
| [Adding Languages](https://docs.helix-editor.com/guides/adding_languages.html)     | /contributing/languages                            |
| [Adding text object queries](https://docs.helix-editor.com/guides/textobject.html) | /contributing/textobject-queries                   |
| [Adding indent queries](https://docs.helix-editor.com/guides/indent.html)          | /contributing/indent-queries                       |
| [Injection Queries](https://docs.helix-editor.com/guides/injection.html)           | /contributing/injection-queries                    |

Additionally:

- The landing page has also been revamped
- The news also are included in the docs, available at `/news`

[^1]: Since the news docs include lots of guides, it means that people who already know modal editors will be able to adjust quickly.

[![Built with Starlight](https://astro.badg.es/v2/built-with-starlight/tiny.svg)](https://starlight.astro.build)

# Helix Better Docs

I have completely rebuilt the Helix website and documentation aswell as the landing page to:

- Unite the main website and docs into a single site
- Make Helix more approachable by newcomers by providing detailed guides, documentation and tips on how to use Helix with a lot of new content being written and old content also revamped in places.
- Add a powerful search feature
- Emphasize its beautiful purple colors

The new website is available at `helix.vercel.app`
