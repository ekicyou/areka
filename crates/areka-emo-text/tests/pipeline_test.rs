//! # pipeline_test — 受信から可視化決定までの通しパイプライン検証（task 10.1・R2.4/R2.5/R3.5）
//!
//! cue 受信（実 `EmoTextSink`→実 `spawn_ui` channel）→状態適用（`TextLayerState`）→
//! レイアウト決定（`LayoutEngine::layout`）→可視窓決定（`LayoutEngine::visible_window`）
//! の通しシナリオを、**実描画なし**（DirectWrite/D2D/COM/GPU 不使用・metrics は
//! [`FixedMetrics`] 注入）の直結経路で、注入時刻列に対して決定論的に検証する
//! （design.md「Testing Strategy > Integration Tests」1. 結線パイプライン が正典）。
//!
//! - **R2.4**: cue 列→行/グリフ状態の遷移が実描画を伴わない純粋な形で通しに実行できる。
//! - **R2.5**: 同一 cue 列＋同一入力条件→状態遷移結果（→レイアウト・可視窓）が一致する。
//! - **R3.5**: 同一 cue 列＋同一注入時刻列→各時刻の可視グリフ数（→可視窓）が一致する。
//!
//! パイプラインの各インスタンスは専用スレッド（＝そのインスタンスの UI/pump スレッド）
//! 上で実 channel（`spawn_ui`）を駆動し、cue の emit はワーカースレッドから行う
//! （受信端はワーカー側・R1.3 の形を踏襲）。時刻は常に注入（talk 起点相対秒）で、
//! 実時間 sleep／`Instant` 不使用（決定論）。
//!
//! FP 誤差を排するため char_wait・cue 時刻・注入時刻はすべて 2 の冪（0.25 の倍数）を
//! 基本とする（state.rs の檻と同方針）。

use std::cell::RefCell;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::rc::Rc;

use areka_emo_text::layout::{FixedMetrics, LayoutEngine, PositionedLine, VisibleWindow};
use areka_emo_text::region::TextRegion;
use areka_emo_text::sink::{handle_text_msg, EmoTextSink, TextMsg};
use areka_emo_text::state::{TextLayerConfig, TextLayerState};
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{
    parse_str, BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_parsers::charset::{decode, DefaultEncoding};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use areka_sakura::TextSink;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf_winmsg_executor::{FilterResult, MessageLoop};

// ── パイプライン土台（実 channel・実 spawn_ui・専用 pump スレッド） ──────────────────

/// テスト用 cue 生成ヘルパ（state.rs／sink.rs の檻と同型）。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
    }
}

/// FP 丸めに依存しない検証用 config（char_wait=0.25 は 2 の冪＝正確表現・state.rs と同方針）。
fn exact_config() -> TextLayerConfig {
    TextLayerConfig {
        char_wait: 0.25,
        ..TextLayerConfig::default()
    }
}

/// 事前 queue 済みメッセージを全て処理し、queue が空になった時点で抜ける bounded pump
/// （sink.rs の檻と同型——`WM_QUIT` は posted メッセージが尽きた後にのみ配送されるため、
/// drain タスクは queue 済み cue を全て処理してから復帰する。決定論・ハングしない）。
fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけの
    // 無害な Win32 呼び出し（呼び出しスレッド＝pump スレッド）。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// 受信から状態適用までを実 channel で通す 1 インスタンス分のパイプライン実行。
///
/// 専用スレッド（＝このインスタンスの UI/pump スレッド）上で:
/// 1. `spawn_ui` の drain（handler は [`handle_text_msg`] へ委譲し `TextLayerState::apply_cue`
///    で状態適用）を起動し、
/// 2. ワーカースレッドから [`EmoTextSink::emit`] で cue 列を FIFO 送出（受信端は
///    ワーカー側・R1.3 の形）し、
/// 3. `close()`→pump で全 cue を処理し切り、適用済み [`TextLayerState`] を返す。
///
/// 描画（DirectWrite/D2D/COM）は一切呼ばない——状態適用まで（レイアウト・可視窓の
/// 決定は呼び出し側が純粋層 API で行う）。
fn run_channel_pipeline(cues: Vec<TalkCue>, config: TextLayerConfig) -> TextLayerState {
    std::thread::spawn(move || {
        let state = Rc::new(RefCell::new(TextLayerState::default()));
        let shared = Rc::clone(&state);
        let (tx, _detached_drain) = areka_actor::spawn_ui(
            "emo-text-pipeline-test",
            move |msg: TextMsg| -> Result<ControlFlow<()>, String> {
                handle_text_msg(msg, |cue| {
                    shared.borrow_mut().apply_cue(&cue, &config);
                    Ok(())
                })
            },
        )
        .expect("spawn_ui on the pipeline pump thread must succeed");
        let sink = EmoTextSink::new(tx);

        // ワーカースレッドから emit（受信端はワーカー側・単一 worker ゆえ FIFO 決定論）。
        let mut worker_sink = sink.clone();
        std::thread::spawn(move || {
            for c in cues {
                worker_sink.emit(c);
            }
        })
        .join()
        .expect("worker emit thread must not panic");

        sink.close();
        pump_until_idle();

        let out = state.borrow().clone();
        out
    })
    .join()
    .expect("pipeline pump thread must not panic")
}

