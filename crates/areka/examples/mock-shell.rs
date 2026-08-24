//! areka モックシェル example（退避）
//!
//! wintfフレームワークを使用したデスクトップマスコット「ぱすたさん」の最小動作実装。
//! 本番アプリ骨格（`main.rs`）から挙動不変で退避したモック UI 資産。
//! - シェルウィンドウ: キャラクター画像（320×420px）の透過表示
//! - バルーンウィンドウ: 縦書きテキスト表示
//! - ドラッグ移動: シェルをドラッグするとバルーンが追従
//! - ダブルクリック終了: シェルをダブルクリックで全ウィンドウ終了
//!
//! # 使い方
//! ```text
//! cargo run -p areka --example mock-shell
//! ```

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Result;
use wintf::ecs::Point;
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::drag::{DragConfig, DragEvent, OnDrag};
use wintf::ecs::layout::{
    BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension, HitTest, LengthPercentageAuto, Rect,
};
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::{BitmapSource, CommandSender};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::{TextDirection, Typewriter, TypewriterTalk, TypewriterToken};
use wintf::ecs::{
    ChildOf, FrameFinalize, FrameTime, SetWindowPosCommand, Window, WindowHandle, WindowPos,
    WindowStyle,
};
use wintf::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// バルーンのシェルに対するオフセット (x: シェル幅320 + 間隔15)
const BALLOON_OFFSET_X: i32 = 335;
const BALLOON_OFFSET_Y: i32 = 0;

/// シェルウィンドウの初期位置
const SHELL_INITIAL_X: i32 = 400;
const SHELL_INITIAL_Y: i32 = 200;

/// シェル画像パス（CARGO_MANIFEST_DIRからの相対）
const SHELL_IMAGE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shell/base.png");

/// バルーンに表示するテキスト
const BALLOON_TEXT: &str = "\
みんながもってる、記憶の糸。\n\
\n\
生まれてから、続いている、\n\
長い長い、一本の道。\n\
\n\
そう、きっと、一本道。\n\
いつか来る、終わりの日まで。\n\
\n\
ぱすた";

// ---------------------------------------------------------------------------
// Marker Components
// ---------------------------------------------------------------------------

/// シェルウィンドウを識別するマーカーコンポーネント
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct ShellWindowMarker;

/// バルーンウィンドウを識別するマーカーコンポーネント
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct BalloonWindowMarker;

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    // tracing-subscriber 初期化（RUST_LOG環境変数対応、デフォルト info）
    // 外部入力の扱い（A1-V）: RUST_LOG が未設定・非UTF-8・不正な構文の場合は
    // try_from_default_env() が Err を返し "info" へフォールバックする（panic 経路なし）。
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    // 非同期タスクでUI構築
    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

    // クリック透過機構への窓登録システムを結線。UISetup の create_windows が
    // WindowHandle を付与した後の FrameFinalize で走らせ、同一 tick 内で Added を捉える。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_click_through_windows);

    // 操作ガイド出力
    println!();
    println!("areka モック実装 — ぱすたさん");
    println!("================================");
    println!("  ドラッグ移動: シェル画像を左クリック & ドラッグ");
    println!("  終了:         シェル画像をダブルクリック");
    println!();

    // ブロッキングメッセージループ
    mgr.run()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Async Setup
// ---------------------------------------------------------------------------

/// クリック透過機構への窓登録システム。
///
/// WUC 化により ULW の自動 α ヒットテストが失われるため、機構が α を評価できるよう
/// shell/balloon の 2 窓を明示登録する。`WindowHandle` は `create_windows`（UISetup）が
/// HWND 生成後に付与するため `Added<WindowHandle>` で「HWND が付いた瞬間」を捉え、各窓を
/// 厳密に 1 回登録する（`register` は同一 Entity 再登録を dedupe するため冪等でもある）。
///
/// `ClickThroughRegistryHandle` は `WinApp::run` の結線で World へ NonSend リソースとして
/// 挿入される。本システムは `mgr.run()` 前に登録されるが tick は `run()` 開始後に回るため
/// 通常は存在する。ごく初期の tick で未挿入の可能性に備え `Option<NonSend<..>>` で防御し、
/// 未挿入なら no-op する（`Added` は次 tick で再度観測されるため取りこぼさない）。
/// `register` は `&self`（内部可変は `Rc<RefCell<..>>`）ゆえ `NonSend` で足りる。
/// 窓破棄時の除去は機構内 `prune_dead_targets`（Entity 生存確認）が担うため明示 remove は不要。
fn register_click_through_windows(
    new_windows: Query<
        (Entity, &WindowHandle),
        (
            Added<WindowHandle>,
            Or<(With<ShellWindowMarker>, With<BalloonWindowMarker>)>,
        ),
    >,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    let Some(handle) = handle else {
        return;
    };
    for (entity, wh) in new_windows.iter() {
        handle.register(entity, wh.hwnd);
        tracing::debug!(?entity, "クリック透過機構へ窓を登録しました");
    }
}

