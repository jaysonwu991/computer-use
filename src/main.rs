#[cfg(target_os = "macos")]
mod macos;
mod mcp;
mod runtime;
mod tools;

use std::path::PathBuf;
use std::process::Command as Process;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

use runtime::Runtime;

#[derive(Parser)]
#[command(
    name = "oxide-computer-use",
    version,
    about = "Computer Use MCP server and CLI for Oxide: drive desktop apps through native accessibility APIs"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the stdio MCP server that Oxide connects to.
    Mcp,
    /// Call one Computer Use tool and print its MCP-style JSON result.
    Call {
        /// Tool name, for example list_apps or get_app_state.
        tool: Option<String>,
        /// Tool arguments as JSON.
        #[arg(long)]
        args: Option<String>,
        /// A JSON array of {"tool", "args"} calls sharing one process and one element-index state.
        #[arg(long)]
        calls: Option<String>,
        /// Read the call array from a file instead.
        #[arg(long = "calls-file")]
        calls_file: Option<PathBuf>,
        /// Seconds to wait between calls in a sequence.
        #[arg(long, default_value_t = 1.0)]
        sleep: f64,
    },
    /// Print the tool names this build exposes.
    Tools,
    /// Check the permissions Computer Use needs.
    Doctor {
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Ask macOS to show the permission prompts.
        #[arg(long)]
        prompt: bool,
    },
    /// Register this binary as an MCP server in Oxide's config.
    InstallOxide {
        /// Write the project config (.oxide/mcp.json) instead of the user config.
        #[arg(long)]
        project: bool,
        /// Server name to register.
        #[arg(long, default_value = "computer-use")]
        name: String,
        /// Print the JSON entry instead of writing a config file.
        #[arg(long)]
        print: bool,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("oxide-computer-use: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
        Some(Command::Mcp) => mcp::serve(),
        Some(Command::Tools) => {
            for name in tools::names() {
                println!("{name}");
            }
            Ok(())
        }
        Some(Command::Call {
            tool,
            args,
            calls,
            calls_file,
            sleep,
        }) => call(tool, args, calls, calls_file, sleep),
        Some(Command::Doctor { json, prompt }) => doctor(json, prompt),
        Some(Command::InstallOxide {
            project,
            name,
            print,
        }) => install_oxide(project, &name, print),
    }
}

fn parse_args(text: Option<&str>) -> Result<Value> {
    let Some(text) = text else {
        return Ok(json!({}));
    };
    let value: Value = serde_json::from_str(text).context("arguments must be a JSON object")?;
    if !value.is_object() {
        bail!("arguments must be a JSON object");
    }
    Ok(value)
}

fn call(
    tool: Option<String>,
    args: Option<String>,
    calls: Option<String>,
    calls_file: Option<PathBuf>,
    sleep: f64,
) -> Result<()> {
    let mut runtime = Runtime::new();
    let sequence = match (calls, calls_file) {
        (Some(text), _) => Some(text),
        (None, Some(path)) => Some(
            std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?,
        ),
        (None, None) => None,
    };

    let Some(sequence) = sequence else {
        let tool = tool.context("pass a tool name, or --calls/--calls-file")?;
        let args = parse_args(args.as_deref())?;
        return report(&mut runtime, &tool, &args);
    };

    let parsed: Value = serde_json::from_str(&sequence).context("--calls must be a JSON array")?;
    let list = parsed
        .as_array()
        .context("--calls must be a JSON array of {\"tool\", \"args\"} objects")?;
    let mut failures = 0;
    for (index, entry) in list.iter().enumerate() {
        let name = entry
            .get("tool")
            .and_then(Value::as_str)
            .with_context(|| format!("call {index} is missing `tool`"))?;
        let arguments = entry.get("args").cloned().unwrap_or_else(|| json!({}));
        if report(&mut runtime, name, &arguments).is_err() {
            failures += 1;
        }
        if index + 1 < list.len() && sleep > 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f64(sleep));
        }
    }
    if failures > 0 {
        bail!("{failures} of {} calls failed", list.len());
    }
    Ok(())
}

