# Guidance for the We are bored repo

## Running tests and the binary

- `cargo test` runs the whole workspace offline. No x0x daemon, `API`, `TOKEN`, or network
  is required, and no `directory_of_boreds.toml` fixture is needed. `bored/src/x0x_client.rs`
  tests use `test_client` with `api_base: "http://127.0.0.1:0"` and an empty token, so they
  never touch the network.
- `surf-bored` compiles with 4 warnings (unused `hyperlink_style`, never-used
  `App::has_local_connection`, unread `Theme::name`, elided lifetime on
  `get_display_notices`). None are bugs; do not `cargo fix` them, they are intentional.
- `app::tests::test_file_load` is `#[ignored]`. If it appears in a test run that is expected;
  use `cargo test -p surf-bored -- --include-ignored` only when you mean to re-run it.
- **`surf-bored` must be `chmod 774` on Linux before running**, or the executable fails. This
  is not visible in Cargo.toml or any doc.
- The only documented CLI argument is `local`. Other arguments are not supported.
- `cargo test -p surf-bored` reports 9 tests: 8 passing, 1 ignored.

## Running and managing the x0x daemon

- Authoritative install/run docs are in `.github/skills/x0x/SKILL.md`, not the READMEs. Prebuilt
  binaries, `x0x` and `x0xd` on PATH, `x0x start` to run it. Port 12700 is used for local
  daemon communication.
- `surf-bored` exits with code 1 in three distinct cases that look the same in the CLI: daemon
  not installed, daemon installed but stopped, and token-denied access (daemon running under
  another account). The stopped-daemon path prompts `y/N` and can run `x0x start` or
  `curl -sfL https://x0x.md | sh -s -- --start` interactively.

## Repository data and fixtures

- `directory_of_boreds.toml` is listed in `.gitignore` but is the runtime address config. The
  app first looks for it in the resolved data dir (HOME / APPDATA / LOCALAPPDATA / UserProfile),
  then falls back to this repo's root. Never delete or rename the repo copy; regenerating it is
  safe.
- `test_directory.toml` is also gitignored (a temp fixture); don't commit it.
- `backtrace` and `notice` at repo root are gitignored build/runtime artifacts, not source.
- `test_sse.py` requires the `API` and `TOKEN` environment variables and streams `/events`, then
  posts `POST /subscribe` and `POST /publish`. It has a 4-second window.

## Known limitations and behaviors to reproduce before changing

- Resizing the terminal breaks rendering until the app is restarted.
- Very large boreds are slow and impractical.
- The next-notice algorithm is idiosyncratic; if navigation seems stuck, cycling tabs or
  alt-tabbing may be the actual workaround.
- Downloads sometimes fail with a record-not-found error. Not all listed records resolve.
- Rust 2024 will not run on Android 9.

## When docs disagree with code

- The executable source wins over prose. `Cargo.toml`, `cargo metadata`, and the tests are
  authoritative over `README.md`, `We_are_bored_protocol.md`, `SMILE_Philosophy.md`, and
  `migrate_to_x0x .md` when they conflict.
- `bored` is 0.6.1 and `surf-bored` is 0.6.3 in `Cargo.lock`.
