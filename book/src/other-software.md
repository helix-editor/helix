# Helix mode in other software

Helix' keymap and interaction model ([Using Helix](#usage.md)) is easier to adopt if it can be used consistently in many editing contexts. Yet, certain use cases cannot easily be addressed directly in Helix. Similar to vim, this leads to the creation of "Helix mode" in various other software products, allowing Helix-style editing for a greater variety of use cases.

"Helix mode" is frequently still in early stages or missing entirely. For such cases, we also link to relevant bugs or discussions.

## Other editors

| Software | Plugin or feature providing Helix editing. | Comments
| --- | --- | --- |
| [IntelliJ IDEA](https://www.jetbrains.com/idea/) / [Android Studio](https://developer.android.com/studio)| [IdeaVim](https://plugins.jetbrains.com/plugin/164-ideavim) plugin + [helix.idea.vim](https://github.com/chtenb/helix.vim) config | Also see [this PR](https://github.com/chtenb/helix.vim/pull/4) for improved e/b/w movements.
| [Visual Studio Code](https://code.visualstudio.com/) | [Dance](https://marketplace.visualstudio.com/items?itemName=gregoire.dance) extension, or its [Helix fork](https://marketplace.visualstudio.com/items?itemName=kend.dancehelixkey) | The Helix fork has diverged. You can also use the original Dance and tweak its keybindings directly.
| [Visual Studio Code](https://code.visualstudio.com/) | [Helix for VS Code](https://marketplace.visualstudio.com/items?itemName=jasew.vscode-helix-emulation) extension| Seems to work less well than Dance.
| [Zed](https://zed.dev/) | native via keybindings ([Bug](https://github.com/zed-industries/zed/issues/4642)) | Still pretty rudimentary.


## Shells

| Shell | Plugin or feature providing Helix editing. | Comments
| --- | --- | --- |
| Fish | [Feature Request](https://github.com/fish-shell/fish-shell/issues/7748) | |
| Fish | [fish-helix](https://github.com/sshilovsky/fish-helix/tree/main) | Not tested, quality unknown. |

## Other software

| Software | Plugin or feature providing Helix editing. | Comments
| --- | --- | --- |
| [Obsidian](https://obsidian.md/) | [Obsidian-Helix](https://github.com/Sinono3/obsidian-helix) |
