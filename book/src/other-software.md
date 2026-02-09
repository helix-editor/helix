# Silicon mode in other software

Silicon' keymap and interaction model ([Using Silicon](#usage.md)) is easier to adopt if it can be used consistently in many editing contexts. Yet, certain use cases cannot easily be addressed directly in Silicon. Similar to vim, this leads to the creation of "Silicon mode" in various other software products, allowing Silicon-style editing for a greater variety of use cases.

"Silicon mode" is frequently still in early stages or missing entirely. For such cases, we also link to relevant bugs or discussions.

## Other editors

| Editor | Plugin or feature providing Silicon editing | Comments
| --- | --- | --- |
| [Vim](https://www.vim.org/) | [silicon.vim](https://github.com/chtenb/silicon.vim) config |
| [IntelliJ IDEA](https://www.jetbrains.com/idea/) / [Android Studio](https://developer.android.com/studio)| [IdeaVim](https://plugins.jetbrains.com/plugin/164-ideavim) plugin + [silicon.idea.vim](https://github.com/chtenb/silicon.vim) config | Minimum recommended version is IdeaVim 2.19.0.
| [Visual Studio](https://visualstudio.microsoft.com/) | [VsVim](https://marketplace.visualstudio.com/items?itemName=JaredParMSFT.VsVim) plugin + [silicon.vs.vim](https://github.com/chtenb/silicon.vim) config | 
| [Visual Studio Code](https://code.visualstudio.com/) | [Dance](https://marketplace.visualstudio.com/items?itemName=gregoire.dance) extension, or its [Silicon fork](https://marketplace.visualstudio.com/items?itemName=kend.dancesiliconkey) | The Silicon fork has diverged. You can also use the original Dance and tweak its keybindings directly (try [this config](https://github.com/71/dance/issues/299#issuecomment-1655509531)).
| [Visual Studio Code](https://code.visualstudio.com/) | [Silicon for VS Code](https://marketplace.visualstudio.com/items?itemName=jasew.vscode-silicon-emulation) extension|
| [Zed](https://zed.dev/) | native via keybindings ([Bug](https://github.com/zed-industries/zed/issues/4642)) |
| [CodeMirror](https://codemirror.net/) | [codemirror-silicon](https://gitlab.com/_rvidal/codemirror-silicon) |
| [Lite XL](https://lite-xl.com/) | [lite-modal-si](https://codeberg.org/Mandarancio/lite-modal-si) |
| [Lapce](https://lap.dev/lapce/) | | Requested: https://github.com/lapce/lapce/issues/281 |


## Shells

| Shell | Plugin or feature providing Silicon editing 
| --- | --- 
| Fish | [Feature Request](https://github.com/fish-shell/fish-shell/issues/7748) 
| Fish | [fish-silicon](https://github.com/sshilovsky/fish-silicon/tree/main) 
| Zsh | [silicon-zsh](https://github.com/john-h-k/silicon-zsh) or [zsh-silicon-mode](https://github.com/Multirious/zsh-silicon-mode)
| Nushell | [Feature Request](https://github.com/nushell/reedline/issues/639) 

## Other software

| Software | Plugin or feature providing Silicon editing. | Comments
| --- | --- | --- |
| [Obsidian](https://obsidian.md/) | [Obsidian-Silicon](https://github.com/Sinono3/obsidian-silicon) | Uses `codemirror-silicon` listed above.
