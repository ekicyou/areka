//! 宛先のゴーストの解決。World から起きているゴーストを読む薄い配線と、`ghost_name` を解く純粋な判断を置く。

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use areka_kanade::KanadeMsg;
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
