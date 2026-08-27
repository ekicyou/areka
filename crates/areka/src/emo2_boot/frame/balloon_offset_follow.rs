//! 拡大率遷移でのバルーン追従オフセットの**追随の適用相**
//! （areka-P0-balloon-offset-dpi・design D6／D9／D16・要件 3.1／3.2／3.6／3.7／9.4／9.8）。
//!
//! # 何をする相か
//!
//! キャラ窓 1 つについて、追従 Component（[`BalloonFollow`]）が持つ**基準対**と、その窓の
//! 現在の表示 DPI（[`DPI`]）を読み、変換規則の純関数 [`rescale_follow_offset`] へ委ね、
//! 返ってきた腕どおりに**係留**（[`BalloonFollow::anchor_base_dpi`]）または**追随の適用**
//! （[`BalloonFollow::apply_rescaled`]）を書き、遷移観測を 1 行出す。
//!
//! 値の決め方は本モジュールに 1 つも無い——丸めも比の組み方も純関数側の権威に閉じており、
//! ここが持つのは「読む・委ねる・書く・記録する」だけである。
//!
//! # 発火条件は `Changed<DPI>` **だけ**（要件 3.2／9.8）
//!
//! 本相の唯一の呼び手は [`dpi_phase_with`](super::dpi::dpi_phase_with) の第 2 巡であり、
//! その対象集合は `Changed<DPI>`（＋整合待ちの札を持つ窓の和集合）である。面の切替・
//! 作業領域の再スナップ・`\![move]` は本相を通らないため、「拡大率が変わらない寸法変化では
//! オフセットを 1 bit も動かさない」（要件 3.2・先行仕様から引き継いだ 9.8）が
//! **構造的に**保たれる。寸法差を発火条件に採らないのは、寸の再導出結果
//! （`refresh_scale_report` の `None`）が拡大率不変を意味しないからでもある（design D6）。
//!
//! # 1 遷移・1 スコープにつき高々 1 行（観測の順序契約）
//!
//! 発行口 [`log_offset_rescale`] が保証するのは「1 呼出につき 1 行」だけであり、
//! 「1 遷移につき高々 1 呼出」は本モジュールと呼び手の側の義務である。3 点で守る:
//!
//! 1. **追従先が無い窓では呼ばない**——[`BalloonFollow`] を持たない窓は最初に抜ける
//!    （縮退ではなくデータ駆動の非該当ゆえ**記録もしない**・要件 9.4 の対象外。
//!    `rederive_keyword_balloon_offset` の同型の腕と同じ流儀）。
//! 2. **判定は窓ごとに 1 度だけ下す**——腕ごとに行を出さず、下した 1 語を 1 行として渡す。
//! 3. **待ち札で見送られた窓は本相へ来ない**——第 1 巡の関門
//!    （`apply_dpi_phase_gate`）を通過した窓だけが第 2 巡へ入るため、見送り中の遷移では
//!    行が出ず、札が外れて再合流したときに初めて 1 行が出る（二重計上が起きない）。
//!
//! # 縮退はすべて警告を伴う（要件 9.4）／ただし「無遷移」は縮退ではない
//!
//! 値を動かせなかった腕のうち **`warn!` を伴うのは縮退した 2 腕だけ**である
//! ——現在の表示 DPI が読めない（[`DPI`] Component 不在）と、比を組めない
//! （[`OffsetRescale::Unresolved`]）。次の 3 つは**縮退ではない**ので警告しない:
//!
//! - **追従先が無い**: 記録もしない（上記 1）。
//! - **係留**（[`OffsetRescale::Anchored`]）: 永続値の腕が正規の口を通っただけ（要件 5.2）。
//! - **無遷移**（[`OffsetRescale::Unchanged`]）: 基準 DPI と現在 DPI が同一。
//!
//! ## `DPI{0,0}` は縮退ではなく無遷移として記録する（要件 3.6 の記録側・9.4）
//!
//! 基準 DPI と現在の表示 DPI が**どちらも 0** の場合、純関数は同値判定が先に立つため
//! [`OffsetRescale::Unchanged`] を返す（`ZeroBaseDpi` にはならない——純関数側の
//! `zero_on_both_sides_is_unchanged_not_unresolved` が固定している）。本相はこれを
//! **無遷移として `verdict=unchanged` で記録し、警告を出さない**。これは取りこぼしではなく
//! 明示の判断である: 0→0 では値が**どちらの向きにも動かない**ため要件 3.6 の規範側
//! （「追従オフセットを変更しない」）は満たされており、片側だけが 0 の腕
//! （`ZeroBaseDpi`／`ZeroCurrentDpi`＝値が動くべきかもしれないのに動かせない）とは
//! 性質が違う。両者を同じ `unresolved` へ畳むと、**真に危険な片側 0 の警告が
//! 「そもそも DPI を一度も観測していない窓」の日常的な雑音に埋もれる**。
//!
//! # 記録の欄の読み方
//!
//! `new_dpi=0` は「現在の表示 DPI を使える値として読めなかった」ことを表す——
//! [`DPI`] Component が無い腕と、読めたが 0 だった腕（`UnresolvedScale::ZeroCurrentDpi`）の
//! 両方がこの字面になる。どちらも `verdict=unresolved` で `warn!` を伴うため、
//! 行だけで両者を切り分ける必要は無い（切り分けは警告本文が持つ）。

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::warn;

