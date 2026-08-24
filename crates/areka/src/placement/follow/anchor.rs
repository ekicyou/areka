//! アンカー射影ポリシー（[`DragPositionPolicy`]・[`BottomSnapPolicy`]・[`project_anchor`]・[`Anchored`]）。

use bevy_ecs::prelude::*;
use tracing::debug;

use super::{Anchor, MonitorSnapshot, PointPx, RectPx, SizePx, work_area_for_window};

// =============================================================================
// DragPositionPolicy（task 8.2R・DD15 v2・4.7）
// =============================================================================

/// 生ドラッグ座標→実窓位置の純粋写像トレイト（DD15 v2・開発者指示 2026-07-11）。
///
/// 「ドラッグ座標管理」（wintf の DraggingState/DragEvent＝カーソル差分の復元）と
/// 「実ウィンドウ位置の算出」を分離する。実装は純粋関数であること——
/// `raw`（生ドラッグ座標＝ドラッグ開始時窓位置＋カーソル差分・物理 px）と
/// 窓寸法・モニタ snapshot だけから実窓位置を返し、World に触れない。
/// 反映段階（`enqueue_window_set_pos`）には**適用済み座標のみ**が渡る＝
/// 事後補正が存在しないため、v1 の wndproc 競合振動は原理的に起きない。
pub trait DragPositionPolicy {
    /// 生ドラッグ座標 `raw` に対する実窓位置を返す（物理 px・純粋）。
    ///
    /// - `size`: 窓寸法（物理 px）。非正値は「寸法不明」を意味する
    /// - `snapshot`: モニタ work area 集合。`None`＝フォールバック経路（未挿入）
    fn resolve(&self, raw: PointPx, size: SizePx, snapshot: Option<&MonitorSnapshot>) -> PointPx;
}

/// bottom 吸着ポリシー（4.7・DD15 v2）: X 素通し・Y＝現在モニタ `bottom − h`。
///
/// Y は **`raw` 位置に置いた窓矩形**の中心が属するモニタ（[`work_area_for_window`]）
/// の work area 下端から live 算出する——イベントごとに引き直すため、モニタを
/// 跨いだら跨いだ先の下端へ自然に再吸着する（4.7 後段）。
///
/// graceful degradation（identity＝`raw` 素通し・panic しない・架空矩形を発明しない）:
/// - `snapshot` 不在: main.rs フォールバック経路は挿入しない設計（8.1 note）。
///   ドラッグ移動イベントごとに発火する経路ゆえ `warn!` は spam——`debug!` に留める
/// - 空 snapshot: [`work_area_for_window`] が `None`
/// - 非正寸法: `WindowPos::default()` の size は `CW_USEDEFAULT`（負のセンチネル）で、
///   `bottom − h` が `i32::MAX` 方向へ暴走するため w/h > 0 のみ吸着する
pub struct BottomSnapPolicy;

impl DragPositionPolicy for BottomSnapPolicy {
    fn resolve(&self, raw: PointPx, size: SizePx, snapshot: Option<&MonitorSnapshot>) -> PointPx {
        let Some(snapshot) = snapshot else {
            debug!("MonitorSnapshot 未挿入（フォールバック経路）のため identity 縮退");
            return raw;
        };
        if size.w <= 0 || size.h <= 0 {
            debug!(?size, "窓寸法が不明（非正）のため identity 縮退");
            return raw;
        }
        let window = RectPx {
            left: raw.x,
            top: raw.y,
            right: raw.x.saturating_add(size.w),
            bottom: raw.y.saturating_add(size.h),
        };
        let Some(wa) = work_area_for_window(snapshot, window) else {
            debug!("空 snapshot のため identity 縮退");
            return raw;
        };
        PointPx {
            x: raw.x,
            // 実 work area／窓寸で溢れない範囲だが、極端入力でも panic しない契約
            // （resolver の saturating 演算と同じ防波堤）
            y: wa.bottom.saturating_sub(size.h),
        }
    }
}

