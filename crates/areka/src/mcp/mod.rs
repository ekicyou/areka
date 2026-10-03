//! MCP のツールの要求を UI スレッドで汲んで振り分ける口（spec: areka-P0-mcp-tool-entrances）。
//!
//! 受け口（`McpInbox`）・据え付け・系の登録・終了の途中の始末・汲む系・振り分けを置く。
//! 各ツールの処理はツールごとのファイルに持ち、テストの接続もそのファイル自身に書く。

mod resolve;

mod dump_balloon;
mod dump_surface;
mod get_active_ghost_list;
mod get_expression_table;
mod get_log;
mod get_property;
mod get_status;
mod raise_event;
mod reload;
mod sakurascript;

use std::sync::mpsc::Receiver;

use areka_mcp::ToolOutcome;
use areka_mcp::tools::{ReplyTo, ToolCall, ToolRequest, outcome};
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use resolve::{ActiveGhost, Omitted};

/// MCP の要求の受け口（World の NonSend・プロセスに 1 つ）。
pub(crate) struct McpInbox(Receiver<ToolRequest>);

/// 後から答える組を覗く関数。`Some` を返したらその結果を送って組を外す。
type Poll = Box<dyn FnMut(&mut World) -> Option<ToolOutcome>>;

/// 後から答える置き場（World の NonSend・`install` が受け口と一緒に置く）。
pub(crate) struct McpLater(Vec<(Poll, ReplyTo)>);

/// 受け口と後から答える置き場を World に置く（`fn main()` が `register_systems` の後に 1 度呼ぶ）。
/// 置く前に届いた要求は受け口に溜まっていて、最初の汲みで答えられる（要件 6.5）。
pub(crate) fn install(world: &mut World, inbox: Receiver<ToolRequest>) {
    world.insert_non_send(McpInbox(inbox));
    world.insert_non_send(McpLater(Vec::new()));
}

/// 汲む系を Input 段（`dispatch_pointer_events` の後）へ登録する（`ghost_session::register_systems` から）。
pub(crate) fn register(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain.after(dispatch_pointer_events));
}

/// 終了を始めた所で受け口と置き場を外す。溜まった要求・預かった組の `ReplyTo` が落ちて
/// 即座に "shutting down" になり、以後の送りも失敗する（要件 6.4）。
pub(crate) fn close(world: &mut World) {
    world.remove_non_send::<McpInbox>();
    world.remove_non_send::<McpLater>();
}

/// 溜まった要求を全件取り出して振り分け、その後で預かった組を全件覗く（Input の系）。
/// 受け口が無ければ無操作。UI スレッドで待つ所は無い（要件 6.1・6.6）。
pub(crate) fn drain(world: &mut World) {
    let Some(inbox) = world.get_non_send::<McpInbox>() else {
        return;
    };
    // 受け口の借用を切ってから振り分ける（処理が World を借りる）。
    let requests: Vec<ToolRequest> = inbox.0.try_iter().collect();
    for request in requests {
        // 1 件ごとに読み直す（前の要求の処理がゴーストを替えうる）。
        let active = resolve::active(world);
        dispatch(world, active.as_ref(), request);
    }
    poll_later(world);
}

/// 1 件を振り分ける（この `match` がツールごとの `ghost_name` の扱いの正本・要件 3.1・3.4・3.5・3.7）。
pub(crate) fn dispatch(world: &mut World, active: Option<&ActiveGhost>, request: ToolRequest) {
    let ToolRequest { call, reply } = request;
    // 解決してから処理へ。失敗は `NG:` で終えて処理を呼ばない。成功はゴーストの名前を記録に添える。
    macro_rules! resolved {
        ($tool:ident, $args:expr, $omitted:expr) => {{
            let args = $args;
            match resolve::resolve(active, args.ghost_name.as_deref(), $omitted) {
                Err(reason) => reply.send(outcome::ng(reason)),
                Ok(ghost) => {
                    let reply = reply.for_ghost(resolve::listed_value(ghost));
                    $tool::handle(world, ghost, args, reply);
                }
            }
        }};
    }
    match call {
        ToolCall::GetActiveGhostList => get_active_ghost_list::handle(active, reply),
        ToolCall::GetExpressionTable(args) => {
            resolved!(get_expression_table, args, Omitted::Reject)
        }
        ToolCall::GetStatus(args) => resolved!(get_status, args, Omitted::UseActive),
        ToolCall::GetProperty(args) => resolved!(get_property, args, Omitted::UseActive),
        ToolCall::Sakurascript(args) => resolved!(sakurascript, args, Omitted::UseActive),
        ToolCall::RaiseEvent(args) => resolved!(raise_event, args, Omitted::UseActive),
        ToolCall::Reload(args) => resolved!(reload, args, Omitted::UseActive),
        ToolCall::DumpSurface(args) => resolved!(dump_surface, args, Omitted::UseActive),
        ToolCall::DumpBalloon(args) => resolved!(dump_balloon, args, Omitted::UseActive),
        // `get_log` は解決しない（要件 3.7）。
        ToolCall::GetLog(args) => get_log::handle(world, args, reply),
    }
}

/// その場で答えられない処理が、返事と「覗く関数」を預ける。覗く関数は毎フレーム呼ばれ、
/// `Some` を返したらその結果が送られて組は外れる。置き場が無い（`install` の前・`close` の後）
/// なら組をその場で落とす＝"shutting down" と答える。
// 呼び手は 3 段目のツールの spec（後から答える処理）。生えるまで未使用の警告を抑える。
#[allow(dead_code)]
pub(crate) fn later(
    world: &mut World,
    reply: ReplyTo,
    poll: impl FnMut(&mut World) -> Option<ToolOutcome> + 'static,
) {
    if let Some(mut pairs) = world.get_non_send_mut::<McpLater>() {
        pairs.0.push((Box::new(poll), reply));
    }
}

/// 預かった組を全件覗く。列を World から取り出して回す（覗く関数が World を借りられ、
/// 覗く中で `later` が呼ばれても壊れない）。待つ側の居ない組は覗かずに捨てる。
fn poll_later(world: &mut World) {
    let Some(mut pairs) = world.get_non_send_mut::<McpLater>() else {
        return;
    };
    let taken = std::mem::take(&mut pairs.0);
    let mut kept = Vec::new();
    for (mut poll, reply) in taken {
        if reply.is_abandoned() {
            continue;
        }
        match poll(world) {
            Some(result) => reply.send(result),
            None => kept.push((poll, reply)),
        }
    }
    // 覗く中で預けられた組（列の後ろ）の前へ戻す。覗く中で閉じられていたら落とす。
    if let Some(mut pairs) = world.get_non_send_mut::<McpLater>() {
        kept.append(&mut pairs.0);
        pairs.0 = kept;
    }
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;
