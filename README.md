# kvasir-cli

A small program that exercises [kvasir](https://github.com/soulwax/kvasir): the Deezer account, gateway search, URL resolution, and one tagged download.

Set `DEEZER_ARL` to a 192-character Deezer `arl` cookie. The local `.env` is not part of this repository.

```sh
kvasir-cli whoami
kvasir-cli search "daft punk"
kvasir-cli resolve https://www.deezer.com/track/3135556
kvasir-cli acquire 3135556 --quality 1 --out track.mp3
```

`--quality` accepts `1`, `3`, `9`, `MP3_128`, `MP3_320`, or `FLAC`.

For personal use with music your account is allowed to stream.
