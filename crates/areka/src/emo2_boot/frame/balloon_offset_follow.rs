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
//! # キーワード由来の基本位置との排他（要件 4.1／4.3／4.5・design D7）
//!
//! キャラ窓が [`BalloonKeywordBase`] を持つあいだ、本相は**オフセットも基準対も 1 bit も
//! 触らない**——`verdict=keyword-pending` を 1 行記録して抜けるだけである。素材が残る間の
//! 正しい揃えは再導出（`rederive_keyword_balloon_offset`）が新しい実表示寸から**絶対値**で
//! 出すので、追随まで効かせると 1 回の遷移で揃えが二重に動く。
//!
//! 分岐は**本仕様の新規コード側**に置く。再導出側の発火条件（寸の変化）と「経路で絞らない」
//! という同関数の設計判断は 1 文字も変えていない（要件 4.5）。
//!
//! ## 受容した残余（開発者裁定 2026-08-27・要件 4.4 の記録に含む）
//!
//! 丸めの偶然でキャラ窓の物理寸が変わらない遷移では、再導出は寸が変わらないので発火せず、
//! 追随は素材があるので見送る——**どちらも走らず**、揃えの更新が次の寸法変化まで
//! 取り残される。これは塞がない裁定である: ⑴ 条件が二重に稀 ⑵ `verdict=keyword-pending`
//! が記録に残るので沈黙しない ⑶ 次の寸法変化で自己回復する ⑷ 塞ぐには追随の判定を新寸確定後へ
//! 回す二段構えが要り、挿入位置の単純さ（`refresh_scale_report` の直前という 1 点）を失う。
//! 腕の挙動と自己回復は `frame_balloon_offset_keyword_gate_tests.rs` が固定している。
//!
//! # 記録の欄の読み方
//!
//! `new_dpi=0` は「現在の表示 DPI を使える値として読めなかった」ことを表す——
//! [`DPI`] Component が無い腕と、読めたが 0 だった腕（`UnresolvedScale::ZeroCurrentDpi`）の
//! 両方がこの字面になる。どちらも `verdict=unresolved` で `warn!` を伴うため、
//! 行だけで両者を切り分ける必要は無い（切り分けは警告本文が持つ）。
//!
//! 見送りの腕（`verdict=keyword-pending`）も `new_dpi=0` になる——門が [`DPI`] の読取より
//! **前**に立つため、そもそも現在の表示 DPI を読んでいないからである。判定語が違うので
//! 縮退の 2 腕とは行の上で区別が付く。

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::{debug, warn};

use wintf::ecs::{DPI, Point, WindowPos};

