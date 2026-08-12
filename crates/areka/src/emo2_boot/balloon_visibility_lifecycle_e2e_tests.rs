// =============================================================================
// 会話終了観測から判断中核までの端から端（task 6.6 の 4 番目の項）
//
// 既存の 2 本は鎖の別々の区間しか見ていない:
//   - `talk_lifecycle_tests.rs`（task 6.3）は**送出側**——台本の占有終端が sink から出ること。
//   - `frame_attach_tests.rs::talk_playback_reaches_emo2_wiring_lifecycle_receiver`（task 4.1）は
//     **受信端まで**——信号がフレーム配線の受け口へ届くこと。
// どちらも「届いた信号が判断中核の計測起点になる」ところを見ていない。ここが繋ぎ目である。
//
// 実台本を compile して実 `CuePlayer` で再生し、実 `BalloonLifecycleSink` から mpsc を通し、
// 本番の相関数を 1 フレーム回して、判断中核が持つ占有終端と、その終端を起点に確立した計測が
// 台本の占有区間の終端と一致することを主張する。
//
// 表示層は headless（GPU 資源なし）で足りる——本項が見るのは時刻の鎖であって描画ではない。
// 再生は注入時刻のみで駆動し、実時間の待機は用いない（Requirements 4.9 / 9.2 / 9.3）。
// =============================================================================

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc;

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_sakura::ActorKey;
use areka_sakura::sysvar::SystemVarSnapshot;
use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
use bevy_ecs::world::World;
use dola::cue::{CueCommand, CuePlayer, TalkCue};
use wintf::ecs::FrameTime;

use crate::emo2_boot::assets::{BootAssets, LoopTables};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::move_cue::MoveDirective;
use crate::emo2_boot::talk_clock::TalkClock;
use crate::emo2_boot::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};
use crate::emo2_boot::target_map::balloon_target;
use crate::input_events::balloon::BalloonWiring;
use crate::placement::test_support::{LogEvent, capture_logs};

use super::{configured_timeout_secs, run_balloon_visibility_phase};

/// 表示資産を 1 つも持たない `BootAssets`（相は資産を読まない——読むのは `balloon_models` だけ）。
fn empty_assets() -> BootAssets {
    BootAssets {
        shells: Vec::new(),
        balloons: Vec::new(),
        resolver: SurfaceResolver::new(BTreeMap::new()),
        static_binds: BindSet::default(),
        bind_resolver: BindResolver::empty(),
        loop_tables: LoopTables {
            shell: AnimationTable::empty(),
            balloon: BTreeMap::new(),
        },
        shell_author_dpi: 96,
        balloon_author_dpi: 96,
    }
}

/// 台本の占有終端が判断中核の計測起点になる（Requirement 4.1 の端から端）。
///
/// 待機（`\w9`）を含む台本を使う——占有終端が最後の文字の出現時刻より**後**になる形であり、
/// 「最後の文字が出た時刻」を起点に採る実装ではここが必ず食い違う。
///
/// # 主張の並び
///
/// ⑴ 判断中核が持つ占有終端が台本の占有区間の終端と一致する。
/// ⑵ その終端を起点として計測が確立し、満了予定が「占有終端＋待ち時間」になる
///    （現在時刻起点で取り直す実装なら、注入した現在時刻の分だけずれて落ちる）。
#[test]
fn script_occupancy_end_becomes_the_measurement_anchor_in_the_decision_core() {
    // 待ち時間の確定は**プロセスで一度きり**の記録であり、最初に相を回したテストの捕捉窓へ
    // 紛れ込む。どのテストが先に走るかは並行実行の都合で決まるため、捕捉窓の外で先に確定させる。
    let timeout_secs = configured_timeout_secs();

    // ── 本番 `wire_emo2_boot` 手順 4/5 と同型: 単一 channel の送出端を sink へ、受信端を配線へ ──
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    let clock = TalkClock::new(std::sync::Arc::new(|| 0.0));
    let mut wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel().1,
        mpsc::channel::<MoveDirective>().1,
        lifecycle_rx,
        Rc::clone(&runtime),
        clock,
        empty_assets(),
    );
    wiring
        .balloon_models
        .insert(0, areka_parsers::balloon::parse_str("", None));

    // ── 表示層は headless（`attach_target` は GPU 資源を要さず、可視状態の照会はそのまま通る） ──
    let mut world = World::new();
    world.insert_non_send_resource(BalloonWiring::new(mpsc::channel().0));
    let window = world.spawn_empty().id();
    wiring
        .presenter
        .attach_target(
            &mut world,
            balloon_target(0),
            window,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96,
        )
        .expect("headless 装着は成功する");

    // ── 実台本 → compile → `CuePlayer` → 実 `BalloonLifecycleSink` → mpsc ──
    let instructions = areka_parsers::sakura::parse(r"\0あい\w9\1うえ\e");
    let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());
    let horizon = compiled.sheet.absolute_end_time();
    let last_fire = compiled
        .sheet
        .cues()
        .iter()
        .map(|cue| compiled.sheet.absolute_fire_time(cue))
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        horizon > last_fire,
        "前提: 待機を含む台本の占有終端は最後の cue の発火時刻より後（同値なら待機込みかどうかを弁別できない）: horizon={horizon} last_fire={last_fire}"
    );

    let mut player = CuePlayer::from_sheet(&compiled.sheet);
    player.register_sink(Box::new(BalloonLifecycleSink::new(lifecycle_tx)));
    // 台本の発火時刻を昇順に辿り、最後に占有終端へ達する（sleep もスピンも使わない）。
    let mut tick_points: Vec<f64> = compiled
        .sheet
        .cues()
        .iter()
        .map(|cue| compiled.sheet.absolute_fire_time(cue))
        .collect();
    tick_points.push(horizon);
    for at in tick_points {
        player.tick(at);
    }

    // ── 判断が計測を確立できる状態を作る: 会話は終わっており、可視コンテンツが現れている ──
    // 可視グリフは相が観測する軸そのものなので、文字層ランタイムへ直接置く（cue の配送経路は
    // 本項の対象ではない）。
    runtime.borrow_mut().apply_cue(&TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("あい".into()),
        duration: 0.5,
    });
    wiring.clock.observe_cue(0.0);
    let now = horizon + 1.0;
    world.insert_resource(FrameTime(now));

    // ── 本番の相を 1 フレーム回す ──
    let (_, events) = capture_logs(|| run_balloon_visibility_phase(&mut wiring, &mut world));

    // ⑴ 判断中核が持つ占有終端が台本の占有区間の終端と一致する。
    assert_eq!(
        wiring.balloon_visibility.display_end,
        Some(horizon),
        "台本の占有区間の終端が sink→受信端→判断中核まで無傷で届く（待機込み・Requirement 4.1）"
    );

    // ⑵ その終端を起点に計測が確立している（起点を現在時刻に取り違えるとここで落ちる）。
    let started: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.message().contains("タイムアウト計測を開始"))
        .collect();
    assert_eq!(
        started.len(),
        1,
        "会話が終わって可視コンテンツが在るフレームで計測が 1 度だけ確立する: {events:?}"
    );
    assert_eq!(
        started[0].field("display_end"),
        format!("{horizon:?}"),
        "計測の起点は台本の占有終端"
    );
    assert_eq!(
        started[0].field("deadline"),
        format!("{:?}", horizon + timeout_secs),
        "満了予定は「占有終端＋待ち時間」（現在時刻 {now} 起点なら {} になって落ちる）",
        now + timeout_secs
    );
}
