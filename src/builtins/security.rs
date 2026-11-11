use std::sync::Arc;

use anyhow::Result;

use crate::{
    application::security::SecurityTriageService,
    core::{CommandContext, CommandOutput, ports::TerminalFactory},
    presentation::tool_screen,
};

use super::BuiltinCommand;

pub struct TriageBuiltin {
    service: Arc<SecurityTriageService>,
    terminal: Arc<dyn TerminalFactory>,
}

impl TriageBuiltin {
    pub fn new(service: Arc<SecurityTriageService>, terminal: Arc<dyn TerminalFactory>) -> Self {
        Self { service, terminal }
    }
}

impl BuiltinCommand for TriageBuiltin {
    fn name(&self) -> &'static str {
        "triage"
    }

    fn help(&self) -> &'static str {
        "triage — análisis activo del equipo, proceso concreto y correlación profunda"
    }

    fn execute(
        &self,
        _invoked_name: &str,
        args: &[String],
        context: CommandContext<'_>,
    ) -> Result<CommandOutput> {
        let output = self.service.triage(args)?;
        let informational = args.iter().any(|arg| matches!(
            arg.as_str(), "-h" | "--help" | "help" | "--status" | "--thermal"
                | "--scan" | "--refresh"
        ));
        let machine = args.iter().any(|arg| matches!(
            arg.as_str(), "--plain" | "--stdout" | "--json" | "--csv"
        ));
        let interactive = crate::adapters::terminal::io::output_sender().is_some();
        let view = args.iter().any(|arg| arg == "--view") || interactive;
        // A report is shown only when a real interactive console owns this
        // command. Piped/scripted text and machine-readable output stay raw.
        if view && !machine && !informational && context.stdin.is_none()
            && output.status == 0 && !output.stdout.trim().is_empty()
        {
            tool_screen::show_report(self.terminal.as_ref(), "TRIAGE", "triage", &output.stdout)?;
            return Ok(CommandOutput::ok(""));
        }
        Ok(output)
    }
}

pub struct IntelBuiltin {
    service: Arc<SecurityTriageService>,
}

impl IntelBuiltin {
    pub fn new(service: Arc<SecurityTriageService>) -> Self {
        Self { service }
    }
}

impl BuiltinCommand for IntelBuiltin {
    fn name(&self) -> &'static str {
        "intel"
    }

    fn help(&self) -> &'static str {
        "intel — estado, fuentes y lookup de inteligencia de seguridad"
    }

    fn execute(
        &self,
        _invoked_name: &str,
        args: &[String],
        _context: CommandContext<'_>,
    ) -> Result<CommandOutput> {
        self.service.intel(args)
    }
}
