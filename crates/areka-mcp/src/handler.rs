//! rmcp の `ServerHandler`（登録表 → `ToolRouter` の写し）。
//!
//! rmcp の `ServerHandler`／`ToolRouter` を綴るのはこのモジュールだけ。登録表の中身は
//! [`ArekaHandler::new`] が 1 度だけ `ToolRouter` へ写し、呼び出し・定義の取得はそこへ委ねる
//! （未登録の名前は `ToolRouter::call` が `-32602` にする＝設計 B-6）。一覧は `ToolRouter` が
//! 名前順に並べ替えるので、写すときに登録順で積んだ列を返す。写しの中で `arguments` を
//! 登録した `inputSchema` に照らし、合わなければ `-32602` にする。

use std::borrow::Cow;
use std::sync::Arc;

use rmcp::handler::server::router::tool::{ToolRoute, ToolRouter};
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
    Implementation, ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities,
    ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use tracing::debug;

use crate::check::check_arguments;
use crate::registry::{ToolContent, ToolOutcome, ToolRegistry};

/// `serverInfo.name`（版は Cargo の版）。
pub(crate) const SERVER_NAME: &str = "areka-mcp-server";

/// `initialize` の `instructions`（英文・3 文）。
pub(crate) const INSTRUCTIONS: &str = "This server controls areka, a desktop mascot (Ukagaka-compatible baseware) running on this machine. It exposes the same tools as the MCP server of SSP; call get_active_ghost_list first to get the ghost name for the ghost_name parameter. A tool that is not implemented yet returns a result starting with \"NG:not implemented yet\".";

/// rmcp へ渡す受け手。中身は登録表を写した `ToolRouter` と登録順の定義の列（複製は `Arc` の複製）。
#[derive(Clone)]
pub(crate) struct ArekaHandler {
    router: Arc<ToolRouter<ArekaHandler>>,
    tools: Arc<Vec<Tool>>,
}

impl ArekaHandler {
    /// 登録表を `ToolRouter` へ 1 度だけ写す。
    pub(crate) fn new(registry: ToolRegistry) -> Self {
        let mut router = ToolRouter::new();
        let mut tools = Vec::with_capacity(registry.len());
        for (spec, handler) in registry.entries() {
            let mut tool = Tool::new_with_raw(
                spec.name.clone(),
                spec.description.clone().map(Cow::Owned),
                spec.input_schema.clone(),
            );
            tool.title = spec.title.clone();
            tools.push(tool.clone());
            let handler = handler.clone();
            let name = spec.name.clone();
            let schema = spec.input_schema.clone();
            router.add_route(ToolRoute::new_dyn(
                tool,
                move |ctx: ToolCallContext<'_, Self>| {
                    // `arguments` が無ければ `{}` として検査し、そのまま渡す（registry の約束）。
                    let args = ctx.arguments.unwrap_or_default();
                    if let Err(reason) = check_arguments(&schema, &args) {
                        debug!(tool = %name, reason = %reason, "MCP: 引数が inputSchema に合わない");
                        return Box::pin(std::future::ready(Err(ErrorData::invalid_params(
                            reason, None,
                        ))));
                    }
                    let pending = handler(serde_json::Value::Object(args));
                    Box::pin(async move {
                        Ok(CallToolResponse::from(to_call_tool_result(pending.await)))
                    })
                },
            ));
        }
        Self {
            router: Arc::new(router),
            tools: Arc::new(tools),
        }
    }
}

/// `ToolOutcome` を `CallToolResult` へそのまま写す（content の順と is_error）。
fn to_call_tool_result(outcome: ToolOutcome) -> CallToolResult {
    let content = outcome
        .content
        .into_iter()
        .map(|c| match c {
            ToolContent::Text(text) => ContentBlock::text(text),
            ToolContent::Image { data, mime_type } => ContentBlock::image(data, mime_type),
        })
        .collect();
    if outcome.is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    }
}

impl ServerHandler for ArekaHandler {
    /// tools の能力だけ（resources・prompts なし）。版の交渉は rmcp の既定に任せる（設計 B-11）。
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        // `ToolRouter::list_all` は名前順に並べ替えるので、登録順の列を返す。
        let result = ListToolsResult::with_all_items(self.tools.as_ref().clone());
        // 2026-07-28 以降は `ttlMs`・`cacheScope` が必須（rmcp は欄を空のまま出す＝設計 B-12）。
        // 無状態では list_changed を送れず、後の spec で道具が増えるので 0／private
        // （`server/discover` の rmcp の既定と同じ）。旧式の版には付けない。
        if context
            .protocol_version()
            .is_some_and(|v| v >= ProtocolVersion::V_2026_07_28)
        {
            return Ok(result.with_ttl_ms(0).with_cache_scope(CacheScope::Private));
        }
        Ok(result)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        self.router
            .call(ToolCallContext::new(self, request, context))
            .await
    }

    /// rmcp が `Mcp-Param-*` ヘッダの検査に使う。
    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.router.get(name).cloned()
    }
}
