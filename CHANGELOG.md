# Changelog

## 0.1.3 — 2026-10-02

- Sign-in stays in the terminal. `kvasir login` asks for Deezer or Tidal, opens your usual browser, and reads the session from that browser.
- The separate app window is gone.

## 0.1.2 — 2026-10-02

- Opening `kvasir` with no command shows a window. Sign in through Deezer in a browser; the session cookie is read and saved.
- Settings are stored in the user config folder (`%APPDATA%\kvasir` on Windows, `~/Library/Application Support/kvasir` on macOS, `~/.config/kvasir` on Linux).
- `kvasir reset` deletes that folder.
- Search, a pasted link, or a result row downloads into the chosen folder. Quality and the folder are remembered.
- `DEEZER_ARL` still overrides the saved login. A `.env` file is no longer required.
- Builds against `kvasir-core` 0.1.1.

## 0.1.1 — 2026-10-02

- `kvasir` is installed beside `kvasir-cli`. Both names run the same program, and `--help` uses the name you invoked.

## 0.1.0 — 2026-10-02

First release.

- `whoami`, `search`, `resolve`, and `acquire` against `kvasir-core` 0.1.
- Banner and status lines modeled on gerdur.
- Quality names `128`, `320`, and `flac`.
- Depends on the published crate, not a sibling path.
