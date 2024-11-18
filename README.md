# Helix Docs

## 🧞 Commands

All commands are run from the root of the project, from a terminal:

| Command                    | Action                                           |
| :------------------------- | :----------------------------------------------- |
| `pnpm install`             | Installs dependencies                            |
| `pnpm run dev`             | Starts local dev server at `localhost:4321`      |
| `pnpm run build`           | Build your production site to `./dist/`          |
| `pnpm run preview`         | Preview your build locally, before deploying     |
| `pnpm run astro ...`       | Run CLI commands like `astro add`, `astro check` |
| `pnpm run astro -- --help` | Get help using the Astro CLI                     |

## Contributing

### Creating Terminal Screenshots and rendering as HTML

1. install `gnome-terminal` and `dbus` on your computer. You don't need anything else like Gnome desktop, but those two packages are required
2. run `gnome-terminal` with the following command:

```sh
dbus-launch gnome-terminal --geometry=64x20
```

This will create a terminal window with a width of `64` characters and a height of `20` characters.
Feel free to change the height to what makes sense, but **do not change the width**.

After you have your desired state:

- click `Edit > Select All` in the top left corner.
- click `View > Copy as HTML` which will create HTML that, when rendered will show the exact state of the terminal.
