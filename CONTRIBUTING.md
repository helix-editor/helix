# `Helix-editor`
### First off, Thanks for contributing! Please read the following: 

Contributors are very welcome! **No contribution is too small and all contributions are valued.**

Some suggestions to get started:

- You can look at the [good first issue](https://github.com/helix-editor/helix/labels/E-easy) label on the issue tracker.
- Help with packaging on various distributions needed!
- To use print debugging to the `~/.cache/helix/helix.log` file, you must:
  * Print using `log::info!`, `warn!`, or `error!`. (`log::info!("helix!")`)
  * Pass the appropriate verbosity level option for the desired log level. (`hx -v <file>` for info, more `v`s for higher severity inclusive)
- If your preferred language is missing, integrating a tree-sitter grammar for
    it and defining syntax highlight queries for it is straight forward and
    doesn't require much knowledge of the internals.

We provide an [architecture.md](./docs/architecture.md) that should give you
a good overview of the internals.

## Before contributing:
- Please check to see all closed and open issues and/or discussions that what problem you are facing has already been discussed, or solved. Please make sure you have the latest version.
- If you have a question or a doubt. Please *DO NOT* open an issue. The [discussions](https://github.com/helix-editor/helix/discussions) tab is exactly for that.

## Coding Conventions:
- Please do indentations with 4 spaces(soft tabs).
- Please give a space between operators [Not(x=3); (x = 3)].
- Give space before every curly bracket `{}`.
- As much as possible, give comments in seperate lines, and give a space to seperate the slash and the comment. i.e. `// My comment`.
- Please explain all your code with necessary comments. Remember, it should be understandable to a beginner also!
- Please give a new line for each function you use for a variable. Eg:
  ```rs
  let g1 = line
      .slice(line.len_chars().saturating_sub(1)..)
      .as_str()
      .unwrap();
   ```
- Give necessary new lines for each function, or different parts of your code.
- Try to fix all warnings from your code before making a pull request.
- Finally, use [rustfmt](https://github.com/rust-lang/rustfmt#on-the-stable-toolchain) to format the entire code according to rust guidelines.

## Pull Requests:
- Please explain what your code does. Make sure you have tested your code, so that it works. 
- If your code fixes an issue, give the neccessary issue number along with it.

## Finally:
Don't be sad if there are some negative comments, or your pull request doesn't get accepted. All of us have to start somewhere! 
- If it doesn't get accepted, ask kindly as to why it was rejected. Try to learn from that experience, and go forward!
