//! 表示の行の水準の出し分け（`animated-surface-log-flood` 要件 1.1〜1.5・2.1〜2.3・8.1・8.2）。
//!
//! 最初の表示と状態の替わり（面の番号・着せ替え・拡大率・窓の大きさ・付け直し・隠した後の出し直し）は
//! INFO、コマ送りだけの適用は DEBUG で、文言と欄は同じ。どの成立した適用にも表示の行がちょうど 1 行
//! 出て、その直後が perf の行である。DEBUG を主張する所では同じ捕捉の中に INFO の対照の行を置き、
//! INFO を主張する所では同じ捕捉の中に DEBUG の対照の行を置く（捕捉が空のまま緑にならない）。

use super::*;

use std::time::Duration;

use areka_actor::reply_channel;
use tracing::Level;

use super::test_support::{
    CapturedEvent, build_two_face_assets, capture, make_world_with_gpu, pattern_overlay_at,
    set_window_dpi, spawn_window_with_dpi,
};
use super::timing::PERF_LINE_MESSAGE;

const SHOW_LINE_MESSAGE: &str = "apply(ShowSurface): 表示・マスクを更新";

/// 小さい外形（4×3）。DPI 96→100 で物理寸が 4×3 のまま変わらないので、拡大率だけの替わりを作れる。
const W: u32 = 4;
const H: u32 = 3;

/// コマ `x`（面 3000 を `(x, 0)` に重ねる現在コマ）。`x` が違えば別のコマ。
fn frame(x: i64) -> PatternState {
    pattern_overlay_at(2000, 3000, x, 0)
}

fn attach(
    presenter: &mut EmoPresenter,
    world: &mut World,
    ownership: VisibilityOwnership,
) -> Entity {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas, _, _) = build_two_face_assets(W, H);
    presenter
        .attach_target(world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    presenter
        .set_visibility_ownership(TargetId(0), ownership)
        .expect("所有権の設定は Ok");
    window
}

fn show(
    presenter: &mut EmoPresenter,
    world: &mut World,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id,
            binds,
            pattern,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "ShowSurface（面 {surface_id}）が Ok でない"
    );
}

fn hide(presenter: &mut EmoPresenter, world: &mut World) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::Hide {
            target: TargetId(0),
            reply: Some(tx),
        },
    );
    assert!(matches!(
        rx.recv_timeout(Duration::from_secs(10)),
        Ok(Ok(()))
    ));
}

/// 表示の行を順に `(水準, size_changed)` で返す。どの行も直後が perf の行で、欄の集合はすべて同じ
/// （最初の行の集合。INFO の行の集合そのものは `presenter_perf_log_tests.rs` が固定している）。
fn show_lines(events: &[CapturedEvent]) -> Vec<(Level, String)> {
    let idx: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.message() == SHOW_LINE_MESSAGE)
        .map(|(i, _)| i)
        .collect();
    let perfs = events
        .iter()
        .filter(|e| e.message() == PERF_LINE_MESSAGE)
        .count();
    assert_eq!(
        idx.len(),
        perfs,
        "表示の行と perf の行の数が違う: {events:?}"
    );
    let fields = events[idx[0]].field_names_sorted();
    idx.iter()
        .map(|&i| {
            let ev = &events[i];
            assert_eq!(
                events.get(i + 1).map(|e| e.message()),
                Some(PERF_LINE_MESSAGE),
                "表示の行の直後が perf の行でない: {events:?}"
            );
            assert_eq!(ev.field_names_sorted(), fields, "表示の行の欄の集合が違う");
            let size_changed = ev.field("size_changed").expect("size_changed").to_string();
            (ev.level, size_changed)
        })
        .collect()
}

fn levels(lines: &[(Level, String)]) -> Vec<Level> {
    lines.iter().map(|(l, _)| *l).collect()
}

/// 要件 1.1・1.4・2.1・8.2: 最初の表示が INFO、続くコマ送りだけ（違うコマ・控えに当たる同じコマ・
/// 同じ中身の繰り返し）が DEBUG で、それぞれ 1 適用 1 行出る。
#[test]
fn first_show_is_info_and_following_frame_steps_are_debug() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(
        &mut presenter,
        &mut world,
        VisibilityOwnership::CommandDriven,
    );

    let ((), events) = capture(|| {
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(0),
        );
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(1),
        );
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(0),
        );
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(0),
        );
    });

    assert_eq!(
        levels(&show_lines(&events)),
        [Level::INFO, Level::DEBUG, Level::DEBUG, Level::DEBUG]
    );
}

