//! 宛先のゴーストの解決。World から起きているゴーストを読む薄い配線と、`ghost_name` を解く純粋な判断と、宛先の kanade へ 1 通送って後から答える配線（[`ask_kanade`]）を置く。

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use areka_actor::{ReplyError, ReplySender, reply_channel};
use areka_kanade::KanadeMsg;
use areka_mcp::ToolOutcome;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use areka_session::ghost_session::GhostSlot;

/// 起動中のゴースト 1 体（World も実行系も持たない値）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveGhost {
    /// descript の `name`（無い・空なら None）。
    pub name: Option<String>,
    /// descript の `sakura.name`＝本体側名（無い・空なら None）。
    pub sakura_name: Option<String>,
    /// ルートフォルダ（`ghost/<フォルダ名>`）の絶対パス。
    pub root: PathBuf,
}

/// `ghost_name` が無いときの扱い（空文字は含まない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Omitted {
    UseActive,
    /// `NG:Specified ghost is not active`。
    Reject,
}

pub(crate) const NOT_ACTIVE: &str = "Specified ghost is not active";
pub(crate) const CANNOT_FIND: &str = "Cannot find active ghost from specified name";

/// World から起動中のゴーストを読む（置き場が空・実行系が無ければ None）。
/// LogSink へ倒れた単位も実行系があれば数える。切替の途中は置き場が空。
pub(crate) fn active(world: &World) -> Option<ActiveGhost> {
    let session = world.get_non_send::<GhostSlot>()?.0.as_ref()?;
    session.runtime()?;
    let names = session.names();
    let name = names.and_then(|n| n.name.clone()).filter(|n| !n.is_empty());
    let sakura_name = names
        .and_then(|n| n.sakura_name.clone())
        .filter(|n| !n.is_empty());
    let dir = session.ghost_dir();
    let root = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    Some(ActiveGhost {
        name,
        sakura_name,
        root,
    })
}

/// 宛先の kanade の送り口と、答える前に降りたときの文言。
pub(crate) struct ScriptTarget {
    pub kanade: Option<Sender<KanadeMsg>>,
    /// 起動中のゴースト無しで解決したときの失敗の理由。
    pub gone: &'static str,
}

/// 置き場の 1 体から送り口を引き、`ghost_name` から「降りた」の文言を求める。
pub(crate) fn script_target(world: &World, ghost_name: Option<&str>) -> ScriptTarget {
    let gone = resolve(None, ghost_name, Omitted::UseActive)
        .err()
        .unwrap_or(NOT_ACTIVE);
    let kanade = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.kanade())
        .cloned();
    ScriptTarget { kanade, gone }
}

/// kanade へ 1 通送って返事を `later` で覗くツールの、名・答えの文言・記録の `event`。
#[derive(Clone, Copy)]
pub(crate) struct KanadeAsk {
    /// 記録の文の頭に付けるツールの名。
    pub tool: &'static str,
    /// 送るものの呼び名（送れないときの記録の文に入れる）。
    pub what: &'static str,
    /// 置き場に送り口が無いときの本文。
    pub unavailable: &'static str,
    /// 送り口が無いとき（`warn!`）。
    pub unavailable_event: &'static str,
    /// kanade が止まっていて送れないとき（`debug!`）。
    pub gone_event: &'static str,
    /// 返事の前に返信端が落ちたとき（`debug!`）。
    pub dropped_event: &'static str,
}

/// 宛先の kanade へ `msg` が組んだ 1 通を送り、返事は待たずに `later` へ預けて届いたら `answer` で答える。
///
/// 送り口が無い・送れない・返事の前に返信端が落ちたときは記録を残して `NG:` で答える（落ちたときと
/// 送れないときは「降りた」の文言）。`ghost` は送り口が無いときの `warn!` の欄にだけ使う。
pub(crate) fn ask_kanade<T: Send + 'static>(
    world: &mut World,
    ghost: &ActiveGhost,
    ghost_name: Option<&str>,
    ask: KanadeAsk,
    reply: ReplyTo,
    msg: impl FnOnce(ReplySender<T>) -> KanadeMsg,
    mut answer: impl FnMut(T, &'static str) -> ToolOutcome + 'static,
) {
    let ScriptTarget { kanade, gone } = script_target(world, ghost_name);
    let Some(kanade) = kanade else {
        tracing::warn!(
            event = ask.unavailable_event,
            ghost = listed_value(ghost).as_str(),
            "[mcp] {}: 宛先のゴーストの kanade の送り口が見つからない",
            ask.tool
        );
        reply.send(outcome::ng(ask.unavailable));
        return;
    };
    let (tx, rx) = reply_channel::<T>();
    if kanade.send(msg(tx)).is_err() {
        tracing::debug!(
            event = ask.gone_event,
            "[mcp] {}: kanade が止まっていて{}を送れない",
            ask.tool,
            ask.what
        );
        reply.send(outcome::ng(gone));
        return;
    }
    super::later(world, reply, move |_| match rx.try_recv() {
        Ok(None) => None,
        Ok(Some(value)) => Some(answer(value, gone)),
        // `Timeout` は `try_recv` からは返らない。型の上の腕は「降りた」に倒す。
        Err(ReplyError::Dropped | ReplyError::Timeout) => {
            tracing::debug!(
                event = ask.dropped_event,
                "[mcp] {}: 返事の前に kanade の返信端が落ちた",
                ask.tool
            );
            Some(outcome::ng(gone))
        }
    });
}

/// 解決の判断（純粋）。失敗は `NG:` の後ろに付ける理由。
/// 名前と本体側名は、渡された文字列の前後の空白を除き半角の英字の大小を同じとみなして比べる。
/// フルパスは渡された文字列のまま比べ、絶対化しない（相対パス・フォルダ名だけは一致しない）。
/// 相方の名前・`sakura.name2` は見ない。空文字・空白だけは何にも当たらず `CANNOT_FIND`。
/// 返すのは起動中のゴーストの値そのもの（綴りは残らない）。
pub(crate) fn resolve<'a>(
    active: Option<&'a ActiveGhost>,
    ghost_name: Option<&str>,
    omitted: Omitted,
) -> Result<&'a ActiveGhost, &'static str> {
    let Some(given) = ghost_name else {
        return match omitted {
            Omitted::Reject => Err(NOT_ACTIVE),
            Omitted::UseActive => active.ok_or(NOT_ACTIVE),
        };
    };
    let trimmed = given.trim();
    let same_name = |n: &Option<String>| {
        n.as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(trimmed))
    };
    active
        .filter(|g| same_name(&g.name) || same_name(&g.sakura_name) || same_path(given, &g.root))
        .ok_or(CANNOT_FIND)
}

/// 一覧に出す値（`name`、無ければルートフォルダのフルパス・末尾の区切りなし）。
/// script の記録の名と同じ作り（`areka_ghost::config::listed_name`）。
pub(crate) fn listed_value(ghost: &ActiveGhost) -> String {
    areka_ghost::config::listed_name(ghost.name.as_deref(), &ghost.root)
}

/// フルパスの照合（大文字小文字・区切り・末尾の区切りの差を同じとみなす）。
fn same_path(given: &str, root: &std::path::Path) -> bool {
    fn norm(s: &str) -> String {
        s.to_lowercase()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_string()
    }
    norm(given) == norm(&root.to_string_lossy())
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod resolve_tests;
