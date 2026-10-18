//! 台本の特別な名前（`random`・`sequential`・`lastinstalled`）の解決（areka-P0-ghost-change-name-resolution）。
//!
//! ゴーストの切替の入口 [`super::ghost_switch`] から純移動した（areka-P0-mcp-reload・行数の確保）。

use areka_ghost::{GhostEntry, same_folder_name};
use bevy_ecs::resource::Resource;

/// 特別な名前の解決の結果（areka-P0-ghost-change-name-resolution）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NameResolution {
    /// 特別な名前ではない（今日どおり名指しとして目録と突き合わせる）。
    Plain,
    /// 解けた。`position` は `sequential` のときの今のゴーストの位置（記録用・他は `None`）。
    Resolved {
        folder: String,
        position: Option<usize>,
    },
    /// 特別な名前だが解けない（理由は記録の語彙）。
    Unresolved(UnresolvedReason),
}

/// 解けない理由（`ghost_switch_unknown` の `reason` 欄の語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnresolvedReason {
    RandomEmpty,
    SequentialEmpty,
    LastInstalledNone,
    LastInstalledMissing,
}

impl UnresolvedReason {
    /// `reason` 欄の語（`random_empty` など）。
    pub(crate) fn as_ref_str(self) -> &'static str {
        match self {
            Self::RandomEmpty => "random_empty",
            Self::SequentialEmpty => "sequential_empty",
            Self::LastInstalledNone => "lastinstalled_none",
            Self::LastInstalledMissing => "lastinstalled_missing",
        }
    }

    /// 人が読む本文（理由ごとに 1 文）。
    pub(crate) fn describe(self) -> &'static str {
        match self {
            Self::RandomEmpty => "目録にゴーストが 1 体も無い——random の切替を無視する",
            Self::SequentialEmpty => "目録にゴーストが 1 体も無い——sequential の切替を無視する",
            Self::LastInstalledNone => {
                "このプロセスでゴーストを入れていない——lastinstalled の切替を無視する"
            }
            Self::LastInstalledMissing => {
                "最後に入れたゴーストが目録に無い——lastinstalled の切替を無視する"
            }
        }
    }
}

/// 純粋: 特別な名前を目録のフォルダ名へ解く（要件 2〜5・9・design「SpecialNameResolver」）。
///
/// 特別な名前は `random`・`sequential`・`lastinstalled` の完全一致だけ（大文字小文字を区別）で、
/// 目録に同じ名前のゴーストがいても特別な名前として解く。`entries` は `list_ghosts` の並び
/// （フォルダ名の昇順）のまま渡す。`current`＝今のゴーストのフォルダ名（`None` は目録に無い扱い）、
/// `last_installed`＝最後に入れたゴーストのフォルダ名の記録（どちらも `same_folder_name` で比べる）。
/// - `random`: 今のゴーストを除いた候補から `pick(候補数)` で 1 体。候補が 0 体なら今のゴースト
///   自身（目録に残っているのはそれだけ）。`pick` を呼ぶのは候補が 1 体以上のときだけ。
/// - `sequential`: 今の位置の次（末尾なら先頭）。今のゴーストが目録に無ければ先頭。
/// - `lastinstalled`: 記録のフォルダ名で目録を引く（今のゴースト自身でも解ける）。
///
/// fs・World・時計は読まない。
pub(crate) fn resolve_special_name(
    name: &str,
    entries: &[GhostEntry],
    current: Option<&str>,
    last_installed: Option<&str>,
    pick: impl FnOnce(usize) -> usize,
) -> NameResolution {
    let resolved = |e: &GhostEntry, position| NameResolution::Resolved {
        folder: e.identity.folder.clone(),
        position,
    };
    let is_current =
        |e: &&GhostEntry| current.is_some_and(|c| same_folder_name(&e.identity.folder, c));
    match name {
        "random" => {
            let others: Vec<&GhostEntry> = entries.iter().filter(|e| !is_current(e)).collect();
            match (others.is_empty(), entries.first()) {
                (false, _) => resolved(others[pick(others.len())], None),
                (true, Some(itself)) => resolved(itself, None),
                (true, None) => NameResolution::Unresolved(UnresolvedReason::RandomEmpty),
            }
        }
        "sequential" => {
            if entries.is_empty() {
                return NameResolution::Unresolved(UnresolvedReason::SequentialEmpty);
            }
            let position = entries.iter().position(|e| is_current(&e));
            let next = position.map_or(0, |i| (i + 1) % entries.len());
            resolved(&entries[next], position)
        }
        "lastinstalled" => match last_installed {
            None => NameResolution::Unresolved(UnresolvedReason::LastInstalledNone),
            Some(want) => entries
                .iter()
                .find(|e| same_folder_name(&e.identity.folder, want))
                .map_or(
                    NameResolution::Unresolved(UnresolvedReason::LastInstalledMissing),
                    |e| resolved(e, None),
                ),
        },
        _ => NameResolution::Plain,
    }
}

/// 同じプロセスで最後に入れたゴーストのフォルダ名の記録（`lastinstalled` の解決に使う）。
///
/// プロセスの中だけ（ファイル・記憶へは書かない）。無ければ「このプロセスで何も入れていない」。
/// 入口は読むだけで、切替に使っても消さない。
#[derive(Resource)]
pub(crate) struct LastInstalledGhost(pub String);
