# desktop-use

Computer Use as a stdio MCP server for any agent on macOS — Oxide, Claude Code,
Codex CLI, Cursor, Claude Desktop. It lists the running apps, reads an app's
accessibility tree, and clicks, types, selects, scrolls and drags inside it.

The idea matches [open-computer-use](https://github.com/iFurySt/open-computer-use);
the implementation is a single Rust binary with no app bundle,
[one tool list](src/tools.rs), and action semantics that report failure instead
of pretending to succeed.

```text
agent (Oxide / Claude Code / Codex / Cursor) ──stdio JSON-RPC──▶ desktop-use
                                                   ├─▶ Accessibility (AX) elements, actions, values
                                                   ├─▶ CoreGraphics pointer, keyboard, scroll
                                                   └─▶ window server + screencapture
```

## Requirements

- macOS 14 or later.
- Accessibility permission for the terminal or app that launches the server
  (System Settings → Privacy & Security → Accessibility).
- Screen Recording permission for `screenshot` and `include_screenshot`.

## Build

```sh
cargo build --release
install -m 755 target/release/desktop-use ~/.local/bin/desktop-use
```

## Connect a client

```sh
desktop-use install --client oxide     # ~/.oxide/mcp.json
desktop-use install --client claude    # ~/.claude.json
desktop-use install --client codex     # ~/.codex/config.toml
desktop-use install --client cursor    # ~/.cursor/mcp.json
desktop-use install --client desktop   # Claude Desktop
desktop-use install --print            # the entry, for any other client
```

The server registers as `desktop-use`; Claude Code reserves the name
`computer-use` and skips a server registered under it, so this project refuses
to write that name. Details and the equivalent per-client commands are in
[docs/clients.md](docs/clients.md).

## Check permissions and policy

```sh
desktop-use doctor          # permissions, app policy, versions
desktop-use doctor --prompt # ask macOS to show the permission prompts
desktop-use apps deny "1Password"
desktop-use apps list
```

## Tools

| Tool | What it does |
| --- | --- |
| `list_apps` | Apps that own windows, with pid and window titles; `include_helpers` also shows window-server helpers such as a native file picker. |
| `get_app_state` | One app's accessibility tree as indexed lines; `window` binds it to a single window, `include_screenshot` attaches a PNG, `max_output_chars` caps the reply. |
| `query` | Search that tree by text and/or role without printing all of it. Also refreshes the indexes. |
| `click` | `AXPress` when the element has it, otherwise a pointer click at its center or at `x`/`y`; the reply says which ran. |
| `perform_secondary_action` | A named AX action such as `AXShowMenu`; an action the element does not support is refused with the list it does support. |
| `set_value` | Writes `AXValue` only when the element reports it settable, then reads it back. |
| `select_text` | Selects a span inside a text element, counted in UTF-16 units like Cocoa ranges. |
| `type_text` | Types into the focused control; activates the app first. |
| `press_key` | One key with optional `cmd`/`shift`/`alt`/`ctrl` modifiers. |
| `scroll` | Wheel events at the frontmost window, an element center, or `x`/`y`; needs no snapshot. |
| `drag` | Paced HID drag from an element center or `from_x`/`from_y` to `to_x`/`to_y`. |
| `screenshot` | A PNG of the screen or of one app's window. |

Element indexes are per-process state: `get_app_state` and `query` replace the
snapshot, each index is bound to the `AXWindow` it came from, and an index from
an old snapshot, the wrong app or the wrong window is refused by name instead of
being applied to whatever is there now.

## Call it without an agent

```sh
desktop-use call list_apps
desktop-use call get_app_state --args '{"app":"TextEdit","window":0}'
desktop-use call click --args '{"app":"TextEdit","element_index":12}'
```

A sequence shares one process and one snapshot state:

```sh
desktop-use call --calls '[
  {"tool":"get_app_state","args":{"app":"TextEdit"}},
  {"tool":"select_text","args":{"app":"TextEdit","element_index":9,"text":"Hello"}},
  {"tool":"press_key","args":{"app":"TextEdit","key":"c","modifiers":"cmd"}}
]'
```

Each call prints the MCP-shaped result, and the process exits nonzero when a
call fails.

## How it works

- **Observe** — `AXUIElementCreateApplication`, then a depth-limited walk of
  `AXChildren` rendering role, title, description, value, identifier, focus and
  disabled state. Truncation is reported, never silent.
- **Act** — accessibility actions and attribute writes first, because they reach
  a background app; synthetic `CGEvent` input only where AX has no equivalent,
  from a private HID event source and paced like a person, so drags and clicks
  land.
- **Refuse** — a non-settable value, an unsupported action, a stale index, a
  moved window or a denied app produces an error an agent can act on, not a
  success message.
- **Server** — newline-delimited JSON-RPC over stdio, serving revisions
  `2025-06-18`, `2025-03-26` and `2024-11-05` by echoing the client's, tools
  only. Tool failures are `isError` results, not protocol errors.

## Layout

```text
src/main.rs        CLI: mcp, call, tools, doctor, install, apps
src/mcp.rs         stdio JSON-RPC server and version negotiation
src/tools.rs       the tool schemas, dispatch and result shape
src/install.rs     per-client config writing (JSON and TOML)
src/policy.rs      the app allow/deny policy
src/runtime.rs     platform selection; macOS implementation, explicit stub elsewhere
src/macos/ax.rs    accessibility elements, attributes, actions, ranges
src/macos/tree.rs  snapshot rendering and window binding
src/macos/apps.rs  running apps and windows via the window server
src/macos/input.rs synthetic pointer, keyboard and scroll events
src/macos/screen.rs screenshot capture
src/macos/cf.rs    Core Foundation bridging
tests/cli.rs       binary-level checks: CLI, MCP handshake, installers, policy, UTF-8
docs/              clients, comparison, issue audit
```

## Documentation

- [docs/clients.md](docs/clients.md) — install and use it from each MCP client.
- [docs/comparison.md](docs/comparison.md) — how it differs from
  `open-computer-use` and Codex Computer Use, including where the others win.
- [docs/issues.md](docs/issues.md) — an audit against the
  `open-computer-use` issue tracker: what is designed out, what is mitigated,
  what is still open.

## Limits

- macOS only. Other platforms compile to a build whose tools return an explicit
  "not implemented here" error.
- Typing and pointer events need the target app in front; `set_value` and
  `AXPress` are the paths that work in the background.
- Native `<input type="date">` segments and web content behind a cross-origin
  picker are not reachable by AX alone.
- No audit log, and no screenshot of what an action changed.

## License

MIT.
