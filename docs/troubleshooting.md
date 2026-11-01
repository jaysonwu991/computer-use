# Troubleshooting

Start with three commands. They separate setup, discovery and reading, which is
where almost every problem lives.

```sh
computer-use doctor                     # permissions, policy, versions
computer-use call list_apps             # what the window server reports
computer-use call get_app_state --args '{"app":"TextEdit","max_tree_nodes":200}'
```

`doctor --json` gives the same information for a script:

```json
{
  "version": "0.1.0",
  "macos": "15.3",
  "accessibility": true,
  "screen_recording": true,
  "allowed_apps": [],
  "denied_apps": ["1Password"],
  "config": "/Users/you/.config/computer-use/config.json"
}
```

## Permissions

### A client says "Connected" but every tool fails

The MCP handshake needs no permission; the first accessibility call does. Run
`computer-use doctor`. If Accessibility is missing, the tool error names the
Settings pane.

Grant it to the **responsible app**, not to this binary: your terminal if you run
the server from a shell, or the app that launches it. macOS records the grant
against that app's bundle identity.

### System Settings shows Accessibility on, but `doctor` says missing

The grant belongs to a different process, or to a stale copy:

- launched from a different terminal than the one you granted;
- launched by an IDE's integrated terminal (grant the IDE);
- launched by `launchd`, `cron`, a remote `ssh` session or a daemon, where no
  app is responsible and the prompt can never appear — run it from a logged-in
  GUI session instead;
- the binary moved or was rebuilt after the entry was added: remove the entry and
  add it again.

`ps -o comm= -p $PPID` shows the parent that needs the grant.

### The permission was granted after the server started

Accessibility is checked on every tool call, so it takes effect immediately.
Screen Recording is cached by macOS per process: restart the server, and if the
capture still fails, restart the app that owns it.

### `doctor --prompt` shows no prompt

macOS shows a given app's prompt once. If it was dismissed, open System Settings
→ Privacy & Security yourself; the prompt is a convenience, not a control.

## Finding the app

| Message | Cause | What to do |
| --- | --- | --- |
| ``no running app matches `X` `` | Name typo, app has no usable window, or it is a hidden helper | `list_apps`; try the pid; `list_apps {"include_helpers":true}` |
| The app is running but not listed | Window smaller than 120×80, layer above 0, minimized, or a window-server agent | `include_helpers`, or target it by pid |
| ``window N does not exist`` / ``no window title contains …`` | Wrong index or title | Read the header of `get_app_state`: it lists the exact `window` values |
| Clicks land in the wrong window | The app has several windows and no `window` was given | Pass `window` (index or title substring); indexes are then bound to it |

## Element indexes

| Message | Cause | What to do |
| --- | --- | --- |
| ``element_index N is out of range`` | The snapshot was replaced, or the index came from an earlier read | Call `get_app_state` or `query` again and use its numbers |
| ``the cached elements belong to `A`, not `B` `` | Acting on one app with another app's indexes | Read the state of the app you are acting on |
| ``is no longer in the window the snapshot bound it to`` | The element moved, a window closed, or a tab/window switched | Read again; do not retry the same index |
| ``the element changed since the snapshot`` | The app rebuilt its tree between read and act | Same: read, then act |
| ``has no on-screen frame`` | The element reports no position (some groups and virtual nodes) | Use `include_frames` with a neighbouring element, or `click` with `x`/`y` |

Indexes never survive a snapshot: the number is a position in that reply, not a
stable identifier.

## Actions that do not do what you expect

| Symptom or message | Cause | What to do |
| --- | --- | --- |
| ``reports AXValue as not settable`` | The control is read-only or text is edited through another attribute | `click` it, then `type_text`; or `press_key` per segment (date fields) |
| ``still reads … after writing`` | The control accepted the write and reverted it, or needs a commit action | `type_text` instead of `set_value`, then `press_key` Return or Tab |
| ``does not support AX…`` | The named action is not on that element | The message lists what it does support; `click` is usually `AXPress` |
| `click` succeeds but nothing changes | The element is `· disabled`, or its press action is a no-op | `include_frames`, then click its coordinates; or use `press_key` |
| Typing goes nowhere | The target is not the focused control, or another app is frontmost | Pass `app` (and `element_index` when known) so it is activated and focused |
| A drag moves nothing | Start point is not on a draggable area, or the window is not active | `include_frames` to pick a title-bar point; activate the app first with any action that takes `app` |
| Scrolling does nothing | Pointer is over a non-scrollable area | Pass `element_index` or `x`/`y` over the scrollable view |
| `unknown key` | Key name not recognised | Use the names from `docs/tools.md` (`Return`, `Tab`, `Page_Down`, `F5`, a single character) or `cmd+s` inline modifiers |

## macOS accessibility errors

