//! モニタ work area 解決（[`MonitorSnapshot`]・[`work_area_for_window`]・[`WorkAreaResolution`]）。

use bevy_ecs::prelude::*;
use wintf::ecs::window::monitor::Monitor;

use super::RectPx;

// =============================================================================
// MonitorSnapshot（task 8.1・DD15 基盤・4.7）
// =============================================================================

/// 全モニタの work area 集合（物理 px・起動時取得のセッション内固定 snapshot・DD15）。
///
/// 起動時に seam（main.rs）／example が [`MonitorSnapshot::from_monitors`] で実モニタ
/// から忠実転写して Resource 挿入し、bottom 吸着ドラッグ（task 8.2）が
/// [`work_area_for_window`] で「窓が現在属するモニタの work area」を引くのに使う。
/// snapshot はセッション内固定＝M1 受容（`WM_DISPLAYCHANGE` 追随は後続・DD15）。
/// 中身は `RectPx` のみの純粋データで、headless テストは合成値を直接構築して
/// 注入する（偽装境界・wintf に触れるのは挿入サイトだけ）。
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MonitorSnapshot {
    /// モニタ列挙順の work area（物理 px）。
    pub work_areas: Vec<RectPx>,
}

impl MonitorSnapshot {
    /// 実モニタ列挙結果から全 work area（`RECT`・物理 px）を列挙順のまま
    /// **単位変換なしで忠実転写**する（mod.rs `primary_work_area` と同じ U 契約:
    /// どちらも物理 px 通貨）。0 台は空 snapshot（panic しない・消費側が
    /// [`work_area_for_window`] の `None` で防御）。
    pub fn from_monitors(monitors: &[Monitor]) -> Self {
        Self {
            work_areas: monitors
                .iter()
                .map(|m| RectPx {
                    left: m.work_area.left,
                    top: m.work_area.top,
                    right: m.work_area.right,
                    bottom: m.work_area.bottom,
                })
                .collect(),
        }
    }
}

/// 窓矩形の中心が属するモニタの work area を引く純粋ヘルパ（4.7・DD15）。
///
/// 決定論規則（テストで固定）:
/// - 中心は `((left+right)/2, (top+bottom)/2)`（i64 演算・ゼロ方向切り捨て）
/// - 帰属判定は half-open（`left ≤ cx < right`・`top ≤ cy < bottom`）＝共有辺上の
///   中心は右／下側のモニタへ属する。複数矩形が含む（重複）場合は昇順 index 先勝ち
/// - どのモニタにも属さない場合は最近傍（中心→矩形 clamp 点の自乗距離最小・
///   等距離は昇順 index 先勝ち＝`min_by_key` の先頭優先）
/// - 空 snapshot は `None`（架空の既定矩形を発明しない・resolver と同方針）
///
/// 距離は i128 で自乗和を取り、極端な仮想スクリーン座標でも溢れない
/// （panic しない契約・resolver の saturating 演算と同じ防波堤）。
///
/// # 判別付き版への委譲（task 2.2・D6）
///
/// 本関数は [`work_area_for_window_with_origin`] の**戻り値から判別を落とすだけ**の
/// 薄いラッパである（契約不変＝既存呼出元の挙動は 1 bit も変わらない。等価性は
/// `work_area_for_window_delegates_to_with_origin` が檻で固定する）。
pub fn work_area_for_window(snapshot: &MonitorSnapshot, window: RectPx) -> Option<RectPx> {
    work_area_for_window_with_origin(snapshot, window).map(|(wa, _)| wa)
}

// =============================================================================
// 可視性の遷移ガード＋work area 解決の判別（task 2.2・D6/S3′・Req 3.1/3.2/5.3）
// =============================================================================

/// [`work_area_for_window_with_origin`] が work area を決めた**規則**（D6・Req 3.2）。
///
/// 最近傍フォールバックは「どのモニタにも属さない」＝モニタ構成情報と実画面の
/// 食い違い／窓が可視領域外という**異常の兆候**でありながら、[`work_area_for_window`]
/// の戻り値だけでは正常な帰属と区別できない（S3 の後半＝「最近傍フォールバックが
/// 異常を無観測で吸収する」）。本 enum はその区別を呼出側へ返すためだけに在る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkAreaResolution {
    /// 窓中心がその work area に帰属した（half-open 判定・正常）。
    Contains,
    /// どのモニタにも属さず、最近傍フォールバックで選ばれた（Req 3.2 の観測点）。
    NearestFallback,
}

/// [`work_area_for_window`] の判別付き版（D6・Req 3.2）。
///
/// 解決規則そのものは [`work_area_for_window`] の doc が定める決定論規則と**完全に
/// 同一**（中心の半開帰属・昇順 index 先勝ち・最近傍は clamp 点自乗距離最小・空
/// snapshot は `None`）。違いは「どちらの規則で決まったか」を
/// [`WorkAreaResolution`] として併せて返す点だけである。
///
/// 消費側（task 6.1 で配線）は `NearestFallback` を非ドラッグ経路でのみ `warn!` へ
/// 昇格させる（ドラッグ経路は毎イベント発火ゆえ従来 `debug!` 水準を維持・Req 3.3）。
pub fn work_area_for_window_with_origin(
    snapshot: &MonitorSnapshot,
    window: RectPx,
) -> Option<(RectPx, WorkAreaResolution)> {
    let cx = (window.left as i64 + window.right as i64) / 2;
    let cy = (window.top as i64 + window.bottom as i64) / 2;

    // 帰属（half-open）・昇順 index 先勝ち
    if let Some(wa) = snapshot.work_areas.iter().find(|wa| {
        (wa.left as i64) <= cx
            && cx < (wa.right as i64)
            && (wa.top as i64) <= cy
            && cy < (wa.bottom as i64)
    }) {
        return Some((*wa, WorkAreaResolution::Contains));
    }

    // どこにも属さない → 最近傍（clamp 点との自乗距離最小・等距離は先勝ち）
    snapshot
        .work_areas
        .iter()
        .min_by_key(|wa| {
            // `i64::clamp` は逆転区間（万一の退化矩形）で panic するため min/max で書く
            // （resolver `clamp_axis` と同じ非 panic 流儀）
            let px = cx.min(wa.right as i64).max(wa.left as i64);
            let py = cy.min(wa.bottom as i64).max(wa.top as i64);
            let dx = (cx - px) as i128;
            let dy = (cy - py) as i128;
            dx * dx + dy * dy
        })
        .copied()
        .map(|wa| (wa, WorkAreaResolution::NearestFallback))
}
