//! The Computer Use tool surface shared by the MCP server and the CLI.

use anyhow::{bail, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde_json::{json, Value};

use crate::runtime::Runtime;

pub struct Outcome {
    pub text: String,
    pub images: Vec<Vec<u8>>,
}

const APP: &str = "Running app name (or pid) exactly as `list_apps` reports it";

pub fn specs() -> Value {
    json!([
        {
            "name": "list_apps",
            "description": "List the running apps that own on-screen windows, with their pid and window titles.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": "get_app_state",
            "description": "Read one app's accessibility tree as indexed lines. Element indexes from the newest snapshot are what click, set_value, perform_secondary_action, drag and scroll take.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": APP },
                    "text_limit": { "description": "Characters shown per text value; a number, or \"max\" for full values." },
                    "max_tree_nodes": { "type": "integer", "description": "Node budget before the tree is reported truncated (default 1200)." },
                    "max_tree_depth": { "type": "integer", "description": "Depth limit before the tree is reported truncated (default 64)." },
                    "include_frames": { "type": "boolean", "description": "Append each element's on-screen center, for coordinate actions." }
                },
                "required": ["app"],
                "additionalProperties": false
            }
        },
        {
            "name": "query",
            "description": "Search one app's accessibility tree by text and/or role without printing the whole tree. Refreshes the element indexes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": APP },
                    "text": { "type": "string", "description": "Case-insensitive substring of role, title, description or value." },
                    "role": { "type": "string", "description": "Case-insensitive role filter, such as AXButton." },
                    "max_results": { "type": "integer", "description": "Default 20." }
                },
                "required": ["app"],
                "additionalProperties": false
            }
        },
        {
            "name": "click",
            "description": "Activate an element. Uses the element's AXPress action when it has one, and otherwise posts a pointer click at the element center (or at x/y).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": APP },
                    "element_index": { "type": "integer", "description": "Index from the newest get_app_state or query." },
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "button": { "type": "string", "enum": ["left", "right"], "description": "Default left." }
                },
                "required": ["app"],
                "additionalProperties": false
            }
        },
        {
            "name": "perform_secondary_action",
            "description": "Perform a named accessibility action on an element, such as AXShowMenu, AXIncrement or AXConfirm.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": APP },
                    "element_index": { "type": "integer" },
                    "action": { "type": "string", "description": "Default AXPress." }
                },
                "required": ["app", "element_index"],
                "additionalProperties": false
            }
        },
        {
            "name": "set_value",
            "description": "Write a string into an editable element's AXValue and read it back. Prefer this over typing for text fields.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": APP },
                    "element_index": { "type": "integer" },
                    "value": { "type": "string" }
                },
                "required": ["app", "element_index", "value"],
                "additionalProperties": false
            }
        },
        {
            "name": "type_text",
            "description": "Type text into the focused control of an app (activates the app, and focuses element_index first when given).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string" },
                    "app": { "type": "string", "description": APP },
                    "element_index": { "type": "integer" }
                },
                "required": ["text"],
                "additionalProperties": false
            }
        },
        {
            "name": "press_key",
            "description": "Press one key, optionally with modifiers. Key names: Return, Tab, Escape, Space, Delete, Forward_Delete, Home, End, Page_Up, Page_Down, Left, Right, Up, Down, F1-F12 or a single character. Modifiers: cmd, shift, alt, ctrl.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string" },
                    "modifiers": { "type": "string", "description": "For example \"cmd+shift\"." },
                    "app": { "type": "string", "description": APP }
                },
                "required": ["key"],
                "additionalProperties": false
            }
        },
        {
            "name": "scroll",
            "description": "Scroll the app's frontmost window, or the pointer position when x/y or element_index is given.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": APP },
                    "direction": { "type": "string", "enum": ["up", "down"], "description": "Default down." },
                    "lines": { "type": "integer", "description": "Default 3." },
                    "element_index": { "type": "integer" },
                    "x": { "type": "number" },
                    "y": { "type": "number" }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "drag",
            "description": "Drag with the left button from an element center or from_x/from_y to to_x/to_y. Moves the real pointer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": APP },
                    "element_index": { "type": "integer" },
                    "from_x": { "type": "number" },
                    "from_y": { "type": "number" },
                    "to_x": { "type": "number" },
                    "to_y": { "type": "number" }
                },
                "required": ["to_x", "to_y"],
                "additionalProperties": false
            }
        },
        {
            "name": "screenshot",
            "description": "Capture the screen, or one app's largest window, as a PNG image. Needs Screen Recording permission.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "app": { "type": "string", "description": "Omit to capture the whole screen." }
                },
                "additionalProperties": false
            }
        }
    ])
}

