# Movements

## Word-based Movement

Rather than relying on `h`, `j`, `k`, and `l` to constantly navigate
document text, you can instead navigate based on words in the document. The
following keybinds are the most common of these:

- `w`: move forward until the next word. The cursor will end on the character
  before the next word.
- `e`: move fowrard to the end of the current word. The cursor will end on the
  last character.
- `b`: move backward to the beginning of the current word.

For each of these motions, the cursor will not only move. It will also create
a selection from where the cursor began to where the cursor ends. This is what
empowers Helix's *selection -> action* model.

## Words and WORDS

Each of the motions listed above have a capital-letter counterpart: `W`, `E`,
and `B`. These will traverse WORDS, instead of words.

- WORDS are separated only by whitespace.
- words are separated by whitespace in addition to other punctuation characters.

## Counts with Motions

You can type a number before a motion command to repeat it that many times.

For example, `2w` will move 2 words forward. `10j` will move ten characters
down the document.

> 💡 **TIP**
> 
> This is particularly powerful when combined with relative line numbers. With
> this setting enabled, in `NOR` mode, lines are numbered by how many lines away from
> the current cursor they are. This allows you to easily see a line you want to
> jump to is, for example, 13 lines above the cursor, and access it by typing
> `13k`.

## Goto

Pressing `g` in `NOR` mode will display the `GOTO` mode, which provides a number
of useful shortcuts for navigating the current buffer quickly. Key commands
in this mode include:

- `gg`: goto file start.
- `ge`: goto file end.
- `gh`: goto line start.
- `gl`: goto line end.
- `gw`: goto word. When executed, each word in sight will be highlighted by two
  letters at their start. When you then type those two letters, you instantly
  jump to the specified word.

## Moving to Characters

To jump the cursor to a specific character in the buffer, the following commands
are available:

- `f<character>`: jump forward to (**f**ind) the provided character, if it exists. The cursor
  will end *on* that character.
- `F<character>`: jump backwards to the provided character, if it exists. The cursor
  will end *on* that character.
- `t<character>`: jump forward until (**t**ill) the provided character. The cursor will end
  on the character directly before the provided character.
- `T<character>`: jump backward until the provided character. The cursor will end
  on the character directly after the provided character.

Each of these commands will create a selection over the character that the cursor
started on and the character the cursor ends on.

## Page Navigation

With large chunks of text, it is useful to scroll large areas of the buffer at
once.

- `ctrl` + `u` will scroll up half a page.
- `ctrl` + `d` will scroll down half a page.

