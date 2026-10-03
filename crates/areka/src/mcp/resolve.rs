//! 宛先のゴーストの解決（spec: areka-P0-mcp-tool-entrances）。
//!
//! World から起きているゴーストを読む薄い配線と、`ghost_name` を解く純粋な判断を置く。

use std::path::PathBuf;

use bevy_ecs::world::World;

use crate::ghost_session::GhostSlot;

/// 起動中のゴースト 1 体（World も実行系も持たない値）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveGhost {
    /// descript の `name`（無い・空なら None）。
    pub name: Option<String>,
    /// ルートフォルダ（`ghost/<フォルダ名>`）の絶対パス。
    pub root: PathBuf,
}

/// `ghost_name` が無い・空のときの扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Omitted {
    /// 起動中の 1 体へ解決する。
    UseActive,
    /// `NG:Specified ghost is not active`。
    Reject,
}

pub(crate) const NOT_ACTIVE: &str = "Specified ghost is not active";
pub(crate) const CANNOT_FIND: &str = "Cannot find active ghost from specified name";

/// World から起動中のゴーストを読む（置き場が空・実行系が無ければ None・要件 3.8）。
/// LogSink へ倒れた単位も実行系があれば数える。切替の途中は置き場が空。
pub(crate) fn active(world: &World) -> Option<ActiveGhost> {
    let session = world.get_non_send::<GhostSlot>()?.0.as_ref()?;
    session.runtime()?;
    let name = session
        .names()
        .and_then(|n| n.name.clone())
        .filter(|n| !n.is_empty());
    let dir = session.ghost_dir();
    let root = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    Some(ActiveGhost { name, root })
}

/// 解決の判断（純粋・要件 3.2〜3.6）。失敗は `NG:` の後ろに付ける理由。
/// 渡された文字列は絶対化しない（相対パス・フォルダ名だけは一致しない）。別名は見ない。
pub(crate) fn resolve<'a>(
    active: Option<&'a ActiveGhost>,
    ghost_name: Option<&str>,
    omitted: Omitted,
) -> Result<&'a ActiveGhost, &'static str> {
    let given = match ghost_name {
        None | Some("") => {
            return match omitted {
                Omitted::Reject => Err(NOT_ACTIVE),
                Omitted::UseActive => active.ok_or(NOT_ACTIVE),
            };
        }
        Some(given) => given,
    };
    active
        .filter(|g| g.name.as_deref() == Some(given) || same_path(given, &g.root))
        .ok_or(CANNOT_FIND)
}

/// 一覧に出す値（`name`、無ければルートフォルダのフルパス・末尾の区切りなし・要件 4.2）。
pub(crate) fn listed_value(ghost: &ActiveGhost) -> String {
    match &ghost.name {
        Some(name) => name.clone(),
        None => ghost
            .root
            .display()
            .to_string()
            .trim_end_matches(['\\', '/'])
            .to_string(),
    }
}

/// フルパスの照合（大文字小文字・区切り・末尾の区切りの差を同じとみなす・要件 3.3）。
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
