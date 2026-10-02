# kvasir-cli

Command-line client for [`kvasir-core`](https://github.com/soulwax/kvasir). It does not vendor the library. `cargo build` in this repository resolves `kvasir-core` 0.1 from crates.io, so this repo works without a checkout of kvasir beside it.

Set `DEEZER_ARL` to your 192-character Deezer `arl` cookie. A local `.env` is gitignored; copy [`.env.example`](.env.example).

```sh
kvasir-cli whoami
kvasir-cli search "daft punk"
kvasir-cli resolve https://www.deezer.com/track/3135556
kvasir-cli acquire 3135556 --quality 128 --out track.mp3
```

`--quality` accepts `128`, `320`, or `flac`. The older shorthand `1`, `3`, and `9` still works.

A run starts with a short banner. Status lines are `info`, `pending`, `success`, `warn`, and `error`, with an indented note for the detail. Color is used on a terminal and omitted when `NO_COLOR` is set or stdout is piped.

GNU GPL v3.0 only. See [LICENSE](LICENSE). For personal use of music your account is allowed to stream.
