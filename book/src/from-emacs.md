# Migrating from Emacs

Helix is a modal editor stronly inspired by Vim and Kakoune. Users of doom Emacs
will find it familiar. Helix follows the `selection → action` model. Where many Emacs
commands can work with the item under point Helix will require a selection first.
A cursor in Helix is simply a single width selection.

Helix has buffers too. <space>-b will let you shift between them.

Commands can be prefixed with numbers to change their behavior in much the same way
as C-u N <command> does in Emacs.

Registers in Helix work much like registers do in Emacs as well. Except there is
currently no mark saving and moving. Check out the jumplist for an alternative.

Many of the same behaviors from rectangle mark mode can be reproduced using multiple
cursors.
