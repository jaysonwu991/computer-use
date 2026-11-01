# oxide-computer-use

Computer Use as an MCP server for [Oxide](https://github.com/iFurySt/oxide):
any Oxide session — terminal, desktop or VS Code — can list the running apps,
read an app's accessibility tree, and click, type, scroll and drag inside it.

It is the same idea as [open-computer-use](https://github.com/iFurySt/open-computer-use),
written in Rust as a single binary, with no app bundle and no runtime
dependencies beyond the macOS frameworks it links.

```text
Oxide session ──stdio JSON-RPC──▶ oxide-computer-use ──▶ macOS Accessibility (AX) + CoreGraphics
```

## Requirements

- macOS 14 or later.
- Accessibility permission for the terminal or app that launches the server
  (System Settings → Privacy & Security → Accessibility).
- Screen Recording permission for the `screenshot` tool.

## Build and install

```sh
cargo build --release
install -m 755 target/release/oxide-computer-use ~/.local/bin/oxide-computer-use
```

## Connect it to Oxide

Either let the binary write the entry itself:

```sh
oxide-computer-use install-oxide            # ~/.oxide/mcp.json
oxide-computer-use install-oxide --project  # ./.oxide/mcp.json
oxide-computer-use install-oxide --print    # just show the JSON
```

or register it with Oxide's own command:

```sh
oxide mcp add computer-use oxide-computer-use mcp
oxide mcp list
```

The config entry it writes, also valid in Claude Desktop or any other MCP
client:

```json
{
  "mcpServers": {
    "computer-use": {
      "command": "oxide-computer-use",
      "args": ["mcp"]
    }
  }
}
```

Oxide asks for approval before an MCP tool runs, so each action stays visible
and is attributed to the `computer-use` server.

## Check the permissions

```sh
oxide-computer-use doctor          # human readable
oxide-computer-use doctor --json   # for scripts
oxide-computer-use doctor --prompt # ask macOS to show the prompts
```

## Tools

| Tool | What it does |
| --- | --- |
| `list_apps` | Running apps that own on-screen windows, with pid and window titles. |
| `get_app_state` | One app's accessibility tree as indexed lines; the indexes drive every later action. |
| `query` | Search that tree by text and/or role without printing all of it. Also refreshes the indexes. |
| `click` | `AXPress` when the element has it, otherwise a pointer click at its center or at `x`/`y`. |
| `perform_secondary_action` | A named AX action such as `AXShowMenu` or `AXConfirm`. |
| `set_value` | Writes `AXValue` and reads it back, so a read-only control fails loudly. |
| `type_text` | Types into the focused control; activates the app first. |
| `press_key` | One key with optional `cmd`/`shift`/`alt`/`ctrl` modifiers. |
| `scroll` | Wheel events at the frontmost window, an element center, or `x`/`y`. |
| `drag` | Left-button drag from an element center or `from_x`/`from_y` to `to_x`/`to_y`. |
| `screenshot` | A PNG of the screen or of one app's largest window. |

The element indexes are per-process state: `get_app_state` and `query` replace
the snapshot, and an index from an old snapshot is rejected instead of being
applied to the wrong element.

## Call it without an agent

```sh
oxide-computer-use call list_apps
oxide-computer-use call get_app_state --args '{"app":"TextEdit"}'
oxide-computer-use call click --args '{"app":"TextEdit","element_index":12}'
```

A sequence shares one process and one snapshot state:

```sh
oxide-computer-use call --calls '[
  {"tool":"get_app_state","args":{"app":"TextEdit"}},
  {"tool":"click","args":{"app":"TextEdit","element_index":9}},
  {"tool":"type_text","args":{"app":"TextEdit","text":"Hello from Oxide"}}
]'
```

Each call prints the MCP-shaped result, and the process exits nonzero when a
call fails.

## How it works

- **Observation** — `AXUIElementCreateApplication` for the target pid, then a
  depth-limited walk of `AXChildren`, rendering role, title, description, value,
  identifier and focus state. Oversized trees are reported as truncated rather
  than silently cut.
- **Actions** — accessibility actions and attribute writes first, because they
  reach a background app; synthetic `CGEvent` input only where AX has no
  equivalent, and the result text says which path was taken.
- **Pointer and keyboard** — `CGEventPost` on the HID tap, so events reach the
  same place a real device would.
- **Screenshots** — `screencapture -x -o`, returned as an MCP `image` part.
- **Server** — newline-delimited JSON-RPC over stdio, protocol version
  `2025-06-18`, tools only. Tool failures come back as `isError` results, not
  protocol errors, so an agent sees the message and can retry.

## Layout

```text
src/main.rs        CLI: mcp, call, tools, doctor, install-oxide
src/mcp.rs         stdio JSON-RPC server
src/tools.rs       tool schemas and dispatch
src/runtime.rs     platform selection (macOS implementation, explicit stub elsewhere)
src/macos/ax.rs    accessibility elements, attributes and actions
src/macos/tree.rs  snapshot rendering and element indexes
src/macos/apps.rs  running apps and windows via the window server
src/macos/input.rs synthetic pointer, keyboard and scroll events
src/macos/screen.rs screenshot capture
src/macos/cf.rs    Core Foundation bridging
tests/cli.rs       binary-level checks of the CLI, the MCP handshake and the config writer
```

## Limits

- macOS only. Other platforms compile to a build whose tools return an
  explicit "not implemented here" error.
- Element indexes describe the app, not a window: a snapshot of a busy app can
  go stale between two calls, and the snapshot is refreshed on the next
  `get_app_state` or `query`.
- Typing and pointer events need the target app in front; `set_value` is the
  reliable path for text fields.
- No coordinate-to-element translation for arbitrary points beyond the
  snapshot, and no window resize or move.

## License

MIT.
