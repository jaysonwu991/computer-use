# Compared with other Computer Use servers

Short answer: for one person on macOS driving their own machine from Oxide,
Claude Code or Codex, this server is a better fit than a signed app bundle —
one binary, strict action semantics, and an installer for every client. It is
not a replacement for a cross-platform project: `open-computer-use` ships
macOS, Linux and Windows runtimes, and Codex Computer Use ships a product.

| | computer-use | open-computer-use | Codex Computer Use |
| --- | --- | --- | --- |
| Platforms | macOS only; other builds return an explicit error | macOS, Linux, Windows | macOS |
| Distribution | one Rust binary (`cargo build --release`) | npm package, app bundle, onboarding app | part of Codex |
| MCP client setup | `install --client oxide\|claude\|codex\|cursor\|desktop`, or `--print` | documented config snippets, `install-codex-mcp` | built in |
| Tool surface | 12: the shared nine plus `query` and `screenshot` | 9–10, `select_text` on macOS | product-defined |
| Observation | accessibility tree, indexes bound to the window they came from | accessibility tree; screenshot embedded in every state by default | screenshots plus accessibility |
| Value writes | refused unless the element reports `AXValue` settable, then read back | reported success on controls that ignored the write ([#83](https://github.com/iFurySt/open-computer-use/issues/83)) | n/a |
| Stale indexes | rejected by name, with the app and window they belong to | not detected | n/a |
| Long text | per-value `text_limit` (`"max"` allowed), explicit truncation notes | fixed truncation per platform ([#36](https://github.com/iFurySt/open-computer-use/issues/36)) | n/a |
| Reply size | `max_output_chars` cap, screenshot opt-in per call | screenshot in every reply ([#83](https://github.com/iFurySt/open-computer-use/issues/83) 6a) | n/a |
| Per-app control | deny/allow lists enforced in the server, before any AX call | configurable allow list ([#12](https://github.com/iFurySt/open-computer-use/issues/12)) | product approvals |
| Dragging | paced HID events with deltas, the fix for [#53](https://github.com/iFurySt/open-computer-use/issues/53) | paced pointer events ([#53](https://github.com/iFurySt/open-computer-use/issues/53) open) | n/a |
| Testing | 46 unit/integration checks, no permissions needed for the config, protocol and policy paths | app-level smoke suite | internal |
| Onboarding | `doctor` in the terminal | `doctor` plus a permission onboarding window | app UI |

## What this server does deliberately differently

- **Accessibility first, pointer second.** `click` uses the element's
  `AXPress` action when it exists, so a background app can be driven without
  stealing the pointer. Only when there is no such action does it activate the
  app and post a real click, and the reply says which path ran.
- **Refusals instead of silent no-ops.** `set_value` checks `AXIsAttributeSettable`
  and reads the value back; `perform_secondary_action` refuses an action the
  element does not list. An agent can tell "nothing happened" from "it worked".
- **Observation is cheap by default.** No screenshot unless
  `include_screenshot: true`, a `max_output_chars` cap with a visible note, and
  `query` for searching a large tree. In an agent loop the text budget is the
  dominant cost.
- **Elements belong to a window.** A snapshot binds each index to its
  `AXWindow`; an index from before a window closed or moved is refused rather
  than applied to whatever is now at that position.
- **One tool list, one test.** `tools::tests` counts the tool specs, and the
  README table, the `tools` command and `tools/list` all come from it, so the
  documented surface cannot drift from the shipped one.

## When the other project is the better choice

- **You need Linux or Windows.** This repository only implements macOS; the
  other platforms compile to a stub whose tools return "not implemented here".
  `open-computer-use` has real runtimes there, with the issues that follow from
  that ([#89](https://github.com/iFurySt/open-computer-use/issues/89),
  [#91](https://github.com/iFurySt/open-computer-use/issues/91),
  [#51](https://github.com/iFurySt/open-computer-use/issues/51)).
- **You want a menu-bar app and onboarding windows.** This server has no UI by
  design: `doctor`, `--prompt` and the config files are the whole surface.
- **You want a signed, notarised download.** This project ships source that you
  build; the other ships releases and an npm package.
- **You need `select_text` on apps whose text is not exposed as `AXValue`.** Both
  servers use the selected-text-range API, but the other has had more field
  testing against real apps.

## Who wins on which machine

- Oxide, Claude Code, Codex or Cursor on one Mac: this server — one binary, an
  installer per client, and action semantics an agent can trust.
- A team automating mixed OS desktops: `open-computer-use` today, until this
  project grows Linux and Windows backends.