/// channel を介さない直結適用の参照インスタンス（channel が cue を変形しないことの対照）。
fn run_direct_pipeline(cues: &[TalkCue], config: &TextLayerConfig) -> TextLayerState {
    let mut state = TextLayerState::default();
    for c in cues {
        state.apply_cue(c, config);
    }
    state
}

// ── 観測（状態→レイアウト決定→可視窓決定の純粋な下流・描画なし） ──────────────────

/// 1 観測点の可視化決定結果（actor×注入時刻ごとの可視グリフ数・行列・可視窓）。
#[derive(Clone, Debug, PartialEq)]
struct Observation {
    actor: ActorKey,
    t: f64,
    visible: usize,
    lines: Vec<PositionedLine>,
    window: VisibleWindow,
}

/// 状態に対し注入時刻列で可視化決定（visible_glyphs→layout→visible_window）を通しで
/// 評価し、観測列を返す（[`FixedMetrics`] 注入＝描画なしの決定論仮想 metrics・R4.5）。
fn observe(
    state: &TextLayerState,
    actors: &[ActorKey],
    probe_times: &[f64],
    region: &TextRegion,
    mode: WritingMode,
    font_height: f32,
) -> Vec<Observation> {
    let mut out = Vec::new();
    for actor in actors {
        let items = state
            .actor_state(actor)
            .map(|s| s.items().to_vec())
            .unwrap_or_default();
        for &t in probe_times {
            let visible = state.visible_glyphs(actor, t);
            let lines =
                LayoutEngine::layout(&items, visible, region, mode, font_height, &FixedMetrics);
            let window = LayoutEngine::visible_window(&lines, region, mode);
            out.push(Observation {
                actor: actor.clone(),
                t,
                visible,
                lines,
                window,
            });
        }
    }
    out
}

/// テスト用 BalloonModel 生成ヘルパ（layout.rs の檻と同型・validrect は top,bottom,left,right）。
fn model_rect(
    origin: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(origin.0, origin.1),
        WordWrapPoint::new(None, None),
        ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
    )
}

/// 横書きシナリオ共通の領域（validrect top0/bottom36・font 10 → pitch 13 で 3 行まで収まり、
/// 4 行目の下端 49 > 36 であふれ発火——layout.rs の檻と同一幾何）。
fn horizontal_region() -> TextRegion {
    TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(36), Some(0), Some(400))),
        (400, 224),
        WritingMode::HorizontalTb,
    )
}

// ══ R2.4/R2.5/R3.5: 同一 cue 列・同一注入時刻列→可視グリフ数と可視窓が常に一致 ══

