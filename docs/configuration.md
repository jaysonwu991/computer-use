# Configuration

There is no config file for behaviour: limits are per-call arguments, so a
session can tighten or loosen them without restarting anything. What *is*
configured on disk is which apps the server may touch, and which MCP client
launches it.

## Files

| File | Written by | Read by | Purpose |
| --- | --- | --- | --- |
| `~/.config/computer-use/config.json` | `computer-use apps allow\|deny\|reset` | the server at start | App allow/deny policy. |
| `$COMPUTER_USE_CONFIG` | you | the server at start | Overrides the policy path (a file, not a directory). |
| `~/.oxide/mcp.json` | `install --client oxide` | Oxide | Global MCP servers. |
| `./.oxide/mcp.json` | `install --client oxide --project` | Oxide | Project MCP servers. |
| `~/.claude.json` | `install --client claude` | Claude Code | User-scope MCP servers. |
| `./.mcp.json` | `install --client claude --project` | Claude Code | Project-scope MCP servers. |
| `~/.codex/config.toml` | `install --client codex` | Codex CLI | `[mcp_servers.<name>]`. |
| `./.codex/config.toml` | `install --client codex --project` | Codex CLI in that repo | Project servers. |
| `~/.cursor/mcp.json` | `install --client cursor` | Cursor | User servers. |
| `./.cursor/mcp.json` | `install --client cursor --project` | Cursor in that repo | Project servers. |
| `~/Library/Application Support/Claude/claude_desktop_config.json` | `install --client desktop` | Claude Desktop | Its only scope. |

`install` merges into the existing file — other servers, other settings and, for
Codex, comments and ordering survive — and writes through a temporary file plus a
rename, so an interrupted write cannot truncate a config.

`HOME` decides where all of the above lives. `COMPUTER_USE_CONFIG` is the only
other environment variable the server reads.

### Server name

The entry is registered as `computer-use`, except for Claude Code, which reserves
that exact server name and would skip the server: `--client claude` registers
`computer-use-local`. `--name` overrides either default, and
`--client claude --name computer-use` is refused with that explanation. Names may
contain letters, digits, `-` and `_` only, so every client accepts them.

## App policy

```json
{
  "allowed_apps": ["Finder", "TextEdit", "Google Chrome"],
  "denied_apps": ["1Password", "Keychain Access"]
}
```

Semantics:

- **Deny wins.** A name in `denied_apps` is refused even if the allow list also
  matches it.
- **An empty allow list means "everything not denied".** Once `allowed_apps` has
  one entry, every app not on it is refused.
- Matching is case-insensitive and exact on the app name as macOS reports it —
  the same string `list_apps` prints. `apps deny 1password` blocks `1Password`.
- The check happens after `app` is resolved and before any accessibility call, so
  a denial reads as a policy decision, not as a permissions problem.
- `list_apps` still shows a blocked app, marked `[blocked by app policy]`, so an
  agent can tell "refused" from "not running".

Commands:

```sh
computer-use apps list              # policy + running apps it marks
computer-use apps allow "Finder"    # starts or extends the allow list
computer-use apps deny "1Password"  # always wins
computer-use apps reset             # clears both lists
```

The policy is read **when a process starts**. A running `computer-use mcp`
server keeps the policy it started with; change it, then restart the session (or
let the client reconnect) for it to take effect. `computer-use call` reads it per
invocation.

## Defaults

| Setting | Default | Overridden by |
| --- | --- | --- |
| Tree nodes per snapshot | 1200 | `get_app_state.max_tree_nodes`, `query.max_tree_nodes` |
| Tree depth | 64 | `get_app_state.max_tree_depth` |
| Characters per text value | 200 | `get_app_state.text_limit` (`"max"` = no limit) |
| Reply text | 20000 characters | `max_output_chars` on `get_app_state` and `query` |
| `query` matches | 20 | `query.max_results` |
| Scroll amount | 3 lines | `scroll.lines` |
| Scroll direction | `down` | `scroll.direction` |
| Delay between `--calls` calls | 1 s | `call --sleep` |
| Screenshot | off | `get_app_state.include_screenshot`, `screenshot` |
| On-screen frames in the tree | off | `get_app_state.include_frames` |

Exceeding a budget is always reported: the reply says how much was cut and which
argument raises the limit.

## What a window must look like to be listed

`list_apps` hides surfaces that are not usable windows:

- layer 0 only, so menu-bar strips, panels and overlays are not apps;
- at least 120×80 points, so 1×1 and 10×10 helpers disappear;
- owners that are window-server agents are hidden: `Window Server`,
  `loginwindow`, `Dock`, `Control Center`, `Notification Center`, `通知中心`,
  `SystemUIServer`, `通用控制`, `自动填充`, and any name ending in `Service`.

Windows in another Space are kept, because accessibility still reaches those
apps. `list_apps {"include_helpers": true}` shows the hidden surfaces — the way to
find a native file picker that belongs to no app. Resolution of `app` searches
those helpers too, so a pid works even when the name is not listed.

## Permissions

| Permission | Needed by | Granted to |
| --- | --- | --- |
| Accessibility | every tool except `list_apps` and `screenshot` | the process that launches the server — normally your terminal, or the app that starts it |
| Screen Recording | `screenshot`, `include_screenshot` | the same process |

macOS records the grant against the *responsible* app, not this binary. Running
from Terminal.app means granting Terminal; running from Oxide's desktop app means
granting that app. Moving or rebuilding the binary can leave a stale entry, in
which case remove and re-add it in System Settings → Privacy & Security.

`computer-use doctor` reports both, plus the policy and its path.
`computer-use doctor --prompt` asks macOS to show the prompts. Neither can grant
on your behalf; only System Settings can.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | The command ran. A tool may still report `isError: true` — see below. |
| 1 | The command failed: bad arguments, unknown tool, client config it could not parse, or `doctor` finding a missing permission. |

`computer-use call` prints the MCP-shaped result for every call, including
failures, and exits 1 if any call in the batch failed. That makes it usable in a
shell pipeline while keeping the agent-facing message intact.
