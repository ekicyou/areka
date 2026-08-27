//! **キーワード由来の基本位置との排他の門**（areka-P0-balloon-offset-dpi・task 6.3・
//! design D7／D8・要件 4.1／4.3／4.4／4.5）の決定論テスト。
//!
//! # ここが押さえる欠陥
//!
//! キーワードで揃えを指定したスコープには、実表示寸が確定した瞬間に**一度だけ**基本位置を
//! 導出し直すための素材（[`BalloonKeywordBase`]）がキャラ窓へ付く。素材が未消費のまま
//! 拡大率遷移を迎えたとき、追随（本仕様）と再導出（先行仕様）の**両方**が offset を書くと
//! 揃えが 1 回の遷移で二重に動く。要件 4.3 はどちらか一方だけを適用せよと定め、design D7 は
//! **素材があるあいだ追随を見送る**方を採った——再導出は新しい実表示寸から**絶対値として**
//! 導くので、遷移後の正しい揃えは再導出だけで出せるからである。
//!
//! 門は本仕様の新規コード側（[`rescale_balloon_follow_offset`]）にあり、再導出側
//! （`rederive_keyword_balloon_offset`）の発火条件と「経路で絞らない」設計判断には
//! 1 文字も触れていない（要件 4.5）。
//!
//! # 受容した残余（開発者裁定 2026-08-27・要件 4.4 の記録に含む）
//!
//! 丸めの偶然でキャラ窓の物理寸が変わらない遷移では、再導出は**寸が変わらないので発火せず**
//! （`keyword_base.rs` の `old_size == Some(new_size)` の腕）、追随は**素材があるので見送る**
//! ——揃えの更新が次の寸法変化まで取り残される。この腕は塞がない裁定であり、
//! [`the_size_held_transition_leaves_the_alignment_until_the_next_size_change`] が
//! 挙動そのものを固定し、[`the_left_behind_alignment_heals_on_the_next_size_change`] が
//! 「次の寸法変化で自己回復する」ことを固定する（沈黙しないことは判定語の檻が持つ）。
//!
//! # 門を外すと落ちる対（完了条件）
//!
//! - [`the_gate_holds_the_offset_and_the_base_pair_bit_identical`]——門を外すと、素材が
//!   未消費のままオフセットが表示 DPI 比で動き、さらに task 6.2 の収束がバルーンを
//!   実際に書く（＝揃えが動く）。値と書込の両方で赤になる。
//! - [`the_gate_records_the_keyword_pending_verdict_without_a_warning`]——門を外すと
//!   判定語が `rescaled` になって赤になる。
//!
//! ⚠ **寸が変わる腕の最終 offset は門の有無で変わらない**——再導出が絶対値で上書きするため
//! である。ゆえに [`only_the_rederivation_writes_the_offset_when_the_size_changes`] が
//! 赤になるのは**判定語の側**であって値の側ではない。帰属をここで取り違えると
//! 「値が合っているから門は効いている」という誤った根拠に使われる。

use wintf::ecs::WindowPos;
use wintf::ecs::window::SetWindowPosCommand;

use crate::placement::config::BalloonXMode;
use crate::placement::follow::BalloonFollow;
use crate::placement::resolver::{PointPx, SizePx, keyword_balloon_pos};
use crate::placement::spawn::BalloonKeywordBase;
use crate::placement::transition_diag::OFFSET_VERDICT_KEYWORD_PENDING;

use super::test_support::{
    FakeReports, FrameHarness, capture_logs, count_level, s2_monitors_with_work_area,
    s2_work_area_for_dpi,
};
use super::*;

/// 遷移前の拡大率水準（作者基準と等倍）。
const LOW_DPI: u16 = 96;

/// 遷移後の拡大率水準（2 倍＝素材が無ければオフセットも 2 倍になる）。
const HIGH_DPI: u16 = 192;

/// キーワード素材の揃え種別（中央上＝バルーン下端がシェル画像上端に接する）。
const KEYWORD_MODE: BalloonXMode = BalloonXMode::CenterTop;

/// キーワード素材の調整量（作者指定 0＝揃えの式だけを見る）。
const KEYWORD_ADJUST: PointPx = PointPx { x: 0, y: 0 };

/// 当該スコープの追従オフセット（現在値）。
fn offset_of(harness: &FrameHarness, scope: usize) -> PointPx {
    follow_of(harness, scope).offset()
}