/// 変換 T: 解決済みアンカー＋生位置＋新寸から、アンカー辺を work area 対応辺へ
/// 固定した窓左上位置を返す純粋射影（5 アンカー・task 2.1・Req1.1/1.2/2.1-2.5/3.4/5.4）。
///
/// シェル座標系（アンカー辺基準）→ ウィンドウ座標系（サーフェス寸法基準）の変換 T の
/// 恒常維持を担う純粋関数（World 不可視・物理 px 単一通貨・`saturating_*` 演算で
/// panic しない）。ドラッグ（`policy_mapped_position`）とリサイズ（後続 task の
/// `resize_window_to`）の**両者が同一 T を呼ぶ**ことで座標系変換の二重化を避ける
/// （R1.6）。
///
/// # 射影規則（`wa` 取得成功かつ正寸のとき）
///
/// `wa` は「生位置 `raw` に置いた窓矩形の中心が属するモニタの work area」
/// （[`work_area_for_window`]）。モニタ跨ぎは live 算出＝跨いだ先の対応辺へ再吸着する。
/// - `Bottom`: 既存 [`BottomSnapPolicy::resolve`] へ**委譲**（X 保持・`y = wa.bottom − h`）。
///   再定義しない（Req1.2・bottom は T の一事例）。
/// - `Top`: `x = raw.x`（保持）・`y = wa.top`。
/// - `Left`: `x = wa.left`・`y = raw.y`（保持）。
/// - `Right`: `x = wa.right − w`・`y = raw.y`（保持）。
/// - `Free`: `raw` 素通し（identity・position 再計算なし・Req2.5）。
///
/// # graceful degradation（identity＝`raw` 素通し・panic しない・既存 `BottomSnapPolicy` 流儀）
///
/// - `Free` は常に identity（`wa` 不要・寸法・snapshot を問わない）。
/// - 非正寸（w≤0 or h≤0）は identity＋`debug!`。`wa.right − w`／`wa.bottom − h` が
///   `i32::MAX` 方向へ暴走する前に弾く（Req3.4・`BottomSnapPolicy` の CW_USEDEFAULT
///   センチネル縮退と整合）。
/// - `snapshot` 不在（`None`）／空 snapshot（[`work_area_for_window`] が `None`）は
///   identity＋`debug!`。ドラッグ経路 spam 回避で `warn!` でなく `debug!`（既存流儀）。
///
/// # 不変条件（テストで固定）
///
/// 正寸・snapshot 有効時、適用後の窓のアンカー辺 ≡ work area 対応辺。既にアンカー辺
/// 一致の位置に対しては同値を返す（べき等の基礎・R3.1）。
#[allow(dead_code)] // consumer（resize_window_to／on_char_drag 改修）は後続 task 2.2-2.4 の領分
pub fn project_anchor(
    anchor: Anchor,
    raw: PointPx,
    size: SizePx,
    snapshot: Option<&MonitorSnapshot>,
) -> PointPx {
    // Free: アンカー辺なし＝常に identity（wa 不要・寸法・snapshot を問わない・Req2.5）
    if let Anchor::Free = anchor {
        return raw;
    }

    // Bottom: 既存 BottomSnapPolicy へ全面委譲（再定義しない・Req1.2）。縮退規約
    // （snapshot 不在/空・非正寸）も同ポリシーが所有し、T の bottom 事例＝同値になる
    if let Anchor::Bottom = anchor {
        return BottomSnapPolicy.resolve(raw, size, snapshot);
    }

    // 以下 Top/Left/Right（bottom の一般化）。非正寸は wa.right−w／暴走の前に弾く
    if size.w <= 0 || size.h <= 0 {
        debug!(?size, ?anchor, "窓寸法が不明（非正）のため identity 縮退");
        return raw;
    }
    let Some(snapshot) = snapshot else {
        debug!(
            ?anchor,
            "MonitorSnapshot 未挿入（フォールバック経路）のため identity 縮退"
        );
        return raw;
    };
    // wa＝生位置に置いた窓矩形の中心が属するモニタの work area（live 算出・跨ぎ再吸着）
    let window = RectPx {
        left: raw.x,
        top: raw.y,
        right: raw.x.saturating_add(size.w),
        bottom: raw.y.saturating_add(size.h),
    };
    let Some(wa) = work_area_for_window(snapshot, window) else {
        debug!(?anchor, "空 snapshot のため identity 縮退");
        return raw;
    };

    match anchor {
        // 上端固定・X 保持（Req2.2）
        Anchor::Top => PointPx {
            x: raw.x,
            y: wa.top,
        },
        // 左端固定・Y 保持（Req2.3）
        Anchor::Left => PointPx {
            x: wa.left,
            y: raw.y,
        },
        // 右端固定（left_X = wa.right − w）・Y 保持（Req2.4）。極端入力でも panic
        // しない契約で saturating_sub（BottomSnapPolicy の bottom−h と同型の防波堤）
        Anchor::Right => PointPx {
            x: wa.right.saturating_sub(size.w),
            y: raw.y,
        },
        // Bottom／Free は冒頭で return 済み（到達不能・網羅性のための恒等既定）
        Anchor::Bottom | Anchor::Free => raw,
    }
}

/// キャラ窓が保持する現在の解決済みアンカー（drag／resize が読む単一の真実源・4.2/1.4）。
///
/// 全 char 窓へ 1 つだけ付与される 5 値アンカー表現（`Anchor` は 5 値ゆえ、二値
/// `BottomSnap` marker を generalize した後継。単一真実源＝二つ目の格納表現を作らない・
/// Req1.6）。値は spawn 時に `config.alignment` 由来（`ScopePlacement.anchor`＝
/// `Anchor::from_alignment` の解決結果）で焼き込まれ（付与は spawn の領分・task 3.1）、
/// runtime は seriko（本 spec 非所有＝`\![set,alignmenttodesktop]` の routing）が
/// 書き換える。`Changed<Anchored>` がアンカー変化での変換 T 再適用トリガとなる
/// （反応 system は後続 task の領分・本 spec は consumer 契約のみ・Req1.4/4.2）。
///
/// ドラッグ（`on_char_drag`）とリサイズ（`resize_window_to`）の**両者がこの値を読んで**
/// 同一射影 T（`project_anchor`）を呼ぶ——`Free` か否かで wndproc 委譲／単一ライターを
/// 分岐する。
#[allow(dead_code)]
// spawn 付与（task 3.1）は後続 task の領分——構築が付くまで dead_code 警告を抑える
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchored(pub Anchor);
