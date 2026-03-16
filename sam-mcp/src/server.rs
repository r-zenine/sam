use crate::loader::SamContext;
use crate::resolver::McpResolver;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use sam_core::algorithms::resolver::{ErrorsResolver, ResolverContext};
use sam_core::algorithms::{
    choice_for_var, execution_sequence_for_dependencies, ErrorDependencyResolution, VarsCollection,
};
use sam_core::engines::AliasCollection;
use sam_core::entities::choices::Choice;
use sam_core::entities::identifiers::Identifier;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SamMcpServer {
    ctx: Arc<tokio::sync::RwLock<SamContext>>,
    #[allow(dead_code)]
    config_path: Option<std::path::PathBuf>,
    tool_router: ToolRouter<Self>,
}

impl SamMcpServer {
    pub fn new(ctx: SamContext, config_path: Option<std::path::PathBuf>) -> Self {
        Self {
            ctx: Arc::new(tokio::sync::RwLock::new(ctx)),
            config_path,
            tool_router: Self::tool_router(),
        }
    }
}

// ── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct ListAliasesRequest {
    /// Filter to a specific namespace (e.g. "docker"). Optional.
    namespace: Option<String>,
    /// Case-insensitive keyword matched against alias name and description. Optional.
    keyword: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct ResolveAliasRequest {
    /// Alias identifier in "namespace::name" or "name" format.
    alias: String,
    /// Variable values collected so far. Keys are "namespace::var" or "var"; values are the chosen string.
    #[serde(default)]
    vars: HashMap<String, String>,
    /// Working directory for running `from_command` variables (e.g. "/home/user/myproject").
    /// Always pass the absolute path of the relevant project directory.
    working_dir: String,
}

// ── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AliasEntry {
    name: String,
    namespace: Option<String>,
    desc: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ResolveAliasResponse {
    NeedsVar {
        #[serde(rename = "var")]
        var: VarInfo,
    },
    Resolved {
        commands: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
struct VarInfo {
    name: String,
    namespace: Option<String>,
    desc: String,
    kind: VarKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    choices: Option<Vec<ChoiceInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum VarKind {
    Choices,
    Input,
}

#[derive(Debug, Serialize)]
struct ChoiceInfo {
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    desc: Option<String>,
}

impl From<&Choice> for ChoiceInfo {
    fn from(c: &Choice) -> Self {
        ChoiceInfo {
            value: c.value().to_string(),
            desc: c.desc().map(str::to_owned),
        }
    }
}

// Var.desc is private with no getter; extract it via its Serialize impl.
fn var_desc(var: &sam_core::entities::vars::Var) -> String {
    serde_json::to_value(var)
        .ok()
        .and_then(|v| v.get("desc").and_then(|d| d.as_str()).map(str::to_owned))
        .unwrap_or_default()
}

fn serialize<T: Serialize>(v: &T) -> Result<String, ErrorData> {
    serde_json::to_string(v).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

/// Fuzzy-filter and rank aliases by `keyword` (fzf/skim style).
/// Matches against "namespace::name desc" so partial queries work across both fields.
/// Returns only matching aliases, sorted best-first.
fn fuzzy_filter<'a>(
    aliases: impl IntoIterator<Item = &'a sam_core::entities::aliases::Alias>,
    keyword: &str,
) -> Vec<&'a sam_core::entities::aliases::Alias> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &sam_core::entities::aliases::Alias)> = aliases
        .into_iter()
        .filter_map(|a| {
            let haystack = format!("{} {}", a.full_name(), a.desc());
            matcher.fuzzy_match(&haystack, keyword).map(|s| (s, a))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, a)| a).collect()
}

// ── Tool implementations ─────────────────────────────────────────────────────

#[tool_router]
impl SamMcpServer {
    #[tool(
        description = "List all SAM aliases, optionally filtered by namespace and/or keyword. \
Use this first to discover available aliases before calling resolve_alias. \
The keyword uses fuzzy matching (fzf-style): characters must appear in order but need not \
be contiguous, and results are ranked by match quality. \
Example: list_aliases() → all aliases; list_aliases(namespace=\"docker\") → docker aliases only; \
list_aliases(keyword=\"dkr run\") → fuzzy-matches name and description."
    )]
    async fn list_aliases(
        &self,
        Parameters(req): Parameters<ListAliasesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tracing::debug!(ns = ?req.namespace, keyword = ?req.keyword, "list_aliases called");

        #[cfg(not(test))]
        {
            let reload_result = match &self.config_path {
                Some(p) => crate::loader::load_from(p.clone()),
                None => crate::loader::load(),
            };
            match reload_result {
                Ok(new_ctx) => {
                    *self.ctx.write().await = new_ctx;
                    tracing::debug!("sam context reloaded");
                }
                Err(e) => tracing::warn!(error = %e, "context reload failed, using stale data"),
            }
        }

        let ctx = self.ctx.read().await;
        let mut aliases = ctx.aliases.aliases();

        if let Some(ns) = &req.namespace {
            aliases.retain(|a| a.namespace() == Some(ns.as_str()));
        }

        let filtered: Vec<_> = match &req.keyword {
            Some(kw) => fuzzy_filter(aliases.iter(), kw),
            None => aliases.iter().collect(),
        };

        let result: Vec<AliasEntry> = filtered
            .into_iter()
            .map(|a| AliasEntry {
                name: a.name().to_string(),
                namespace: a.namespace().map(str::to_owned),
                desc: a.desc().to_string(),
            })
            .collect();

        tracing::info!(count = result.len(), "list_aliases ok");
        Ok(CallToolResult::success(vec![Content::text(serialize(&result)?)]))
    }

    #[tool(description = "Resolve a SAM alias to a runnable shell command. \
Call this in a loop: each call either returns the next variable that needs a value \
(status=\"needs_var\") or the final resolved command (status=\"resolved\"). \
Pass all previously chosen variable values in the `vars` map. \
For aliases with `from_command` variables (dynamic choices that depend on the project), \
pass `working_dir` with the absolute path to the relevant project directory. \
Example: resolve_alias(alias=\"docker::run\", vars={}) \
→ {status:\"needs_var\", var:{name:\"image\", kind:\"choices\", choices:[{\"value\":\"nginx\"}]}} \
then: resolve_alias(alias=\"docker::run\", vars={\"docker::image\":\"nginx\"}) \
→ {status:\"resolved\", commands:[\"docker run nginx\"]}")]
    async fn resolve_alias(
        &self,
        Parameters(req): Parameters<ResolveAliasRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tracing::debug!(alias = %req.alias, vars_count = req.vars.len(), "resolve_alias called");

        let ctx = self.ctx.read().await;
        let id = Identifier::from_str(&req.alias);
        let alias = ctx
            .aliases
            .get(&id)
            .ok_or_else(|| {
                let all = ctx.aliases.aliases();
                let similar: Vec<String> = fuzzy_filter(all.iter(), &req.alias)
                    .into_iter()
                    .take(5)
                    .map(|a| a.full_name().to_string())
                    .collect();
                if similar.is_empty() {
                    tracing::error!(alias = %req.alias, "alias not found");
                    ErrorData::internal_error(
                        format!(
                            "alias '{}' not found. Use list_aliases() to see available aliases.",
                            req.alias
                        ),
                        None,
                    )
                } else {
                    tracing::error!(alias = %req.alias, suggestions = %similar.join(", "), "alias not found");
                    ErrorData::internal_error(
                        format!(
                            "alias '{}' not found. Did you mean: {}?",
                            req.alias,
                            similar.join(", ")
                        ),
                        None,
                    )
                }
            })?
            .clone();

        let exec_seq = execution_sequence_for_dependencies(&ctx.vars, alias.clone())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let choices: HashMap<Identifier, Vec<Choice>> = req
            .vars
            .iter()
            .map(|(k, v)| (Identifier::from_str(k), vec![Choice::from_value(v)]))
            .collect();

        let resolver = McpResolver {
            env_variables: &ctx.env_variables,
            cache: &*ctx.cache,
            working_dir: &req.working_dir,
        };

        let resolver_ctx = ResolverContext {
            alias: alias.clone(),
            full_name: alias.full_name().to_string(),
            choices: choices.clone(),
            execution_sequence: exec_seq.identifiers(),
        };

        for var_id in exec_seq.as_slice() {
            if choices.contains_key(var_id) {
                continue;
            }
            let var = ctx.vars.get(var_id).ok_or_else(|| {
                tracing::error!(var_id = %var_id, "var not found");
                ErrorData::internal_error(format!("var '{var_id}' not found"), None)
            })?;

            let var_name = var.name();
            let base = (
                var_name.name().to_string(),
                var_name.namespace.clone(),
                var_desc(var),
            );

            let response = match choice_for_var(&resolver, var, &choices, &resolver_ctx) {
                Ok(found_choices) => ResolveAliasResponse::NeedsVar {
                    var: VarInfo {
                        name: base.0,
                        namespace: base.1,
                        desc: base.2,
                        kind: VarKind::Choices,
                        choices: Some(found_choices.iter().map(ChoiceInfo::from).collect()),
                        prompt: None,
                    },
                },
                Err(ErrorDependencyResolution::NoChoiceForVar {
                    error: ref boxed_error,
                    ..
                }) if matches!(**boxed_error, ErrorsResolver::NoInputWasProvided(_, _)) => {
                    if let ErrorsResolver::NoInputWasProvided(_, ref prompt) = **boxed_error {
                        ResolveAliasResponse::NeedsVar {
                            var: VarInfo {
                                name: base.0,
                                namespace: base.1,
                                desc: base.2,
                                kind: VarKind::Input,
                                choices: None,
                                prompt: Some(prompt.clone()),
                            },
                        }
                    } else {
                        unreachable!()
                    }
                }
                Err(e) => return Err(ErrorData::internal_error(e.to_string(), None)),
            };

            if let ResolveAliasResponse::NeedsVar { var: ref v } = response {
                tracing::debug!(var_name = %v.name, kind = ?v.kind, "resolve_alias needs_var");
            }
            return Ok(CallToolResult::success(vec![Content::text(serialize(
                &response,
            )?)]));
        }

        let resolved_alias = alias
            .with_choices(&choices)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        tracing::info!(alias = %req.alias, commands_count = resolved_alias.commands().len(), "resolve_alias resolved");
        let response = ResolveAliasResponse::Resolved {
            commands: resolved_alias.commands().to_vec(),
        };
        Ok(CallToolResult::success(vec![Content::text(serialize(
            &response,
        )?)]))
    }
}

