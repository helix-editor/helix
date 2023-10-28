# Helix, `wasm32` port, alpha

This is `Helix`, compiling to `wasm32` and running in a browser.

Live demp: https://makemeunsee.github.io/helix/demo

## What works

* Compiling to `wasm32`, with limited features
* Integration with `xterm-js`
* Bundling Helix into a static web app
* Running in a browser

Building and tests on Linux or Windows should still work, but only building to
`x86_64-unknown-linux-gnu` was actually tested.

## What does not work

By necessity (would require significant design changes in Helix and/or rely on
tools only accessible natively so far), when building with target `wasm32`, all
functionalities related to the following points are disabled (or should be):

* language server support
* debugging
* shell commands execution and piping
* vcs features, e.g. `git` info and diff'ing
* cloning & compiling `tree-sitter` grammars
* most filesystem operations; reading/writing 'files' is crudely emulated using the Web Storage API

Most features of the editor are untested, so expect bugs; see 'Known issues'.

## How to...

### Setup

```sh
cargo install cargo-wasm

npm install

cd www/

npm install
```

### Build

```sh
wasm-pack build
```

### Run

```sh
cd www/

NODE_OPTIONS=--openssl-legacy-provider npm run start

# then visit https://localhost:8080
```

### Deploy

See `web_demo` GitHub workflow.

### Test

Would be done this way:

```sh
wasm-pack test --headless --firefox
```

## Known issues:

* `TODO(wasm32)` around the code indicate noted issues or limitations.
* Lots of warnings; clippy would have a lot to say too.
* Emojis cause glitches, maybe an encoding issue.
* UI behavior on mobile is broken; mobile usability is not a goal but this may
  indicate issues nonetheless.
* Some shortcuts are shadowed by the browser (e.g. Ctrl-w), other defaults could
  be used.
* All pickers (file, command, buffer) are broken, as they rely on `nucleo` and
  its threadpool.

## How does it work

* feature-gated everything which cannot readily be ported to `wasm`.
* adapted primitives to what's available on `wasm`; notably `async` code.
* re-used `crossterm` and `xterm-js` integration from [rrbutani](https://github.com/rrbutani),
  see [their fork of `crossterm`](https://github.com/rrbutani/crossterm/tree/xtermjs).
  I ported the required subset of their work to my own forks to bring
  compatibility with the most recent versions, including a very partial, adhoc
  `xterm-js` Rust bindings library.

## Ideas for down the road:

* restore `tree-sitter` grammars integration
* restore/replace diff'ing (probably without `git`)
* web workers for some operations

