## Why this change ?

<!---
Explain here why you want to add this change to Helix.

This can be as simple as  "Fixes #issue" or as complex as you need it to be, for refactoring or
deeper issues with design tradeoffs.

For theme and UI changes, post screenshots of the new look (and the old one if relevant).

If your change does not match one of the predefined categories or need a longer explanation, remove
the list and write your description.
--->

<!--- Add issue number in the format "Closes #issue" to make GitHub automatically close it --->
- [ ] Bug fix
- [ ] Feature <!--- Will very often need a longer description and a doc update --->
- [ ] Language changed/added (highlights, queries, injections, config in `languages.toml` ...)
- [ ] Refactor
- [ ] Theme changed/added
- [ ] UI change

-----

## Tasks (not applicable to all PRs)

If something is unneeded, just don't check it, if it was needed, either the CI or a reviewer will
tell you 😄.

<!--- Especially needed if you introduce a new feature or language --->
- [ ] *(Doc)* I ran `cargo xtask docgen` to ensure the generated documentation is up-to-date
- [ ] *(Doc)* I updated the user-facing documentation under `book/`
- [ ] *(Languages)* I ran `cargo xtask query-check` <!--- For when you add/modify a new language --->
- [ ] *(Themes)* I ran `cargo xtask themelint` <!--- For when you add/modify a new theme --->
- [ ] *(User-facing changes)* I added an entry to the [`CHANGELOG.md`](https://github.com/helix-editor/helix/blob/master/CHANGELOG.md)

## Questions

<!--- Do you have points you're not sure about ? Add them here --->