/// 当該スコープの追従 Component（基準対まで読む腕で使う）。
fn follow_of(harness: &FrameHarness, scope: usize) -> BalloonFollow {
    harness
        .world
        .get::<BalloonFollow>(harness.char_window(scope))
        .copied()
        .expect("キャラ窓に BalloonFollow がある")
}

/// 当該スコープに素材が残っているか。
fn has_material(harness: &FrameHarness, scope: usize) -> bool {
    harness
        .world
        .get::<BalloonKeywordBase>(harness.char_window(scope))
        .is_some()
}

/// 窓の現寸（`WindowPos.size`）。
fn size_px(harness: &FrameHarness, entity: Entity) -> SizePx {
    let size = harness
        .world
        .get::<WindowPos>(entity)
        .and_then(|wp| wp.size)
        .expect("窓に寸がある");
    SizePx {
        w: size.width,
        h: size.height,
    }
}

/// 指定スコープ・指定種別の窓書込だけを取り出す（`frame_balloon_offset_converge_tests` と同型）。
fn writes_for(writes: &[SetWindowPosCommand], scope: u32, kind: &str) -> Vec<SetWindowPosCommand> {
    writes
        .iter()
        .filter(|cmd| cmd.tag.scope == Some(scope) && cmd.tag.kind == kind)
        .cloned()
        .collect()
}

/// 起動直後の整地（`frame_balloon_offset_converge_tests::settle` と同型・基準を係留する）。
///
/// 素材は整地の**後**に付ける——門は係留の腕より前に立つので、整地中に素材があると
/// 基準が未係留のままになり、以降の遷移が追随の腕へ入らなくなる（探針の退化）。
fn settle(harness: &mut FrameHarness, source: &mut FakeReports) {
    harness.set_monitor_sources_for_dpi(LOW_DPI);
    harness.set_monitor_table_for_dpi(LOW_DPI);
    harness.set_window_dpi(LOW_DPI);
    harness.advance_frame();
    harness.run_placement_phases(source);
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
    for scope in harness.scopes().to_vec() {
        assert!(
            follow_of(harness, scope).base().dpi.is_some(),
            "scope={scope}: 整地で基準が係留されていない（探針が退化している）"
        );
    }
}

/// 全スコープのキャラ窓へキーワード素材を付ける（本番の spawn が `ScopePlacement` から
/// 付けるのと同じ Component をそのまま置く）。
fn attach_material(harness: &mut FrameHarness) {
    for scope in harness.scopes().to_vec() {
        let char_window = harness.char_window(scope);
        harness
            .world
            .entity_mut(char_window)
            .insert(BalloonKeywordBase {
                mode: KEYWORD_MODE,
                adjust: KEYWORD_ADJUST,
            });
    }
}

/// **拡大率だけ**を [`HIGH_DPI`] へ上げる（作業領域は [`LOW_DPI`] のまま＝べき等 skip 腕）。
fn raise_scale_without_moving_the_work_area(harness: &mut FrameHarness) {
    harness.set_monitor_table(s2_monitors_with_work_area(
        HIGH_DPI,
        s2_work_area_for_dpi(LOW_DPI),
    ));
    harness.set_window_dpi(HIGH_DPI);
}

/// 拡大率を上げ、**キャラ窓の実表示寸も倍で報告する**（再導出が発火する通常腕）。
fn raise_scale_with_a_doubled_size_report(harness: &mut FrameHarness, source: &mut FakeReports) {
    for scope in harness.scopes().to_vec() {
        let current = size_px(harness, harness.char_window(scope));
        let target = shell_target(u32::try_from(scope).expect("scope は u32 域")).0;
        source.refresh.insert(
            target,
            (
                u32::try_from(current.w * 2).expect("寸は非負"),
                u32::try_from(current.h * 2).expect("寸は非負"),
            ),
        );
    }
    harness.set_monitor_table_for_dpi(HIGH_DPI);
    harness.set_window_dpi(HIGH_DPI);
}