pub fn execute(runtime: &mut Runtime, name: &str, args: &Value) -> Result<Outcome> {
    if name == "screenshot" {
        let bytes = runtime.screenshot(args)?;
        return Ok(Outcome {
            text: format!(
                "Captured a screenshot ({} KB PNG).{}",
                bytes.len() / 1024,
                if args.get("app").is_some() {
                    ""
                } else {
                    " It covers the whole screen."
                }
            ),
            images: vec![bytes],
        });
    }

    let text = match name {
        "list_apps" => runtime.list_apps()?,
        "get_app_state" => {
            runtime.guard()?;
            runtime.get_app_state(args)?
        }
        "query" => {
            runtime.guard()?;
            runtime.query(args)?
        }
        "click" => {
            runtime.guard()?;
            runtime.click(args)?
        }
        "perform_secondary_action" => {
            runtime.guard()?;
            runtime.perform_action(args)?
        }
        "set_value" => {
            runtime.guard()?;
            runtime.set_value(args)?
        }
        "type_text" => {
            runtime.guard()?;
            runtime.type_text(args)?
        }
        "press_key" => {
            runtime.guard()?;
            runtime.press_key(args)?
        }
        "scroll" => {
            runtime.guard()?;
            runtime.scroll(args)?
        }
        "drag" => {
            runtime.guard()?;
            runtime.drag(args)?
        }
        other => bail!("unknown tool `{other}`; call tools/list for the available tools"),
    };
    Ok(Outcome {
        text,
        images: Vec::new(),
    })
}

/// The MCP `CallToolResult` for an outcome: text first, then any images.
pub fn content_json(outcome: &Outcome) -> Value {
    let mut content = vec![json!({ "type": "text", "text": outcome.text })];
    for image in &outcome.images {
        content.push(json!({
            "type": "image",
            "data": BASE64.encode(image),
            "mimeType": "image/png"
        }));
    }
    json!({ "content": content, "isError": false })
}

pub fn names() -> Vec<String> {
    specs()
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| tool.get("name").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_declares_a_description_and_an_object_schema() {
        let tools = specs();
        let tools = tools.as_array().expect("specs is an array");
        assert_eq!(tools.len(), 11);
        for tool in tools {
            let name = tool["name"].as_str().expect("a tool has a name");
            assert!(
                tool["description"].as_str().unwrap_or_default().len() > 20,
                "{name} needs a usable description"
            );
            assert_eq!(tool["inputSchema"]["type"], json!("object"), "{name}");
        }
    }

    #[test]
    fn tool_names_are_unique() {
        let mut names = names();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), specs().as_array().map(Vec::len).unwrap_or(0));
    }

    #[test]
    fn a_text_only_outcome_is_one_text_part() {
        let outcome = Outcome {
            text: "Clicked element 4.".to_string(),
            images: Vec::new(),
        };
        let value = content_json(&outcome);
        assert_eq!(value["isError"], json!(false));
        assert_eq!(value["content"].as_array().map(Vec::len), Some(1));
        assert_eq!(value["content"][0]["type"], json!("text"));
        assert_eq!(value["content"][0]["text"], json!("Clicked element 4."));
    }

    #[test]
    fn an_image_outcome_adds_a_png_part_after_the_text() {
        let outcome = Outcome {
            text: "Captured a screenshot.".to_string(),
            images: vec![b"png".to_vec()],
        };
        let value = content_json(&outcome);
        assert_eq!(value["content"][0]["type"], json!("text"));
        assert_eq!(value["content"][1]["type"], json!("image"));
        assert_eq!(value["content"][1]["mimeType"], json!("image/png"));
        assert_eq!(value["content"][1]["data"], json!("cG5n"));
    }
}
