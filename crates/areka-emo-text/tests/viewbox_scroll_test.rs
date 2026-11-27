//! 再描画レス・ダーティ限定描画の決定論カウント檻（実 pump 通し）。
//!
//! `register_actor` → `present_frame` → `draw_stats` の実配線で、フレーム間の [`DrawStats`] の増分を
//! 検査する。時刻は常に注入（`talk_time`）で実時間 sleep は使わない。`ViewboxExecutor` 単体の
//! 露出帯限定描画の檻は `viewbox_draw` の子の `viewbox_draw_*_tests.rs` が持つ。
//!
//! 1. 内容・可視窓とも不変のフレームは全統計の増分が 0。
//! 2. あふれてスクロールするフレームは `draw_text_layout_calls` の増分が可視行数より小さく、露出帯交差行の
//!    上限以下。`line_layout_creations` の増分も可視行数より小さい（確定行は再生成されない）。
//!
//! 全域再描画へ戻すと、スクロールフレームの増分が可視行数（8）になって条件 2 が落ち、不変フレームでも
//! 描き直すので条件 1 も落ちる。
//!
//! 幾何: 既定フォント（ＭＳ ゴシック 12px）の行送り 14（字の丈 12 ＋ 行間 2）・validrect＝画像全域
//! （供給面 100×120）。行 i の下端 `i×14 + 12` ≤ 120 を満たす 8 行が可視で、行 8（下端 124）があふれて
//! 1 行ぶん（block_offset −14）スクロールする。ブロック軸の行寸は `font_height` 由来で DirectWrite の
//! 実測に依存しない。

#[path = "support/mod.rs"]
mod support;

use dola::time::testing::ms;
use support::present_at;
use support::{make_world_with_gpu, pump_until_idle, spawn_reserved_slot};

use std::cell::RefCell;
use std::rc::Rc;

use areka_emo_text::actor::{
    ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, spawn_emo_text,
};
use areka_emo_text::sink::EmoTextSink;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::viewbox_draw::DrawStats;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue, TimeSpan};
use bevy_ecs::prelude::World;

/// テスト用 cue（共有の部品の間隔つき cue へ本ファイルの間隔を渡す）。
fn cue(actor: &str, at: TimeSpan, command: CueCommand) -> TalkCue {
    support::cue(actor, at, command, ms(0))
}

// ── 幾何定数（決定論・ブロック軸は整数格子） ──────────────────────────────────────────

/// バルーン画像原寸（image px・k=1.0 恒常ゆえ物理供給面と同値）。validrect＝画像全域。
const IMAGE: (u32, u32) = (100, 120);
/// 既定フォント高さ（image px・ＭＳ ゴシック 12px）。
const FONT_H: u32 = 12;
/// 行送りピッチ ＝ 字の丈 12 ＋ 行間 2 ＝ 14（block-axis・整数）。
const PITCH: u32 = 14;
/// 充填フレームで並べる可視行数（あふれ前の上限）。行 7 の下端は 110 ≤ 120、行 8 は 124 > 120。
const FILL_LINES: usize = 8;
/// 露出帯交差行の上限。増分はダーティ矩形ごとの交差行数の和で数える。
/// 実測 2（露出帯の矩形 ∩ 1 行 ＋ 変化行の矩形 ∩ 1 行）に 1 の余裕を見て 3。
const EXPOSURE_BAND_DRAW_BOUND: u64 = 3;
/// 全 emit グリフが可視な注入時刻（reveal 時刻は ≤ 0.5s）。フレーム間差にリビール進行が混ざらない。
const FULLY_REVEALED: TimeSpan = ms(100_000);
/// 各行 1 グリフの相異なる全角文字。
const LINE_GLYPHS: &[char] = &[
    'あ', 'い', 'う', 'え', 'お', 'か', 'き', 'く', 'け', 'こ', 'さ', 'し',
];

// 可視 FILL_LINES 行が validrect 高（IMAGE.1）へちょうど収まり、次の 1 行があふれることをコンパイル時に固定する。
const _: () = assert!((FILL_LINES as u32 - 1) * PITCH + FONT_H <= IMAGE.1);
const _: () = assert!(FILL_LINES as u32 * PITCH + FONT_H > IMAGE.1);

// ── テスト土台（headless GPU） ──

