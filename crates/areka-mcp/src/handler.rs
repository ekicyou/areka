//! rmcp の `ServerHandler`（登録表 → `ToolRouter` の写し）。
//!
//! rmcp の `ServerHandler`／`ToolRouter` を綴るのはこのモジュールだけ。登録表の中身は
//! [`ArekaHandler::new`] が 1 度だけ `ToolRouter` へ写し、一覧・呼び出し・定義の取得は
//! そこへ委ねる（未登録の名前は `ToolRouter::call` が `-32602` にする＝設計 B-6）。

use std::borrow::Cow;
use std::sync::Arc;

use rmcp::handler::server::router::tool::{ToolRoute, ToolRouter};
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};

use crate::registry::{ToolContent, ToolOutcome, ToolRegistry};

/// `serverInfo.name`（版は Cargo の版）。
pub(crate) const SERVER_NAME: &str = "areka-mcp-server";

/// `initialize` の `instructions`（英文・2 文）。
pub(crate) const INSTRUCTIONS: &str = "This server controls areka, a desktop mascot (Ukagaka-compatible baseware) running on this machine. Tools are added in later releases; this build registers none, so tools/list is empty.";

/// rmcp へ渡す受け手。中身は登録表を写した `ToolRouter` だけ（複製は `Arc` の複製）。
#[derive(Clone)]
pub(crate) struct ArekaHandler {
    router: Arc<ToolRouter<ArekaHandler>>,
}

impl ArekaHandler {
    /// 登録表を `ToolRouter` へ 1 度だけ写す。
    pub(crate) fn new(registry: ToolRegistry) -> Self {
        let mut router = ToolRouter::new();
        for (spec, handler) in registry.entries() {
            let mut tool = Tool::new_with_raw(
                spec.name.clone(),
                spec.description.clone().map(Cow::Owned),
                spec.input_schema.clone(),
            );
            tool.title = spec.title.clone();
            let handler = handler.clone();
            router.add_route(ToolRoute::new_dyn(
                tool,
                move |ctx: ToolCallContext<'_, Self>| {
                    // `arguments` が無ければ `{}` を渡す（registry の約束）。
                    let pending =
                        handler(serde_json::Value::Object(ctx.arguments.unwrap_or_default()));
                    Box::pin(async move {
                        Ok(CallToolResponse::from(to_call_tool_result(pending.await)))
                    })
                },
            ));
        }
        Self {
            router: Arc::new(router),
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
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.router.list_all()))
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
