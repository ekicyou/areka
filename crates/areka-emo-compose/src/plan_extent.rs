//! キャンバス外形（[`Extent`]）の算出（`plan.rs` から中身を変えずに移した・animated-image-playback）。

use areka_emo_atlas::AtlasTable;

use super::{Extent, is_bind_interval, surface_and_binding};
use crate::nesting::ElementKind;
use crate::world::EmoWorld;

#[cfg(test)]
use super::*;

/// キャンバス外形（[`Extent`]）を**有効 bind 集合に依存せず静的に**算出する（要件 6.5・議題2裁定 (A)）。
///
/// 母集合は **surface の全定義層**＝全 element ＋**全 bind animation の pattern0**（`binds` に
/// 含まれるか否かに関係なく全部）。各層 element の「累積配置オフセット＋原寸（[`AtlasEntry::original`]）」
/// の和集合を、原点 (0,0) 固定・負オフセット分は原点でクリップして取る。`extent = (max_x, max_y)`、
/// 各層で `max_x = max(max_x, max(0, offset_x + original.w))`・同様に `max_y`。**`placement: None`
/// （全透明）の element も原寸は既知ゆえ外形へ寄与する**（design 明記）。
///
/// これにより **bind のオン/オフでサイズが変わらない**（同一 surface へ異なる `BindSet` を渡しても
/// [`Extent`] が不変・emo-present のバッファ再利用/窓サイズ安定に必須）。入れ子 surface 参照は
/// [`flatten_surface`] と同一の pattern の (x,y) オフセット累積＋visited 祖先スタックによる循環検出で
/// 走査する（ops 経路との違いは「全 bind pattern0 を母集合とする」点のみ）。
///
/// element ゼロ・bind ゼロの surface（外形へ寄与する層が皆無）や不在 surface では `Extent { w:0, h:0 }`
/// を返す（0×0 退化の Err 分類は [`build_plan`] の責務・本関数は分類しない）。
pub(crate) fn compute_extent(
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
) -> Extent {
    let mut max_x: i64 = 0;
    let mut max_y: i64 = 0;
    // ops 経路と同じ祖先スタック規律で循環検出する（別走査・別集約器だが構造は共有）。visited は
    // 呼び手のスクラッチを借用して再利用する（要件 10.3）。走査開始前に空へ戻す。
    visited.clear();
    flatten_extent(
        &mut max_x, &mut max_y, visited, world, atlas, surface_id, 0, 0,
    );
    Extent {
        // max_x/max_y は原点クリップ済みゆえ常に >= 0。u32 化は飽和で安全側（負にはならない）。
        w: max_x.max(0) as u32,
        h: max_y.max(0) as u32,
    }
}

