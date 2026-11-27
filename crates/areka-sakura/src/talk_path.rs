//! 台本に書かれた相対パスを、実行するゴーストの置き場と探す順で解く共通の決まり。

use std::path::{Path, PathBuf};

use crate::contract::GhostPlace;

/// 相対パスを探す順（どのタグがどちらを使うかは、そのタグの ukadoc の項目に従う）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchOrder {
    /// ghost/master だけ。
    MasterOnly,
    /// ghost/master → ゴーストの最上位。
    MasterThenRoot,
}

/// 解けなかった理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    /// 書かれたパスが空。
    Empty,
    /// 相対パスなのに付帯情報に置き場が無い。
    NoPlace,
    /// どの候補にも無い（最後に試した候補）。
    NotFound { last_tried: PathBuf },
}

impl Unresolved {
    /// 記録の欄 `reason` の値（`"empty_path"`／`"no_place"`／`"not_found"`）。
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Empty => "empty_path",
            Self::NoPlace => "no_place",
            Self::NotFound { .. } => "not_found",
        }
    }
}

/// 書かれたパスを付帯情報の置き場と探す順で解く（在る無しは `Path::exists`）。
pub fn resolve(
    place: Option<&GhostPlace>,
    written: &str,
    order: SearchOrder,
) -> Result<PathBuf, Unresolved> {
    resolve_in(place, written, order, Path::exists)
}

/// `resolve` の決まりそのもの（在る無しを `exists` で判定する）。
pub fn resolve_in(
    place: Option<&GhostPlace>,
    written: &str,
    order: SearchOrder,
    exists: impl Fn(&Path) -> bool,
) -> Result<PathBuf, Unresolved> {
    if written.is_empty() {
        return Err(Unresolved::Empty);
    }
    let found = |candidate: PathBuf| {
        if exists(&candidate) {
            Ok(candidate)
        } else {
            Err(Unresolved::NotFound {
                last_tried: candidate,
            })
        }
    };
    let written = Path::new(written);
    if written.is_absolute() {
        return found(written.to_path_buf());
    }
    let place = place.ok_or(Unresolved::NoPlace)?;
    let in_master = found(place.master.join(written));
    match order {
        SearchOrder::MasterThenRoot if in_master.is_err() => found(place.root.join(written)),
        _ => in_master,
    }
}

#[cfg(test)]
#[path = "talk_path_tests.rs"]
mod tests;
