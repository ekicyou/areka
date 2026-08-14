//! `windowposition.limit=1` の「バルーンを作業領域内へ収める」補正式（純関数）。
//!
//! 本モジュールは **クランプ式を 1 本だけ**定義し、以降の全関門（起動時関門・
//! `enqueue_window_set_pos` 内の runtime 関門・バルーンドラッグ解放時補正）へ供給する
//! （design「C5: balloon_limit 純関数」）。「バルーンは作業領域内」という不変量
//! （要件 2.1）の唯一の所有者であり、辺の組み合わせによらず決定論的に同一規則で
//! 補正する（要件 2.3）。
//!
//! # 単位と丸め
//!
//! 入出力は**すべて物理 px**（座標単位契約 U1〜U5・要件 2.10）。k 適用後の実表示寸で
//! 判定するのは呼び出し側の責務であり、本モジュールは `min`／`max` しか持たない
//! ——スケーリングも丸めも一切持ち込まない（新たな丸め規約を作らない）。
//!
//! # limit の有効／無効
//!
//! 本モジュールの式は**常にクランプする式**である。`limit=0` の scope を素通しさせる
//! 有効判定（要件 2.7）は関門の責務であり、ここには分岐を置かない（design C5）。
//!
//! # 依存規約
//!
//! クランプ核は `resolver` の物理 px 値型（[`PointPx`]／[`SizePx`]／[`RectPx`]）以外に
//! 依存しない純関数であり、wintf／bevy_ecs を import しない
//! （design「Boundary Commitments / Allowed Dependencies」）。

use super::resolver::{PointPx, RectPx, SizePx};

// =============================================================================
// 観測タグ（design「C10: 観測」・以降の全関門が本モジュールの定数を共有する）
// =============================================================================

/// limit 補正がバルーン位置を実際に動かしたことを表す判定語（要件 6.1・`info`）。
///
/// 実機サインオフの grep 判定語であり、`[balloon-limit] ` 接頭辞 1 語で本機能の記録を
/// 全部拾えることが 2 段 grep の第 1 段になる（`[zorder-pair] ` の先例と同じ手口）。
/// 出力点は各関門（task 2.4／3.3／3.4）が持ち、本モジュールは語彙だけを所有する。
// クランプ核と同時に定義するが消費点の結線は後続 task。定数だけが先に確定していることに
// 意味がある（全関門が同じタグを共有する単一真実源）。
#[allow(dead_code)] // 消費は task 2.4（起動時関門）／3.3（runtime 関門）／3.4（解放時補正）
pub(crate) const BALLOON_LIMIT_CLAMP_TAG: &str = "[balloon-limit] Clamp";

/// 作業領域が解決できず limit 補正を評価できなかったことを表す判定語（`warn`）。
///
/// 縮退（＝補正せず素通し）は必ずこの語で観測できる——ログの無い縮退経路を作らない
/// （log-first steering・要件 6.3 と同じ規律）。
#[allow(dead_code)] // 同上（縮退経路を持つのは関門側）
pub(crate) const BALLOON_LIMIT_UNRESOLVED_TAG: &str = "[balloon-limit] Unresolved";

// =============================================================================
// クランプ核
// =============================================================================

/// 1 軸クランプ（`lo ≤ v ≤ hi`）。
///
/// バルーンが作業領域より大きく `hi < lo` に逆転する場合は `lo`（left／top）側を
/// 優先する（要件 2.4）。これは**キャラ窓の既存クランプと同一の意味論**であり、
/// 式も `resolver.rs` の `clamp_axis`（P4）と逐語同一に保つ
/// ——バルーンとキャラで画面内維持の意味が割れないことが本仕様の要点である。
/// `i32::clamp` は逆転区間で panic するため使わない（本層は panic しない契約）。
#[allow(dead_code)] // scaffold（task 1.3）: 消費点の結線は task 2.4／3.3／3.4（現状は檻のみが消費）
fn clamp_axis(v: i32, lo: i32, hi: i32) -> i32 {
    v.min(hi).max(lo)
}

/// 矩形 `(pos, size)` を `area` 内へ 4 辺クランプした位置を返す（物理 px）。
///
/// - Preconditions: 座標・寸法は物理 px。`size.w > 0 && size.h > 0`（窓寸は正・既存契約）。
/// - Postconditions: 戻り位置の矩形は `area` と 4 辺内包する。逆転区間
///   （`size` が `area` より大きい軸）では left／top 辺が一致する（要件 2.4）。
///   本関数は**冪等**（結果を再入力しても不変）。
/// - 上下左右の全 4 辺を同一規則で扱い、はみ出しの方向・辺の組み合わせに分岐を
///   持たない（要件 2.3）。
///
/// `area.right`／`area.bottom` は排他側ゆえ上限は `right − w`／`bottom − h`。
/// 差の算出は `saturating_sub` で行う（極端値でも debug オーバーフロー panic しない
/// ＝`resolver` と同じ飽和流儀。通常入力では通常の減算と同値）。
#[allow(dead_code)] // scaffold（task 1.3）: 3 関門への供給は task 2.4／3.3／3.4
pub(crate) fn clamp_rect_to_work_area(pos: PointPx, size: SizePx, area: RectPx) -> PointPx {
    PointPx {
        x: clamp_axis(pos.x, area.left, area.right.saturating_sub(size.w)),
        y: clamp_axis(pos.y, area.top, area.bottom.saturating_sub(size.h)),
    }
}

/// 補正が必要なときだけ `Some(補正後位置)` を返す（不要なら `None`）。
///
/// `None` は「はみ出していない＝何も書かない」ことを呼び出し側と檻の双方が
/// 観測できるようにするための表現である——補正の要否を戻り値で判別できるので、
/// 関門は無補正時に窓位置の書き込みも `[balloon-limit] Clamp` ログも発生させない
/// （要件 6.1 は「実際に動かしたとき」に限って記録することを求める）。
///
/// 判定は [`clamp_rect_to_work_area`] の結果と入力位置の一致比較のみ——内包判定の
/// 二つ目の式を持たない（式は 1 本・単一真実源）。
#[allow(dead_code)] // scaffold（task 1.3）: 3 関門への供給は task 2.4／3.3／3.4
pub(crate) fn limit_correction(pos: PointPx, size: SizePx, area: RectPx) -> Option<PointPx> {
    let clamped = clamp_rect_to_work_area(pos, size, area);
    if clamped == pos { None } else { Some(clamped) }
}

#[cfg(test)]
#[path = "balloon_limit_tests.rs"]
mod tests;
