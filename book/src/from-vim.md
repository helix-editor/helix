# Migrating from Vim

Helix's editing model is strongly inspired from vim and kakoune, and a notable
difference from vim (and the most striking similarity to kakoune) is that Helix
follows the `selection → action` model. This means that the whatever you are
going to act on (a word, a paragraph, a line, etc) is selected first and the
action itself (delete, change, yank, etc) comes second. A cursor is simply a
single width selection.

See also Kakoune's [Migrating from Vim](https://github.com/mawww/kakoune/wiki/Migrating-from-Vim) and Helix's [Migrating from Vim](https://github.com/helix-editor/helix/wiki/Migrating-from-Vim).

> TODO: Mention textobjects, surround, registers