/// [`compute_extent`] の再帰ワーカ: 当 surface の全定義層を走査し `max_x`/`max_y` を更新する。
///
/// [`flatten_surface`]（ops 経路）と同一の入れ子 flatten 構造（オフセット累積＋visited 祖先スタック
/// による循環検出）を持つが、**bind の母集合が異なる**: ops は有効 bind（`binds` ∩ bind animation）
/// のみ、本関数は**全 bind animation の pattern0**（`binds` 非依存・有効/非有効を問わない）を辿る。
/// これが「外形は有効 bind 集合に依存しない静的量」（要件 6.5）の実装核心である。
///
/// 各静的 element については `AtlasBinding` が `Some(ElementId)` のもののみ、`atlas.entry(id).original`
/// を「累積オフセット＋原寸」として外形へ寄与させる（未束縛 None は原寸不明ゆえ寄与しない）。
/// `placement` が None でも `original` は既知ゆえ寄与する（ops ではスキップされる層も外形は数える）。
///
/// element定義の子（[`ElementKind::Surface`]）は element定義の X,Y を足して再帰する（surface-element-
/// nesting 要件 2.5）。コマ（[`PatternState`]）は見ないので、部品のアニメーションでも外形は動かない
/// （要件 2.6）。飛ばす子は外形に数えず、記録は命令の経路だけが出す。
///
/// 引数は max_x/max_y/visited のスクラッチ3本＋world/atlas＋surface_id＋累積 offset(x,y) の計8本。
/// [`flatten_surface`] と同型の再帰 walker ゆえ全引数が各段で必要（スクラッチ構造体化は将来余地）。
#[allow(clippy::too_many_arguments)]
fn flatten_extent(
    max_x: &mut i64,
    max_y: &mut i64,
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    offset_x: i64,
    offset_y: i64,
) {
    // 循環検出（ops 経路と同一規律・要件 7.2/7.3）: 現在の祖先経路に既出なら打ち切り。
    if visited.contains(&surface_id) {
        tracing::warn!(
            target: "areka_emo_compose",
            surface_id,
            "外形算出: 入れ子参照の循環を検出・枝を打ち切り"
        );
        return;
    }
    visited.push(surface_id);

    if let Some((master, binding)) = surface_and_binding(world, surface_id) {
        // 当 surface の静的 element を外形へ寄与させる（束縛済み・placement 有無を問わず原寸で数える）。
        for (i, element) in master.elements.iter().enumerate() {
            // element定義の子（surface-element-nesting 要件 2.5）: 位置を足して子へ再帰する。範囲を
            // 超える数・先祖は黙って飛ばし、面の表に無い子は再帰先で何も足さない。記録は命令の経路の
            // debug! が合成 1 回につき 1 度出す（先祖へ再帰すると循環の warn! になるので入口の前で見る）。
            if let ElementKind::Surface(child) = element.kind {
                if !visited.contains(&child) {
                    let (ex, ey) = element.transform.offset();
                    flatten_extent(
                        max_x,
                        max_y,
                        visited,
                        world,
                        atlas,
                        child,
                        offset_x + ex,
                        offset_y + ey,
                    );
                }
                continue;
            }
            let Some(element_id) = binding.0.get(i).copied().flatten() else {
                // 未束縛（原寸不明）は外形に寄与できない。ops 側でも skip 済み。
                continue;
            };
            let original = atlas.entry(element_id).original;
            // 負オフセットは原点でクリップ（外形は (0,0) を左上に固定・負方向はみ出しは転写時クリップ）。
            *max_x = (*max_x).max((offset_x + original.w as i64).max(0));
            *max_y = (*max_y).max((offset_y + original.h as i64).max(0));
        }

        // **全** bind animation の pattern0 を母集合として辿る（有効/非有効を問わない・6.5 の核心）。
        // 描画順は外形に無関係（max の和集合ゆえ順序不変）だが、決定性のため id 昇順で走査する。
        let mut bind_ids: Vec<u32> = master
            .animations
            .iter()
            .filter(|a| is_bind_interval(&a.interval))
            .map(|a| a.id)
            .collect();
        bind_ids.sort_unstable();
        bind_ids.dedup();

        for id in bind_ids {
            let Some(anim) = master.animations.iter().find(|a| a.id == id) else {
                continue;
            };
            let Some(pattern0) = anim.patterns.iter().min_by_key(|p| p.index) else {
                continue;
            };
            if pattern0.surface_id < 0 {
                // センチネル（非描画）は外形に寄与しない（ops 経路と一致）。
                continue;
            }
            let nested_id = pattern0.surface_id as u32;
            flatten_extent(
                max_x,
                max_y,
                visited,
                world,
                atlas,
                nested_id,
                offset_x + pattern0.x,
                offset_y + pattern0.y,
            );
        }
    }

    // 枝離脱: 祖先スタックから pop（非循環の重複参照は別経路で再走査可能・要件 7.1）。
    let popped = visited.pop();
    debug_assert_eq!(
        popped,
        Some(surface_id),
        "外形算出: visited は祖先スタック（LIFO）"
    );
}

#[cfg(test)]
#[path = "plan_extent_tests.rs"]
mod extent_tests;

#[cfg(test)]
#[path = "plan_nesting_extent_tests.rs"]
mod nesting_extent_tests;