/// 代表シナリオ（actor 複数・Text/NewLine/Clear・あふれ込み）を実 channel の独立
/// インスタンス 2 本＋直結適用の参照 1 本へ同一 cue 列で流し、同一注入時刻列に対する
/// 観測列（可視グリフ数・行列・可視窓）が**全観測点で完全一致**することを檻化する。
///
/// 一致だけの空虚な檻にしないため、末尾時刻の絶対値（actor 0 のあふれ発火窓・
/// actor 1 の Clear 後可視数）も同時に固定する。
#[test]
fn same_cues_and_times_yield_identical_visible_counts_and_windows_across_instances() {
    let config = exact_config();
    // 代表 cue 列: actor "0"（全角・明示改行 4 行→あふれ）と actor "1"（半角・Clear 割込み）。
    let cues = vec![
        cue("0", 0.0, CueCommand::Text("あい".into())),
        cue("1", 0.0, CueCommand::Text("abcd".into())),
        cue("0", 0.5, CueCommand::NewLine { ratio: 1.0 }),
        cue("0", 0.5, CueCommand::Text("うえ".into())),
        cue("1", 0.6, CueCommand::Clear),
        cue("1", 0.75, CueCommand::Text("xyz".into())),
        cue("0", 1.0, CueCommand::NewLine { ratio: 1.0 }),
        cue("0", 1.0, CueCommand::Text("お".into())),
        cue("0", 1.5, CueCommand::NewLine { ratio: 1.0 }),
        cue("0", 1.5, CueCommand::Text("か".into())),
        cue("1", 1.75, CueCommand::NewLine { ratio: 0.5 }),
        cue("1", 1.75, CueCommand::Text("!".into())),
    ];
    // 注入時刻列（2 の冪グリッド・cue 境界とリビール境界を跨ぐ 13 点）。
    let probe_times: Vec<f64> = (0..=12).map(|i| i as f64 * 0.25).collect();
    let actors = [ActorKey::from("0"), ActorKey::from("1")];
    let region = horizontal_region();

    // 実 channel の独立インスタンス 2 本（各自の pump スレッド）＋直結参照 1 本。
    let via_channel_a = run_channel_pipeline(cues.clone(), config);
    let via_channel_b = run_channel_pipeline(cues.clone(), config);
    let direct = run_direct_pipeline(&cues, &config);

    let trace_a = observe(
        &via_channel_a,
        &actors,
        &probe_times,
        &region,
        WritingMode::HorizontalTb,
        10.0,
    );
    let trace_b = observe(
        &via_channel_b,
        &actors,
        &probe_times,
        &region,
        WritingMode::HorizontalTb,
        10.0,
    );
    let trace_direct = observe(
        &direct,
        &actors,
        &probe_times,
        &region,
        WritingMode::HorizontalTb,
        10.0,
    );

    // R2.5/R3.5: 同一 cue 列＋同一注入時刻列→全観測点（可視数・行列・可視窓）が常に一致。
    assert_eq!(
        trace_a, trace_b,
        "実 channel の独立インスタンス間で可視グリフ数・可視窓が一致する"
    );
    // channel 経由と直結適用が一致＝搬送路は cue を変形しない（受信→状態適用の忠実性）。
    assert_eq!(
        trace_a, trace_direct,
        "channel 経由と直結適用で可視化決定が一致する"
    );

    // 絶対値の檻（末尾時刻 t=3.0）: actor 0 は 4 行あふれ→1 行スキップ・オフセット −13。
    let last_of = |trace: &[Observation], actor: &str| -> Observation {
        trace
            .iter()
            .filter(|o| o.actor == ActorKey::from(actor))
            .next_back()
            .expect("observation exists")
            .clone()
    };
    let a0 = last_of(&trace_a, "0");
    assert_eq!(a0.visible, 6, "actor 0 は全 6 グリフがリビール済み");
    assert_eq!(a0.lines.len(), 4);
    assert_eq!(
        a0.window,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -13.0
        },
        "actor 0 は 4 行あふれで縦スクロール（横書き）"
    );
    // actor 1 は Clear 後の "xyz"＋"!" のみ（Clear 前の "abcd" は破棄）・あふれなし。
    let a1 = last_of(&trace_a, "1");
    assert_eq!(a1.visible, 4, "actor 1 は Clear 後の 4 グリフのみ可視");
    assert_eq!(a1.lines.len(), 2);
    assert_eq!(
        a1.window,
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        }
    );
}

// ══ R3.5（進行の絶対値）: 注入時刻列に沿った typewriter 進行が可視窓決定まで貫通する ══