use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
use crate::placement::follow::{
    BalloonFollow, BalloonFollowTrigger, OffsetRescale, follow_balloon, rescale_follow_offset,
};
use crate::placement::spawn::BalloonKeywordBase;
use crate::placement::transition_diag::{
    OFFSET_VERDICT_ANCHORED, OFFSET_VERDICT_KEYWORD_PENDING, OFFSET_VERDICT_RESCALED,
    OFFSET_VERDICT_SATURATED, OFFSET_VERDICT_UNCHANGED, OFFSET_VERDICT_UNRESOLVED,
    log_offset_rescale,
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

    // 門（task 6.3・design D7・要件 4.1／4.3／4.5）: キーワード由来の基本位置の素材が
    // 未消費のあいだは、オフセットも基準対も **1 bit も触らず**見送りの判定語だけを記録して
    // 抜ける。素材があるうちは再導出が新しい実表示寸から**絶対値として**正しい揃えを出すので、
    // ここで追随まで効かせると 1 回の遷移で揃えが二重に動く（要件 4.3）。
    //
    // 分岐が**本仕様の新規コード側**に在ることが D7 の要点である——再導出側
    // （`rederive_keyword_balloon_offset`）の発火条件（寸の変化）と「経路で絞らない」設計判断は
    // 1 文字も変えない（要件 4.5）。読取は `DPI` より**前**（design「追随の判断」の流れ図の
    // とおり）——書込にも記録にも先立って抜けるため、見送った遷移では基準の係留すら起きない。
    if world.get::<BalloonKeywordBase>(char_window).is_some() {
        log_offset_rescale(
            world,
            Some(scope),
            base_dpi_field,
            0,
            base.offset,
            old_offset,
            old_offset,
            OFFSET_VERDICT_KEYWORD_PENDING,
        );
        return OffsetFollowOutcome::Unchanged;
    }

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

/// **収束の保証**（design D16・要件 3.1／3.4）——追随でオフセットが変わったのに、続く
/// 窓書込が起きなかった腕で、バルーンを新しいオフセットの位置へ 1 度だけ寄せる。
///
/// # なぜ要るか
///
/// 通常の遷移では [`resize_window_to`](crate::placement::follow::resize_window_to) の手順 6 が
/// 新しいオフセットで随伴追従を出すため、キャラ 1・バルーン 1 の書込で両窓が同時に落ち着く。
/// しかし位置と寸がともに同一だと同関数は**手順 4 のべき等 skip で `false` を返し**
/// （`window_move.rs:337-345`）、手順 6 の追従へ到達しない。この腕を放置すると
/// 「オフセットは直ったのにバルーンは次に何かが動くまで古い位置に居る」という、
/// 本仕様が消しに来た欠陥そのものが残る。
///
/// # 予算（要件 3.4・Adjacent expectations）
///
/// 遷移 1 回あたりの窓書込は キャラ ≤1・バルーン ≤1・別経路 0。本関数が走るのは
/// **キャラ書込が 0 だった腕だけ**ゆえ、バルーン書込 0→1 でも合計は増えない。
/// バルーンは 1 度の書込で最終位置へ行くので中間位置も提示されない。
///
/// # 呼出契約
///
/// 呼び手（[`dpi_phase_with`](super::dpi::dpi_phase_with)）は、当該窓について
/// ⑴ 本モジュールの追随が [`OffsetFollowOutcome::Changed`] を返し、かつ
/// ⑵ 続く反映（`reconcile_window_size`／`reproject_char_window_at_current_size`）が
/// `false` を返した——の 2 条件が揃ったときだけ、**1 遷移・1 スコープにつき 1 度**呼ぶ。
///
/// # 縮退（log-first・[`reproject_char_window_at_current_size`] と同じ 2 分）
///
/// - **entity 破棄済み**: 終了処理の正常終了系ゆえ [`DESPAWNED_SKIP_TAG`] の `debug!`。
/// - **実在するが `WindowPos.position` 不在**（窓生成前）: 真の異常ゆえ `warn!`。
pub(super) fn converge_balloon_after_skipped_write(world: &mut World, char_window: Entity) {
    let Some(pos) = world
        .get::<WindowPos>(char_window)
        .and_then(|wp| wp.position)
    else {
        if world.get_entity(char_window).is_err() {
            debug!(
                entity = ?char_window,
                "{DESPAWNED_SKIP_TAG} balloon offset: キャラ窓 entity が破棄済み → 収束の随伴追従を正常系として打ち切り"
            );
        } else {
            warn!(
                entity = ?char_window,
                "balloon offset: キャラ窓の WindowPos.position が未確定（窓生成前）→ 収束の随伴追従を見送る"
            );
        }
        return;
    };
    // 引き金はキャラ窓を動かす**はずだった**経路そのもの——本相の唯一の実在トリガは
    // `Changed<DPI>` エッジである（design D13 の 1 語＝1 実在トリガ）。書込自身の route は
    // 定義上つねに `BalloonFollow` ゆえ、可視性の遷移ガードの発火可否はここでしか決まらない
    // （通常腕＝`resize_window_to` 手順 6 と**同一の引き金**にすることで、書込が起きた腕と
    // 起きなかった腕でバルーンの落ち着き先が食い違わない）。
    follow_balloon(
        world,
        char_window,
        Point { x: pos.x, y: pos.y },
        BalloonFollowTrigger::Placement(PlacementRoute::DpiReproject),
    );
}
