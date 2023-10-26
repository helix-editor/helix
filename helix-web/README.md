# Helix, `wasm32` port, wip

This branch aims at providing a `wasm32` Helix eventually integrated with `xterm.js` in a web app.

This branch preserves native features (i.e. you should still be able to run successfully `cargo build` from the root).

When building with target `wasm32`, these features are disabled:

* language server support
* debugging
* shell commands execution and piping
* vcs features, e.g. `git` info and diff'ing
* cloning & compiling `tree-sitter` grammars
* anything related to the filesystem
* integration with an actual backend (`TestBackend` is used for now) and the event loop

Next steps:

* review and port other relevant efforts from [the original attempt](https://github.com/helix-editor/helix/tree/gui)
* design & implement a web bindable backend and event loop
* review & tackle remaining `TODO(wasm32)`s

Ideas for down the road:

* load/write configuration to web storage
* restore `tree-sitter` grammars integration
* restore/replace diff'ing (probably without `git`)
* web workers for some operations

## Building

```sh
wasm-pack build

# alternatively, without webpacking and integration into the node app:
cargo build --lib --no-default-features --target wasm32-unknown-unknown
```

## Init

```sh
cd www/

nvm use 16

npm install

# possibly
npm audit fix
```

## Running

```sh
cd www/

nvm use 16

npm run start
```

## Testing

(none so far)

```sh
wasm-pack test --headless --firefox
```
