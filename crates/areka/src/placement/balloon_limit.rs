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
//! （design「Boundary Commitments / Allowed Dependencies」）。起動時関門
//! [`apply_balloon_limit`] だけは [`MonitorSnapshot`]（bevy Resource derive だが
//! headless 構築可）と [`work_area_for_window`] を消費してよい——決定論性は保たれ、
//! 檻は headless で成立する（design「Allowed Dependencies」明記）。

use tracing::{info, warn};

use super::follow::{MonitorSnapshot, work_area_for_window};
use super::resolver::{PointPx, RectPx, ScopePlacement, SizePx};

// =============================================================================
// 観測タグ（design「C10: 観測」・以降の全関門が本モジュールの定数を共有する）
// =============================================================================

/// limit 補正がバルーン位置を実際に動かしたことを表す判定語（要件 6.1・`info`）。
///
/// 実機サインオフの grep 判定語であり、`[balloon-limit] ` 接頭辞 1 語で本機能の記録を
/// 全部拾えることが 2 段 grep の第 1 段になる（`[zorder-pair] ` の先例と同じ手口）。
/// 出力点は各関門（task 2.4／3.3／3.4）が持ち、本モジュールは語彙だけを所有する。
pub(crate) const BALLOON_LIMIT_CLAMP_TAG: &str = "[balloon-limit] Clamp";

/// 作業領域が解決できず limit 補正を評価できなかったことを表す判定語（`warn`）。
///
/// 縮退（＝補正せず素通し）は必ずこの語で観測できる——ログの無い縮退経路を作らない
/// （log-first steering・要件 6.3 と同じ規律）。
pub(crate) const BALLOON_LIMIT_UNRESOLVED_TAG: &str = "[balloon-limit] Unresolved";

/// 起動時関門であることを表す契機ラベル（要件 6.1 の「契機」欄）。
///
/// runtime 関門（task 3.3）と解放時補正（task 3.4）は [`super::diag::PlacementRoute`] を
/// 契機に持つが、起動時関門は窓書込より前のデータ変換であり route 語彙を持たない。
/// 補正ログの `context` フィールドで 3 関門を弁別できるようにするための語である
/// （実機サインオフの 2 段 grep で「どの関門が動いたか」を切り分ける）。
pub(crate) const BALLOON_LIMIT_BOOT_CONTEXT: &str = "boot-gate";

/// runtime 関門（`enqueue_window_set_pos` 内・task 3.3）であることを表す契機ラベル。
///
/// runtime 関門は [`super::diag::PlacementRoute`] も併せて出せる（元書込の route ＝
/// DD7「関門内の補正は元書込の route を保つ」）が、route だけでは起動時関門・解放時
/// 補正と同じ 2 段 grep に乗らない——起動時関門は route を持たないからである。
/// `context` フィールドを 3 関門で共通に出すことで「どの関門が動いたか」を 1 語で
/// 切り分けられる（`BALLOON_LIMIT_BOOT_CONTEXT` の doc と対を成す規約）。
pub(crate) const BALLOON_LIMIT_RUNTIME_CONTEXT: &str = "runtime-gate";

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
pub(crate) fn limit_correction(pos: PointPx, size: SizePx, area: RectPx) -> Option<PointPx> {
    let clamped = clamp_rect_to_work_area(pos, size, area);
    if clamped == pos { None } else { Some(clamped) }
}

// =============================================================================
// C6: 起動時関門（design「C6: 起動時関門」・要件 2.2／4.7／4.9／5.5／6.1）
// =============================================================================

