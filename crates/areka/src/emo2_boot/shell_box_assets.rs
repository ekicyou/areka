//! 箱の束（areka-P0-shell-balloon・design.md「箱の束の結線」）。
//!
//! 文字の層（`TextLayerRuntime::set_box_layout`）へ渡す材料を 1 つに束ね、`\s` の鍵の解決と
//! フォントを探す場所の順を純関数で決める。組み立てと受け渡しは起動・装着の相とシェルの切替が行う。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use areka_emo_compose::BoxLayout;
use areka_emo_text::state::SurfaceKeyOutcome;
use areka_seriko::{SurfaceResolver, SurfaceTarget};

/// 文字の層へ渡す箱の束（シェル 1 つぶん）。
// 組み立てと受け渡しは 8.2（起動・装着の相）と 8.3（シェルの切替）で使い始める。
#[allow(dead_code)]
pub(crate) struct ShellBoxAssets {
    /// 箱の定義の表とサーフェス番号ごとの置き場所の表（`ShellTarget::boxes`）。
    pub layout: BoxLayout,
    /// 別名の写し（`EmoWorld::alias_snapshot`・seriko の解決器と同じ材料）。
    pub aliases: BTreeMap<String, Vec<u32>>,
    /// 面の表に在るサーフェス番号（`EmoWorld::surface_ids`）。
    pub surface_ids: BTreeSet<u32>,
    /// 箱の `font.name` のフォントを探す場所の順（[`box_font_search_dirs`]）。
    pub font_dirs: Vec<PathBuf>,
}

/// 箱のフォントを探す場所の順（シェルのフォルダ、ゴーストのフォルダの順・要件 3.10）。
#[allow(dead_code)] // 8.2 で使い始める
pub(crate) fn box_font_search_dirs(shell_dir: &Path, ghost_dir: &Path) -> Vec<PathBuf> {
    vec![shell_dir.to_path_buf(), ghost_dir.to_path_buf()]
}

/// `\s` の鍵を seriko と同じ解決で番号にし、今のシェルの面の表に無ければ「解決できない」にする
/// （要件 4.1・5.1・design.md「箱の束の結線」）。
///
/// 表示の層は面の表に無い番号へ切り替えられず前の表示を保つので、文字の層もそれに合わせて
/// 何も変えない。記録は呼び手（seriko と文字の層）が出す。
#[allow(dead_code)] // 8.2 で閉包に包んで使い始める
pub(crate) fn resolve_for_text(
    resolver: &SurfaceResolver,
    surface_ids: &BTreeSet<u32>,
    key: &str,
) -> SurfaceKeyOutcome {
    match resolver.resolve(key) {
        SurfaceTarget::Show(id) if surface_ids.contains(&id) => SurfaceKeyOutcome::Show(id),
        SurfaceTarget::Hide => SurfaceKeyOutcome::Hide,
        SurfaceTarget::Show(_) | SurfaceTarget::Unresolved => SurfaceKeyOutcome::Unresolved,
    }
}

#[cfg(test)]
#[path = "shell_box_assets_tests.rs"]
mod tests;
