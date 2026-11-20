# Commands

- [Typable commands](#typable-commands)
- [Static commands](#static-commands)

## Typable commands

Typable commands are used from command mode and may take arguments. The first part of typable commands are also case-insensitive, meaning `:WQ File1.txt` will be executed as `:wq File1.txt`. Command mode can be activated by pressing `:`. The built-in typable commands are:

{{#include ./generated/typable-cmd.md}}

## Static Commands

Static commands take no arguments and can be bound to keys. Static commands can also be executed from the command picker (`<space>?`). The built-in static commands are:

{{#include ./generated/static-cmd.md}}