/// 起動時関門: `balloon_limit=true` の scope について、キャラ窓が属するモニタの
/// 作業領域で `balloon_pos` をクランプした placements を返す。
///
/// # なぜここが 1 点で足りるか（design C6）
///
/// 経路①（spawn 初期値）と経路②（復元 merge）は**共に `restore_merged_placements` の
/// 出力を消費する**——すなわち merge 後のこの 1 点が両経路の合流点であり、キーワード
/// 由来の初期既定位置（resolver P5 の `CenterTop`／`CenterBottom` 出力）も同じ列として
/// ここを通る（要件 4.9: キーワード由来の初期位置にも同一規則が掛かる）。保存値優先の
/// 合流規則（要件 4.7）は上流 `persist::merge_scope` の所有であり、本関門は**その後**に
/// 位置を補正するだけで merge 規則には触れない。
///
/// # 何を書き換え、何を書き換えないか（DD6）
///
/// 書き換えるのは `balloon_pos`（＝表示位置）**だけ**である。`balloon_offset`
/// （作者指定・保存値の系譜＝論理相対位置）は生値のまま残す——補正を相対位置へ
/// 焼き付けない（要件 3.1(d)）。この結果、resolver の恒等式
/// `balloon_offset ≡ balloon_pos − char_pos` は「resolver 出力時点の事後条件」へ
/// 再スコープされ、本関門の通過後は成立しない**のが正しい**（DD6）。キャラ窓が
/// 作業領域内の余裕ある位置へ戻れば、追従計算（char + 生 offset）が作者指定・保存の
/// 相対位置へ自然に復帰する。
///
/// # 基準モニタ（要件 5.5）
///
/// 内包判定の基準はバルーン窓ではなく**キャラ窓**が属するモニタの作業領域である。
/// 帰属規則そのもの（窓中心 half-open ＋最近傍フォールバック）は
/// [`work_area_for_window`] の既存規則をそのまま呼ぶだけで、本仕様は変更しない。
///
/// # 縮退（design Error Handling「起動時関門で work area 解決不能（snapshot 空）」）
///
/// 作業領域が決まらない scope（空 `MonitorSnapshot`＝モニタ 0 台）は
/// [`BALLOON_LIMIT_UNRESOLVED_TAG`] の `warn` を残したうえで**素通し**する。架空の
/// 作業領域を発明せず、書き込み（後続の spawn 初期位置）を阻害もしない
/// （log-first・`work_area_for_window` と同方針）。
///
/// - Postconditions: 出力長＝入力長・順序不変。`balloon_limit=false` の scope と
///   縮退した scope は入力に完全恒等。
/// - Invariants: `balloon_pos` 以外のフィールド（特に `balloon_offset`）を変更しない。
pub(crate) fn apply_balloon_limit(
    placements: Vec<ScopePlacement>,
    snapshot: &MonitorSnapshot,
) -> Vec<ScopePlacement> {
    placements
        .into_iter()
        .map(|p| apply_balloon_limit_one(p, snapshot))
        .collect()
}

/// [`apply_balloon_limit`] の要素写像（1 scope 分）。
fn apply_balloon_limit_one(mut p: ScopePlacement, snapshot: &MonitorSnapshot) -> ScopePlacement {
    // 要件 2.7: limit=0 の scope は補正しない（式は常にクランプする式であり、
    // 有効判定は関門の責務＝design C5）。縮退ではないので警告も出さない。
    if !p.balloon_limit {
        return p;
    }

    // 要件 5.5: 基準は**キャラ窓**の帰属モニタ（バルーン窓ではない）。
    let char_rect = RectPx {
        left: p.char_pos.x,
        top: p.char_pos.y,
        right: p.char_pos.x.saturating_add(p.char_size.w),
        bottom: p.char_pos.y.saturating_add(p.char_size.h),
    };
    let Some(area) = work_area_for_window(snapshot, char_rect) else {
        warn!(
            scope = p.scope,
            context = BALLOON_LIMIT_BOOT_CONTEXT,
            char_pos = ?p.char_pos,
            char_size = ?p.char_size,
            "{BALLOON_LIMIT_UNRESOLVED_TAG} モニタ 0 台（空 snapshot）のためキャラ窓の作業領域を決められない → 補正せず素通し"
        );
        return p;
    };

    // `None` は「クランプしても位置が動かない」＝書き込み不要の意（**内包の判定では
    // ない**——逆転区間で既に左上に居るときははみ出したままでも `None`）。要件 6.1 が
    // 求めるのは「実際に動かしたとき」の記録ゆえ、`None` ではログも書換も起こさない。
    let Some(corrected) = limit_correction(p.balloon_pos, p.balloon_size, area) else {
        return p;
    };

    info!(
        scope = p.scope,
        context = BALLOON_LIMIT_BOOT_CONTEXT,
        from = ?p.balloon_pos,
        to = ?corrected,
        balloon_size = ?p.balloon_size,
        work_area = ?area,
        "{BALLOON_LIMIT_CLAMP_TAG} 起動時関門: バルーン表示位置を作業領域内へ補正した（balloon_offset は生値のまま）"
    );
    // DD6: 表示位置のみ。`balloon_offset` には触れない（焼き付けない）。
    p.balloon_pos = corrected;
    p
}

#[cfg(test)]
#[path = "balloon_limit_gate_tests.rs"]
mod gate_tests;
#[cfg(test)]
#[path = "balloon_limit_tests.rs"]
mod tests;
