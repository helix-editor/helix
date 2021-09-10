The Helix project still has a ways to go before reaching its goals.  This document outlines some of those goals and the overall vision for the project.

# Vision

An efficient, batteries-included editor you can take anywhere and be productive... if it's your kind of thing.

* **Cross-platform.**  Whether on Linux, Windows, or OSX, you should be able to take your editor with you.
* **Terminal first.**  Not all environments have a windowing system, and you shouldn't have to abandon your preferred editor in those cases.
* **Native.**  No Electron or HTML DOM here.  We want an efficient, native-compiled editor that can run with minimal resources when needed.  If you're working on a Raspberry Pi, your editor shouldn't consume half of your RAM.
* **Batteries included.**  Both the default configuration and bundled features should be enough to have a good editing experience and be productive.  You shouldn't need a massive custom config or external executables and plugins for basic features and functionality.
* **Don't try to be everything for everyone.**  There are many great editors out there to choose from.  Let's make Helix *one of* those great options, with its own take on things.

# Goals

Vision statements are all well and good, but are also vague and subjective.  Here is a (non-exhaustive) list of some of Helix's more concrete goals, to help give a clearer idea of the project's direction:

* **Modal.**  Vim is a great idea.
* **Selection -> Action**, not Verb -> Object.  Interaction models aren't linguistics, and "selection first" lets you see what you're doing (among other benefits).
* **We aren't playing code golf.**  It's more important for the keymap to be consistent and easy to memorize than it is to save a key stroke or two when editing.
* **Built-in tools** for working with code bases efficiently.  Most projects aren't a single file, and an editor should handle that as a first-class use case.  In Helix's case, this means (among other things) a fuzzy-search file navigator and LSP support.
* **Edit anything** that comes up when coding, within reason.  Whether it's a 200 MB XML file, a megabyte of minified javascript on a single line, or Japanese text encoded in ShiftJIS, you should be able to open it and edit it without problems.  (Note: this doesn't mean handle every esoteric use case.  Sometimes you do just need a specialized tool, and Helix isn't that.)
* **Configurable**, within reason.  Although the defaults should be good, not everyone will agree on what "good" is.  Within the bounds of Helix's core interaction models, it should be reasonably configurable so that it can be "good" for more people.  This means, for example, custom key maps among other things.
* **Extensible**, within reason.  Although we want Helix to be productive out-of-the-box, it's not practical or desirable to cram every useful feature and use case into the core editor.  The basics should be built-in, but you should be able to extend it with additional functionality as needed.  Right now we're thinking Wasm-based plugins.
* **Clean code base.**  Sometimes other factors (e.g. significant performance gains, important features, correctness, etc.) will trump strict readability, but we nevertheless want to keep the code base straightforward and easy to understand to the extent we can.
