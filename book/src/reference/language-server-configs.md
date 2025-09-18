# Language server configurations

## Introduction

Use `hx --health` to check the status of any of the language servers that are
configured by default or in your personal `languages.toml` file. See
[the documentation](https://docs.helix-editor.com/languages.html) for more
information about where these files are and how they work.

For Helix to use a language server, it must first be installed onto your
computer. If it is one of the default language servers, it will be used
automatically with no further setup needed once it is installed.

You can see the configuration of the default language servers for each language
in the
[helix repo `languages.toml` file](https://github.com/helix-editor/helix/blob/master/languages.toml).
By adding the configurations below, you can supplement or replace the default
configurations.

Check if your operating system repository has them available, or install them
manually, following the instructions below.

If your language server does not support stdio, you can use `netcat` as a
drop-in proxy, just add this to your `languages.toml`:

```toml
[language-server.example-language-server]
command = "nc" 
args = ["127.0.0.1", "6008"]

[[language]]
name = "example-language"
language-servers = [ "example-language-server" ]
```

Much of this information was originally sourced from
[nvim-lspconfig](https://github.com/neovim/nvim-lspconfig/blob/master/doc/configs.md),
[emacs-lspconfig](https://github.com/emacs-lsp/lsp-mode/wiki/Install-Angular-Language-server)
thanks to those authors!

# Table of Contents

<!--toc:start-->
- [Languages](#languages)
  - [Angular](#angular)
  - [Ansible](#ansible)
  - [Astro](#astro)
  - [AWK](#awk)
  - [Bash](#bash)
  - [Bass](#bass)
  - [Bicep](#bicep)
  - [BQN](#bqn)
  - [C/C++](#cc)
  - [Clojure](#clojure)
  - [CMake](#cmake)
  - [Crystal](#crystal)
  - [CSS](#css)
  - [C#](#c)
  - [D](#d)
  - [Dart](#dart)
  - [Deno](#deno)
  - [Docker](#docker)
  - [Docker Compose](#docker-compose)
  - [dot Graphviz](#dot-graphviz)
  - [Elixir](#elixir)
  - [Elm](#elm)
  - [Forth](#forth)
  - [FSharp/F#](#fsharpf)
  - [GDScript](#gdscript)
  - [Github Actions](#github-actions)
  - [Gleam](#gleam)
  - [Glimmer](#glimmer)
  - [GLSL](#glsl)
  - [Go](#go)
  - [GraphQL](#graphql)
  - [Haskell](#haskell)
  - [Helm](#helm)
  - [HTML](#html)
  - [Java](#java)
  - [JavaScript](#javascript)
  - [JSON](#json)
  - [Jsonnet](#jsonnet)
  - [Julia](#julia)
  - [Kotlin](#kotlin)
  - [LaTex](#latex)
  - [Lean 3](#lean-3)
  - [Lua](#lua)
  - [Markdoc](#markdoc)
  - [Markdown](#markdown)
  - [MATLAB](#matlab)
  - [Mint](#mint)
  - [Mojo](#mojo)
  - [Nim](#nim)
  - [Nix](#nix)
  - [OCaml](#ocaml)
  - [Odin](#odin)
  - [OpenPolicyAgent](#openpolicyagent)
  - [Perl](#perl)
  - [Pest](#pest)
  - [PHP](#php)
  - [PKGBUILD](#pkgbuild)
  - [PowerShell](#powershell)
  - [Prisma](#prisma)
  - [Prolog](#prolog)
  - [Python](#python)
  - [R](#r)
  - [Racket](#racket)
  - [ReScript](#rescript)
  - [Rust](#rust)
  - [Scala](#scala)
  - [Scheme](#scheme)
  - [SCSS](#scss)
  - [Slint](#slint)
  - [Smithy](#smithy)
  - [Ruby](#ruby)
  - [solc](#solc)
  - [Svelte](#svelte)
  - [Swift](#swift)
  - [Sql](#sql)
  - [TailwindCSS](#tailwindcss)
  - [Terraform](#terraform)
  - [TOML](#toml)
  - [TypeScript](#typescript)
  - [TypeSpec](#typespec)
  - [Typst](#typst)
  - [Uiua](#uiua)
  - [Unison](#unison)
  - [V](#v)
  - [Vue](#vue)
  - [WGSL](#wgsl)
  - [Wikitext](#wikitext)
  - [XML](#xml)
  - [YAML](#yaml)
  - [Zig](#zig)
- [Other Language Server Protocol Types](#other-language-server-protocol-types)
  - [Harper-ls](#harper-ls)
  - [Vale-ls](#vale-ls)
<!--toc:end-->

# Languages

## Angular

https://github.com/angular/vscode-ng-language-service

> In this example we use `npm`, the exact same results can be achieved with `pnpm`, by replacing `npm` with `pnpm`.
```sh
# Install language-service(backend), typescript and language-server(vscode-ng-ls..) in one go. This provides all necessary dependencies:
npm install -g @angular/language-service@next typescript @angular/language-server

# Append this to your shell config (~/.bashrc, ~/.zshrc, etc.) to make it persistent
echo 'export NODE_MODULES_GLOBAL="$(npm root -g)"' >> ~/.bashrc
```
`languages.toml`
```toml
[language-server.angular-ls]
command = "ngserver"

args = [
  "--ngProbeLocations",
  "${$NODE_MODULES_GLOBAL}",
  "--tsProbeLocations",
  "${$NODE_MODULES_GLOBAL}",
  "--stdio",
  # If this for some reason doesn't work, try replacing ${NODE_MODULES_GLOBAL} with the absolute paths, e.g. output of `npm root -g`
]


file-types = ["ts", "typescript", "html"]

[[language]]
name = "typescript"
language-servers = ["angular-ls", "typescript-language-server"]

[[language]]
name = "html"
language-servers = ["angular-ls", "vscode-html-language-server"]
```

To enable the latest Language Service features, set the ```strictTemplates``` option in ```tsconfig.json``` by setting strictTemplates to true, as shown in the following example:
```json
"angularCompilerOptions": {
  "strictTemplates": true
}
```
For more information, see https://angular.dev/tools/language-service.

## Ansible

If your OS package manager hasn't packaged
[`ansible-language-server`](https://ansible.readthedocs.io/projects/vscode-ansible/als/),
you can install it from NPM:

```sh
npm i -g @ansible/ansible-language-server
```

In turn, this binary will check if `ansible-lint` and `yamllint` are installed
on your system and use them if found.

In addition to highlighting and linting your code, the Ansible language server
can also be used to look up docs for Ansible keywords. With your cursor on a
keyword, use "\<space\>k".

## Astro

https://github.com/withastro/language-tools/tree/main/packages/language-server

```sh
npm i -g @astrojs/language-server
```

Sample settings in `languages.toml`

```toml
[language-server.astro-ls]
command = "astro-ls"
args = ["--stdio"]
config = {typescript = {tsdk = "/Users/user/.bun/install/global/node_modules/typescript/lib"}, environment = "node"}

[[language]]
name = "astro"
auto-format = true
language-servers = [ "astro-ls" ]
```

Please note that a valid `config.typescript.tsdk` path must be passed to the LSP
config. You will need `typescript` installed. If you have `typescript` installed
globally you can find where by running `npm list -g | head -1`.

## AWK

https://github.com/Beaglefoot/awk-language-server

```sh
npm i -g "awk-language-server@>=0.5.2"
```

## Bash

Language server for Bash, written using tree-sitter in TypeScript.

https://github.com/mads-hartmann/bash-language-server

`bash-language-server` can be installed via `NPM`:

```sh
npm i -g bash-language-server
```

Note that `bash-language-server` has external dependencies for certain features!

| Feature | Binary |
| ------- | ------ |
| Diagnostics | `shellcheck` |
| Formatting | `shfmt` |

## Bass

https://github.com/vito/bass/releases/latest

Bass's language server is built in to the `bass` command as `bass --lsp`. See
the [Guide](https://bass-lang.org/guide.html#getting-started) for more info.

## Bicep

https://github.com/Azure/bicep/releases/latest

The Bicep language server is published separately as a Windows package. Download
the bicep-langserver.zip from the releases page.

To run this under Linux/WSL you will need to install the `dotnet` runtime
[for your OS](https://learn.microsoft.com/en-nz/dotnet/core/install/linux?WT.mc_id=dotnet-35129-website).

Unzip this in a directory of your choosing for example, `/home/myUser/.cache/`.
Create the following bash script somewhere in your `$PATH`:

`/usr/local/bin/bicep-langserver`

```shell
#!/usr/bin/env bash

exec dotnet /home/myUser/.cache/bicep-langserver/Bicep.LangServer.dll
```

## BQN

bqnlsp: https://git.sr.ht/~detegr/bqnlsp, which depends on:

[cbqn-sys](https://github.com/Detegr/cbqn-sys) and
[cbqn-rs](https://github.com/Detegr/cbqn-rs)

Sample settings in `languages.toml`

```toml
[language-server.bqnlsp]
command = "bqnlsp"

[[language]]
name = "bqn"
file-types = ["bqn"]
comment-token = "#"
indent = { tab-width = 2, unit = "  " }
shebangs = ["bqn", "cbqn"]
roots = []
injection-regex = "bqn"
scope = "scope.bqn"
language-servers = ["bqnlsp"]
language-id = "bqn"
```

Note: you can input the glyphs by
[key remapping](https://docs.helix-editor.com/remapping.html) in `config.toml`
like:

```toml
[keys.insert."\\"]
"=" = [ ":insert-output /bin/echo -n ×", "move_char_right" ]
minus = [ ":insert-output /bin/echo -n ÷", "move_char_right" ]
"+" = [ ":insert-output /bin/echo -n ⋆", "move_char_right" ]
# ...
```

## C/C++

https://clangd.llvm.org/installation.html

**NOTE:** Clang >= 9 is recommended!

clangd relies on a
[JSON compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html)
specified as `compile_commands.json` or, for simpler projects, a
`compile_flags.txt`. For details on how to automatically generate one using
CMake look
[here](https://cmake.org/cmake/help/latest/variable/CMAKE_EXPORT_COMPILE_COMMANDS.html).
Alternatively, you can use [Bear](https://github.com/rizsotto/Bear).

## Clojure

Please go to the installation page. https://clojure-lsp.io/installation/

An example for MacOs/Linux (copied from the installation page)

```shell
brew remove clojure-lsp # if you have old clojure-lsp installed via brew
brew install clojure-lsp/brew/clojure-lsp-native
```

## CMake

CMake LSP Implementation.

https://github.com/regen100/cmake-language-server

## Crystal

Core `languages.toml` contains configuration options, view at the top level of
this repo.\
Install the unofficial LSP server
[Crystalline](https://github.com/elbywan/crystalline). Below is copied from the
LSP repo.

##### Linux (x86_64)

```sh
wget https://github.com/elbywan/crystalline/releases/latest/download/crystalline_x86_64-unknown-linux-musl.gz -O crystalline.gz &&\
gzip -d crystalline.gz &&\
chmod u+x crystalline
```

###### ArchLinux

```sh
yay -S crystalline
```

##### MacOS

Install using [homebrew](https://brew.sh):

```sh
brew install crystalline
```

## CSS

https://github.com/hrsh7th/vscode-langservers-extracted

`vscode-css-language-server` can be installed via `npm`:

```sh
npm i -g vscode-langservers-extracted
```

## C#

https://github.com/omnisharp/omnisharp-roslyn OmniSharp server based on Roslyn
workspaces

`omnisharp-roslyn` can be installed by downloading and extracting a release for
your platform from
[here](https://github.com/OmniSharp/omnisharp-roslyn/releases).
Be sure to use the "net6.0" version of Omnisharp - https://github.com/OmniSharp/omnisharp-roslyn/issues/2676.

Omnisharp can also be built from source by following the instructions
[here](https://github.com/omnisharp/omnisharp-roslyn#downloading-omnisharp).

Omnisharp requires the [dotnet-sdk](https://dotnet.microsoft.com/download) to be
installed.

### Usage

#### Linux and Windows

To use Omnisharp, you only need to have the `OmniSharp` binary in your
environment path. The default `languages.toml` configuration should work fine.

#### macOS

Download the `-netX.0` build. Because OmniSharp is not shipped as a binary file,
instead as `OmniSharp.dll`, it needs to be run using `dotnet`. As such the
`languages.toml` config should be changed to this:

```toml
[language-server.omnisharp]
command = "dotnet"
args = [ "path/to/OmniSharp.dll", "--languageserver" ]
```

If the language server immediately exits or otherwise doesn't appear to work,
try running `dotnet restore` and/or `dotnet build` in the current project
directory.

## D

Serve-D.

https://github.com/Pure-D/serve-d

Install using `dub fetch serve-d`

## Dart

Language server for dart.

https://github.com/dart-lang/sdk/tree/master/pkg/analysis_server/tool/lsp_spec

## Deno

Install Deno from https://docs.deno.com/runtime/getting_started/installation/

Deno requires custom configuration in `languages.toml` see the [Helix environment setup section](https://docs.deno.com/runtime/getting_started/setup_your_environment/#helix) for an up-to-date configuration.

## Docker

https://github.com/rcjsuen/dockerfile-language-server-nodejs

`docker-langserver` can be installed via `npm`:

```sh
npm install -g dockerfile-language-server-nodejs
```

## Docker Compose

https://github.com/microsoft/compose-language-service

`docker-compose-langserver` can be installed via `npm`:

```sh
npm install -g @microsoft/compose-language-service
```

## dot Graphviz

https://github.com/nikeee/dot-language-server

```sh
npm i -g dot-language-server
```

## Elixir

https://github.com/elixir-lsp/elixir-ls

`elixir-ls` can be installed by following the instructions
[here](https://github.com/elixir-lsp/elixir-ls#building-and-running).

```bash
curl -fLO https://github.com/elixir-lsp/elixir-ls/releases/latest/download/elixir-ls.zip
unzip elixir-ls.zip -d /path/to/elixir-ls
# Unix
chmod +x /path/to/elixir-ls/language_server.sh
```

Rename `language_server.sh` to `elixir-ls` and add it to your `$PATH` as this is
how `helix` expects to find it.

##### MacOS

Install using [homebrew](https://brew.sh):

```sh
brew install elixir-ls
```

## Elm

https://github.com/elm-tooling/elm-language-server#installation

```sh
npm install -g elm elm-test elm-format @elm-tooling/elm-language-server
```

## Forth

https://github.com/alexanderbrevig/forth-lsp#install

```sh
cargo install forth-lsp
```

## FSharp/F#

https://github.com/fsharp/FsAutoComplete

```sh
dotnet tool install --global fsautocomplete
```

## GDScript

We need to install `nc` or `netcat`. Port 6005 is used in Godot 4.0 beta6. You
will find the right value in the editor configuration panel.

```toml
[language-server.godot]
command = "nc" 
args = [ "127.0.0.1", "6005"]

[[language]]
name = "gdscript"
language-servers = [ "godot" ]
```

**For Windows 10/11**

Use `winget` to install `nmap`. This will install `ncat`.

```powershell
winget install nmap
```

Once installed, make sure the folder that `nmap` is now located at is added to
your PATH, as `winget` fails to do this automatically for some people.

In Godot 3.5.1 port used is `6008`. You have to change the command used also.
Instead of `nc` type `ncat` and modify the port. You can find the port when you
open the Godot editor and navigate here:
`Editor -> Editor Settings -> Network -> Language Server -> Remote Port`.

```toml
[language-server.godot]
command = "ncat" 
args = [ "127.0.0.1", "6008"]

[[language]]
name = "gdscript"
language-servers = [ "godot" ]
```
## Github Actions
`npm install --global gh-actions-language-server`

```toml
[language-server.gh-actions-language-server]
command = 'gh-actions-language-server'
args = ['--stdio']
config = { sessionToken = ""}

# GitHub Actions workflow language
[[language]]
name = "yaml"
file-types = [{ glob = ".github/workflows/*.yml" }, { glob = ".github/workflows/*.yaml" }]
language-servers = ["gh-actions-language-server"]
roots = [".github", ".git"]
```
## Gleam

Starting with version `0.21.0`, the Gleam language server is built-in to the
`gleam` command-line interface.
[See the official announcement for more information.](https://gleam.run/news/v0.21-introducing-the-gleam-language-server/)

```sh
gleam lsp
```

## Glimmer
https://github.com/ember-tooling/ember-language-server

Does not currently work with embroider-vite and .gjs/.gts files, only .hbs

```sh
install -g @ember-tooling/ember-language-server
```

```toml
[[language]]
name = "glimmer"
language-servers = ["ember-language-server"]
formatter = { command = "prettier", args = ["--parser", "glimmer"] }
```


## GLSL

The default is [`glsl_analyzer`](https://github.com/nolanderc/glsl_analyzer)

## Go

The folder for go packages (typically $HOME/go/bin) will need to be added to
your PATH for any of the below to work.

### Install tools

```
go install golang.org/x/tools/gopls@latest                               # LSP
go install github.com/go-delve/delve/cmd/dlv@latest                      # Debugger
go install golang.org/x/tools/cmd/goimports@latest                       # Formatter
go install github.com/nametake/golangci-lint-langserver@latest           # Linter
go install github.com/golangci/golangci-lint/v2/cmd/golangci-lint@latest # Linter cli
```

### Autoformatting

The LSP formatter (`gofmt`) does not fix imports, `goimports` should be used
instead.

`languages.toml`

```toml
[[language]]
name = "go"
auto-format = true
formatter = { command = "goimports" }
```

## GraphQL

https://github.com/graphql/graphiql/blob/main/packages/graphql-language-service-cli/

```sh
npm i -g graphql-language-service-cli
```

## Haskell

Haskell Language Server.

https://github.com/haskell/haskell-language-server

## Helm

The Helm-ls installation guide can be found under the "Getting Started" section
in the readme.

https://github.com/mrjosh/helm-ls#getting-started

## HTML

### vscode-html-language-server

https://github.com/hrsh7th/vscode-langservers-extracted

`vscode-html-language-server` can be installed via `npm`:

```sh
npm i -g vscode-langservers-extracted
```

### superhtml

https://github.com/kristoff-it/superhtml

- Download from https://github.com/kristoff-it/superhtml/releases 
- Extract the superhtml executable
- chmod +x superhtml
- add it to your $PATH

```toml
[[language]]
name = "html"
language-servers = [{ name = "superhtml", except-features = ["format"] }, "vscode-html-language-server"]
auto-format = true
```

## Java

https://github.com/eclipse/eclipse.jdt.ls

Installation instructions can be found on the
[projects README](https://github.com/eclipse/eclipse.jdt.ls).

On macOS installation can also be done via `brew install jdtls`.

On debian based distros try a fantastic install script made by
[eruizc-dev](https://github.com/eruizc-dev):
[jdtls-launcher](https://github.com/eruizc-dev/jdtls-launcher).

For the Arch Linux: `yay -Sy jdtls`
[AUR jdtls source](https://aur.archlinux.org/packages/jdtls).

After installing, test to see if the `jdtls` works out of the box (it should
work for the debian script). For versions older than `1.16.0`: the `-data`
parameter must be passed to `jdtls` and it must be different for each project.
This can be achieved by adding a `.helix/languages.toml` configuration to the
project root:

```toml
[language-server]
jdtls = { command = "jdtls" }
language-server = { command = "jdtls", args = [
  "-data", "/home/<USER>/.cache/jdtls/workspace"
  ]}

[[language]]
name = "java"
scope = "source.java"
injection-regex = "java"
file-types = ["java"]
roots = ["pom.xml", "build.gradle", ]
indent = { tab-width = 4, unit = "    " }
language-servers = [ "jdtls" ]
```

Note: the `-data` parameter must be up one directory from the project directory.

### Inlay Hints
The following will enable inlay hints for the jdtls language server when added to ``.helix/languages.toml``

```toml
[language-server.jdtls.config.java.inlayHints]
parameterNames.enabled = "all"
```

## JavaScript

See
[tsserver](https://github.com/helix-editor/helix/wiki/Language-Server-Configurations#typescript).

## JSON

https://github.com/hrsh7th/vscode-langservers-extracted

vscode-json-language-server, a language server for JSON and JSON schema

`vscode-json-language-server` can be installed via `npm`:

```sh
npm i -g vscode-langservers-extracted
```

Available settings can be found here:
https://github.com/microsoft/vscode/blob/4f69cdf95a12cef48d405b38bf7812a7f297c310/extensions/json-language-features/server/src/jsonServer.ts#L183

Usage

```toml
config = { "provideFormatter" = true, "json" = { "keepLines" = { "enable" = true } } }
```

## Jsonnet

https://github.com/grafana/jsonnet-language-server

A [Language Server Protocol (LSP)](https://langserver.org/) server for
[Jsonnet](https://jsonnet.org/).

Can be installed either via the
[latest release binary](https://github.com/grafana/jsonnet-language-server/releases)
or if you have golang installed, you can use:

```sh
go install github.com/grafana/jsonnet-language-server@latest
```

## Julia

https://github.com/julia-vscode/LanguageServer.jl

LanguageServer.jl can be installed with `julia` and `Pkg`:

```sh
julia -e 'using Pkg; Pkg.add("LanguageServer")'
```

To update an existing install, use the following command:

```sh
julia -e 'using Pkg; Pkg.update()'
```

## Kotlin

A Kotlin language server which was developed for internal usage and released
afterward. Maintaining is not done by the original author, but by fwcd.

It is built via gradle and developed on GitHub. Source and additional
description: https://github.com/fwcd/kotlin-language-server

## LaTex

[TexLab](https://github.com/latex-lsp/texlab): A cross-platform implementation
of the
[Language Server Protocol](https://microsoft.github.io/language-server-protocol)
providing rich cross-editing support for the
[LaTeX](https://www.latex-project.org/) typesetting system.

Add the following to your languages.toml to enable build on save:

```toml
[language-server.texlab.config.texlab.build]
onSave = true
```

TexLab can be further configured to jump to your current location in the pdf
following the build, among other useful things. For all available options, see
the [TexLab wiki](https://github.com/latex-lsp/texlab/wiki/Configuration).
Everything under the `texlab` key goes under
`language-server.texlab.config.texlab` (or possibly `language.config.texlab` on
older releases). For instance, setting the `texlab.build.onSave` property to
`true` (as per the
[TexLab wiki](https://github.com/latex-lsp/texlab/wiki/Configuration#texlabbuildonsave))
is achieved with the above `languages.toml`.

## Lean 3

https://github.com/leanprover/lean-client-js/tree/master/lean-language-server

Lean installation instructions can be found
[here](https://leanprover-community.github.io/get_started.html#regular-install).

Once Lean is installed, you can install the Lean 3 language server by running

```sh
npm install -g lean-language-server
```

## Lua

Binaries are available from:
https://github.com/LuaLS/lua-language-server/releases

`mac`

```sh
brew install lua-language-server
```

## Markdoc

[@markdoc/language-server](https://www.npmjs.com/package/@markdoc/language-server) -
an experimental language server for markdoc. `markdoc-ls` should be available
after installation.

Install using

```sh
npm install -g @markdoc/language-server
```

## Markdown

### Marksman

The primary default language server is [Marksman](https://github.com/artempyanykh/marksman)

Binaries are available from [here](https://github.com/artempyanykh/marksman/releases)

macOS and Linux:
```sh
brew install marksman
```
or
```sh
yay -S marksman-bin
```

Windows:
```pwsh
scoop install marksman
```

### ltex-ls

As an alternative you can use `ltex-ls` which provides grammar and spelling
errors in markup documents: https://valentjn.github.io/ltex/

```toml
[[language]]
name = "markdown"
language-servers = [ "marksman", "ltex-ls" ]
```

Additional configuration settings can be added, for example to disable the
profanity rules and add the word `builtin` to two dictionaries:

```toml
[language-server.ltex-ls.config]
ltex.disabledRules = { "en-US" = ["PROFANITY"], "en-GB" = ["PROFANITY"] }
ltex.dictionary = { "en-US" = ["builtin"], "en-GB" = ["builtin"] }
```

Currently,
[the ability to add to your user dictionary while running Helix is not supported](https://github.com/valentjn/ltex-ls/issues/231),
so adding words to the config is the best workaround.

> [!important]
> `ltex-ls` has not been updated in a while, but there is a new fork with bug fixes and new features to try here: https://github.com/ltex-plus/ltex-ls-plus. If your build of Helix is before 16th Dec 24 you need to add:
> 
> ```toml
> [language-server]
> ltex-ls-plus = { command = "ltex-ls-plus" }

### `markdown-oxide`

An alternative to `marksman` that provides support for advanced markdown PKM
systems in your favorite text editor. It features complete compatibility with
Obsidian.md markdown syntax and bases its features on the features of the
Obsidian.md editor. For a list of all features, check out the
[README](https://github.com/Feel-ix-343/markdown-oxide?tab=readme-ov-file#features)

- It can be installed for Arch from the AUR under the name `markdown-oxide-git`

```sh
paru -S markdown-oxide-git
```

- Or it can be installed by `cargo`

```sh
cargo install --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
```

- Or manually by following
  [these directions](https://github.com/Feel-ix-343/markdown-oxide?tab=readme-ov-file#manual-for-macos-windows-and-other-linux-distributions)

## MATLAB

[matlab-language-server](https://github.com/mathworks/MATLAB-language-server)

Note: as per the README, "MATLAB language server requires MATLAB version R2021a
or later."

```toml
[language-server.matlab-ls]
command = "matlab-language-server"
args = ["--stdio"]

[language-server.matlab-ls.config.MATLAB]
indexWorkspace = false
installPath = "/PATH/TO/MATLAB/INSTALLATION"
matlabConnectionTiming = "onStart"
telemetry = false

[[language]]
name = "matlab"
scope = "source.m"
file-types = ["m"]
language-servers = ["matlab-ls"]
comment-token = "%"
shebangs = ["octave-cli", "matlab"]
indent = { tab-width = 2, unit = "  " }
```

## Mint

https://www.mint-lang.com

Install Mint using the [instructions](https://www.mint-lang.com/install). The
language server is included since version 0.12.0.

## Mojo

https://www.modular.com/mojo

- Install the Modular CLI
- Install the Mojo SDK

See this PR: https://github.com/helix-editor/helix/pull/8583

## Nim

https://github.com/nim-lang/langserver

```sh
# May require choosenim
nimble install nimlangserver
```

## Nix

The default language server is `nil` since the 2022-12 release.

https://github.com/oxalica/nil

This program is available in [NixOS/nixpkgs](https://github.com/NixOS/nixpkgs)
under attribute `nil`, and is regularly updated.

- If you use `nix-env`, run `nix-env -iA nixpkgs.nil`
- If you use `nix profile`, run `nix profile install nixpkgs#nil`
- Check out the GitHub repository for additional options

The formatter `nixpkgs-fmt` is not included and can be installed with
`nix-env -iA nixpkgs.nixpkgs-fmt`

To set up the formatter, set the following in your `languages.toml` :

```toml
[[language]]
name = "nix"
formatter = { command = "nixpkgs-fmt" }
```

To use the previous default language server, check out
https://github.com/nix-community/rnix-lsp

## OCaml

https://github.com/ocaml/ocaml-lsp

The OCaml language server `ocamllsp` can be installed via OPAM:

```sh
opam install ocaml-lsp-server
```

## Odin

[ols](https://github.com/DanielGavin/ols) - https://github.com/DanielGavin/ols

Provides syntax highlighting, auto-complete, code formatting and more for Odin.

## OpenPolicyAgent

An implementation of the language server protocol for OpenPolicyAgent's rego.

You can download it from its
[releases page](https://github.com/kitagry/regols/releases), or

```sh
$ go install github.com/kitagry/regols@latest
```

## Perl

https://github.com/bscan/PerlNavigator

Provides syntax checking, autocompletion, perlcritic, code navigation, hover for
Perl.

Implemented as a Language Server using the Microsoft LSP libraries along with
Perl doing the syntax checking and parsing.

Perl Navigator can be installed by downloading the latest release for your
platform at
[the project's releases page](https://github.com/bscan/PerlNavigator/releases)
and putting the perlnavigator executable somewhere in your PATH.

## Pest

https://github.com/pest-parser/pest-ide-tools

This repository contains an implementation of the Language Server Protocol in
Rust, for the Pest parser generator.

## PHP

### Phpactor

https://phpactor.readthedocs.io/en/master/index.html

Phpactor requires PHP 8.1.

You can download `phpactor.phar` as follows:

```sh
curl -Lo phpactor.phar https://github.com/phpactor/phpactor/releases/latest/download/phpactor.phar
```

Then make it executable and symlink it somewhere in your
[PATH](https://en.wikipedia.org/wiki/PATH_(variable)):

```sh
chmod a+x phpactor.phar
mv phpactor.phar ~/.local/bin/phpactor
```

Check support using the status command:

```sh
phpactor status
✔ Composer detected - faster class location and more features!
✔ Git detected - enables faster refactorings in your repository scope!
```

Then, to enable this LSP you have to create a file `languages.toml` in your
project directory `.helix/languages.toml` and place the following code inside
(or if you want you could do this to helix languages.toml file globally):

```toml
[language-server.phpactor]
command = "phpactor"
args = [ "language-server" ]
[[language]]
name = "php"
language-servers = [ "phpactor" ]
```

### Intelephense

> [!WARNING]
> Intelephense is proprietary, so be sure to review its licensing terms.

https://intelephense.com

`intelephense` can be installed via `npm`:

```sh
npm install -g intelephense
```

#### Premium version

To enable the premium features you have to provide a license key for which you
have a few options:

1. Adding the license key directly to your languages.toml file:

```toml
[language-server.intelephense.config]
licenceKey = "MY_LICENSE_KEY"
```

2. Adding the path to your license file

```toml
[language-server.intelephense.config]
licenceKey = "/home/username/.config/intelephense/license.txt"
```

3. Adding the license file to your home directory under
   `$HOME/intelephense/licence.txt`

**Note**: Keep in mind how the word is written licen**c**eKey and not
licen**s**eKey, also for step 3) it has to be licen**c**e.txt.

## PKGBUILD

https://github.com/termux/termux-language-server

### Archlinux

```bash
yay -S termux-language-server
```

## PowerShell

A Language Server for Powershell is not present by default, but can be added with the following configuration.

Download the latest
[PowerShellEditorServices](https://github.com/PowerShell/PowerShellEditorServices/releases)
zip and extract it. In this example, I extracted it to C:\\.
You must then run `Get-ChildItem <PowerShellEditorServices-Path> -Recurse | Unblock-File` (in my case `Get-ChildItem C:\PowerShellEditorServices\ -Recurse | Unblock-File`) to remove the 'Mark of the Web' and allow PowerShellEditorServices to be executed.

After PowerShellEditorServices is present, you can add this config to your languages.toml:

```toml
[[language]]
name = 'powershell'
scope = 'source.ps1'
file-types = ['ps1', 'psm1']
roots = ['.git']
comment-token = '#'
indent = { tab-width = 4, unit = '    ' }
language-servers = [ 'powershell-editor-services' ]

[language-server.powershell-editor-services]
name = 'powershell-editor-services'
transport = 'stdio'
command = 'pwsh'
args = ['-NoLogo', '-NoProfile', '-Command', 'C:\\PowerShellEditorServices\\PowerShellEditorServices\\Start-EditorServices.ps1 -SessionDetailsPath C:\\PowerShellEditorServices\\PowerShellEditorServices.sessions.lsp.json -LanguageServiceOnly -Stdio']
```

In combination with this LSP Config, you might want to use the DAP Config I wrote here: [Debugger Configurations](https://github.com/helix-editor/helix/wiki/Debugger-Configurations#powershell-powershelleditorservices)


## Prisma

https://github.com/prisma/language-tools/tree/main/packages/language-server

`prisma-language-server` can be installed via npm:

```sh
npm install -g @prisma/language-server
```

## Prolog

An implementation of the language server protocol for SWI-Prolog

https://github.com/jamesnvc/lsp_server

Install the `swi-prolog` package and run `swipl`:

```
?- pack_install(lsp_server).
```

## Python

### pylsp

[python-lsp/python-lsp-server](https://github.com/python-lsp/python-lsp-server)
(`pylsp`) is a fork of the python-language-server project (`pyls`), maintained
by the Spyder IDE team and the community. It is a Python 3.7+ implementation of
the
[Language Server Protocol](https://github.com/Microsoft/language-server-protocol)
(versions <1.4 should still work with Python 3.6).

Installation instructions can be found in the
[project's README](https://github.com/python-lsp/python-lsp-server#installation),
but it consists of installing a package using `pip` (or
[`pipx`](https://github.com/pypa/pipx)):

```console
pip install -U 'python-lsp-server[all]'
```

The `[all]` above refers to the optional providers supported. You can fine-tune
what to install following the instructions
[here](https://github.com/python-lsp/python-lsp-server#installation).

### pylsp-mypy

[python-lsp/pylsp-mypy](https://github.com/python-lsp/pylsp-mypy) (`pylsp-mypy`)
is a Mypy (type checker) plugin for Pylsp. First do the steps in Pylsp section
and then install pylsp-mypy:

```console
pip install pylsp-mypy
```

`languages.toml`

```toml
[[language]]
name = "python"
language-servers = ["pylsp"]

[language-server.pylsp.config.pylsp]
plugins.pylsp_mypy.enabled = true
plugins.pylsp_mypy.live_mode = true
```

### pyrefly

Pyrefly is a fast type checker for Python. It's designed to replace the existing Pyre type checker at Meta. Built in Rust and built for speed, Pyrefly aims to increase development velocity with IDE features and by checking your Python code.

https://github.com/facebook/pyrefly

The language server can be installed by running
`pip install pyrefly`

`languages.toml`

```toml
[language-server.pyrefly]
command = "pyrefly"
args = [ "lsp" ]

[[language]]
name = "python"
language-servers = [ "pyrefly" ]
```

### pyright

Pyright is a fast type checker and language server from Microsoft, meant for
large Python source bases. It is the LSP part of pylance (the VS Code python
daemon).

https://github.com/microsoft/pyright

The language server can be installed by running
`npm install --location=global pyright`

`languages.toml`

```toml
[[language]]
name = "python"
language-servers = [ "pyright" ]
```

### basedpyright

Basedpyright is a fork of pyright with various type checking improvements,
improved vscode support and pylance features built into the language server.

https://docs.basedpyright.com

The language server can be installed by running `pip install basedpyright`

`languages.toml`

```toml
[[language]]
name = "python"
language-servers = [ "basedpyright" ]
```

### ruff

[charliermarsh/ruff](https://github.com/charliermarsh/ruff) is an extremely fast
Python linter, written in Rust (see
[installation instructions](https://docs.astral.sh/ruff/installation/)).

Ruff ships with a builtin LSP, see
[ruff docs for integration with helix](https://docs.astral.sh/ruff/editors/setup/#helix).

A suggested Helix configuration using ruff as lsp is given below:

```toml
[[language]]
name = "python"
language-servers = [ "ruff" ]
auto-format = true
[language-server.ruff]
command = "ruff"
args = ["server"]
```

**Note** that ruff lacks basic features and is meant to be used alongside
another LSP
([helix-editor/helix#5399 (comment)](https://github.com/helix-editor/helix/issues/5399#issuecomment-1373470899),
[charliermarsh/ruff-lsp#23](https://github.com/charliermarsh/ruff-lsp/issues/23),
[charliermarsh/ruff-lsp#23 (comment)](https://github.com/charliermarsh/ruff-lsp/issues/23#issuecomment-1367903296)).

As an alternative, [pylsp](#pylsp) has support for ruff via a plugin.
[See instructions for Helix here](#python-lsp-ruff)

### python-lsp-ruff

Refer to [Ruff's documentation](https://docs.astral.sh/ruff/configuration/) and the [python-lsp-ruff docs](https://github.com/python-lsp/python-lsp-ruff)

```toml
[language-server.pylsp.config.pylsp.plugins.ruff]
lineLength = 88
preview = true
select = ["E4", "E7", "E9", "F"] # Rules to be enabled by ruff
ignore = ["D210"] # Rules to be ignored by ruff
```

### pyright + ruff

[pyright](https://github.com/microsoft/pyright) - `npm install pyright -g`

[ruff-lsp](https://github.com/astral-sh/ruff-lsp) - `pip install ruff-lsp` or
[ruff](https://github.com/astral-sh/ruff) - `pip install ruff` (ruff-lsp is A
[Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
implementation for ruff, you can choose one)

```toml
[[language]]
name = "python"
language-servers = [ "pyright", "ruff" ]

[language-server.pyright.config.python.analysis]
typeCheckingMode = "basic"

# if you choose `ruff-lsp`
[language-server.ruff]
command = "ruff-lsp"
[language-server.ruff.config.settings]
args = ["--ignore", "E501"]
# if you choose `ruff` itself
[language-server.ruff]
command = "ruff"
args = ["server"]
```

### ruff + pyright + pylyzer

- [ruff](https://github.com/astral-sh/ruff)

- [pyright](https://github.com/microsoft/pyright)

- [pylyzer](https://github.com/mtshiba/pylyzer)

```toml
# in languages.toml
[[language]]
name = "python"
language-servers = ["pyright", "ruff", "pylyzer"]
[language-server.pyright.config.python.analysis]
typeCheckingMode = "basic"
[language-server.ruff]
command = "ruff"
args = ["server"]
[language-server.pylyzer]
command = "pylyzer"
args = ["--server"]
```

## R

An implementation of the Language Server Protocol for R.

https://github.com/REditorSupport/languageserver

The language server can be installed by running
`R -e 'install.packages("languageserver")'`.

## Racket

[https://github.com/jeapostrophe/racket-langserver](https://github.com/jeapostrophe/racket-langserver)

The Racket language server. This project seeks to use
[DrRacket](https://github.com/racket/drracket)'s public API to provide
functionality that mimics DrRacket's code tools as closely as possible.

Install via `raco`: `raco pkg install racket-langserver`

## ReScript

https://github.com/rescript-lang/rescript-vscode

// ReScript language server.

## Rust

[rust-analyzer](https://github.com/rust-analyzer/rust-analyzer), a language
server for Rust.

You can install rust-analyzer using `rustup` starting from Rust 1.64, and it
will be added to your system's `$PATH`
[starting from Rustup 1.26.0](https://blog.rust-lang.org/2023/04/25/Rustup-1.26.0.html#whats-new-in-rustup-1260):

```sh
rustup component add rust-analyzer
```

Add the following to your `languages.toml` to enable
[clippy](https://github.com/rust-lang/rust-clippy) on save:

```toml
[language-server.rust-analyzer.config.check]
command = "clippy"
```

You may also wish to enable all features, as it will allow you to use rust-analyzer in `integration-test` for example.

```toml
[language-server.rust-analyzer.config.cargo]
features = "all"
``` 

See [docs](https://rust-analyzer.github.io/manual.html) for extra settings.
Everything under the rust-analyzer key goes under
`language-server.rust-analyzer.config` key in helix (for example,
`rust-analyzer.check.command = "clippy"` is translated into the `language.toml`
as above.)

## Scala

Scala language server with rich IDE features.

https://scalameta.org/metals/

1. Install [Coursier](https://get-coursier.io/)
2. Run `coursier install metals`

## Scheme

### Steel Dialect

Steel's language server is available at
[github.com/mattwparas/crates/steel-language-server](https://github.com/mattwparas/steel/tree/master/crates/steel-language-server).

Recommended installation for LSP + interpreter
([link](https://github.com/mattwparas/steel?tab=readme-ov-file#full-install)):

```sh
git clone git@github.com:mattwparas/steel.git
cd steel
cargo xtask install
```

Then follow the
[configuration](https://github.com/mattwparas/steel/tree/master/crates/steel-language-server#configuration)
instructions.

Finally, add the following to your language configuration file:

```toml
[[language]]
name = "scheme"
language-servers = ["steel-language-server"]

[language-server.steel-language-server]
command = "steel-language-server"
args = []
```

## SCSS

SCSS's language server is available from the vscode-langservers-extracted
collection:

https://github.com/hrsh7th/vscode-langservers-extracted

You may install it by running:

```sh
npm i -g vscode-langservers-extracted
```

## Slint

<https://github.com/slint-ui/slint/tree/HEAD/tools/lsp>\
<https://slint-ui.com/>

```sh
cargo install slint-lsp
```

## Smithy

For Smithy projects the following LSP is used:
https://github.com/awslabs/smithy-language-server

[coursier](https://get-coursier.io/) must be installed so that the language
server can be launched. To install coursier please see their
[installation instructions](https://get-coursier.io/docs/cli-installation#native-launcher).
Since coursier will take care of everything else, no other steps are necessary!

## Ruby

### Solargraph

https://solargraph.org/

Solargraph, a language server for Ruby

You can install Solargraph via gem install.

```sh
gem install --user-install solargraph
```
or 

### Ruby-lsp

https://github.com/Shopify/ruby-lsp

You can install ruby-lsp via gem install too.

```sh
gem install --user-install ruby-lsp
```

## solc

solc is the native language server for the Solidity language.

https://docs.soliditylang.org/en/latest/installing-solidity.html

## Svelte

https://github.com/sveltejs/language-tools/tree/master/packages/language-server

`svelte-language-server` can be installed via `npm`:

```sh
npm i -g svelte-language-server
```

For integration with `.js` and `.ts` files install `typescript-svelte-plugin`
via `npm`:

```sh
npm i -g typescript-svelte-plugin
```

Then for each svelte project update your `tsconfig.json`/`jsconfig.json` to add
the `typescript-svelte-plugin`

```json
{
    "compilerOptions": {
        ...
        "plugins": [{
            "name": "typescript-svelte-plugin"
        }]
    }
}
```

Further information on `js` and `ts` integration for svelte can be found
[here](https://github.com/sveltejs/language-tools/tree/master/packages/typescript-plugin).

## Swift

A language server for Swift, formatting provided via swift-format

https://github.com/apple/sourcekit-lsp

https://github.com/apple/swift-format

Follow the
[Getting Started](https://github.com/apple/sourcekit-lsp#getting-started) guide
to get sourcekit-lsp installed correctly for your OS. No additional
configuration is needed, though note to use the same toolchain for both your
installed LSP, and that you use to build.

## Sql

https://github.com/joe-re/sql-language-server

```sh
npm i -g sql-language-server
```

`languages.toml` setting

```toml
[language-server.sql-language-server]
command = "sql-language-server"
args = ["up", "--method", "stdio"]

[[language]]
name = "sql"
language-servers = [ "sql-language-server" ]
```

- Note: There is also https://github.com/sqls-server/sqls written in Go

## TailwindCSS

https://github.com/tailwindlabs/tailwindcss-intellisense

`tailwindcss-language-server` can be installed via `npm`:

```sh
npm i -g @tailwindcss/language-server
```

Add this to your local `languages.toml` file to enable it

`languages.toml`:

```toml
[language-server.tailwindcss-ls]
command = "tailwindcss-language-server"
args = ["--stdio"]

[[language]]
name = "html"
language-servers = [ "vscode-html-language-server", "tailwindcss-ls" ]

[[language]]
name = "css"
language-servers = [ "vscode-css-language-server", "tailwindcss-ls" ]

[[language]]
name = "jsx"
language-servers = [ "typescript-language-server", "tailwindcss-ls" ]

[[language]]
name = "tsx"
language-servers = [ "typescript-language-server", "tailwindcss-ls" ]

[[language]]
name = "svelte"
language-servers = [ "svelteserver", "tailwindcss-ls" ]
```

If you want to use TailwindCSS language server with other languages, configure
like example below. The example is with rust.

```toml
[[language]]
name = "rust"
language-servers = ["rust-analyzer", "tailwindcss-ls"]

[language-server.tailwindcss-ls]
config = { userLanguages = { rust = "html", "*.rs" = "html" } }
```

## Terraform

You'll need `terraform-ls` installed; the instructions are
[here](https://github.com/hashicorp/terraform-ls/blob/main/docs/installation.md).

Add this to your local languages.toml file to enable it

`languages.toml`:

```toml
[[language]]
name = "hcl"
language-servers = [ "terraform-ls" ]
language-id = "terraform"

[[language]]
name = "tfvars"
language-servers = [ "terraform-ls" ]
language-id = "terraform-vars"

[language-server.terraform-ls]
command = "terraform-ls"
args = ["serve"]
# config.indexing.ignorePaths = ["ignore.hcl"]
```

## TOML

https://taplo.tamasfe.dev/

To configure as your main formatter, [read this](https://github.com/helix-editor/helix/wiki/Formatter-Configurations#taplo).

The NPM versions and default builds of taplo does not contain the language server at this time. The
`full` version (with the language server) can be installed with:

##### Binary Releases

[taplo releases](https://github.com/tamasfe/taplo/releases)

##### Cargo

```sh
cargo install taplo-cli --locked --features lsp
```

##### MacOS

```sh
brew install taplo
```

##### Docker

```sh
docker run tamasfe/taplo --help
```

### etc

`languages.toml`:
```toml
[[language]]
name = "toml"
# https://github.com/tamasfe/taplo/issues/580#issuecomment-2004174721
roots = ["."]
language-servers = ["taplo"]
```

> [!NOTE]
> To avoid potential issues with `roots`, that config doesn't include `tombi`.
> Since [tombi is a default LSP](https://github.com/helix-editor/helix/blob/ab97585b69f11b159a447c85dfd528cc241cf1e3/languages.toml#L349), that will disable it.

Run `taplo lsp --help` for more info.

## TypeScript

### typescript-language-server

https://github.com/typescript-language-server/typescript-language-server

`typescript-language-server` depends on `typescript`. Both packages can be
installed via `npm`:

```sh
npm install -g typescript typescript-language-server
```

To configure type language server, add a
[`tsconfig.json`](https://www.typescriptlang.org/docs/handbook/tsconfig-json.html)
or [`jsconfig.json`](https://code.visualstudio.com/docs/languages/jsconfig) to
the root of your project.

Here's an example that disables type checking in JavaScript files.

```json
{
  "compilerOptions": {
    "module": "commonjs",
    "target": "es6",
    "checkJs": false
  },
  "exclude": ["node_modules"]
}
```

### Biome

Biome is a fast and efficient toolchain for web development that formats and
lints code, supporting JavaScript, TypeScript, JSX, and JSON, with over 190
rules and high compatibility with existing tools like Prettier.

```sh
npm install --save-dev --save-exact @biomejs/biome
```

Follow the official instructions on
[how to configure Biome for Helix](https://biomejs.dev/guides/editors/third-party-extensions/#helix).

> [!important]
The Biome CLI package doesn't ship with prebuilt binaries for Android yet. https://github.com/biomejs/biome/issues/1340

### ESLint

> [!important]
> Version `@4.10` is broken for `hx`. You can:
>
> - [downgrade to `@4.8`](https://github.com/hrsh7th/vscode-langservers-extracted/commit/859ca87fd778a862ee2c9f4c03017775208d033a#commitcomment-142613101)
> - Await [this feature](https://github.com/helix-editor/helix/pull/11315)

1. Install the LSP:

```sh
# Omitting version will default to major `@latest`!
npm i -g vscode-langservers-extracted@4.8
# `npm update -g` will install the latest non-major version,
# only do that after the issue has been fixed!
# Unfortunately, there's no proper way to pin/lock
# global packs
```

2. Add this to your `languages.toml`:

```toml
[[language]]
name = "javascript"
language-servers = [
  "typescript-language-server", # optional
  "vscode-eslint-language-server",
]
[[language]]
name = "jsx"
language-servers = [
  "typescript-language-server",
  "vscode-eslint-language-server",
]

[[language]]
name = "typescript"
language-servers = [
  "typescript-language-server",
  "vscode-eslint-language-server",
]
[[language]]
name = "tsx"
language-servers = [
  "typescript-language-server",
  "vscode-eslint-language-server",
]
```

3. [Install ESL](https://eslint.org/docs/latest/use/getting-started)

## TypeSpec

<https://github.com/microsoft/typespec>

The language server is installed along with the compiler using `npm`:

```sh
npm install -g @typespec/compiler
```

## Typst

<https://github.com/uben0/tree-sitter-typst/>

```sh
cargo install --git https://github.com/nvarner/typst-lsp typst-lsp
```

## Uiua

- language: https://www.uiua.org
- lsp: https://github.com/uiua-lang/uiua?tab=readme-ov-file#language-server
- tree-sitter: https://github.com/shnarazk/tree-sitter-uiua

```toml
[language-server.uiua-lsp]
command = "uiua"
args = ["lsp"]

[[language]]
name = "uiua"
scope = "source.uiua"
injection-regex = "uiua"
file-types = ["ua"]
roots = []
auto-format = true
comment-token = "#"
language-servers = [ "uiua-lsp" ]
indent = { tab-width = 2, unit = "  " }
shebangs = ["uiua"]
auto-pairs = {'(' = ')', '{' = '}', '[' = ']', '"' = '"'}
```

## Unison

Unison language server.

More info:
https://github.com/unisonweb/unison/blob/trunk/docs/language-server.markdown

Requirements:

- `ucm` started
- `ncat`, `nc` or `netcat`

To `~/.config/helix/languages.toml` append this code:

```toml
[language-server.ucm]
command = "ncat"
args = ["localhost", "5757"]

[[language]]
name = "unison"
language-servers = [ "ucm" ]
```

## V

https://github.com/vlang/v-analyzer

Clone, install and build:

```sh
git clone --filter=blob:none --recursive --shallow-submodules https://github.com/vlang/v-analyzer
cd v-analyzer
v build.vsh release
```

or use the installer, and follow the instructions that it will print:
`v -e "$(curl -fsSL https://raw.githubusercontent.com/vlang/v-analyzer/main/install.vsh)"`

config path:

set the v-analyzer to environment variable:

```shell
PATH=your/path/v-analyzer/bin:$PATH
```

## Vue

https://github.com/vuejs/language-tools/tree/master/packages/language-server

The Vue language server `vue-language-server` can be installed via `npm`:

```sh
npm i -g @vue/language-server
```

If you're using typescript, you'll also need the vue typescript plugin:

```sh
npm i -g @vue/typescript-plugin
```

To autoformat your `.vue` files upon save, you can first install `prettier` via
`npm`:

```sh
npm i -g prettier
```

And then add this to your `languages.toml` file in your Helix configuration
directory:

```toml
[[language]]
name = "vue"
auto-format = true
formatter = { command = "prettier", args = ["--parser", "vue"] }
language-servers = ["typescript-language-server"]

[[language-server.typescript-language-server.config.plugins]]
name = "@vue/typescript-plugin"
location = "/full/path/to/node_modules/@vue/typescript-plugin"
languages = ["vue"]
```

## WGSL

https://github.com/wgsl-analyzer/wgsl-analyzer

`wgsl-analyzer` can be installed via `cargo`:

```sh
cargo install --git https://github.com/wgsl-analyzer/wgsl-analyzer wgsl-analyzer
```

## Wikitext

https://github.com/bhsd-harry/vscode-extension-wikiparser

`wikitext-lsp` can be installed via `npm`:

```sh
npm i -g wikitext-lsp
```


## XML

https://github.com/redhat-developer/vscode-xml

wget
https://github.com/redhat-developer/vscode-xml/releases/download/0.27.1/lemminx-linux.zip

1. Download lemminx-linux.zip
2. Extract and copy lemminx-linux binary to location in $PATH
3. Install xmllint via package manger yay, dnf, pacman apt-get etc (rpm: dnf
   install libxml2)

Configuration:

```
[[language]]
name = "xml"
file-types = [ "xml", "svg", "xsd", "xslt", "xsl" ]
auto-format = true
formatter = { command = "xmllint", args = ["--format", "-"] }
language-servers = [ "xml" ]

[language-server.xml]
command = "lemminx-linux"
```

## YAML

https://github.com/redhat-developer/yaml-language-server

`yaml-language-server` can be installed via `brew` on Mac:

```sh
brew install yaml-language-server
```

or via `npm`:

```sh
npm i -g yaml-language-server@next
```

Example configuration using json schemas.

```toml
[language-server.yaml-language-server.config.yaml]
format = { enable = true }
validation = true

[language-server.yaml-language-server.config.yaml.schemas]
"https://json.schemastore.org/github-workflow.json" = ".github/workflows/*.{yml,yaml}"
"https://raw.githubusercontent.com/ansible-community/schemas/main/f/ansible-tasks.json" = "roles/{tasks,handlers}/*.{yml,yaml}"
```

## Zig

Zig LSP implementation + Zig Language Server.

https://github.com/zigtools/zls

# Other Language Server Protocol Types

Non language-specific Language Server Protocols, non-coding LSPs, and others.

## Harper-ls

[Harper](https://github.com/elijah-potter/harper/) is an English grammar and
spelling checker for comments and text.\
Installing:

```sh
cargo install harper-ls --locked
```

Harper will need to be manually added to each language you'd like check's
language server list.\
Usage:

```TOML
[language-server.harper-ls]
command = "harper-ls"
args = ["--stdio"]
```

Disabling specific
[rules](https://writewithharper.com/docs/rules).

```TOML
[language-server.harper-ls.config.harper-ls.linters]
spaces = false
```

Change the default `diagnosticSeverity` from hint to warning:

```TOML
[language-server.harper-ls.config.harper-ls]
diagnosticSeverity = "warning"
```

## Vale-ls

Vale is a markup-aware linter for prose built with speed and extensibility in
mind.

https://github.com/errata-ai/vale

| Tool | Extensible     | Checks          | Supports Markup                                            | Built With | License |
| ---- | -------------- | --------------- | ---------------------------------------------------------- | ---------- | ------- |
| Vale | Yes (via YAML) | spelling, style | Yes (Markdown, AsciiDoc, reStructuredText, HTML, XML, Org) | Go         | MIT     |

The Vale Language Server is an implementation of the Language Server Protocol
for Vale.

https://github.com/errata-ai/vale-ls

Vale supports
[linting source code comments in a number of languages](https://vale.sh/docs/topics/scoping/#code-1)

You must install both `vale` and `vale-ls` into your `$PATH` and add it to each
language you wish to lint, for example:

Since https://github.com/helix-editor/helix/pull/11636/ you only need to add:

```toml
[[language]]
name = "html"
language-servers = [ "vscode-html-language-server", "vale-ls" ]
```

Before that PR:

`languages.toml`

```toml
[language-server.vale-ls]
command = "vale-ls"

[[language]]
name = "html"
language-servers = [ "vscode-html-language-server", "vale-ls" ]
```

