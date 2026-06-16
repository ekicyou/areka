//! GUI 非依存の純粋ロジックテスト
//!
//! 対象: トークン構築（純関数）、Entity 構築（bevy_ecs World のみで完結）、
//! イベントハンドラの分岐ロジック。
//! 実ウィンドウ生成（CreateWindowExW）・COM 初期化・メッセージループは
//! wintf のシステム実行時にのみ発生するため、ここでは headless World で検証する。

use super::*;
use std::future::Future;
use std::time::Instant;
use windows::Win32::Foundation::{HINSTANCE, HWND};

/// 即時完了する Future を 1 回の poll で駆動する（run_setup 用）
fn poll_ready<F: Future>(fut: F) -> F::Output {
    let mut fut = std::pin::pin!(fut);
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(v) => v,
        std::task::Poll::Pending => panic!("future was not immediately ready"),
    }
}

// =======================================================================
// build_typewriter_tokens（純関数）
// =======================================================================

#[test]
fn tokens_single_line_is_one_text_token() {
    let tokens = build_typewriter_tokens("こんにちは");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(&tokens[0], TypewriterToken::Text(s) if s == "こんにちは"));
}

#[test]
fn tokens_inserts_newline_between_lines() {
    let tokens = build_typewriter_tokens("a\nb");
    assert_eq!(tokens.len(), 3);
    assert!(matches!(&tokens[0], TypewriterToken::Text(s) if s == "a"));
    assert!(matches!(&tokens[1], TypewriterToken::Text(s) if s == "\n"));
    assert!(matches!(&tokens[2], TypewriterToken::Text(s) if s == "b"));
}

#[test]
fn tokens_empty_line_becomes_paragraph_pause() {
    // 空行 = 改行トークン + Wait(0.3) の段落間ポーズ
    let tokens = build_typewriter_tokens("a\n\nb");
    assert_eq!(tokens.len(), 5);
    assert!(matches!(&tokens[0], TypewriterToken::Text(s) if s == "a"));
    assert!(matches!(&tokens[1], TypewriterToken::Text(s) if s == "\n"));
    assert!(matches!(&tokens[2], TypewriterToken::Wait(w) if *w == 0.3));
    assert!(matches!(&tokens[3], TypewriterToken::Text(s) if s == "\n"));
    assert!(matches!(&tokens[4], TypewriterToken::Text(s) if s == "b"));
}

#[test]
fn tokens_empty_input_is_single_pause() {
    // 現状仕様: 空文字列は 1 行の空行とみなされ Wait のみとなる
    let tokens = build_typewriter_tokens("");
    assert_eq!(tokens.len(), 1);
    assert!(matches!(&tokens[0], TypewriterToken::Wait(w) if *w == 0.3));
}

#[test]
fn tokens_balloon_text_structure() {
    let lines = BALLOON_TEXT.split('\n').count();
    let empty_lines = BALLOON_TEXT.split('\n').filter(|l| l.is_empty()).count();
    let tokens = build_typewriter_tokens(BALLOON_TEXT);

    // 行ごとに 1 トークン + 行間の改行トークン (lines - 1)
    assert_eq!(tokens.len(), lines * 2 - 1);

    let waits = tokens
        .iter()
        .filter(|t| matches!(t, TypewriterToken::Wait(_)))
        .count();
    assert_eq!(waits, empty_lines);

    // 先頭は本文テキスト（改行や Wait で始まらない）
    assert!(matches!(&tokens[0], TypewriterToken::Text(s) if s != "\n" && !s.is_empty()));
}

// =======================================================================
// 定数・アセット
// =======================================================================

#[test]
fn shell_image_asset_exists() {
    // コンパイル時埋め込みパスが実ファイルを指すこと（アセット移動の回帰検知）
    assert!(
        std::path::Path::new(SHELL_IMAGE_PATH).is_file(),
        "shell image not found: {SHELL_IMAGE_PATH}"
    );
}

// =======================================================================
// create_shell_window（headless World での Entity 構築）
// =======================================================================