The server prefixes these with the call that failed, for example
`AXPress failed: kAXErrorCannotComplete (-25204) (the app did not answer; …)`.

| Code | Name | Usually means | What to do |
| --- | --- | --- | --- |
| 0 | success | — | — |
| -25200 | `kAXErrorFailure` | The app failed to answer | Retry after reading state again |
| -25201 | `kAXErrorIllegalArgument` | The server passed something the app rejected | Report it with the element's role |
| -25202 | `kAXErrorInvalidUIElement` | The element is gone | `get_app_state` again |
| -25203 | `kAXErrorInvalidUIElementObserver` | Observation handle is invalid | Does not apply to this server |
| -25204 | `kAXErrorCannotComplete` | App is busy, modal, or hung | Wait, dismiss the sheet, then read state |
| -25205 | `kAXErrorAttributeUnsupported` | The element has no such attribute | Use another attribute or a different tool |
| -25206 | `kAXErrorActionUnsupported` | The element has no such action | `perform_secondary_action` lists supported actions |
| -25207 | `kAXErrorNotificationUnsupported` | Notifications unsupported | Does not apply |
| -25208 | `kAXErrorNotImplemented` | The app does not implement this | Try the pointer path |
| -25209 | `kAXErrorNotificationAlreadyRegistered` | Notifications already registered | Does not apply |
| -25210 | `kAXErrorNotificationNotRegistered` | Notifications not registered | Does not apply |
| -25211 | `kAXErrorAPIDisabled` | Accessibility permission missing | `doctor`, then grant it |
| -25212 | `kAXErrorNoValue` | No value for that attribute | Element is stale or empty; read again |
| -25213 | `kAXErrorParameterizedAttributeUnsupported` | Range-based text reads unsupported | `select_text` cannot work on that element |
| -25214 | `kAXErrorNotEnoughPrecision` | Coordinate conversion lost precision | Use `include_frames` coordinates |

## Screen capture

| Message | Cause | What to do |
| --- | --- | --- |
| ``screencapture produced an empty image`` | Screen Recording not granted | Grant it to the responsible app, then restart |
| ``screencapture failed (…)`` | Window in another Space, minimized, or gone | Capture the whole screen, or bring the window forward first |
| ``has no on-screen window to capture`` | No usable window for that app | Activate it with an action that takes `app` |
| A covered window returns the covering content | The platform capture composited the screen | Capture the specific window with `{"app": …}`, or bring it forward |
| `include_screenshot` says unavailable | Capture failed, tree still returned | Fix capture separately; the observation is not lost |

## Policy and approvals

| Symptom | Cause | What to do |
| --- | --- | --- |
| ``is on the deny list`` | `denied_apps` matched | `computer-use apps allow "X"` if intended; the deny list otherwise wins |
| ``is not on the allow list`` | An allow list exists and lacks the app | `computer-use apps allow "X"` |
| The policy changed but the server still refuses | The policy is read when the server starts | Restart the session or reconnect the client |
| Nothing prompts before an action | The client, not the server, owns approvals | Configure approval rules in the client; this server only marks read-only tools |

## Clients

| Symptom | Cause | What to do |
| --- | --- | --- |
| Claude Code ignores the server | `computer-use` is a reserved server name there | Register `computer-use-local` (`install --client claude` does), or another `--name` |
| A client cannot parse the config after an edit | The file was invalid before `install` ran; it refuses to rewrite it | Fix the JSON/TOML, rerun `install` |
| The client never starts the server | Wrong absolute path, or the binary is not executable | `install --print` to see the exact command, then run it by hand |
| Tools appear but calls fail with "Connection closed" | The server exited: bad launch command, or a panic | Run the same command in a terminal and send one request; a clean server prints JSON and nothing else |
| Non-ASCII text looks mangled | The client's console encoding, not the transport | The server writes UTF-8 JSON; evidence in `tests/cli.rs` |

## Context and performance

- Large tree: `query` first, or lower `max_tree_nodes`; the reply states when it
  truncated.
- Long text values: leave `text_limit` at 200, raise it for one field only.
- Huge replies: `max_output_chars` caps them; screenshots are opt-in.
- Many steps: prefer `--calls` in the CLI, or one `get_app_state` followed by
  several actions in a session, over re-reading the tree each time.
- A single slow call: accessibility calls are synchronous and blocking; an app
  that is busy blocks the next request until it answers or times out.

## Collecting evidence for a report

```sh
computer-use doctor --json
computer-use tools
computer-use call list_apps
computer-use call get_app_state --args '{"app":"<app>","max_tree_nodes":400,"include_frames":true}'
```

Include the exact `element_index` or `window` used, and the reply verbatim: the
messages carry the role, the index, the app and the accessibility error code,
which is what makes a failure reproducible.
