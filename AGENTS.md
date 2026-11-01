# oxide-computer-use

Computer Use as a stdio MCP server for Oxide: the running apps on this Mac, an
app's accessibility tree, and the actions that drive it.

## Layout

- `src/main.rs`: the CLI (`mcp`, `call`, `tools`, `doctor`, `install-oxide`).
- `src/mcp.rs`: newline-delimited JSON-RPC over stdio, protocol `2025-06-18`.
- `src/tools.rs`: the single source of the tool list. A tool's schema, its
  dispatch arm and its entry in the README table are the same tool; adding one
  means changing all three, and `tools::tests` counts them.
- `src/runtime.rs`: picks the backend. macOS is real; every other platform
  compiles to a stub whose methods return an explicit error, so the tool list
  stays identical.
- `src/macos/`: `ax` (elements and actions), `tree` (snapshot rendering and the
  element indexes), `apps` (window server), `input` (synthetic events),
  `screen` (screenshot), `cf` (Core Foundation bridging).
- `tests/cli.rs`: the binary as an MCP client sees it.

## Contracts to keep

- Element indexes come from the newest snapshot, are per process, and are
  rejected when the app or the range does not match. Never act on a stale index
  by guessing.
- Tool failures are `isError` results with a message an agent can act on, not
  JSON-RPC errors. Protocol errors are reserved for bad JSON and unknown
  methods.
- Every action reports the path it took (accessibility action versus pointer
  event), because the two behave differently for a background app.
- A refused or impossible action is reported as such: no success text without
  a read-back (`set_value` reads the value again) or a confirmed event post.
- Accessibility and Screen Recording are user permissions. Ask, explain, and
  fail with instructions; never loop on a permission prompt.

## Commands

- `cargo build`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt`
- `./target/debug/oxide-computer-use tools`
- `./target/debug/oxide-computer-use doctor`
- `./target/debug/oxide-computer-use call list_apps`
- `printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./target/debug/oxide-computer-use mcp`

## Conventions

- Rust 2021, 4-space indent, default rustfmt, `anyhow::Result` with
  `.context(...)` for human-readable failures.
- No comments that restate the code; a comment explains an FFI signature, a
  platform quirk or an intent that is not obvious from the call.
- All `unsafe` is confined to `src/macos/`, where it names the framework call it
  wraps. Nothing unsafe leaks into `tools.rs` or the CLI.
- Keep the binary dependency-light: standard library, `serde`, `serde_json`,
  `anyhow`, `base64`, `clap`, and `core-foundation-sys` on macOS.
