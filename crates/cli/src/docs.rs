//! Keeps `site/cli.json` (the website's CLI reference) in sync with the clap definition.
//! Regenerate with `UPDATE_SITE=1 cargo test -p dresume-cli`.
use clap::{Arg, ArgAction, Command, CommandFactory};
use serde_json::{Value, json};

fn arg(a: &Arg) -> Value {
    json!({
        "name": a.get_id().as_str(),
        "long": a.get_long(),
        "short": a.get_short().map(String::from),
        "positional": a.is_positional(),
        "required": a.is_required_set(),
        "multiple": matches!(a.get_action(), ArgAction::Append),
        "flag": matches!(a.get_action(), ArgAction::SetTrue),
        "help": a.get_help().map(|h| h.to_string()),
        "default": a.get_default_values().first().map(|v| v.to_string_lossy().into_owned()),
    })
}

fn args(cmd: &Command, global: bool) -> Vec<Value> {
    cmd.get_arguments()
        .filter(|a| a.is_global_set() == global && !matches!(a.get_action(), ArgAction::Help | ArgAction::Version))
        .map(arg)
        .collect()
}

fn describe() -> Value {
    let cli = super::Cli::command();
    json!({
        "about": cli.get_about().map(|s| s.to_string()),
        "global": args(&cli, true),
        "commands": cli.get_subcommands().filter(|c| !c.is_hide_set()).map(|c| json!({
            "name": c.get_name(),
            "about": c.get_about().map(|s| s.to_string()),
            "args": args(c, false),
        })).collect::<Vec<_>>(),
    })
}

#[test]
fn site_cli_reference_is_current() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../site/cli.json");
    let fresh = serde_json::to_string_pretty(&describe()).unwrap() + "\n";
    if std::env::var_os("UPDATE_SITE").is_some() {
        std::fs::write(path, &fresh).unwrap();
        return;
    }
    let current = std::fs::read_to_string(path).unwrap_or_default().replace("\r\n", "\n");
    assert!(current == fresh, "site/cli.json is stale: run `UPDATE_SITE=1 cargo test -p dresume-cli`");
}
