---
name: oxide-computer-use
description: Drive this Mac's apps through the oxide-computer-use MCP server — list apps, read an accessibility tree, then click, type, scroll or drag. Use when a task needs a GUI app Oxide cannot reach through files or a CLI.
---

# Oxide Computer Use

The `computer-use` MCP server (binary `oxide-computer-use`) exposes the desktop
as tools. Reading is cheap; acting moves the real pointer and keyboard, so look
first and act on what you saw.

## Loop

1. `list_apps` — find the app's exact name or pid.
2. `get_app_state` with that `app` — read the indexed tree. Line indexes are the
   handle every later action takes.
3. Act with `element_index`, or `x`/`y` from `include_frames: true`.
4. Read again after the action. Indexes belong to the newest snapshot; an old
   one is rejected, and that rejection means the state moved, not that the tool
   is broken.

## Choosing a tool

- `click` uses the element's AXPress action when it has one. A control that
  refuses AXPress gets a pointer click at its center, which needs the app in
  front.
- `set_value` is the reliable way to fill a text field; `type_text` needs the
  control focused and the app frontmost.
- `press_key` takes one key plus `modifiers` (`cmd`, `shift`, `alt`, `ctrl`),
  for example `{"key":"s","modifiers":"cmd"}` to save.
- `query` when a tree is large: filter by text or role, and take the index from
  its output, which also refreshes the snapshot.
- `perform_secondary_action` for AX actions such as `AXShowMenu` on a row.
- `screenshot` only when the tree is not enough — it needs Screen Recording,
  and the image is a PNG part an agent may not be able to see.

## Failures worth reading

- `Accessibility permission is missing` — the terminal running the server is
  not trusted. Run `oxide-computer-use doctor --prompt`, approve it in System
  Settings → Privacy & Security → Accessibility, then restart the server.
- `kAXErrorCannotComplete` — the app is busy, modal, or unresponsive. Wait,
  then read the tree again rather than repeating the click.
- `the element changed since the snapshot` — call `get_app_state` before
  retrying; the index no longer points at what you saw.

## Rules

- One action, one confirmation: read the state back instead of chaining five
  blind clicks.
- Never send a password, a payment or an irreversible confirmation through this
  server without the user asking for exactly that.
- The app must be in front for typing and pointer events; activating it is
  allowed, taking over the pointer without need is not.