#[tool_handler]
impl ServerHandler for SamMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("sam-mcp", env!("CARGO_PKG_VERSION")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::SamContext;
    use rmcp::model::RawContent;
    use sam_core::entities::aliases::Alias;
    use sam_core::entities::choices::Choice;
    use sam_core::entities::namespaces::NamespaceUpdater;
    use sam_core::entities::vars::Var;
    use sam_persistence::repositories::{AliasesRepository, VarsRepository};
    use sam_persistence::NoopVarsCache;
    use std::collections::HashMap;

    fn make_test_ctx() -> SamContext {
        // docker::run — one static var: docker::image
        let mut alias_run = Alias::new("run", "Run a container", "docker run {{ docker::image }}");
        NamespaceUpdater::update(&mut alias_run, "docker");

        // git::push — no vars
        let mut alias_push = Alias::new("push", "Push to remote", "git push");
        NamespaceUpdater::update(&mut alias_push, "git");

        let aliases = AliasesRepository::new(vec![alias_run, alias_push].into_iter()).unwrap();

        let mut var_image = Var::new(
            "image",
            "Docker image",
            vec![
                Choice::new("nginx", None::<&str>),
                Choice::new("redis", None::<&str>),
            ],
        );
        NamespaceUpdater::update(&mut var_image, "docker");

        let vars = VarsRepository::new(vec![var_image].into_iter());

        SamContext {
            aliases,
            vars,
            cache: Box::new(NoopVarsCache {}),
            env_variables: HashMap::new(),
        }
    }

    fn text_from(result: &CallToolResult) -> &str {
        if let RawContent::Text(t) = &result.content[0].raw {
            &t.text
        } else {
            panic!("expected text content")
        }
    }

    #[tokio::test]
    async fn list_aliases_no_filter_returns_all() {
        let server = SamMcpServer::new(make_test_ctx(), None);
        let result = server
            .list_aliases(Parameters(ListAliasesRequest::default()))
            .await
            .unwrap();
        let entries: serde_json::Value = serde_json::from_str(text_from(&result)).unwrap();
        assert_eq!(entries.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn list_aliases_filtering() {
        let server = SamMcpServer::new(make_test_ctx(), None);

        let result = server
            .list_aliases(Parameters(ListAliasesRequest {
                namespace: Some("docker".into()),
                keyword: None,
            }))
            .await
            .unwrap();
        let entries: serde_json::Value = serde_json::from_str(text_from(&result)).unwrap();
        assert_eq!(entries.as_array().unwrap().len(), 1);

        let result = server
            .list_aliases(Parameters(ListAliasesRequest {
                namespace: None,
                keyword: Some("zzznomatch".into()),
            }))
            .await
            .unwrap();
        let entries: serde_json::Value = serde_json::from_str(text_from(&result)).unwrap();
        assert_eq!(entries.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn resolve_alias_needs_var() {
        let server = SamMcpServer::new(make_test_ctx(), None);
        let result = server
            .resolve_alias(Parameters(ResolveAliasRequest {
                alias: "docker::run".into(),
                vars: HashMap::new(),
                working_dir: ".".into(),
            }))
            .await
            .unwrap();
        let resp: serde_json::Value = serde_json::from_str(text_from(&result)).unwrap();
        assert_eq!(resp["status"], "needs_var");
        assert_eq!(resp["var"]["kind"], "choices");
        assert!(!resp["var"]["choices"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn resolve_alias_resolved() {
        let server = SamMcpServer::new(make_test_ctx(), None);
        let mut vars = HashMap::new();
        vars.insert("docker::image".to_string(), "nginx".to_string());
        let result = server
            .resolve_alias(Parameters(ResolveAliasRequest {
                alias: "docker::run".into(),
                vars,
                working_dir: ".".into(),
            }))
            .await
            .unwrap();
        let resp: serde_json::Value = serde_json::from_str(text_from(&result)).unwrap();
        assert_eq!(resp["status"], "resolved");
        assert!(!resp["commands"].as_array().unwrap().is_empty());
    }
}