use wintf::ecs::DPI;

use crate::placement::follow::{BalloonFollow, OffsetRescale, rescale_follow_offset};
use crate::placement::transition_diag::{
    OFFSET_VERDICT_ANCHORED, OFFSET_VERDICT_RESCALED, OFFSET_VERDICT_SATURATED,
    OFFSET_VERDICT_UNCHANGED, OFFSET_VERDICT_UNRESOLVED, log_offset_rescale,
};

/// 追随の適用結果（呼び手＝`dpi_phase_with` が収束の要否を決めるために読む・design D16）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OffsetFollowOutcome {
    /// offset が実際に変わった——窓書込が起きなければ収束が要る（D16・task 6.2 が消費する）。
    Changed,
    /// 値は変わっていない（同一 DPI・係留・縮退・追従先なし）。
    Unchanged,
}

/// 1 つのキャラ窓について追随を適用する（`refresh_scale_report` より**前**に呼ぶ）。
///
/// - **Preconditions**: 第 1 巡の待ち札の関門を通過した窓であること。`char_window` は
///   キャラ窓（`GhostWindowKind::Char`）であること。
/// - **Postconditions**: [`BalloonFollow`] の現在値と基準対は [`OffsetRescale`] の腕どおりに
///   のみ変わる。窓（`SetWindowPosCommand`）は本関数からは**一度も**発行されない。
/// - **Invariants**: 遷移 1 回あたりの窓書込は キャラ ≤1・バルーン ≤1・別経路 0
///   （先行仕様から引き継いだ予算・要件 3.4）。本関数は書込を 1 つも足さない。
pub(super) fn rescale_balloon_follow_offset(
    world: &mut World,
    char_window: Entity,
    scope: u32,
) -> OffsetFollowOutcome {
    // 追従先が無い窓＝データ駆動の非該当。何もせず、**記録も残さない**（module doc 参照）。
    let Some(follow) = world.get::<BalloonFollow>(char_window).copied() else {
        return OffsetFollowOutcome::Unchanged;
    };
    let base = follow.base();
    let old_offset = follow.offset();
    let base_dpi_field = base.dpi.map(|d| d.dpi_x as u32);

    // 継ぎ目（task 6.3・design D7）: キーワード由来の基本位置の素材（`BalloonKeywordBase`）が
    // 未消費のあいだは、オフセットも基準対も 1 bit も触らず `verdict=keyword-pending` を
    // 記録して抜ける門がここへ入る。門は本仕様の新規コード側に置き、再導出側の発火条件と
    // 「経路で絞らない」設計判断には触れない。

    let Some(current) = world.get::<DPI>(char_window).copied() else {
        // 縮退（要件 9.4）: 現在の表示 DPI が読めない＝比を組む材料が無い。値は据え置く。
        warn!(
            entity = ?char_window,
            scope,
            "balloon offset: キャラ窓に DPI component が無く表示 DPI 比を組めない → 追随を見送る"
        );
        log_offset_rescale(
            world,
            Some(scope),
            base_dpi_field,
            0,
            base.offset,
            old_offset,
            old_offset,
            OFFSET_VERDICT_UNRESOLVED,
        );
        return OffsetFollowOutcome::Unchanged;
    };

    // 値の決定は純関数の権威へ委ねる（丸めも比の組み方もここには無い・要件 9.3）。
    let (new_offset, verdict) = match rescale_follow_offset(base, current) {
        OffsetRescale::Anchored { base_dpi } => {
            // 永続値の腕。**値は 1 bit も変えず**基準へ現在の表示 DPI を刻む（要件 5.2）。
            if let Some(mut f) = world.get_mut::<BalloonFollow>(char_window) {
                f.anchor_base_dpi(base_dpi);
            }
            (old_offset, OFFSET_VERDICT_ANCHORED)
        }
        OffsetRescale::Unchanged => (old_offset, OFFSET_VERDICT_UNCHANGED),
        OffsetRescale::Rescaled { offset, saturated } => {
            if let Some(mut f) = world.get_mut::<BalloonFollow>(char_window) {
                f.apply_rescaled(offset);
            }
            if saturated {
                // 縮退（要件 2.5 と同型）: 回り込ませず飽和値を採ったことを記録する。
                warn!(
                    entity = ?char_window,
                    scope,
                    base = ?base.offset,
                    offset = ?offset,
                    "balloon offset: 追随の換算が i32 域で飽和した（回り込ませず飽和値を採用）"
                );
                (offset, OFFSET_VERDICT_SATURATED)
            } else {
                (offset, OFFSET_VERDICT_RESCALED)
            }
        }
        OffsetRescale::Unresolved { reason } => {
            // 縮退（要件 3.6／9.4）: 値も基準も変えない。
            warn!(
                entity = ?char_window,
                scope,
                ?reason,
                "balloon offset: 表示 DPI 比を解決できない → 追随を見送る（値・基準とも据え置き）"
            );
            (old_offset, OFFSET_VERDICT_UNRESOLVED)
        }
    };

    log_offset_rescale(
        world,
        Some(scope),
        base_dpi_field,
        current.dpi_x as u32,
        base.offset,
        old_offset,
        new_offset,
        verdict,
    );

    if new_offset == old_offset {
        OffsetFollowOutcome::Unchanged
    } else {
        OffsetFollowOutcome::Changed
    }
}
