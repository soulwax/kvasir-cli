# kvasir-cli

Command-line client for [`kvasir-core`](https://github.com/soulwax/kvasir). It does not vendor the library. `cargo build` in this repository resolves `kvasir-core` 0.1 from crates.io, so this repo works without a checkout of kvasir beside it.

`cargo install kvasir-cli` installs two commands, `kvasir` and `kvasir-cli`. They are the same program.

Run `kvasir` with no command to open the window. **Sign in with Deezer** opens a browser window; after you log in, Kvasir reads the `arl` cookie and stores it. You do not need a `.env` file. `DEEZER_ARL` in the environment still overrides the saved login.

Settings live in the user config folder:

- Windows: `%APPDATA%\kvasir`
- macOS: `~/Library/Application Support/kvasir`
- Linux: `$XDG_CONFIG_HOME/kvasir` or `~/.config/kvasir`

`kvasir reset` deletes that folder so the next launch starts signed out. Quit the window first.

```sh
kvasir
kvasir login
kvasir reset
kvasir whoami
kvasir search "daft punk"
kvasir resolve https://www.deezer.com/track/3135556
kvasir acquire 3135556 --quality 128 --out track.mp3
```

`--quality` accepts `128`, `320`, or `flac`. The older shorthand `1`, `3`, and `9` still works.

A run starts with a short banner. Status lines are `info`, `pending`, `success`, `warn`, and `error`, with an indented note for the detail. Color is used on a terminal and omitted when `NO_COLOR` is set or stdout is piped.

GNU GPL v3.0 only. See [LICENSE](LICENSE). For personal use of music your account is allowed to stream.
