//! macOS computer-use backend: accessibility snapshots plus synthetic input.

pub mod apps;
pub mod ax;
pub mod cf;
pub mod input;
pub mod screen;
pub mod tree;

use anyhow::{anyhow, bail, Result};
use serde_json::Value;

use ax::{CGPoint, Element};
use tree::{Limits, Snapshot};

struct Cached {
    element: Element,
    center: Option<CGPoint>,
    role: String,
}

#[derive(Default)]
pub struct Runtime {
    nodes: Vec<Cached>,
    cached_app: Option<String>,
}

fn string_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
}

fn required_app(args: &Value) -> Result<String> {
    string_arg(args, "app")
        .ok_or_else(|| anyhow!("`app` is required; call list_apps to see the running apps"))
}

fn number_arg(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|value| match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    })
}

fn integer_arg(args: &Value, key: &str) -> Option<i64> {
    number_arg(args, key).map(|value| value as i64)
}

fn usize_arg(args: &Value, key: &str) -> Option<usize> {
    integer_arg(args, key).and_then(|value| usize::try_from(value).ok())
}

fn text_limit(args: &Value) -> Option<usize> {
    match args.get("text_limit") {
        Some(Value::Number(number)) => number.as_u64().map(|value| value as usize),
        Some(Value::String(text)) if text.eq_ignore_ascii_case("max") => Some(usize::MAX),
        _ => None,
    }
}

impl Runtime {
    pub fn new() -> Self {
        Runtime::default()
    }

    fn resolved(&self, name: &str) -> Result<apps::App> {
        apps::resolve(name).ok_or_else(|| {
            let known = apps::app_names();
            let hint = if known.is_empty() {
                "no app with a window is running".to_string()
            } else {
                format!("running apps: {}", known.join(", "))
            };
            anyhow!("no running app matches `{name}`; {hint}")
        })
    }

    fn app_element(app: &apps::App) -> Result<Element> {
        ax::application(app.pid)
            .ok_or_else(|| anyhow!("could not attach to {} ({})", app.name, app.pid))
    }

    fn store(&mut self, app: &apps::App, snapshot: Snapshot) {
        self.cached_app = Some(app.name.clone());
        self.nodes = snapshot
            .nodes
            .into_iter()
            .map(|node| Cached {
                element: node.element,
                center: node.center,
                role: node.text.split(' ').next().unwrap_or_default().to_string(),
            })
            .collect();
    }

    fn element(&self, index: i64, app: Option<&str>) -> Result<&Cached> {
        let Some(cached) = &self.cached_app else {
            bail!("no snapshot yet; call get_app_state before using element_index");
        };
        if let Some(app) = app {
            if !app.eq_ignore_ascii_case(cached) {
                bail!(
                    "the cached elements belong to `{cached}`, not `{app}`; call get_app_state for `{app}` first"
                );
            }
        }
        let index = usize::try_from(index).map_err(|_| anyhow!("element_index must be >= 0"))?;
        self.nodes.get(index).ok_or_else(|| {
            anyhow!(
                "element_index {index} is out of range; the last snapshot has {} elements — call get_app_state again",
                self.nodes.len()
            )
        })
    }

    pub fn list_apps(&self) -> Result<String> {
        let apps = apps::running_apps();
        let mut lines = Vec::new();
        for app in apps {
            let windows: Vec<&apps::Window> = app
                .windows
                .iter()
                .filter(|window| window.layer == 0)
                .collect();
            if windows.is_empty() {
                continue;
            }
            let described: Vec<String> = windows
                .iter()
                .map(|window| {
                    if window.title.trim().is_empty() {
                        format!("{:.0}x{:.0}", window.width, window.height)
                    } else {
                        format!(
                            "\"{}\" {:.0}x{:.0}",
                            window.title.trim(),
                            window.width,
                            window.height
                        )
                    }
                })
                .collect();
            lines.push(format!(
                "{} (pid {}) — {}",
                app.name,
                app.pid,
                described.join(", ")
            ));
        }
        if lines.is_empty() {
            return Ok("No app with an on-screen window is running.".to_string());
        }
        Ok(lines.join("\n"))
    }