/// 非同期タスクでシェル＋バルーンウィンドウを生成
async fn run_setup(tx: CommandSender) {
    let _ = tx.send(Box::new(|world: &mut World| {
        create_shell_window(world);
        create_balloon_window(world);
        tracing::info!("シェルウィンドウとバルーンウィンドウを生成しました");
    }));
}

// ---------------------------------------------------------------------------
// Shell Window
// ---------------------------------------------------------------------------

/// シェルウィンドウEntity構築
///
/// - WS_POPUP透過ウィンドウ（タイトルバーなし）
/// - BitmapSourceでキャラクター画像表示
/// - DragConfigでネイティブドラッグ有効化
/// - OnDrag / OnPointerPressedでインタラクション
fn create_shell_window(world: &mut World) -> Entity {
    let shell_entity = world
        .spawn((
            Name::new("Shell-Window"),
            ShellWindowMarker,
            Window {
                title: "areka shell".to_string(),
                // WUC 合成に固定化された。factory の compute_ex_style は合成モード選択を持たず、
                // 常に WS_EX_LAYERED を外し WS_EX_NOREDIRECTIONBITMAP を付与するため WindowStyle
                // の ex_style は据え置きでよい（下の WS_EX_LAYERED は factory が剥がす）。
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                // ex_style は factory が固定計算する（compute_ex_style が WS_EX_LAYERED を剥がす）。
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point {
                    x: SHELL_INITIAL_X,
                    y: SHELL_INITIAL_Y,
                }),
                ..Default::default()
            },
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(320.0)),
                    height: Some(Dimension::Px(420.0)),
                }),
                ..Default::default()
            },
            // 窓自身はヒット対象外（全面ヒットで透過を殺さない）。当たりは子の画像（α判定）が担う。
            HitTest::none(),
            DragConfig::default(),
            OnDrag(on_shell_drag),
            OnPointerPressed(on_shell_pressed),
        ))
        .id();

    // キャラクター画像 子Entity
    world.spawn((
        Name::new("Shell-Image"),
        BitmapSource::new(SHELL_IMAGE_PATH),
        // キャラの不透明ピクセルだけ受領・透明部は背面へ透過（αマスク自動生成の必須条件）。
        HitTest::alpha_mask(),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(320.0)),
                height: Some(Dimension::Px(420.0)),
            }),
            ..Default::default()
        },
        ChildOf(shell_entity),
    ));

    shell_entity
}

// ---------------------------------------------------------------------------
// Balloon Window
// ---------------------------------------------------------------------------

/// バルーンウィンドウEntity構築
///
/// - WS_POPUP透過ウィンドウ
/// - シェル右側に配置（+335px, +0px）
/// - 半透明クリーム色背景 + 縦書きTypewriterテキスト
fn create_balloon_window(world: &mut World) -> Entity {
    let balloon_entity = world
        .spawn((
            Name::new("Balloon-Window"),
            BalloonWindowMarker,
            Window {
                title: "areka balloon".to_string(),
                // WUC 合成に固定化された。factory の compute_ex_style は合成モード選択を持たず、
                // 常に WS_EX_LAYERED を外し WS_EX_NOREDIRECTIONBITMAP を付与するため据え置きでよい。
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                // ex_style は factory が固定計算する（compute_ex_style が WS_EX_LAYERED を剥がす）。
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point {
                    x: SHELL_INITIAL_X + BALLOON_OFFSET_X,
                    y: SHELL_INITIAL_Y + BALLOON_OFFSET_Y,
                }),
                ..Default::default()
            },
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(200.0)),
                    height: Some(Dimension::Px(350.0)),
                }),
                ..Default::default()
            },
        ))
        .id();

    // バルーン背景矩形（薄いクリーム色、半透明）
    let background = world
        .spawn((
            Name::new("Balloon-Background"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 0.95,
                a: 0.85,
            }),
            BoxStyle {
                flex_grow: Some(1.0),
                ..Default::default()
            },
            ChildOf(balloon_entity),
        ))
        .id();

    // 縦書き Typewriter テキスト
    let current_time = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);

    let tokens = build_typewriter_tokens(BALLOON_TEXT);

    world.spawn((
        Name::new("Balloon-Typewriter"),
        Typewriter {
            font_family: "メイリオ".to_string(),
            font_size: 16.0,
            direction: TextDirection::VerticalRightToLeft,
            default_char_wait: 0.08,
            ..Default::default()
        },
        Brushes::with_colors(
            D2D1_COLOR_F {
                r: 0.1,
                g: 0.1,
                b: 0.1,
                a: 1.0,
            },
            D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        ),
        TypewriterTalk::new(tokens, current_time),
        BoxStyle {
            flex_grow: Some(1.0),
            margin: Some(BoxMargin(Rect {
                left: LengthPercentageAuto::Px(8.0),
                right: LengthPercentageAuto::Px(8.0),
                top: LengthPercentageAuto::Px(8.0),
                bottom: LengthPercentageAuto::Px(8.0),
            })),
            ..Default::default()
        },
        ChildOf(background),
    ));

    balloon_entity
}

