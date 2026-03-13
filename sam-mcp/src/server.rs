use crate::loader::SamContext;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Default, Deserialize, JsonSchema)]
struct ListAliasesRequest {
    /// Filter to a specific namespace (e.g. "docker"). Optional.
    namespace: Option<String>,
    /// Case-insensitive keyword matched against alias name and description. Optional.
    keyword: Option<String>,
}

#[derive(Debug, Serialize)]
struct AliasEntry {
    name: String,
    namespace: Option<String>,
    desc: String,
    template: String,
}

#[tool_router]
impl SamMcpServer {
    #[tool(description = "List SAM aliases. Filter by namespace and/or keyword (matched against name and description).")]
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
                namespace: a.namespace().map(|s| s.to_string()),
                desc: a.desc().to_string(),
                template: a.alias().to_string(),
            })
            .collect();

        let json = serde_json::to_string(&result)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
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
