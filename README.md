# Helix Better Docs

I've completely rebuilt the Helix documentation site and landing page with the idea that Helix should have a lot better documentation and be more approachable to newcomers who have never used a modal editor before.

- I picked out Helix's best bits and placed it on the main landing page to get users hooked. It should be obvious from the get-go the features that Helix has.
- I placed the scattered documentation from the GitHub Wiki and the Docs site into a single documentation site.

Now, there's a clear structure that is followed and easy to parse for newcomers who want to learn about Helix. I've specifically spent a lot of time creating:

- `The Basics` is now a completely interactive tutorial of the Helix editor, showcasing useful keymappings intended for people not familiar with modal editors.
- The entire `Usage` section is split into various topics of interest such as how to add Language Servers, how to use Multiple Cursors, how to configure Helix. The way it is written is not a reference, but rather a how-to with useful tips.
- `Reference` has been taken from the docs site, and includes thorough information on each setting, language, formatter etc. It hasn't been edited by me, but I plan to improve this section aswell.
- `Contributing` section has been taken 1:1 from the docs site, with plans to improve their documentation aswell. For example, I want to add screenshots to what each feature does.
- `Help` section takens items from the Wiki and places them here. It includes tutorials on how to extend the editor.

## Why I made this site

- It feels inconsistent that Helix's main website is completely different to the documentation website
- I wanted to create a better landing page for Helix so people get hooked. The current landing page wasn't very captivating
- Design-wise, helix has a clear brand. It's unique, not many have a purple website. So I want to build on it to make it memorable.
- Better tutorial. Yes, having a tutorial in-editor is good. But I wanted to create a really good showcase of the editor to people that will just be reading the website and not instantly try to install it. It's also useful because you always have your phone with you.

### Repo Structure

The docs are written in MDX which is an enhanced version of markdown with support for custom components and custom syntax. These docs can be edited by anyone, and the repository structure is extremely simple.

All of the documentation is located inside `src/content/docs`. Each folder is a category containing several `mdx` files or folders with an `index.mdx` file inside, which is the entry point.

This allows to easily have page-specific components.

For the framework, I am using [Starlight](https://starlight.astro.build/) which is absolutely fantastic and builds on top of Astro, which is exactly perfect for content-heavy sites exactly like this one.

### Visualizations

Since Helix is a terminal app, we're not using `.png` files. Instead, we are representing a terminal view as plain HTML by taking a snapshot of all of the escape codes in the terminal and transforming it into HTML.

For recording videos, we're not using `.mp4` but rather using [`Asciinema`](https://asciinema.org/) which converts all of the escape codes over time into a file which is then transformed into HTML.

### Why Starlight and not Docusaurus?

In the beginning I tried using Docusaurus, which is more popular. However after trying for a bit, I ran into many headaches especially regarding customization. It is extremely important to me to reflect Helix's purple color theme and doing that with Docusaurus felt basically impossible. It isn't designed for customization. With starlight I was able to use Helix's colors for the entire docs easily thanks to their first-class theming support.

## Running

To run the project:

- `git clone`
- `pnpm install` installs all the dependencies. You can also use `npm` or whatever JavaScript package manager you prefer.
- `pnpm start` will start the dev server on `localhost:4321`
- `pnpm preview` will build and preview
- `pnpm build` will generate static HTML & CSS files in the `dist` folder

## Contributing

### Creating Terminal Screenshots and rendering as HTML

1. install `gnome-terminal` and `dbus` on your computer. You don't need anything else like Gnome desktop, but those two packages are required
2. run `gnome-terminal` with the following command:

```sh
dbus-launch gnome-terminal --geometry=32x20
```

This will create a terminal window with a width of `32` characters and a height of `20` characters.
Feel free to change the height to what makes sense, but **do not change the width**.

After you have your desired state:

- click `Edit > Select All` in the top left corner.
- click `View > Copy as HTML` which will create HTML that, when rendered will show the exact state of the terminal.