/// validrect＝画像全域・origin (0,0)・font 高さ 12・横書きの BalloonModel。折返し閾値は効かせない。
fn model() -> BalloonModel {
    support::geometry_model(
        (Some(0), Some(0)),
        (None, None),
        (Some(0), Some(IMAGE.1 as i32), Some(0), Some(IMAGE.0 as i32)),
        Some(FONT_H),
        None,
    )
}

/// 1 行（先頭行以外は先に改行マーカー）を emit する。全 cue は at=0.0・duration=0 で即時可視。
/// 行末尾に改行を付けず、行構造を「1 行 1 グリフ」に保つ。
fn emit_line(sink: &mut EmoTextSink, line_index: usize) {
    if line_index > 0 {
        sink.emit(cue("0", ms(0), CueCommand::NewLine { ratio: 1.0 }));
    }
    sink.emit(cue(
        "0",
        ms(0),
        CueCommand::Text(LINE_GLYPHS[line_index].to_string()),
    ));
}

/// フレーム提示。後続の `pump_until_idle` の drain handler が runtime を `try_borrow_mut` するので、借用を残さない。
fn present(runtime: &Rc<RefCell<TextLayerRuntime>>, world: &mut World, t: TimeSpan) {
    let mut rt = runtime.borrow_mut();
    present_at(&mut rt, world, t).expect("present_frame");
}

/// 装着済み actor の決定論観測統計。
fn stats(runtime: &Rc<RefCell<TextLayerRuntime>>, actor: &ActorKey) -> DrawStats {
    runtime
        .borrow()
        .draw_stats(actor)
        .expect("装着済み actor の draw_stats は Some")
}

// ══ 実 pump 通し: 再描画レス・ダーティ限定描画のカウント檻 ══════════