/// 再導出が出すはずの offset（P5 と同一の式を同じ口から呼ぶ＝幾何を書き写さない）。
fn expected_keyword_offset(harness: &FrameHarness, scope: usize) -> PointPx {
    let char_size = size_px(harness, harness.char_window(scope));
    let balloon_size = size_px(harness, harness.balloon_window(scope));
    keyword_balloon_pos(
        KEYWORD_MODE,
        PointPx { x: 0, y: 0 },
        char_size,
        balloon_size,
        KEYWORD_ADJUST,
    )
    .expect("中央揃えのモードは基本位置を持つ")
}

/// `kind=offset` の観測行だけを取り出す。
fn offset_lines(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("kind=offset")).collect()
}

// ---------------------------------------------------------------------------
// 門そのもの（要件 4.3・design D7）
// ---------------------------------------------------------------------------

/// **完了条件の前半**: 素材が残る遷移では、追随はオフセットも基準対も 1 bit も触らない。
///
/// 門を外すと ⑴ offset が表示 DPI 比で動き ⑵ task 6.2 の収束がバルーンを 1 度書く
/// ——値と書込の両方でこのテストが赤になる（＝揃えが実際に二重に動く対）。
#[test]
fn the_gate_holds_the_offset_and_the_base_pair_bit_identical() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    attach_material(&mut harness);

    let before: Vec<(usize, BalloonFollow)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| (scope, follow_of(&harness, scope)))
        .collect();

    raise_scale_without_moving_the_work_area(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let writes = harness.drain_writes();

    // 揃えが**実際に動いたか**を先に問う——門を外すと収束（task 6.2）がバルーンを 1 度書き、
    // その 1 行がここで見える。値の主張より前に置くのは、摂動したときに「動いた」証跡が
    // 先に出るようにするためである。
    assert!(
        writes.is_empty(),
        "見送った遷移で窓書込が出ている（追随が動いて収束が走った＝揃えが動いた）: {writes:?}"
    );

    for (scope, old) in before {
        let now = follow_of(&harness, scope);
        assert_eq!(
            now.offset(),
            old.offset(),
            "scope={scope}: 素材が残っているのに追随がオフセットを動かした（要件 4.3）"
        );
        assert_eq!(
            now.base(),
            old.base(),
            "scope={scope}: 素材が残っているのに追随が基準対を触った（係留も含めて 1 bit も触らない）"
        );
        assert!(
            has_material(&harness, scope),
            "scope={scope}: 追随の側が素材を消費した（消費は再導出だけの権能・要件 4.5）"
        );
    }
}

/// **陽性の対**: 同じ遷移でも素材が無ければ追随は普通に効く（門が常時発火していない）。
#[test]
fn without_the_material_the_same_transition_still_rescales() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);

    let before: Vec<(usize, PointPx)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| (scope, offset_of(&harness, scope)))
        .collect();

    raise_scale_without_moving_the_work_area(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    for (scope, old_offset) in before {
        assert_ne!(
            offset_of(&harness, scope),
            old_offset,
            "scope={scope}: 素材が無いのに追随が起きていない（門が常時発火している）"
        );
    }
}

/// **判定語**（要件 3.7・4.3・design Error Handling）: 見送りは 1 行の記録を残し、
/// **警告ではない**。
///
/// 門を外すとここは `verdict=rescaled` になって赤になる。
#[test]
fn the_gate_records_the_keyword_pending_verdict_without_a_warning() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    attach_material(&mut harness);

    raise_scale_without_moving_the_work_area(&mut harness);
    harness.advance_frame();
    let logs = capture_logs(|| {
        harness.run_placement_phases(&mut source);
    });

    let lines = offset_lines(&logs);
    assert_eq!(
        lines.len(),
        harness.scopes().len(),
        "追随の観測行がスコープ数と一致しない（1 遷移・1 スコープにつき高々 1 行）: {logs:?}"
    );
    for line in &lines {
        assert!(
            line.contains(&format!("verdict={OFFSET_VERDICT_KEYWORD_PENDING}")),
            "見送りの判定語が記録されていない: {line}"
        );
    }
    assert_eq!(
        count_level(&logs, "WARN"),
        0,
        "見送りは縮退ではないので警告を伴わないはず: {logs:?}"
    );
}

// ---------------------------------------------------------------------------
// 再導出だけが書く（要件 4.1・4.5）
// ---------------------------------------------------------------------------