fn report(runtime: &mut Runtime, tool: &str, args: &Value) -> Result<()> {
    match tools::execute(runtime, tool, args) {
        Ok(outcome) => {
            println!("{}", serde_json::to_string(&tools::content_json(&outcome))?);
            Ok(())
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "content": [{ "type": "text", "text": format!("{error:#}") }],
                    "isError": true
                }))?
            );
            bail!("{error:#}")
        }
    }
}

fn doctor(as_json: bool, prompt: bool) -> Result<()> {
    let version = macos_version();
    let accessibility = accessibility(prompt);
    let screen_recording = screen_recording(prompt);

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "version": env!("CARGO_PKG_VERSION"),
                "macos": version,
                "accessibility": accessibility,
                "screen_recording": screen_recording,
            }))?
        );
    } else {
        println!("oxide-computer-use {}", env!("CARGO_PKG_VERSION"));
        println!(
            "macOS: {}",
            version.clone().unwrap_or_else(|| "unknown".to_string())
        );
        println!(
            "Accessibility: {}",
            if accessibility { "granted" } else { "missing" }
        );
        println!(
            "Screen Recording: {}",
            if screen_recording {
                "granted"
            } else {
                "missing"
            }
        );
        if !accessibility || !screen_recording {
            println!(
                "\nGrant both to the terminal app that runs this server, in System Settings → Privacy & Security."
            );
        }
    }

    if accessibility && screen_recording {
        Ok(())
    } else {
        bail!("permissions are missing; Computer Use cannot read or drive apps until both are granted")
    }
}

#[cfg(target_os = "macos")]
fn macos_version() -> Option<String> {
    let output = Process::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(not(target_os = "macos"))]
fn macos_version() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn accessibility(prompt: bool) -> bool {
    if prompt {
        macos::ax::request_trust(true)
    } else {
        macos::ax::trusted()
    }
}

#[cfg(not(target_os = "macos"))]
fn accessibility(_prompt: bool) -> bool {
    false
}

#[cfg(target_os = "macos")]
fn screen_recording(prompt: bool) -> bool {
    if prompt {
        macos::apps::request_screen_recording()
    } else {
        macos::apps::screen_recording_allowed()
    }
}

#[cfg(not(target_os = "macos"))]
fn screen_recording(_prompt: bool) -> bool {
    false
}

fn install_oxide(project: bool, name: &str, print: bool) -> Result<()> {
    let executable = std::env::current_exe().context("resolving this executable")?;
    let entry = json!({
        "command": executable.to_string_lossy(),
        "args": ["mcp"]
    });

    if print {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "mcpServers": { name: entry } }))?
        );
        return Ok(());
    }

    let path = if project {
        std::env::current_dir()?.join(".oxide").join("mcp.json")
    } else {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        PathBuf::from(home).join(".oxide").join("mcp.json")
    };

    let mut root: Value = if path.exists() {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        if text.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?
        }
    } else {
        json!({})
    };

    if !root.is_object() {
        root = json!({});
    }
    let object = root.as_object_mut().expect("object");
    if !object.get("mcpServers").is_some_and(Value::is_object) {
        object.insert("mcpServers".to_string(), json!({}));
    }
    object
        .get_mut("mcpServers")
        .and_then(Value::as_object_mut)
        .expect("mcpServers object")
        .insert(name.to_string(), entry);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&root)?))
        .with_context(|| format!("writing {}", path.display()))?;
    println!("Registered `{name}` in {}", path.display());
    println!("Run `oxide mcp list` to confirm it is visible.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_arguments_default_to_an_empty_object() {
        assert_eq!(parse_args(None).unwrap(), json!({}));
    }

    #[test]
    fn arguments_must_be_a_json_object() {
        assert!(parse_args(Some("{\"app\":\"Finder\"}")).is_ok());
        assert!(parse_args(Some("[1,2]")).is_err());
        assert!(parse_args(Some("not json")).is_err());
    }
}
