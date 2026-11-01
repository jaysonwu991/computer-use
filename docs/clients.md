# Use it from any MCP client

`oxide-computer-use mcp` is a plain stdio MCP server, so any client that can
launch a local command can use it. `oxide-computer-use install --client <name>`
writes the entry that client reads; `--print` shows it instead of writing.

```sh
oxide-computer-use install --client oxide     # ~/.oxide/mcp.json
oxide-computer-use install --client claude    # ~/.claude.json
oxide-computer-use install --client codex     # ~/.codex/config.toml
oxide-computer-use install --client cursor    # ~/.cursor/mcp.json
oxide-computer-use install --client desktop   # Claude Desktop
oxide-computer-use install --client claude --project   # ./.mcp.json
oxide-computer-use install --client <any> --print      # the entry, unedited
```

The registered name defaults to `oxide-computer-use`. Keep it: Claude Code
skips a server configured under the reserved name `computer-use`, and this
project refuses to write that name.

## Oxide

```sh
oxide-computer-use install --client oxide
oxide mcp list          # computer-use … Connected
```

`oxide mcp add computer-use oxide-computer-use mcp` is the equivalent when the
binary is on `PATH`. Oxide's own approval prompt covers each tool call.

## Claude Code

Docs: <https://docs.claude.com/en/docs/claude-code/mcp>

```sh
claude mcp add --transport stdio oxide-computer-use -- /absolute/path/oxide-computer-use mcp
claude mcp list
```

Or write the file and skip the CLI:

```sh
oxide-computer-use install --client claude           # user scope, ~/.claude.json
oxide-computer-use install --client claude --project # project scope, ./.mcp.json
```

Entry shape, for either file:

```json
{
  "mcpServers": {
    "oxide-computer-use": {
      "command": "/absolute/path/oxide-computer-use",
      "args": ["mcp"]
    }
  }
}
```

Two details that matter for Claude Code:

- `computer-use` is a reserved server name there; the server is skipped at load
  time if it is registered under that name. This project refuses the name when
  it installs and names the server `oxide-computer-use`.
- Tools that only read — `list_apps`, `get_app_state`, `query`, `screenshot` —
  carry `readOnlyHint: true`, so an approval rule can allow them without
  approving the actions too.
- Claude Code's oldest supported revision, `2024-11-05`, is answered with the
  same revision; the server echoes whichever supported version the client asks
  for.

## Codex CLI

Docs: <https://developers.openai.com/codex/mcp>

```sh
codex mcp add oxide-computer-use -- /absolute/path/oxide-computer-use mcp
codex mcp list
```

Or let this project edit `~/.codex/config.toml`, preserving the rest of the
file:

```sh
oxide-computer-use install --client codex
```

```toml
[mcp_servers.oxide-computer-use]
command = "/absolute/path/oxide-computer-use"
args = ["mcp"]
```

## Cursor

```sh
oxide-computer-use install --client cursor
```

Writes `~/.cursor/mcp.json` in the same `mcpServers` shape; `--project` writes
`.cursor/mcp.json` in the repository instead. Restart Cursor and check
Settings → MCP.

## Claude Desktop

```sh
oxide-computer-use install --client desktop
```

Writes `~/Library/Application Support/Claude/claude_desktop_config.json` and
asks for a restart. Desktop sessions can attach the PNG from `screenshot`, so
that tool is most useful there.

## Any other client

`oxide-computer-use install --print` emits the entry; a client that reads a
different file only needs to be pointed at it. The server needs nothing but a
command, arguments and a working `HOME`.

## Permissions and approvals

Two gates sit in front of every action:

1. **The client's approval prompt** — Oxide, Claude Code, Codex and Cursor each
   ask before a tool runs, per their own rules.
2. **The app policy** — `oxide-computer-use apps deny 1Password` (or
   `apps allow` to start an allow list) is enforced inside the server, before
   the accessibility call, and answers with the command that changes it.

macOS itself grants Accessibility to the process, not to the app being driven,
so the policy list is the per-app control; there is no OS prompt per target app.
