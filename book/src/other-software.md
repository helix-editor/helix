# Helix mode in other software

Helix' keymap and interaction model ([Using Helix](#usage.md)) is easier to adopt if it can be used consistently in many editing contexts. Yet, certain use cases cannot easily be addressed directly in Helix. Similar to vim, this leads to the creation of "Helix mode" in various other software products, allowing Helix-style editing for a greater variety of use cases.

"Helix mode" is frequently still in early stages or missing entirely. For such cases, we also link to relevant bugs or discussions.

## Other editors

| Editor | Plugin or feature providing Helix editing | Comments
| --- | --- | --- |
| [Vim](https://www.vim.org/) | [helix.vim](https://github.com/chtenb/helix.vim) config |
| [IntelliJ IDEA](https://www.jetbrains.com/idea/) / [Android Studio](https://developer.android.com/studio)| [IdeaVim](https://plugins.jetbrains.com/plugin/164-ideavim) plugin + [helix.idea.vim](https://github.com/chtenb/helix.vim) config | Minimum recommended version is IdeaVim 2.19.0.
| [Visual Studio](https://visualstudio.microsoft.com/) | [VsVim](https://marketplace.visualstudio.com/items?itemName=JaredParMSFT.VsVim) plugin + [helix.vs.vim](https://github.com/chtenb/helix.vim) config | 
| [Visual Studio Code](https://code.visualstudio.com/) | [Dance](https://marketplace.visualstudio.com/items?itemName=gregoire.dance) extension, or its [Helix fork](https://marketplace.visualstudio.com/items?itemName=kend.dancehelixkey) | The Helix fork has diverged. You can also use the original Dance and tweak its keybindings directly (try [this config](https://github.com/71/dance/issues/299#issuecomment-1655509531)).
| [Visual Studio Code](https://code.visualstudio.com/) | [Helix for VS Code](https://marketplace.visualstudio.com/items?itemName=jasew.vscode-helix-emulation) extension|
| [Zed](https://zed.dev/) | native via keybindings ([Bug](https://github.com/zed-industries/zed/issues/4642)) |
| [CodeMirror](https://codemirror.net/) | [codemirror-helix](https://gitlab.com/_rvidal/codemirror-helix) |


## Shells

| Shell | Plugin or feature providing Helix editing 
| --- | --- 
| Fish | [Feature Request](https://github.com/fish-shell/fish-shell/issues/7748) 
| Fish | [fish-helix](https://github.com/sshilovsky/fish-helix/tree/main) 
| Zsh | [helix-zsh](https://github.com/john-h-k/helix-zsh) or [zsh-helix-mode](https://github.com/Multirious/zsh-helix-mode)
| Nushell | [Feature Request](https://github.com/nushell/reedline/issues/639) 

## Other software

| Software | Plugin or feature providing Helix editing. | Comments
| --- | --- | --- |
| [Obsidian](https://obsidian.md/) | [Obsidian-Helix](https://github.com/Sinono3/obsidian-helix) | Uses `codemirror-helix` listed above.
