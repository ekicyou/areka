//! # viewbox_scroll_test — 再描画レス・ダーティ限定描画の決定論カウント檻（task 11・実 pump 通し）
//!
//! design.md「Testing Strategy → Integration Tests → 2. 再描画レス カウント檻
//! （tests/viewbox_scroll_test.rs・実 pump 通し）」の正典述語を、**実配線**
//! （`register_actor` → `present_frame`（装着＋viewbox ダーティ矩形スクロール描画）→
//! `draw_stats`）で檻化する（R3.1/R3.2/R3.5/R10.3）。
//!
//! viewbox_draw.rs の in-source 檻が `ViewboxExecutor` 単体（テストが canvas/window を直結）で
//! 露出帯限定描画を張るのに対し、本テストは結線層（[`present_frame`]）を通した**実 pump**で、
//! actor 別の [`DrawStats`] を [`TextLayerRuntime::draw_stats`] から読み、フレーム間の**増分**を
//! 検査する。時刻は常に注入（`talk_time`）・実時間 sleep 不使用（決定論）。
//!
//! ## 観測する 2 条件（task 11 の観測可能な完了状態）
//!
//! 1. **内容・可視窓とも不変のフレーム → 全統計増分 0**（NoChange gating）: 描画済みの状態で新規
//!    cue を加えず同一注入時刻 `t` で `present_frame` をもう一度呼ぶと、`draw_text_layout_calls`／
//!    `line_layout_creations`／`blits`／`full_clears` の増分がすべて 0。
//! 2. **可視窓が移動するスクロールフレーム → 行描画実行回数の増分が小さい**（確定行を再描画しない）:
//!    typewriter があふれてスクロールが発火するフレームで `draw_text_layout_calls` の増分が
//!    可視行数（＝全域再描画で描かれる行数）より**厳密に小さく**、かつ露出帯交差行の tight bound
//!    以下。`line_layout_creations` の増分も可視行数より小さい（確定行は再生成されない）。
//!
//! ## 負のコントロール（検証の有効性）
//!
//! `ViewboxExecutor` を全域再描画へ戻す改変（可視窓移動フレームでも全可視行を毎回
//! `DrawTextLayout` する）を入れると、スクロールフレームの `draw_text_layout_calls` 増分＝
//! 可視行数となり、条件 2 の `draw_delta < visible_line_count` が落ちる。同時に条件 1 も、
//! 不変フレームで全域を描き直すため増分 0 が破れて落ちる。ゆえに本檻は再描画レス実装のみで green。
//!
//! ## 幾何（決定論・ブロック軸は整数格子で厳密）
//!
//! 既定フォント（ＭＳ ゴシック 12px）→ 行送り pitch = 字の丈 12 ＋ 行間 2 = 14。validrect＝画像全域
//! （物理供給面 100×120）。行 i の下端 = `i×14 + 12` ≤ 120 を満たすのは i = 0..7 の 8 行＝可視行数
//! V＝8（行 7 は 110 ≤ 120）。行 8 の下端 124 > 120 であふれ、1 行ぶん（block_offset −14）
//! スクロールする。ブロック軸の
//! 行寸は layout の `font_height`（12）由来で DirectWrite 実測に依存しない（行内軸の送り幅のみ実測
//! 依存だが、1 行 1 グリフ＋明示改行ゆえ折返しに影響しない）。

use std::cell::RefCell;
use std::rc::Rc;

use areka_emo_text::actor::{
    ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame, spawn_emo_text,
};
use areka_emo_text::sink::EmoTextSink;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::viewbox_draw::DrawStats;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

// ── 幾何定数（決定論・ブロック軸は整数格子） ──────────────────────────────────────────

/// バルーン画像原寸（image px・k=1.0 恒常ゆえ物理供給面と同値）。validrect＝画像全域。
const IMAGE: (u32, u32) = (100, 120);
/// 既定フォント高さ（image px・ＭＳ ゴシック 12px）。
const FONT_H: u32 = 12;
/// 行送りピッチ ＝ 字の丈 12 ＋ 行間 2 ＝ 14（block-axis・整数）。
const PITCH: u32 = 14;
/// 充填フレームで並べる可視行数（あふれ前の上限＝可視行数 V）。
/// 行 i 下端 = i×14+12 ≤ 120（validrect 高）を満たすのは i=0..7 の 8 行（行 7 は 110 ≤ 120・
/// 行 8 は 124 > 120）。旧ピッチ 15 でも 8 行（117 ≤ 120・132 > 120）——**行数は偶然一致する**
/// だけで、根拠の式は 15 の等差から 14 の等差へ変わっている。
const FILL_LINES: usize = 8;
/// 露出帯交差行の tight bound（実測 2）。増分は**ダーティ矩形ごとの交差行数の和**で数える。
/// 内訳は「露出帯の矩形 ∩ 1 行」＋「変化行の矩形 ∩ 1 行」＝ 1 + 1 ——露出帯は概ね 1 行 pitch 高
/// ゆえ交差は「あふれで流入した変化行」だけに限られ、2 枚とも同じその 1 行に交差する。
const EXPOSURE_BAND_DRAW_BOUND: u64 = 3;
/// 全 emit グリフが可視な注入時刻（reveal 時刻は ≤ 0.5s ゆえ 100.0 で必ず全可視＝
/// フレーム間差はスクロール／内容変化のみに帰着し、リビール進行が混ざらない）。
const FULLY_REVEALED: f64 = 100.0;
/// 各行 1 グリフの相異なる全角文字（スクロールで供給面の内容が実際に移動することも担保）。
const LINE_GLYPHS: &[char] = &[
    'あ', 'い', 'う', 'え', 'お', 'か', 'き', 'く', 'け', 'こ', 'さ', 'し',
];

