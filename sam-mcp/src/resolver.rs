use sam_core::algorithms::resolver::{ErrorsResolver, Resolver, ResolverContext};
use sam_core::entities::aliases::AliasAndDependencies;
use sam_core::entities::choices::Choice;
use sam_core::entities::vars::Var;
use sam_persistence::VarsCache;
use sam_readers::read_choices;
use sam_terminals::processes::ShellCommand;
use std::collections::HashMap;

/// MCP-aware resolver: runs dynamic commands (with caching) and returns all
/// available choices. For input vars it signals `NoInputWasProvided` so the
/// caller can convert it into a `needs_var / kind: input` response.
pub struct McpResolver<'a> {
    pub env_variables: &'a HashMap<String, String>,
    pub cache: &'a dyn VarsCache,
    pub working_dir: &'a str,
}

impl Resolver for McpResolver<'_> {
    fn resolve_input(
        &self,
        var: &Var,
        prompt: &str,
        _ctx: &ResolverContext,
    ) -> Result<Choice, ErrorsResolver> {
        Err(ErrorsResolver::NoInputWasProvided(
            var.name(),
            prompt.to_string(),
        ))
    }

    fn resolve_dynamic(
        &self,
        var: &Var,
        cmd: String,
        _ctx: &ResolverContext,
    ) -> Result<Vec<Choice>, ErrorsResolver> {
        let sh_cmd: ShellCommand<String> = cmd.into();
        let cmd_key = sh_cmd
            .replace_env_vars_in_command(self.env_variables)
            .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), Box::new(e)))?;

        let stdout_output = if let Ok(Some(out)) = self.cache.get(cmd_key.value()) {
            out.as_bytes().to_owned()
        } else {
            let mut to_run = ShellCommand::make_command(sh_cmd);
            to_run.envs(self.env_variables);
            to_run.current_dir(self.working_dir);
            let output = to_run
                .output()
                .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), Box::new(e)))?;
            if output.status.code() == Some(0) && output.stderr.is_empty() {
                self.cache
                    .put(
                        &var.name().to_string(),
                        cmd_key.value(),
                        &String::from_utf8_lossy(&output.stdout),
                    )
                    .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), Box::new(e)))?;
            }
            output.stdout
        };

        read_choices(stdout_output.as_slice())
            .map_err(|e| ErrorsResolver::DynamicResolveFailure(var.name(), Box::new(e)))
    }

    fn resolve_static(
        &self,
        var: &Var,
        choices: impl Iterator<Item = Choice>,
        _ctx: &ResolverContext,
    ) -> Result<Vec<Choice>, ErrorsResolver> {
        let v: Vec<Choice> = choices.collect();
        if v.is_empty() {
            return Err(ErrorsResolver::NoChoiceWasAvailable(var.name()));
        }
        Ok(v)
    }

    fn select_identifier(
        &self,
        _identifiers: &[AliasAndDependencies],
        _prompt: &str,
    ) -> Result<AliasAndDependencies, ErrorsResolver> {
        Err(ErrorsResolver::IdentifierSelectionEmpty())
    }
}