#[test]
fn shell_window_has_expected_components() {
    let mut world = World::new();
    let shell = create_shell_window(&mut world);

    assert!(world.get::<ShellWindowMarker>(shell).is_some());

    let window = world.get::<Window>(shell).expect("Window component");
    assert_eq!(window.title, "areka shell");

    let style = world.get::<WindowStyle>(shell).expect("WindowStyle");
    assert_eq!(style.style, WS_POPUP | WS_VISIBLE);
    assert_eq!(style.ex_style, WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST);

    let pos = world.get::<WindowPos>(shell).expect("WindowPos");
    assert_eq!(
        pos.position,
        Some(Point {
            x: SHELL_INITIAL_X,
            y: SHELL_INITIAL_Y
        })
    );

    let box_style = world.get::<BoxStyle>(shell).expect("BoxStyle");
    assert_eq!(box_style.position, Some(BoxPosition::Absolute));
    assert_eq!(
        box_style.size,
        Some(BoxSize {
            width: Some(Dimension::Px(320.0)),
            height: Some(Dimension::Px(420.0)),
        })
    );

    // インタラクション用コンポーネント
    assert!(world.get::<DragConfig>(shell).is_some());
    assert!(world.get::<OnDrag>(shell).is_some());
    assert!(world.get::<OnPointerPressed>(shell).is_some());
}

#[test]
fn shell_window_spawns_image_child() {
    let mut world = World::new();
    let shell = create_shell_window(&mut world);

    let mut query = world.query::<(&BitmapSource, &ChildOf)>();
    let children: Vec<_> = query.iter(&world).collect();
    assert_eq!(children.len(), 1);

    let (bitmap, child_of) = children[0];
    assert_eq!(bitmap.path, SHELL_IMAGE_PATH);
    assert_eq!(child_of.parent(), shell);
}

// =======================================================================
// create_balloon_window（headless World での Entity 構築）
// =======================================================================

#[test]
fn balloon_window_is_offset_from_shell_initial_position() {
    let mut world = World::new();
    create_shell_window(&mut world);
    let balloon = create_balloon_window(&mut world);

    assert!(world.get::<BalloonWindowMarker>(balloon).is_some());

    let window = world.get::<Window>(balloon).expect("Window component");
    assert_eq!(window.title, "areka balloon");

    let pos = world.get::<WindowPos>(balloon).expect("WindowPos");
    assert_eq!(
        pos.position,
        Some(Point {
            x: SHELL_INITIAL_X + BALLOON_OFFSET_X,
            y: SHELL_INITIAL_Y + BALLOON_OFFSET_Y,
        })
    );

    let box_style = world.get::<BoxStyle>(balloon).expect("BoxStyle");
    assert_eq!(box_style.position, Some(BoxPosition::Absolute));
    assert_eq!(
        box_style.size,
        Some(BoxSize {
            width: Some(Dimension::Px(200.0)),
            height: Some(Dimension::Px(350.0)),
        })
    );
}

#[test]
fn balloon_hierarchy_is_window_background_typewriter() {
    let mut world = World::new();
    create_shell_window(&mut world);
    let balloon = create_balloon_window(&mut world);

    // 背景矩形はバルーンウィンドウの子
    let mut bg_query = world.query::<(Entity, &Rectangle, &ChildOf)>();
    let backgrounds: Vec<_> = bg_query
        .iter(&world)
        .filter(|(_, _, c)| c.parent() == balloon)
        .map(|(e, _, _)| e)
        .collect();
    assert_eq!(backgrounds.len(), 1);
    let background = backgrounds[0];

    // Typewriter は背景矩形の子
    let mut tw_query = world.query::<(&Typewriter, &ChildOf)>();
    let typewriters: Vec<_> = tw_query.iter(&world).collect();
    assert_eq!(typewriters.len(), 1);
    let (typewriter, child_of) = typewriters[0];
    assert_eq!(child_of.parent(), background);

    // Typewriter 設定（縦書き・メイリオ 16px・0.08 秒/文字）
    assert_eq!(typewriter.font_family, "メイリオ");
    assert_eq!(typewriter.font_size, 16.0);
    assert_eq!(typewriter.direction, TextDirection::VerticalRightToLeft);
    assert_eq!(typewriter.default_char_wait, 0.08);
}