/// **完了条件の後半**: 素材が残る遷移で寸が変われば、offset を書くのは再導出だけである。
///
/// ⚠ 最終 offset は門の有無で変わらない（再導出が絶対値で上書きするため）。門を外して
/// 赤になるのは**判定語の主張**の側であり、値の主張ではない（module doc の警告を参照）。
#[test]
fn only_the_rederivation_writes_the_offset_when_the_size_changes() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    attach_material(&mut harness);

    raise_scale_with_a_doubled_size_report(&mut harness, &mut source);
    harness.advance_frame();
    let logs = capture_logs(|| {
        harness.run_placement_phases(&mut source);
    });
    let writes = harness.drain_writes();

    for line in offset_lines(&logs) {
        assert!(
            line.contains(&format!("verdict={OFFSET_VERDICT_KEYWORD_PENDING}")),
            "寸が変わる腕でも追随は見送られるはず（再導出と二重に動かさない）: {line}"
        );
    }

    for scope in harness.scopes().to_vec() {
        let scope_u32 = u32::try_from(scope).expect("scope は u32 域");
        assert!(
            !has_material(&harness, scope),
            "scope={scope}: 寸が変わったのに再導出が素材を消費していない（探針の退化）"
        );
        let expected = expected_keyword_offset(&harness, scope);
        assert_eq!(
            offset_of(&harness, scope),
            expected,
            "scope={scope}: 最終 offset が再導出の導出値でない（追随が混ざった）"
        );
        assert_eq!(
            follow_of(&harness, scope).base().offset,
            expected,
            "scope={scope}: 再導出の確立点が基準へ焼き直されていない"
        );
        assert_eq!(
            writes_for(&writes, scope_u32, "balloon").len(),
            1,
            "scope={scope}: バルーンが 1 度だけ書かれていない（中間位置の禁止・要件 3.4）: {writes:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 受容した残余（開発者裁定 2026-08-27・要件 4.4）
// ---------------------------------------------------------------------------

/// **受容した残余の腕**: 素材が未消費のまま**寸が据え置き**の遷移を迎えると、追随も
/// 再導出も走らず、揃えの更新が取り残される。
///
/// これは欠陥の記録ではなく**裁定で受容した挙動の固定**である（要件 4.4・design D7／D8）。
/// 沈黙しないこと（`verdict=keyword-pending` が残ること）は
/// [`the_gate_records_the_keyword_pending_verdict_without_a_warning`] が持つ。
#[test]
fn the_size_held_transition_leaves_the_alignment_until_the_next_size_change() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    attach_material(&mut harness);

    let stale: Vec<(usize, PointPx)> = harness
        .scopes()
        .to_vec()
        .into_iter()
        .map(|scope| (scope, offset_of(&harness, scope)))
        .collect();

    raise_scale_without_moving_the_work_area(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    for (scope, old_offset) in stale {
        assert_eq!(
            offset_of(&harness, scope),
            old_offset,
            "scope={scope}: 受容した残余の腕なのに offset が動いた"
        );
        assert!(
            has_material(&harness, scope),
            "scope={scope}: 寸が据え置きなのに素材が消費された"
        );
        // 揃えは**古いまま**——新しい寸に対する中央揃えとは一致しない（残余そのもの）。
        assert_ne!(
            old_offset,
            expected_keyword_offset(&harness, scope),
            "scope={scope}: 探針が退化している（旧 offset が偶然いまの中央揃えと一致した）"
        );
    }
}

/// **自己回復**: 取り残された揃えは、次に寸が変わった時点で再導出が直す。
#[test]
fn the_left_behind_alignment_heals_on_the_next_size_change() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    settle(&mut harness, &mut source);
    attach_material(&mut harness);

    // 1 度目＝受容した残余の腕（何も起きない）。
    raise_scale_without_moving_the_work_area(&mut harness);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);
    let _residual = harness.drain_writes();

    // 2 度目＝寸が変わる遷移。素材はまだ残っているので再導出が発火する。
    raise_scale_with_a_doubled_size_report(&mut harness, &mut source);
    harness.advance_frame();
    harness.run_placement_phases(&mut source);

    for scope in harness.scopes().to_vec() {
        assert!(
            !has_material(&harness, scope),
            "scope={scope}: 次の寸法変化でも素材が消費されていない（自己回復しない）"
        );
        assert_eq!(
            offset_of(&harness, scope),
            expected_keyword_offset(&harness, scope),
            "scope={scope}: 次の寸法変化で揃えが直っていない"
        );
    }
}
