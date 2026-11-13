//! 箱の束（areka-P0-shell-balloon・design.md「箱の束の結線」）。
//!
//! 文字の層（`TextLayerRuntime::set_box_layout`）へ渡す材料を 1 つに束ね、`\s` の鍵の解決と
//! フォントを探す場所の順を純関数で決める。組み立てと受け渡しは起動・装着の相とシェルの切替が行う。

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use areka_emo_atlas::{AlphaParams, ElementDecoder};
use areka_emo_compose::{BoxLayout, BoxName};
use areka_emo_present::balloon_marker::{ARROW_FAMILY, MarkerPair, resolve_marker_pair};
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::{SurfaceFallback, SurfaceKeyOutcome};
use areka_sakura::ActorKey;
use areka_seriko::{SurfaceResolver, SurfaceTarget};
use bevy_ecs::world::World;

/// 文字の層へ渡す箱の束（シェル 1 つぶん）。既定値は箱の無いシェル（空の表・探す場所なし）。
#[derive(Default)]
pub(crate) struct ShellBoxAssets {
    /// 箱の定義の表とサーフェス番号ごとの置き場所の表（`ShellTarget::boxes`）。
    pub layout: BoxLayout,
    /// 別名の写し（`EmoWorld::alias_snapshot`・seriko の解決器と同じ材料）。
    pub aliases: BTreeMap<String, Vec<u32>>,
    /// 面の表に在るサーフェス番号（`EmoWorld::surface_ids`）。
    pub surface_ids: BTreeSet<u32>,
    /// 箱の `font.name` のフォントを探す場所の順（[`box_font_search_dirs`]）。
    pub font_dirs: Vec<PathBuf>,
    /// スコープごとの戻り先の候補の並び（`DefaultSurfaces::candidates`・descript-window-keys 要件 3.2）。
    pub fallback_candidates: BTreeMap<ActorKey, Vec<u32>>,
    /// スコープ → 箱の名前 → 矢印の絵（シェルのフォルダから解いたもの・[`resolve_box_markers`]）。
    /// 足りない番号は文字の層がスコープの普通のバルーンの絵で埋める（balloon-scroll-arrows 要件 7.3）。
    pub markers: BTreeMap<ActorKey, BTreeMap<BoxName, MarkerPair>>,
}

impl ShellBoxAssets {
    /// 文字の層へ渡す（装着の相とシェルの切替が呼ぶ）。箱の配置の直後に戻り先の引き直しを呼ぶ。`\s` の解決の閉包は、別名の写しから
    /// seriko と同じ解決器を組み、面の表に在る番号だけを通す（[`resolve_for_text`]）。
    pub(crate) fn hand_to(self, runtime: &mut TextLayerRuntime, world: &mut World) {
        let ShellBoxAssets {
            layout,
            aliases,
            surface_ids,
            font_dirs,
            fallback_candidates,
            markers,
        } = self;
        let resolver = SurfaceResolver::new(aliases);
        let fallback = SurfaceFallback {
            surface_ids: surface_ids.clone(),
            candidates: fallback_candidates,
        };
        runtime.set_box_layout(
            world,
            layout,
            Box::new(move |key| resolve_for_text(&resolver, &surface_ids, key)),
            font_dirs,
        );
        // シェルの切替で今の面が無いスコープを seriko と同じ戻り先へ替える（descript-window-keys 要件 3.2・3.5）。
        runtime.fall_back_box_surfaces(&fallback);
        // `set_box_layout` は前の矢印の絵を消さないので、空の束でも毎回入れ替える。
        runtime.set_box_markers(markers);
    }
}

/// スコープごと・置き場所を持つ箱ごとに、矢印の絵をシェルのフォルダから解く
/// （読み込みの時だけ呼ぶ・名前の順はスコープごと・要件 2.1・7.3）。
///
/// 同じ (スコープ, `arrow.filename`) は 1 回だけ解いて共有する（警告を箱の数だけ出さない）。
pub(crate) fn resolve_box_markers(
    layout: &BoxLayout,
    scopes: &[u32],
    shell_dir: &Path,
    decoder: &impl ElementDecoder,
    alpha: AlphaParams,
) -> BTreeMap<ActorKey, BTreeMap<BoxName, MarkerPair>> {
    let names: BTreeSet<&BoxName> = layout
        .surfaces()
        .flat_map(|(_, placements)| placements.iter().map(|p| &p.name))
        .collect();
    let mut by_filename: HashMap<(u32, Option<String>), MarkerPair> = HashMap::new();
    scopes
        .iter()
        .map(|&scope| {
            let boxes = names
                .iter()
                .filter_map(|&name| {
                    let filename = layout.def(name)?.model.arrow_keys().filename.clone();
                    let pair = by_filename
                        .entry((scope, filename.clone()))
                        .or_insert_with(|| {
                            resolve_marker_pair(
                                &[shell_dir],
                                &ARROW_FAMILY,
                                scope,
                                filename.as_deref(),
                                decoder,
                                alpha,
                            )
                        })
                        .clone();
                    Some((name.clone(), pair))
                })
                .collect();
            (ActorKey::from(scope.to_string()), boxes)
        })
        .collect()
}

/// 箱のフォントを探す場所の順（シェルのフォルダ、ゴーストのフォルダの順・要件 3.10）。
pub(crate) fn box_font_search_dirs(shell_dir: &Path, ghost_dir: &Path) -> Vec<PathBuf> {
    vec![shell_dir.to_path_buf(), ghost_dir.to_path_buf()]
}

/// `\s` の鍵を seriko と同じ解決で番号にし、今のシェルの面の表に無ければ「解決できない」にする
/// （要件 4.1・5.1・design.md「箱の束の結線」）。
///
/// 表示の層は面の表に無い番号へ切り替えられず前の表示を保つので、文字の層もそれに合わせて
/// 何も変えない。記録は呼び手（seriko と文字の層）が出す。
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