/// テキストを TypewriterToken 列に変換
///
/// - 行境界は必ず `\n` トークンを挿入（縦書き時は次カラムへ移動）
/// - 空行は `\n` + `Wait(0.3)` で段落間ポーズを表現
fn build_typewriter_tokens(text: &str) -> Vec<TypewriterToken> {
    let mut tokens = Vec::new();

    for (i, line) in text.split('\n').enumerate() {
        // 2行目以降は前の行との間に改行を挿入
        if i > 0 {
            tokens.push(TypewriterToken::Text("\n".to_string()));
        }
        if line.is_empty() {
            // 空行 → 段落間ポーズ
            tokens.push(TypewriterToken::Wait(0.3));
        } else {
            tokens.push(TypewriterToken::Text(line.to_string()));
        }
    }

    tokens
}

// ---------------------------------------------------------------------------
// Event Handlers
// ---------------------------------------------------------------------------

/// OnDrag ハンドラ: シェルドラッグ時にバルーンを追従移動させる
///
/// DragConfig { move_window: true } によりシェルウィンドウ自体は
/// wndprocレベルで自動的に移動される。このハンドラではバルーンの
/// 位置をシェルに合わせて更新する。
fn on_shell_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragEvent>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(_) => {
            // シェルの現在位置を取得（wndprocレベルで既に更新済み）
            let Some(pos) = world.get::<WindowPos>(entity).and_then(|wp| wp.position) else {
                return false;
            };

            // バルーンのHWNDを取得してSetWindowPosCommandを発行
            let mut query = world.query_filtered::<&WindowHandle, With<BalloonWindowMarker>>();
            if let Some(handle) = query.iter(world).next() {
                // 不変条件（A1-V）: pos は wndproc が実ウィンドウ位置から更新する
                // 論理座標であり、Windows の仮想スクリーン座標範囲に収まるため、
                // オフセット加算が i32 を溢れることはない（溢れは入力源の異常）。
                debug_assert!(
                    pos.x.checked_add(BALLOON_OFFSET_X).is_some()
                        && pos.y.checked_add(BALLOON_OFFSET_Y).is_some(),
                    "shell window position out of virtual-screen range: {pos:?}"
                );
                let new_x = pos.x + BALLOON_OFFSET_X;
                let new_y = pos.y + BALLOON_OFFSET_Y;

                SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
                    handle.hwnd,
                    new_x,
                    new_y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    None,
                ));
            }

            false
        }
    }
}

/// OnPointerPressed ハンドラ: ダブルクリックで全ウィンドウを終了する
///
/// Phase::Bubble で DoubleClick::Left を検出し、
/// ShellWindowMarker / BalloonWindowMarker を持つ全エンティティを despawn する。
/// despawn → on_window_handle_remove → PostMessage(WM_CLOSE) → PostQuitMessage(0)
fn on_shell_pressed(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(state) => {
            if state.double_click == DoubleClick::Left {
                tracing::info!("ダブルクリック検出 — アプリケーションを終了します");

                // 全ウィンドウエンティティを収集して despawn
                // （on_window_handle_remove → WM_CLOSE → PostQuitMessage）
                let windows: Vec<Entity> = world
                    .query_filtered::<Entity, Or<(With<ShellWindowMarker>, With<BalloonWindowMarker>)>>()
                    .iter(world)
                    .collect();
                for e in windows {
                    world.despawn(e);
                }

                return true;
            }
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Tests（挙動不変で src/tests.rs から同居退避）
// ---------------------------------------------------------------------------

/// GUI 非依存の純粋ロジックテスト
///
/// 対象: トークン構築（純関数）、Entity 構築（bevy_ecs World のみで完結）、
/// イベントハンドラの分岐ロジック。
/// 実ウィンドウ生成（CreateWindowExW）・COM 初期化・メッセージループは
/// wintf のシステム実行時にのみ発生するため、ここでは headless World で検証する。
#[cfg(test)]
mod tests {
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
        assert_eq!(
            style.ex_style,
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST
        );

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
        let shell = world.spawn((ShellWindowMarker, WindowPos::default())).id();
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
}