/// リビール進行（注入時刻駆動）が可視グリフ数→レイアウト行数→可視窓決定まで通しで
/// 効くことを、手計算の期待表で檻化する。あふれ発火は「最後のグリフがリビールされた
/// 注入時刻」に正確に一致して起きる（時刻を無視する変異はここで死ぬ）。
///
/// 幾何: validrect bottom36・font 10 → pitch 13。行構成（全量時）: L0"あい" L1"うえ"
/// L2"お" L3"か"（top 0/13/26/39・下端 10/23/36/49）。リビール時刻:
/// r=[0.0, 0.25, 0.5, 0.75, 1.0, 1.5]（char_wait 0.25・chunk at が下限）。
#[test]
fn typewriter_reveal_drives_layout_and_window_through_the_pipeline() {
    let config = exact_config();
    let cues = vec![
        cue("0", 0.0, CueCommand::Text("あい".into())),
        cue("0", 0.25, CueCommand::NewLine { ratio: 1.0 }),
        cue("0", 0.5, CueCommand::Text("うえ".into())),
        cue("0", 0.75, CueCommand::NewLine { ratio: 1.0 }),
        cue("0", 1.0, CueCommand::Text("お".into())),
        cue("0", 1.0, CueCommand::NewLine { ratio: 1.0 }),
        cue("0", 1.5, CueCommand::Text("か".into())),
    ];
    let state = run_channel_pipeline(cues, config);
    let region = horizontal_region();
    let actor = ActorKey::from("0");

    // 期待表: (注入時刻, 可視グリフ数, 行数, 先頭可視行, ブロック軸オフセット)。
    // 改行マーカーは可視 prefix 内で即時反映されるため空行も行数に現れる（R2.2 後出し優先）。
    let expected: &[(f64, usize, usize, usize, f32)] = &[
        (0.0, 1, 1, 0, 0.0),   // r_0=0.0: "あ" のみ・1 行
        (0.25, 2, 2, 0, 0.0),  // "あい"＋改行の空行（下端 23 ≤ 36）
        (0.5, 3, 2, 0, 0.0),   // "う" が L1 へ
        (0.75, 4, 3, 0, 0.0),  // "うえ"＋改行の空行（下端 36＝境界ちょうど・非発火）
        (0.99, 4, 3, 0, 0.0),  // r_4=1.0 の直前——まだあふれない
        (1.0, 5, 4, 1, -13.0), // "お" リビール＋改行の空行（下端 49 > 36）→ あふれ発火
        (1.5, 6, 4, 1, -13.0), // "か" リビール（可視窓は据え置き）
        (100.0, 6, 4, 1, -13.0), // 末尾到達後は飽和
    ];
    for &(t, visible, line_count, first, offset) in expected {
        let got_visible = state.visible_glyphs(&actor, t);
        assert_eq!(got_visible, visible, "t={t}: 可視グリフ数");
        let items = state
            .actor_state(&actor)
            .expect("actor state exists")
            .items()
            .to_vec();
        let lines = LayoutEngine::layout(
            &items,
            got_visible,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
        );
        assert_eq!(lines.len(), line_count, "t={t}: 行数");
        let window = LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb);
        assert_eq!(
            window,
            VisibleWindow {
                first_visible_line: first,
                block_offset: offset
            },
            "t={t}: 可視窓"
        );
    }
}

// ══ R2.3 経路の通し: リビール中の Clear が可視化決定まで貫通する ══

/// Clear が未リビール分も含めて全消去し（後出し優先）、以降の可視化決定（レイアウト・
/// 可視窓）が新テキストのみで決まることを通しで檻化する。Clear 後の新 chunk のリビール
/// は旧 tail に影響されず自身の at 起点で始まる。
#[test]
fn clear_mid_reveal_resets_visibility_and_window_through_the_pipeline() {
    let config = exact_config();
    let cues = vec![
        // r=[0.0, 0.25, 0.5, 0.75] のうち後半 2 グリフが未リビールの時刻で Clear が届く。
        cue("0", 0.0, CueCommand::Text("あいうえ".into())),
        cue("0", 0.3, CueCommand::Clear),
        cue("0", 0.75, CueCommand::Text("xy".into())),
    ];
    let state = run_channel_pipeline(cues, config);
    let region = horizontal_region();
    let actor = ActorKey::from("0");

    // 状態の正本: Clear 前の 4 グリフ（未リビール分含む）は破棄され "xy" のみが残る。
    let items = state
        .actor_state(&actor)
        .expect("actor state exists")
        .items()
        .to_vec();
    assert_eq!(items.len(), 2, "Clear で旧テキストは未リビール分も含め全消去");

    // 新 chunk のリビールは自身の at=0.75 起点: r=[0.75, 1.0]。
    let expected: &[(f64, usize)] = &[(0.5, 0), (0.74, 0), (0.75, 1), (1.0, 2), (100.0, 2)];
    for &(t, visible) in expected {
        assert_eq!(state.visible_glyphs(&actor, t), visible, "t={t}: 可視グリフ数");
    }

    // 可視 0 の時刻では行も生まれず、可視窓は既定（描画対象なし）。
    let lines_empty = LayoutEngine::layout(
        &items,
        state.visible_glyphs(&actor, 0.5),
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
    );
    assert!(lines_empty.is_empty(), "Clear 直後（新 chunk リビール前）は行なし");
    assert_eq!(
        LayoutEngine::visible_window(&lines_empty, &region, WritingMode::HorizontalTb),
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        }
    );

    // 全量リビール後も 1 行に収まり、あふれは発火しない。
    let lines_full = LayoutEngine::layout(
        &items,
        state.visible_glyphs(&actor, 100.0),
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
    );
    assert_eq!(lines_full.len(), 1);
    assert_eq!(
        LayoutEngine::visible_window(&lines_full, &region, WritingMode::HorizontalTb),
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        }
    );
}