#[test]
fn balloon_typewriter_talk_starts_at_zero_without_frame_time() {
    let mut world = World::new();
    create_shell_window(&mut world);
    create_balloon_window(&mut world);

    let mut query = world.query::<(&Typewriter, &TypewriterTalk)>();
    let (_, talk) = query.iter(&world).next().expect("TypewriterTalk");
    // FrameTime リソース不在時は 0.0 にフォールバック
    assert_eq!(talk.start_time(), 0.0);
    assert!(!talk.tokens().is_empty());
}

#[test]
fn balloon_typewriter_talk_uses_frame_time_resource() {
    let mut world = World::new();
    world.insert_resource(FrameTime(12.5));
    create_shell_window(&mut world);
    create_balloon_window(&mut world);

    let mut query = world.query::<(&Typewriter, &TypewriterTalk)>();
    let (_, talk) = query.iter(&world).next().expect("TypewriterTalk");
    assert_eq!(talk.start_time(), 12.5);
}

// =======================================================================
// run_setup（CommandSender 経由の UI 構築コマンド）
// =======================================================================

#[test]
fn run_setup_sends_command_that_builds_both_windows() {
    let (tx, rx) = std::sync::mpsc::channel();
    poll_ready(run_setup(tx));

    let cmd = rx.try_recv().expect("run_setup should send one command");
    // 追加コマンドは送信されない
    assert!(rx.try_recv().is_err());

    let mut world = World::new();
    cmd(&mut world);

    let shells = world
        .query_filtered::<Entity, With<ShellWindowMarker>>()
        .iter(&world)
        .count();
    let balloons = world
        .query_filtered::<Entity, With<BalloonWindowMarker>>()
        .iter(&world)
        .count();
    assert_eq!(shells, 1);
    assert_eq!(balloons, 1);
}

// =======================================================================
// on_shell_pressed（ダブルクリック終了ハンドラ）
// =======================================================================

fn pressed_event(double_click: DoubleClick) -> PointerState {
    PointerState {
        double_click,
        ..Default::default()
    }
}

#[test]
fn double_click_left_despawns_all_marked_windows() {
    let mut world = World::new();
    let shell = world.spawn(ShellWindowMarker).id();
    let shell2 = world.spawn(ShellWindowMarker).id();
    let balloon = world.spawn(BalloonWindowMarker).id();
    let other = world.spawn_empty().id();

    let ev = Phase::Bubble(pressed_event(DoubleClick::Left));
    let handled = on_shell_pressed(&mut world, shell, shell, &ev);

    assert!(handled);
    assert!(world.get_entity(shell).is_err());
    assert!(world.get_entity(shell2).is_err());
    assert!(world.get_entity(balloon).is_err());
    // マーカーを持たないエンティティは残る
    assert!(world.get_entity(other).is_ok());
}

#[test]
fn non_left_double_click_does_not_despawn() {
    let mut world = World::new();
    let shell = world.spawn(ShellWindowMarker).id();
    let balloon = world.spawn(BalloonWindowMarker).id();

    for dc in [DoubleClick::None, DoubleClick::Right, DoubleClick::Middle] {
        let ev = Phase::Bubble(pressed_event(dc));
        assert!(!on_shell_pressed(&mut world, shell, shell, &ev));
    }
    assert!(world.get_entity(shell).is_ok());
    assert!(world.get_entity(balloon).is_ok());
}

#[test]
fn tunnel_phase_double_click_is_ignored() {
    let mut world = World::new();
    let shell = world.spawn(ShellWindowMarker).id();

    let ev = Phase::Tunnel(pressed_event(DoubleClick::Left));
    assert!(!on_shell_pressed(&mut world, shell, shell, &ev));
    assert!(world.get_entity(shell).is_ok());
}