    pub fn get_app_state(&mut self, args: &Value) -> Result<String> {
        let name = required_app(args)?;
        let app = self.resolved(&name)?;
        let element = Self::app_element(&app)?;
        let mut limits = Limits::default();
        if let Some(value) = usize_arg(args, "max_tree_nodes") {
            limits.max_nodes = value;
        }
        if let Some(value) = usize_arg(args, "max_tree_depth") {
            limits.max_depth = value;
        }
        if let Some(value) = text_limit(args) {
            limits.text_limit = value;
        }
        limits.include_frames = args
            .get("include_frames")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let snapshot = tree::snapshot(&element, &limits);
        let body = tree::render(&snapshot.nodes);
        let truncated = snapshot.truncated;
        let count = snapshot.nodes.len();
        self.store(&app, snapshot);

        let mut header = format!("App: {} (pid {})", app.name, app.pid);
        let windows: Vec<String> = apps::windows_for(app.pid)
            .into_iter()
            .filter(|window| window.width > 0.0 && window.height > 0.0)
            .map(|window| {
                format!(
                    "\"{}\" @({:.0},{:.0}) {:.0}x{:.0}",
                    window.title.trim(),
                    window.x,
                    window.y,
                    window.width,
                    window.height
                )
            })
            .collect();
        if !windows.is_empty() {
            header.push_str(&format!("\nWindows: {}", windows.join(" | ")));
        }
        header.push_str(&format!("\nElements: {count}"));
        if truncated {
            header.push_str(" (truncated; raise max_tree_nodes or max_tree_depth)");
        }
        Ok(format!("{header}\n\n{body}"))
    }