// 幾何の静的檻: 行 i の下端 = i×PITCH + FONT_H（block-axis・整数）。可視 V=FILL_LINES 行が
// validrect 高（IMAGE.1）へちょうど収まり、次の 1 行があふれることをコンパイル時に固定する
// （幾何がずれたらビルドが落ちる＝可視行数 V の前提を守る）。
const _: () = assert!((FILL_LINES as u32 - 1) * PITCH + FONT_H <= IMAGE.1);
const _: () = assert!(FILL_LINES as u32 * PITCH + FONT_H > IMAGE.1);

// ── テスト土台（draw_readback_test / actor.rs runtime_tests と同一方針・headless GPU） ──

/// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World を組む
/// （本番 UI スレッド＝MTA・記憶: areka WUC は MTA スレッドで動く）。
fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視——テストスレッド毎）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

/// emo-present `VisualMount` と同型の予約スロット（surface.rs/actor.rs テストと同型）。
/// 返り値は (window, slot)。
fn spawn_reserved_slot(world: &mut World) -> (Entity, Entity) {
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((
            Name::new("emo-text-layer-slot"),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    world.flush();
    (window, slot)
}

/// テスト用 BalloonModel：validrect＝画像全域（top0/bottom120/left0/right100）・origin (0,0)・
/// font 高さ 12（pitch 15 を明示）・横書き（既定）。1 行 1 グリフゆえ折返し閾値は効かせない。
fn model() -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(Some(0), Some(IMAGE.1 as i32), Some(0), Some(IMAGE.0 as i32)),
        Font::new(None, Some(FONT_H), FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// テスト用 cue。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

/// 1 行（先頭行以外は先に改行マーカー）を emit する。全 cue は at=0.0・duration=0（配送 duration=0
/// ＝全グリフが at=0.0 で即時可視ゆえ FULLY_REVEALED で全可視）。行末尾に改行を付けない＝部分リビール時の空行を作らない
/// （常に全可視で描くため実際には無関係だが、行構造を「1 行 1 グリフ」に厳密化する）。
fn emit_line(sink: &mut EmoTextSink, line_index: usize) {
    if line_index > 0 {
        sink.emit(cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 }));
    }
    sink.emit(cue(
        "0",
        0.0,
        CueCommand::Text(LINE_GLYPHS[line_index].to_string()),
    ));
}

/// 事前 queue 済みメッセージを全て処理し queue が空になった時点で抜ける bounded pump
/// （draw_readback_test / sink.rs の決定論パターン——WM_QUIT は posted メッセージが尽きた後に配送）。
fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ（pump スレッド）。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// フレーム提示（runtime を borrow_mut し present_frame を回して即座に解放する——後続の
/// pump_until_idle は drain handler が runtime を try_borrow_mut するため借用を残さない）。
fn present(runtime: &Rc<RefCell<TextLayerRuntime>>, world: &mut World, t: f64) {
    let mut rt = runtime.borrow_mut();
    present_frame(&mut rt, world, t).expect("present_frame");
}

/// 装着済み actor の決定論観測統計（`Copy` なので借用を残さず値で返す）。
fn stats(runtime: &Rc<RefCell<TextLayerRuntime>>, actor: &ActorKey) -> DrawStats {
    runtime
        .borrow()
        .draw_stats(actor)
        .expect("装着済み actor の draw_stats は Some")
}

// ══ 実 pump 通し: 再描画レス・ダーティ限定描画のカウント檻（R3.1/R3.2/R3.5/R10.3） ══════════

/// 観測可能な完了状態（task 11）: 実 pump（register→present_frame）で actor を駆動し、
/// (1) 内容・可視窓とも不変のフレームで全統計増分が 0（NoChange gating）、(2) あふれ→スクロール
/// フレームで `draw_text_layout_calls` 増分が可視行数より厳密に小さく露出帯交差行の tight bound
/// 以下・`line_layout_creations` 増分も可視行数より小さい（確定行を再描画・再生成しない）を檻化する。
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

    let (mut sink, _handle) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");

    // ── 充填セッション: あふれ前の上限（V=8 行）を emit する ──
    for i in 0..FILL_LINES {
        emit_line(&mut sink, i);
    }
    pump_until_idle();

    // frame 0（初回装着＋全域ダーティ）: 可視 8 行を全域再描画で描く＝可視行数 V の基準
    // （＝スクロールフレームで全域再描画に戻したら描かれる行数）。
    present(&runtime, &mut world, FULLY_REVEALED);
    let fill = stats(&runtime, &actor);
    let visible_line_count = fill.draw_text_layout_calls;
    assert_eq!(
        visible_line_count, FILL_LINES as u64,
        "充填フレームは可視 {FILL_LINES} 行を全域ダーティで描く（可視行数 V の基準）: {fill:?}"
    );
    assert!(
        visible_line_count >= 5,
        "十分な可視行数（5 行以上）で全域再描画との差を際立たせる: {visible_line_count}"
    );
    assert_eq!(
        fill.blits, 0,
        "初回全域ダーティは blit=(0,0)＝全面 CopyResource で blits を増やさない: {fill:?}"
    );
    assert_eq!(fill.full_clears, 0, "充填は FullClear を伴わない: {fill:?}");

    // ── 条件 1（NoChange gating）: 内容・可視窓とも不変（同一注入時刻・新規 cue なし）→ 全統計増分 0 ──
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

    // ── 条件 2（可視窓移動＝スクロールフレーム）: 行 8 を追記して 1 行あふれさせる ──
    let before_scroll = stats(&runtime, &actor);
    emit_line(&mut sink, FILL_LINES); // 行 index 8（下端 8×14+12 = 124 > 120 であふれ）。
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

    // 可視 8 行 fixture 限定の tight check（fixture 非依存の主檻は下部の blit==1＋depth-invariance）:
    // 全域再描画へ戻すと draw_text_layout_calls 増分＝可視行数となり本 assert が落ちる。実 viewbox は
    // 確定行を面内 blit で保持し露出帯 ∪ 変化行しか描かない＝増分は可視行数より厳密に小さい。
    // ※ この `< visible_line_count` は可視 8 行ゆえ成立する fixture 依存の tight check であって、
    //   fixture 非依存の再描画レス決定的証明はテスト末尾の blit==1＋depth-invariance（下部参照）。
    assert!(
        draw_delta < visible_line_count,
        "再描画レス(fixture依存 tight check): スクロールフレームの DrawTextLayout 増分 {draw_delta} は可視行数 {visible_line_count} より厳密に小さい（全域再描画なら {visible_line_count}）"
    );
    // tight bound: 露出帯は概ね 1 行 pitch 高＝交差は「あふれで流入した変化行」だけ。
    // 実測 2 ＝ 露出帯の矩形 ∩ 1 行 ＋ 変化行の矩形 ∩ 1 行（矩形ごとの交差行数の和）。
    assert!(
        draw_delta <= EXPOSURE_BAND_DRAW_BOUND,
        "DrawTextLayout 増分 {draw_delta} は露出帯交差行の tight bound {EXPOSURE_BAND_DRAW_BOUND} 以下（露出帯 ∪ 変化行に限定）"
    );
    // 確定行の行レイアウトは再生成されない（現在行/変化行ぶんのみ・保持機構＝面ピクセル＋blit）。
    assert!(
        create_delta < visible_line_count,
        "確定行は再生成されない: 行レイアウト生成増分 {create_delta} < 可視行数 {visible_line_count}"
    );

    // ── 条件 1（スクロール後も NoChange gating が効く）: 同一注入時刻の再提示で全統計増分 0 ──
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

    // ── 反復スクロール（2 度目のあふれでも同じ廉価な計上＝確定行を保持し続ける）──
    let before_scroll2 = stats(&runtime, &actor);
    emit_line(&mut sink, FILL_LINES + 1); // 行 index 9。
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
        draw_delta2 < visible_line_count,
        "2 度目のスクロールも再描画レス: DrawTextLayout 増分 {draw_delta2} < 可視行数 {visible_line_count}"
    );
    assert!(
        draw_delta2 <= EXPOSURE_BAND_DRAW_BOUND,
        "2 度目のスクロールも露出帯交差行に限定: {draw_delta2} ≤ {EXPOSURE_BAND_DRAW_BOUND}"
    );
    assert!(
        create_delta2 < visible_line_count,
        "2 度目も確定行は再生成されない: 生成増分 {create_delta2} < 可視行数 {visible_line_count}"
    );

    // ── 再描画レスの fixture 非依存な決定的不変（G3・脆弱な `< visible_line_count` に依存しない）──
    // 上の `< visible_line_count` は可視 8 行 fixture でしか成立しない（可視 2〜3 行の小 fixture では
    // draw_delta＝矩形ごとの交差行数の和が可視行数に達し得る）。以下は fixture の可視行数に依らず成立する
    // 再描画レスの決定的証拠であり、これらが本檻の真の負のコントロールである:
    //   (a) blit==+1（上で assert 済み）: viewbox は確定ピクセルを面内 blit で保持する。全域再描画は
    //       保持せず毎フレーム描き直す＝blit 0。全域再描画へ戻すと blits +1 assert が fixture に依らず落ちる。
    //   (b) depth-invariance: スクロール描画/生成増分は**スクロール深さに依らず一定**。確定行を蓄積
    //       再描画する退行なら深い段ほど増分が増える（この不変が破れる）。
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