/// 不変のフレームは全統計増分 0（NoChange gating）、あふれてスクロールするフレームは描画増分が
/// 露出帯交差行の上限以下で、確定行を再描画・再生成しない。
#[test]
fn real_pump_scroll_redraws_only_dirty_and_unchanged_frames_touch_nothing() {
    let mut world = make_world_with_gpu();
    let (window, slot) = spawn_reserved_slot(&mut world);

    let actor = ActorKey::from("0");
    let model = model();
    let binding = TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE);
    let resolved = ResolvedBalloonText::resolve(&model, binding.image_size);
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    runtime
        .borrow_mut()
        .register_actor(actor.clone(), binding, resolved);

    let (mut sink, _handle) = spawn_emo_text(Rc::clone(&runtime));

    // ── 充填: あふれ前の上限（8 行）を emit する ──
    for i in 0..FILL_LINES {
        emit_line(&mut sink, i);
    }
    pump_until_idle();

    // 初回装着＋全域ダーティ: 可視 8 行を全域再描画で描く（全域再描画に戻したときの行数の基準）。
    present(&runtime, &mut world, FULLY_REVEALED);
    let fill = stats(&runtime, &actor);
    let visible_line_count = fill.draw_text_layout_calls;
    assert_eq!(
        visible_line_count, FILL_LINES as u64,
        "充填フレームは可視 {FILL_LINES} 行を全域ダーティで描く（可視行数 V の基準）: {fill:?}"
    );
    assert_eq!(
        fill.blits, 0,
        "初回全域ダーティは blit=(0,0)＝全面 CopyResource で blits を増やさない: {fill:?}"
    );
    assert_eq!(fill.full_clears, 0, "充填は FullClear を伴わない: {fill:?}");

    // ── 条件 1: 内容・可視窓とも不変（同一注入時刻・新規 cue なし）→ 全統計増分 0 ──
    present(&runtime, &mut world, FULLY_REVEALED);
    let unchanged = stats(&runtime, &actor);
    assert_eq!(
        unchanged.draw_text_layout_calls, fill.draw_text_layout_calls,
        "NoChange フレームは DrawTextLayout を発生させない（増分 0）"
    );
    assert_eq!(
        unchanged.line_layout_creations, fill.line_layout_creations,
        "NoChange フレームは行レイアウト生成を発生させない（増分 0）"
    );
    assert_eq!(
        unchanged.blits, fill.blits,
        "NoChange フレームは面内 blit を発生させない（増分 0）"
    );
    assert_eq!(
        unchanged.full_clears, fill.full_clears,
        "NoChange フレームは FullClear を発生させない（増分 0）"
    );

    // ── 条件 2: 行 8 を追記して 1 行あふれさせる ──
    let before_scroll = stats(&runtime, &actor);
    emit_line(&mut sink, FILL_LINES);
    pump_until_idle();
    present(&runtime, &mut world, FULLY_REVEALED);
    let after_scroll = stats(&runtime, &actor);

    assert_eq!(
        after_scroll.blits,
        before_scroll.blits + 1,
        "あふれ 1 行で面内 blit がちょうど 1 回発生する（可視窓が移動した決定論的証拠）"
    );
    assert_eq!(
        after_scroll.full_clears, before_scroll.full_clears,
        "スクロールは全域透明フラッシュ（FullClear）ではない"
    );

    let draw_delta = after_scroll.draw_text_layout_calls - before_scroll.draw_text_layout_calls;
    let create_delta = after_scroll.line_layout_creations - before_scroll.line_layout_creations;

    assert!(
        draw_delta <= EXPOSURE_BAND_DRAW_BOUND,
        "DrawTextLayout 増分 {draw_delta} は露出帯交差行の上限 {EXPOSURE_BAND_DRAW_BOUND} 以下（露出帯 ∪ 変化行に限定）"
    );
    assert!(
        create_delta < visible_line_count,
        "確定行は再生成されない: 行レイアウト生成増分 {create_delta} < 可視行数 {visible_line_count}"
    );

    // ── スクロール後も同一注入時刻の再提示で全統計増分 0 ──
    present(&runtime, &mut world, FULLY_REVEALED);
    let after_idle = stats(&runtime, &actor);
    assert_eq!(
        after_idle.draw_text_layout_calls, after_scroll.draw_text_layout_calls,
        "スクロール後の不変フレームも DrawTextLayout 増分 0"
    );
    assert_eq!(
        after_idle.line_layout_creations, after_scroll.line_layout_creations,
        "スクロール後の不変フレームも行レイアウト生成 増分 0"
    );
    assert_eq!(
        after_idle.blits, after_scroll.blits,
        "スクロール後の不変フレームも blit 増分 0"
    );
    assert_eq!(
        after_idle.full_clears, after_scroll.full_clears,
        "スクロール後の不変フレームも FullClear 増分 0"
    );

    // ── 2 度目のあふれでも同じ計上 ──
    let before_scroll2 = stats(&runtime, &actor);
    emit_line(&mut sink, FILL_LINES + 1);
    pump_until_idle();
    present(&runtime, &mut world, FULLY_REVEALED);
    let after_scroll2 = stats(&runtime, &actor);

    assert_eq!(
        after_scroll2.blits,
        before_scroll2.blits + 1,
        "2 度目のあふれも面内 blit がちょうど 1 回"
    );
    let draw_delta2 = after_scroll2.draw_text_layout_calls - before_scroll2.draw_text_layout_calls;
    let create_delta2 = after_scroll2.line_layout_creations - before_scroll2.line_layout_creations;
    assert!(
        draw_delta2 <= EXPOSURE_BAND_DRAW_BOUND,
        "2 度目のスクロールも露出帯交差行に限定: {draw_delta2} ≤ {EXPOSURE_BAND_DRAW_BOUND}"
    );
    assert!(
        create_delta2 < visible_line_count,
        "2 度目も確定行は再生成されない: 生成増分 {create_delta2} < 可視行数 {visible_line_count}"
    );

    // 露出帯の上限 3 と `create_delta < visible_line_count` は可視 8 行のこの fixture 専用の数。
    // 可視行数に依らない証拠は 2 つ: blit が +1（全域再描画は確定ピクセルを保持せず blit 0）と、
    // 描画・生成の増分がスクロールの深さに依らず一定（確定行を蓄積再描画すると深いほど増える）。
    assert_eq!(
        draw_delta, draw_delta2,
        "スクロール描画増分は深さに依らず一定＝確定行を蓄積再描画しない（fixture 非依存の再描画レス不変）: 1 度目 {draw_delta} / 2 度目 {draw_delta2}"
    );
    assert_eq!(
        create_delta, create_delta2,
        "確定行の再生成なしも深さに依らず一定（fixture 非依存）: 1 度目 {create_delta} / 2 度目 {create_delta2}"
    );

    sink.close();
    pump_until_idle();
}