/// 要件 1.2・1.3・1.5・8.1: 面の番号・着せ替え・拡大率・窓の大きさの替わりと、シェルを隠した後の
/// 同じ面・同じコマの出し直しは INFO。あいだのコマ送りだけが DEBUG（対照）。
#[test]
fn state_changes_and_reshow_after_hide_are_info_on_the_shell() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = attach(
        &mut presenter,
        &mut world,
        VisibilityOwnership::CommandDriven,
    );
    let b1 = || BindSet::from_ids([1]);

    let ((), events) = capture(|| {
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(0),
        ); // 最初
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(1),
        ); // コマ送り
        show(
            &mut presenter,
            &mut world,
            3000,
            BindSet::default(),
            frame(1),
        ); // 面
        show(&mut presenter, &mut world, 3000, b1(), frame(1)); // 着せ替え
        set_window_dpi(&mut world, window, 100);
        show(&mut presenter, &mut world, 3000, b1(), frame(1)); // 拡大率だけ（物理寸は同じ）
        show(&mut presenter, &mut world, 3000, b1(), frame(2)); // コマ送り
        // 窓の大きさだけが替わる形（同じ面では起きないので前の原寸を書き換えて作る）。
        presenter.targets.get_mut(&TargetId(0)).unwrap().native_size = Some((1, 1));
        show(&mut presenter, &mut world, 3000, b1(), frame(2)); // 大きさ
        hide(&mut presenter, &mut world);
        show(&mut presenter, &mut world, 3000, b1(), frame(2)); // 隠した後の出し直し
        show(&mut presenter, &mut world, 3000, b1(), frame(2)); // コマ送り（同じ中身）
    });

    let lines = show_lines(&events);
    use Level as L;
    assert_eq!(
        levels(&lines),
        [
            L::INFO,
            L::DEBUG,
            L::INFO,
            L::INFO,
            L::INFO,
            L::DEBUG,
            L::INFO,
            L::INFO,
            L::DEBUG
        ]
    );
    assert_eq!(
        lines[4].1, "false",
        "前提: 拡大率だけの替わりは物理寸を変えない"
    );
    assert_eq!(
        lines[6].1, "true",
        "前提: 大きさの替わりは size_changed=true"
    );
    assert_eq!(
        lines[7].1, "false",
        "前提: 隠した後の出し直しは大きさを変えない"
    );
}

/// 要件 1.4: 付け直し（`ReplaceTarget`）の後の最初の表示は、前と同じ面・拡大率でも INFO。続く同じ
/// 適用は DEBUG（対照）。
#[test]
fn first_show_after_replace_is_info() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(
        &mut presenter,
        &mut world,
        VisibilityOwnership::CommandDriven,
    );
    show(
        &mut presenter,
        &mut world,
        1000,
        BindSet::default(),
        PatternState::default(),
    );

    let ((), events) = capture(|| {
        let (emo_world, atlas, _, _) = build_two_face_assets(W, H);
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ReplaceTarget {
                target: TargetId(0),
                emo_world: Box::new(emo_world),
                atlas,
                author_dpi: 96,
                show: Some((1000, BindSet::default())),
                reply: Some(tx),
            },
        );
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(10)),
            Ok(Ok(()))
        ));
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            PatternState::default(),
        );
    });

    assert_eq!(levels(&show_lines(&events)), [Level::INFO, Level::DEBUG]);
}

/// 要件 1.5・8.1: バルーン（外から所有）は、最初の表示と `show_target` による出し直しが INFO。
/// 見えているあいだのコマ送りと、隠れているあいだに届いた同じコマの指令（`show_target` を通らない）は
/// DEBUG。
#[test]
fn balloon_reveal_is_info_and_same_frame_while_hidden_is_debug() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, VisibilityOwnership::External);

    let ((), events) = capture(|| {
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(0),
        ); // 最初（不可視）
        presenter
            .show_target(&mut world, TargetId(0))
            .expect("show_target"); // 出す
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(1),
        ); // コマ送り
        hide(&mut presenter, &mut world);
        show(
            &mut presenter,
            &mut world,
            1000,
            BindSet::default(),
            frame(1),
        ); // 隠れて同じコマ
        presenter
            .show_target(&mut world, TargetId(0))
            .expect("show_target"); // 出し直し
    });

    use Level as L;
    assert_eq!(
        levels(&show_lines(&events)),
        [L::INFO, L::INFO, L::DEBUG, L::DEBUG, L::INFO]
    );
}
