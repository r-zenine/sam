use crate::loader::SamContext;
use crate::resolver::McpResolver;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use sam_core::algorithms::{
    choice_for_var, execution_sequence_for_dependencies, ErrorDependencyResolution, VarsCollection,
};
use sam_core::algorithms::resolver::{ErrorsResolver, ResolverContext};
use sam_core::engines::AliasCollection;
use sam_core::entities::choices::Choice;
use sam_core::entities::identifiers::Identifier;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub struct SamMcpServer {
    pub ctx: Arc<SamContext>,
    tool_router: ToolRouter<Self>,
}

impl SamMcpServer {
    pub fn new(ctx: Arc<SamContext>) -> Self {
        Self {
            ctx,
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
}

// ── Response types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AliasEntry {
    name: String,
    namespace: Option<String>,
    desc: String,
    template: String,
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

// ── Tool implementations ─────────────────────────────────────────────────────

#[tool_router]
impl SamMcpServer {
    #[tool(description = "List all SAM aliases, optionally filtered by namespace and/or keyword. \
Use this first to discover available aliases before calling resolve_alias. \
Example: list_aliases() → all aliases; list_aliases(namespace=\"docker\") → docker aliases only; \
list_aliases(keyword=\"run\") → aliases whose name or description contains \"run\".")]
    async fn list_aliases(
        &self,
        Parameters(req): Parameters<ListAliasesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut aliases = self.ctx.aliases.aliases();

        if let Some(ns) = &req.namespace {
            aliases.retain(|a| a.namespace() == Some(ns.as_str()));
        }
        if let Some(kw) = &req.keyword {
            let kw_lower = kw.to_lowercase();
            aliases.retain(|a| {
                a.name().to_lowercase().contains(&kw_lower)
                    || a.desc().to_lowercase().contains(&kw_lower)
            });
        }

        let result: Vec<AliasEntry> = aliases
            .iter()
            .map(|a| AliasEntry {
                name: a.name().to_string(),
                namespace: a.namespace().map(str::to_owned),
                desc: a.desc().to_string(),
                template: a.alias().to_string(),
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::text(serialize(&result)?)]))
    }

    #[tool(description = "Resolve a SAM alias to a runnable shell command. \
Call this in a loop: each call either returns the next variable that needs a value \
(status=\"needs_var\") or the final resolved command (status=\"resolved\"). \
Pass all previously chosen variable values in the `vars` map. \
Example: resolve_alias(alias=\"docker::run\", vars={}) \
→ {status:\"needs_var\", var:{name:\"image\", kind:\"choices\", choices:[{\"value\":\"nginx\"}]}} \
then: resolve_alias(alias=\"docker::run\", vars={\"docker::image\":\"nginx\"}) \
→ {status:\"resolved\", commands:[\"docker run nginx\"]}")]
    async fn resolve_alias(
        &self,
        Parameters(req): Parameters<ResolveAliasRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let id = Identifier::from_str(&req.alias);
        let alias = self
            .ctx
            .aliases
            .get(&id)
            .ok_or_else(|| {
                let needle = req.alias.to_lowercase();
                let similar: Vec<String> = self
                    .ctx
                    .aliases
                    .aliases()
                    .into_iter()
                    .filter(|a| {
                        let full = a.full_name().to_lowercase();
                        full.contains(&needle) || needle.contains(full.as_str())
                    })
                    .map(|a| a.full_name().to_string())
                    .take(5)
                    .collect();
                if similar.is_empty() {
                    ErrorData::internal_error(
                        format!(
                            "alias '{}' not found. Use list_aliases() to see available aliases.",
                            req.alias
                        ),
                        None,
                    )
                } else {
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

        let exec_seq = execution_sequence_for_dependencies(&self.ctx.vars, alias.clone())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let choices: HashMap<Identifier, Vec<Choice>> = req
            .vars
            .iter()
            .map(|(k, v)| (Identifier::from_str(k), vec![Choice::from_value(v)]))
            .collect();

        let resolver = McpResolver {
            env_variables: &self.ctx.env_variables,
            cache: &*self.ctx.cache,
        };

        let ctx = ResolverContext {
            alias: alias.clone(),
            full_name: alias.full_name().to_string(),
            choices: choices.clone(),
            execution_sequence: exec_seq.identifiers(),
        };

        for var_id in exec_seq.as_slice() {
            if choices.contains_key(var_id) {
                continue;
            }
            let var = self
                .ctx
                .vars
                .get(var_id)
                .ok_or_else(|| {
                    ErrorData::internal_error(format!("var '{var_id}' not found"), None)
                })?;

            let var_name = var.name();
            let base = (
                var_name.name().to_string(),
                var_name.namespace.clone(),
                var_desc(var),
            );

            let response = match choice_for_var(&resolver, var, &choices, &ctx) {
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
                    error: ErrorsResolver::NoInputWasProvided(_, ref prompt),
                    ..
                }) => ResolveAliasResponse::NeedsVar {
                    var: VarInfo {
                        name: base.0,
                        namespace: base.1,
                        desc: base.2,
                        kind: VarKind::Input,
                        choices: None,
                        prompt: Some(prompt.clone()),
                    },
                },
                Err(e) => return Err(ErrorData::internal_error(e.to_string(), None)),
            };

            return Ok(CallToolResult::success(vec![Content::text(serialize(&response)?)]));
        }

        let resolved_alias = alias
            .with_choices(&choices)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let response = ResolveAliasResponse::Resolved {
            commands: resolved_alias.commands().to_vec(),
        };
        Ok(CallToolResult::success(vec![Content::text(serialize(&response)?)]))
    }
}

#[tool_handler]
impl ServerHandler for SamMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = Implementation::new("sam-mcp", env!("CARGO_PKG_VERSION"));
        info
    }
}
