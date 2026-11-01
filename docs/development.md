# Development

## Prerequisites

- macOS 14 or later — the accessibility and event frameworks are macOS only.
- Rust 1.85 or newer (`rust-toolchain` is not pinned; the crate's
  `rust-version` is).
- Xcode Command Line Tools for the frameworks and `screencapture`.
- Nothing else: the release binary is one file.

## Commands

```sh
cargo build                     # debug binary
cargo build --release           # what you install
cargo test                      # unit tests + tests/cli.rs
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo fmt --check
```

```sh
./target/debug/computer-use tools
./target/debug/computer-use doctor
./target/debug/computer-use call list_apps
./target/debug/computer-use install --client codex --print
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./target/debug/computer-use mcp
```

## What the tests cover

Run in the same binary, so no permissions and no GUI session are needed except
where noted.

| Where | Guards |
| --- | --- |
| `src/mcp.rs` tests | initialize shape, revision echo (all three supported, unknown → newest), notifications not answered, unknown method → `-32601`, tool failure → `isError` result |
| `src/tools.rs` tests | every tool has a description, an object schema and a read-only hint; observers are read-only and actions are not; names unique; result shape with and without an image; every tool documented in `docs/tools.md` and listed in the README table |
| `src/install.rs` tests | client parsing and aliases, name validation (reserved name only for Claude Code), per-client default names, JSON and TOML snippets |
| `src/policy.rs` tests | empty policy allows all, deny beats allow, allow list excludes the rest, save/load round trip |
| `src/macos/input.rs` tests | key codes for named keys, modifier aliases, unknown key refused |
| `src/macos/tree.rs` tests | value truncation with true length, index/indent rendering, multi-line values flattened |
| `src/macos/mod.rs` tests | UTF-16 range math (including an emoji), case-insensitive fallback, output cap note, lenient argument parsing |
| `tests/cli.rs` | tool list, MCP handshake with a real subprocess, installers writing every client's file, merge preserving other settings, reserved-name refusal, policy round trip, UTF-8 through stdio, unknown tool exit code, `--calls-file` error naming the resolved path |

What they cannot cover: anything that needs Accessibility or Screen Recording —
window binding, `set_value` read-back against a real control, `select_text`,
paced drags, `AXPress` versus pointer clicks, and window-scoped screenshots.
Those need a trusted terminal; the manual checklist below is the substitute.

## Adding a tool

A tool exists in six places, and `tools::tests` fails if the docs drift:

1. `src/tools.rs` — the schema in `specs()`, with `readonly(...)` or
   `action(...)`, and the dispatch arm in `execute`.
2. `src/runtime.rs` — the method on the non-macOS stub, returning an explicit
   error.
3. `src/macos/mod.rs` — the real implementation on `Runtime`: resolve the app,
   `self.guard()?`, act, verify, and return a sentence that says what happened.
4. `docs/tools.md` — a `### \`name\`` section: parameters, behaviour, failure
   modes, example reply. The test greps for the heading.
5. `README.md` — a `| \`name\` | … |` row in the tool table. The test greps for
   the row.
6. `src/tools.rs` tests — update the expected count, and add behaviour tests for
   pure helpers the tool uses.

Rules that keep the surface honest:

- An action must verify what it changed, or say it could not.
- A refusal must name the fix, including the argument or command.
- Anything that moves the pointer or types must be described as doing so.
- Read-only tools get `readonly(...)`; state-changing ones get `action(...)` with
  `idempotent: true` only when repeating it changes nothing.

## Adding an MCP client

`src/install.rs` holds one enum. Add a variant and fill in:

- `parse` (with aliases), `label`, `format` (`Json` or `Toml`),
  `supports_project`, and `target` (user scope and project scope paths);
- a line in the trailing message that tells the user how to confirm the install;
- `default_name` if the client reserves a name;
- tests: parsing, target paths, and the written file in `tests/cli.rs`.

The writer merges rather than replaces, preserves unrelated keys, and refuses to
rewrite a file it cannot parse.

## Adding a platform backend

`src/runtime.rs` is the seam:

1. define `State { text, image }` (already shared);
2. implement every method the macOS `Runtime` implements, plus `guard`;
3. re-export the backend for the target OS;
4. never return a success-shaped guess — a tool that cannot work must fail with
   a message saying so.

Keep `unsafe` inside the backend. The macOS backend confines it to
`src/macos/` and names the framework call at every `extern` block; a new backend
should do the same. Filter helper surfaces at the enumeration layer, as
`apps.rs` does, so agents see apps rather than window-server internals.

## FFI rules in `src/macos/`

- `cf::Owned` wraps a `+1` Core Foundation reference and releases on drop. Use
  `from_create` for `Create`/`Copy` results and `from_get` for borrowed ones.
  Getting this backwards is a leak or a crash, so match the API's ownership
  convention, not a guess.
- `ax::Element` is the same idea for `AXUIElementRef`; `clone` retains, drop
  releases, and snapshots hold retained elements on purpose.
- Decode attribute values by type (`CFString`, `CFNumber`, `CFBoolean`,
  `CFArray`, `AXValue`) before use; a type mismatch is `None`, never a cast.
- A new `extern` block needs the framework name in its `#[link(...)]` and the
  header's signature verbatim; comment only where the signature is surprising
  (variadic-like APIs, out-parameters, ownership).

## Design rules worth preserving

- **Accessibility first, pointer second** — AX reaches background apps and does
  not steal the pointer. Say which path ran.
- **Policy before permission** — a refusal should be about the app, not about
  setup.
- **No silent success** — check settability, read values back, list supported
  actions, refuse stale indexes by name.
- **Pace synthetic input** — one unpaced drag event is a bug report.
- **Cap the output and say so** — context is the scarce resource in an agent
  loop.
- **One tool list** — schemas, dispatch, docs and tests come from `specs()`.

## Manual smoke checklist

On a machine where the terminal is trusted (`computer-use doctor` reports both
permissions granted):

```sh
computer-use call list_apps
computer-use call get_app_state --args '{"app":"TextEdit","window":0,"include_frames":true}'
computer-use call click --args '{"app":"TextEdit","element_index":6}'
computer-use call set_value --args '{"app":"TextEdit","element_index":6,"value":"hello"}'
computer-use call select_text --args '{"app":"TextEdit","element_index":6,"text":"hello"}'
computer-use call press_key --args '{"app":"TextEdit","key":"c","modifiers":"cmd"}'
computer-use call drag --args '{"app":"TextEdit","from_x":300,"from_y":300,"to_x":500,"to_y":300}'
computer-use call get_app_state --args '{"app":"TextEdit","include_screenshot":true}'
```

Expected: the click reports AXPress or a pointer event; `set_value` echoes the
value it read back; a read-only control fails instead of claiming success;
`select_text` reports a character range; the drag selects text or moves the
window; the screenshot returns a PNG. Then run the same sequence through
`--calls` and confirm the indexes stay valid across calls.

## Release checklist

1. `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
2. `cargo build --release`
3. Run `doctor` and the smoke checklist above on a trusted machine.
4. `install --client oxide|claude|codex|cursor|desktop` into a throwaway `HOME`
   and confirm each client lists the server.
5. Update `README.md`, `docs/tools.md`, `docs/architecture.md` and
   `docs/issues.md` for any behaviour change; the doc test enforces the first
   two for tool drift.
6. Commit with the commands you ran in the message.
