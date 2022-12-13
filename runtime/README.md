Runtime

Helix looks for some additional files on startup to provide features
like the tutorial (`:tutor`) or syntax highlighting.

The directory that contains these files is called the _runtime
directory_. Check the "Runtime directory" line from `hx --health` to
find where Helix looks for the runtime directory on your machine.

When installed correctly, the runtime directory should have a shape
like this:

```
$ tree $HELIX_RUNTIME
runtime
├── tutor            The tutorial file.
├── grammars         Compiled tree-sitter parsers.
│   ├── rust.so
│   ├── awk.so
│   ├── bash.so
│   └── sources      (Optional) source files for tree-sitter parsers.
│       └── ...      Used in 'hx --grammar build'.
├── help             Documentation for the :help command.
│   └── ...
└── queries          Tree-sitter queries.
    ├── rust
    │   ├── highlights.scm
    │   ├── injections.scm
    │   └── textobjects.scm
    └── ...
```

When looking up the runtime directory, Helix checks these places in
order and returns the first value that exists:

* The `HELIX_RUNTIME` environment variable. If that variable is set,
  the value is treated as the path to the runtime directory.
* The root of the Helix repository under a directory named `runtime`,
  if running from source.
* The config directory under a directory named `runtime`.
* The parent directory of the Helix executable under a directory
  named `runtime`.

If you installed Helix through a package manager, the package manager
most likely installed the runtime directory and wrapped Helix in a
script that sets `HELIX_RUNTIME`.

If you are building from source, you should set up a link between
the `runtime` directory in the Helix git repository and the config
directory.

| OS                   | Command                                          |
| -------------------- | ------------------------------------------------ |
| Windows (Cmd)        | `xcopy /e /i runtime %AppData%\helix\runtime`    |
| Windows (PowerShell) | `xcopy /e /i runtime $Env:AppData\helix\runtime` |
| Linux / macOS        | `ln -s $PWD/runtime ~/.config/helix/runtime`     |

