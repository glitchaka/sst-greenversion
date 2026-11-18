use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use anyhow::Result;

use crate::core::{CommandContext, CommandOutput};

use super::BuiltinCommand;

pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn BuiltinCommand>>,
    primary_names: BTreeSet<String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            primary_names: BTreeSet::new(),
        }
    }

    pub fn register(&mut self, command: Arc<dyn BuiltinCommand>) -> Result<()> {
        let primary = command.name().to_owned();
        self.insert_name(&primary, Arc::clone(&command))?;
        if !command.hidden() {
            self.primary_names.insert(primary);
        }

        for alias in command.aliases() {
            self.insert_name(alias, Arc::clone(&command))?;
        }

        Ok(())
    }

    pub fn names(&self) -> Vec<String> {
        self.commands
            .iter()
            .filter_map(|(name, command)| {
                (!command.hidden() || name.as_str() != command.name()).then(|| name.clone())
            })
            .collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn execute(
        &self,
        name: &str,
        args: &[String],
        context: CommandContext<'_>,
    ) -> Result<CommandOutput> {
        let Some(command) = self.commands.get(name) else {
            return Ok(CommandOutput::error(
                format!("comando interno desconocido: {name}"),
                127,
            ));
        };

        let mut output = command.execute(name, args, context)?;
        // This presentation layer is shared by every builtin. It is never
        // applied to pipelines, scripts or machine-readable protocols.
        if should_style_stdout(
            name, args, context.stdin.is_some(),
            crate::adapters::terminal::io::output_sender().is_some(),
            output.status, output.stdout.len(),
        ) {
            output.stdout = crate::presentation::tool_screen::style_plain_output(&output.stdout);
        }
        Ok(output)
    }

    pub fn help(&self, topic: Option<&str>) -> CommandOutput {
        if let Some(topic) = topic {
            let Some(command) = self.commands.get(topic) else {
                return CommandOutput::error(format!("help: tema desconocido: {topic}"), 1);
            };

            let mut help = command.help().to_owned();
            if !help.ends_with('\n') {
                help.push('\n');
            }
            return CommandOutput::ok(help);
        }

        let mut out = String::from(
            "Shell Shock Tool\n\nConsola con comandos administrativos nativos.\nEditor de scripts: helix archivo.sh. Primeros pasos: help helix.\nGuía interactiva de comandos: guide.\n\nComandos:\n",
        );

        for name in &self.primary_names {
            if let Some(command) = self.commands.get(name) {
                let first_line = command.help().lines().next().unwrap_or("");
                out.push_str(&format!("  {:<16} {}\n", name, first_line));
            }
        }

        out.push_str("\nUsa 'guide' para explorar comandos o 'guide COMANDO' para abrir directamente su ficha.\n");
        CommandOutput::ok(out)
    }

    fn insert_name(&mut self, name: &str, command: Arc<dyn BuiltinCommand>) -> Result<()> {
        if self.commands.contains_key(name) {
            anyhow::bail!("builtin duplicado: {name}");
        }

        self.commands.insert(name.to_owned(), command);
        Ok(())
    }
}

// Pure policy protects machine-readable formats and scripted input from
// terminal escape codes. Applied once, at the shared builtin dispatch boundary.
fn should_style_stdout(
    name: &str,
    args: &[String],
    piped_input: bool,
    interactive: bool,
    status: i32,
    output_len: usize,
) -> bool {
    interactive && !piped_input && status == 0
        && (1..=256 * 1024).contains(&output_len)
        && !matches!(name, "guide" | "helix" | "sigsour" | "nwash" | "vim")
        && !args.iter().any(|arg| matches!(
            arg.as_str(),
            "--csv" | "--json" | "--xml" | "--plain" | "--raw"
                | "--stdout" | "--quiet" | "-q" | "--no-color"
        ))
}

#[cfg(test)]
mod green_output_policy_tests {
    use super::should_style_stdout;

    #[test]
    fn cli_data_and_automations_are_never_colorized() {
        let text = |words: &[&str]| words.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert!(should_style_stdout("notsh", &text(&["interfaces"]), false, true, 0, 220));
        assert!(should_style_stdout("sys", &[], false, true, 0, 80));
        assert!(!should_style_stdout("notsh", &text(&["interfaces", "--csv"]), false, true, 0, 200));
        assert!(!should_style_stdout("triage", &text(&["--json"]), false, true, 0, 200));
        assert!(!should_style_stdout("sys", &[], true, true, 0, 200));
        assert!(!should_style_stdout("sys", &[], false, false, 0, 200));
        assert!(!should_style_stdout("guide", &[], false, true, 0, 200));
        assert!(!should_style_stdout("instp", &[], false, true, 1, 200));
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