    pub fn query(&mut self, args: &Value) -> Result<String> {
        let name = required_app(args)?;
        let app = self.resolved(&name)?;
        let element = Self::app_element(&app)?;
        let mut limits = Limits::default();
        if let Some(value) = usize_arg(args, "max_tree_nodes") {
            limits.max_nodes = value;
        }
        let snapshot = tree::snapshot(&element, &limits);
        let needle = string_arg(args, "text").map(|text| text.to_lowercase());
        let role = string_arg(args, "role").map(|role| role.to_uppercase());
        let limit = usize_arg(args, "max_results").unwrap_or(20);

        let matches: Vec<(usize, String)> = snapshot
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                let text_ok = needle
                    .as_deref()
                    .map(|needle| node.text.to_lowercase().contains(needle))
                    .unwrap_or(true);
                let role_ok = role
                    .as_deref()
                    .map(|role| node.text.to_uppercase().contains(role))
                    .unwrap_or(true);
                text_ok && role_ok
            })
            .take(limit)
            .map(|(index, node)| (index, format!("{}\n", node.text)))
            .collect();

        self.store(&app, snapshot);

        if matches.is_empty() {
            return Ok(format!(
                "No element in {} matched {}{}.",
                app.name,
                needle
                    .map(|needle| format!("text `{needle}` "))
                    .unwrap_or_default(),
                role.map(|role| format!("role `{role}`"))
                    .unwrap_or_default()
            ));
        }
        let mut out = format!("{} matches in {}:", matches.len(), app.name);
        for (index, text) in matches {
            out.push_str(&format!("\n{index:>4} {text}"));
        }
        Ok(out)
    }

    pub fn click(&mut self, args: &Value) -> Result<String> {
        let name = required_app(args)?;
        let app = self.resolved(&name)?;
        let index = integer_arg(args, "element_index");
        let button = match string_arg(args, "button").as_deref() {
            Some("right") => input::Button::Right,
            _ => input::Button::Left,
        };

        if let Some(index) = index {
            let cached = self.element(index, Some(&app.name))?;
            let element = &cached.element;
            if cached.role != "AXMenu" && element.has_action("AXPress") {
                element.perform("AXPress")?;
                return Ok(format!(
                    "Clicked element {index} ({}) with AXPress.",
                    cached.role
                ));
            }
            let center = cached.center.ok_or_else(|| {
                anyhow!("element {index} has no on-screen frame; pass x and y instead")
            })?;
            let role = cached.role.clone();
            Self::app_element(&app)?.activate_app()?;
            input::click(center, button)?;
            return Ok(format!(
                "Clicked element {index} ({role}) at ({:.0},{:.0}) with a pointer event.",
                center.x, center.y
            ));
        }

        let (x, y) = match (number_arg(args, "x"), number_arg(args, "y")) {
            (Some(x), Some(y)) => (x, y),
            _ => {
                bail!("pass element_index, or x and y together")
            }
        };
        Self::app_element(&app)?.activate_app()?;
        input::click(CGPoint { x, y }, button)?;
        Ok(format!("Clicked ({x:.0},{y:.0}) with a pointer event."))
    }

    pub fn perform_action(&mut self, args: &Value) -> Result<String> {
        let name = required_app(args)?;
        let app = self.resolved(&name)?;
        let index = integer_arg(args, "element_index")
            .ok_or_else(|| anyhow!("`element_index` is required"))?;
        let action = string_arg(args, "action").unwrap_or_else(|| "AXPress".to_string());
        let cached = self.element(index, Some(&app.name))?;
        cached.element.perform(&action)?;
        Ok(format!(
            "Performed {action} on element {index} ({}).",
            cached.role
        ))
    }

    pub fn set_value(&mut self, args: &Value) -> Result<String> {
        let name = required_app(args)?;
        let app = self.resolved(&name)?;
        let index = integer_arg(args, "element_index")
            .ok_or_else(|| anyhow!("`element_index` is required"))?;
        let value = args
            .get("value")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("`value` must be a string"))?;
        let cached = self.element(index, Some(&app.name))?;
        cached.element.set_string("AXValue", value)?;
        let read_back = cached
            .element
            .string_attribute("AXValue")
            .unwrap_or_default();
        if read_back != value {
            bail!(
                "element {index} reports value {read_back:?} after writing {value:?}; the control may be read-only"
            );
        }
        Ok(format!("Set element {index} value ({:?}).", value))
    }

    pub fn type_text(&mut self, args: &Value) -> Result<String> {
        let text = args
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("`text` is required"))?;
        if let Some(name) = string_arg(args, "app") {
            let app = self.resolved(&name)?;
            if let Some(index) = integer_arg(args, "element_index") {
                let cached = self.element(index, Some(&app.name))?;
                cached.element.focus()?;
            }
            Self::app_element(&app)?.activate_app()?;
        }
        input::type_text(text)?;
        Ok(format!("Typed {} characters.", text.chars().count()))
    }

    pub fn press_key(&mut self, args: &Value) -> Result<String> {
        let key = string_arg(args, "key").ok_or_else(|| anyhow!("`key` is required"))?;
        let modifiers = string_arg(args, "modifiers").unwrap_or_default();
        if let Some(name) = string_arg(args, "app") {
            let app = self.resolved(&name)?;
            Self::app_element(&app)?.activate_app()?;
        }
        input::press(&key, &modifiers)?;
        Ok(format!("Pressed {key}."))
    }

    pub fn scroll(&mut self, args: &Value) -> Result<String> {
        let direction = string_arg(args, "direction").unwrap_or_else(|| "down".to_string());
        let lines = integer_arg(args, "lines").unwrap_or(3).abs().max(1) as i32;
        let signed = if direction.eq_ignore_ascii_case("up") {
            lines
        } else if direction.eq_ignore_ascii_case("down") {
            -lines
        } else {
            bail!("`direction` must be `up` or `down`")
        };

        let point = if let Some(index) = integer_arg(args, "element_index") {
            let cached = self.element(index, string_arg(args, "app").as_deref())?;
            cached.center
        } else {
            match (number_arg(args, "x"), number_arg(args, "y")) {
                (Some(x), Some(y)) => Some(CGPoint { x, y }),
                _ => None,
            }
        };

        if let Some(name) = string_arg(args, "app") {
            let app = self.resolved(&name)?;
            Self::app_element(&app)?.activate_app()?;
        }
        input::scroll(point, signed)?;
        Ok(format!("Scrolled {direction} {lines} lines."))
    }

    pub fn drag(&mut self, args: &Value) -> Result<String> {
        let start = match integer_arg(args, "element_index") {
            Some(index) => {
                let cached = self.element(index, string_arg(args, "app").as_deref())?;
                cached
                    .center
                    .ok_or_else(|| anyhow!("element {index} has no on-screen frame"))?
            }
            None => CGPoint {
                x: number_arg(args, "from_x")
                    .ok_or_else(|| anyhow!("pass element_index or from_x/from_y"))?,
                y: number_arg(args, "from_y")
                    .ok_or_else(|| anyhow!("pass element_index or from_x/from_y"))?,
            },
        };
        let end = CGPoint {
            x: number_arg(args, "to_x").ok_or_else(|| anyhow!("`to_x` is required"))?,
            y: number_arg(args, "to_y").ok_or_else(|| anyhow!("`to_y` is required"))?,
        };
        if let Some(name) = string_arg(args, "app") {
            let app = self.resolved(&name)?;
            Self::app_element(&app)?.activate_app()?;
        }
        input::drag(start, end)?;
        Ok(format!(
            "Dragged from ({:.0},{:.0}) to ({:.0},{:.0}).",
            start.x, start.y, end.x, end.y
        ))
    }

    pub fn screenshot(&self, args: &Value) -> Result<Vec<u8>> {
        let window =
            match string_arg(args, "app") {
                Some(name) => {
                    let app = self.resolved(&name)?;
                    let windows = apps::windows_for(app.pid);
                    let widest = windows.iter().max_by(|left, right| {
                        (left.width * left.height)
                            .partial_cmp(&(right.width * right.height))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    Some(widest.map(|window| window.number).ok_or_else(|| {
                        anyhow!("{} has no on-screen window to capture", app.name)
                    })?)
                }
                None => None,
            };
        screen::capture(window)
    }

    pub fn guard(&self) -> Result<()> {
        if !ax::trusted() {
            bail!(
                "Accessibility permission is missing: run `oxide-computer-use doctor` and approve this terminal in System Settings → Privacy & Security → Accessibility"
            );
        }
        Ok(())
    }
}