// ══ 縦書き（9.1 fixture 変種）: 縦書き解決→横スクロールの可視窓決定まで通しで決定論 ══

/// fixture 変種の配置（design.md File Structure Plan 正典・vertical_fixture_test.rs と同一）。
fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("fixtures")
        .join("emo2-vertical")
}

/// fixture 変種ファイルを読み、charset 宣言に従いデコードする（9.1 の檻と同一規約）。
fn read_decoded(name: &str) -> String {
    let path = fixture_dir().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "fixture 変種 {} の読取に失敗した（task 9.1 の観測 fixture が必要）: {e}",
            path.display()
        )
    });
    decode(&bytes, DefaultEncoding::Utf8)
}

/// 9.1 縦書き fixture 変種（2 層マージ済み）で writing_mode 解決→領域解決→受信→状態適用→
/// レイアウト→可視窓決定を通し、横方向スクロール（vertical_rl＝内容が右へ）の可視窓が
/// 注入時刻に対して決定論的に決まることを檻化する。
///
/// 幾何（vertical_fixture_test.rs の実測値）: validrect (36,46)-(356,168)・書字開始角
/// (356,46)・font 10 → pitch 13。明示改行で 25 列（列 i の左端 = 346−13i）を作ると、
/// 25 列目の左端 34 < validrect.left 36 であふれ発火→1 列スキップ・オフセット +13。
#[test]
fn vertical_fixture_pipeline_scrolls_horizontally_and_is_deterministic() {
    let merged = parse_str(
        &read_decoded("descript.txt"),
        Some(&read_decoded("balloons0s.txt")),
    );
    let mode = WritingMode::resolve(&merged);
    assert_eq!(mode, WritingMode::VerticalRl, "fixture 変種は縦書きへ解決される");
    let region = TextRegion::resolve(&merged, (400, 224), mode);

    // 25 列（各列 全角 1 グリフ・明示改行区切り）。cue 時刻は 0.25 グリッド＝
    // グリフ i のリビール時刻 r_i = 0.25×i（char_wait 0.25・at が下限）。
    let config = exact_config();
    let mut cues = Vec::new();
    for i in 0..25 {
        let at = i as f64 * 0.25;
        if i > 0 {
            cues.push(cue("0", at, CueCommand::NewLine { ratio: 1.0 }));
        }
        cues.push(cue("0", at, CueCommand::Text("あ".into())));
    }

    let state_a = run_channel_pipeline(cues.clone(), config);
    let state_b = run_channel_pipeline(cues, config);
    let actors = [ActorKey::from("0")];
    let probe_times = [0.0, 1.0, 3.0, 6.0, 10.0];

    let trace_a = observe(&state_a, &actors, &probe_times, &region, mode, 10.0);
    let trace_b = observe(&state_b, &actors, &probe_times, &region, mode, 10.0);
    // R2.5/R3.5: 縦書きでも同一 cue 列＋同一注入時刻列→可視数・可視窓が常に一致。
    assert_eq!(trace_a, trace_b, "縦書き fixture 経路でも観測列が一致する");

    let actor = ActorKey::from("0");
    // リビール途中（t=1.0・可視 5）: 6 列目（改行の空列）まで・左端 281 ≥ 36 → 非発火。
    assert_eq!(state_a.visible_glyphs(&actor, 1.0), 5);
    let at_1s = trace_a.iter().find(|o| o.t == 1.0).expect("t=1.0 の観測");
    assert_eq!(at_1s.lines.len(), 6);
    assert_eq!(
        at_1s.window,
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        },
        "リビール途中はまだあふれない"
    );

    // 全量リビール（t=10.0・可視 25）: 25 列目の左端 34 < 36 → 横スクロール発火
    // （vertical_rl＝内容が右へ＝ブロック軸オフセット +13）。
    assert_eq!(state_a.visible_glyphs(&actor, 10.0), 25);
    let at_10s = trace_a.iter().find(|o| o.t == 10.0).expect("t=10.0 の観測");
    assert_eq!(at_10s.lines.len(), 25);
    assert_eq!(
        at_10s.window,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: 13.0
        },
        "縦書きのあふれは横スクロール（内容が右へ）"
    );
}