// =======================================================================
// on_shell_drag（バルーン追従ハンドラ）
// =======================================================================

fn drag_event(target: Entity) -> DragEvent {
    DragEvent {
        target,
        start_position: Point::new(0, 0),
        position: Point::new(10, 10),
        is_primary: true,
        timestamp: Instant::now(),
    }
}

#[test]
fn drag_tunnel_phase_is_ignored() {
    let mut world = World::new();
    let shell = world.spawn(ShellWindowMarker).id();

    let ev = Phase::Tunnel(drag_event(shell));
    assert!(!on_shell_drag(&mut world, shell, shell, &ev));
}

#[test]
fn drag_without_window_pos_returns_false() {
    let mut world = World::new();
    let shell = world.spawn(ShellWindowMarker).id();

    let ev = Phase::Bubble(drag_event(shell));
    assert!(!on_shell_drag(&mut world, shell, shell, &ev));
}

#[test]
fn drag_with_unset_position_returns_false() {
    let mut world = World::new();
    let shell = world
        .spawn((
            ShellWindowMarker,
            WindowPos {
                position: None,
                ..Default::default()
            },
        ))
        .id();

    let ev = Phase::Bubble(drag_event(shell));
    assert!(!on_shell_drag(&mut world, shell, shell, &ev));
}

#[test]
fn drag_without_balloon_handle_returns_false() {
    let mut world = World::new();
    let shell = world
        .spawn((
            ShellWindowMarker,
            WindowPos {
                position: Some(Point::new(500, 300)),
                ..Default::default()
            },
        ))
        .id();
    // バルーンは存在するが WindowHandle 未付与（ウィンドウ未作成相当）
    world.spawn(BalloonWindowMarker);

    let ev = Phase::Bubble(drag_event(shell));
    assert!(!on_shell_drag(&mut world, shell, shell, &ev));
}

#[test]
fn drag_with_balloon_handle_enqueues_without_panic() {
    // SetWindowPosCommand はスレッドローカルキューへの enqueue のみで
    // 実 SetWindowPos は flush 時まで発生しない（本テストでは flush しない）。
    // キュー内容の検査 API は wintf に存在しないため、ここでは
    // 正常系がパニックせずバブルを継続する（false を返す）ことのみ検証する。
    let mut world = World::new();
    let shell = world
        .spawn((
            ShellWindowMarker,
            WindowPos {
                position: Some(Point::new(500, 300)),
                ..Default::default()
            },
        ))
        .id();
    world.spawn((
        BalloonWindowMarker,
        WindowHandle {
            hwnd: HWND(std::ptr::null_mut()),
            instance: HINSTANCE(std::ptr::null_mut()),
        },
    ));

    let ev = Phase::Bubble(drag_event(shell));
    assert!(!on_shell_drag(&mut world, shell, shell, &ev));
}

#[test]
fn drag_at_extreme_virtual_screen_coords_does_not_overflow() {
    // 脆弱性点検（A1-V）: マルチモニタ環境で生じうる負座標・大座標でも
    // バルーン追従のオフセット加算（i32）が溢れずパニックしないことを
    // 境界値で固定する（仮想スクリーン座標の現実的上限を大きく超える値）。
    let mut world = World::new();
    let shell = world
        .spawn((ShellWindowMarker, WindowPos::default()))
        .id();
    world.spawn((
        BalloonWindowMarker,
        WindowHandle {
            hwnd: HWND(std::ptr::null_mut()),
            instance: HINSTANCE(std::ptr::null_mut()),
        },
    ));

    for pos in [
        Point::new(-1_000_000, -1_000_000),
        Point::new(1_000_000, 1_000_000),
    ] {
        world
            .get_mut::<WindowPos>(shell)
            .expect("WindowPos")
            .position = Some(pos);

        let ev = Phase::Bubble(drag_event(shell));
        assert!(!on_shell_drag(&mut world, shell, shell, &ev));
    }
}
