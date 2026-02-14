#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! areka モック実装
//!
//! wintfフレームワークを使用したデスクトップマスコット「ぱすたさん」の最小動作実装。
//! - シェルウィンドウ: キャラクター画像（320×420px）の透過表示
//! - バルーンウィンドウ: 縦書きテキスト表示
//! - ドラッグ移動: シェルをドラッグするとバルーンが追従
//! - ダブルクリック終了: シェルをダブルクリックで全ウィンドウ終了

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Result;
use wintf::ecs::drag::{DragConfig, DragEvent, OnDrag};
use wintf::ecs::layout::{
    BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension, LengthPercentageAuto, Rect,
};
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::{BitmapSource, CommandSender};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::{TextDirection, Typewriter, TypewriterTalk, TypewriterToken};
use wintf::ecs::{
    ChildOf, FrameTime, SetWindowPosCommand, Window, WindowHandle, WindowPos, WindowStyle,
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
    human_panic::setup_panic!();

    // tracing-subscriber 初期化（RUST_LOG環境変数対応、デフォルト info）
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinThreadMgr::new()?;
    let world = mgr.world();

    // 非同期タスクでUI構築
    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

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

/// 非同期タスクでシェル＋バルーンウィンドウを生成
async fn run_setup(tx: CommandSender) {
    let _ = tx.send(Box::new(|world: &mut World| {
        let shell_entity = create_shell_window(world);
        create_balloon_window(world, shell_entity);
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
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(POINT {
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
            DragConfig::default(),
            OnDrag(on_shell_drag),
            OnPointerPressed(on_shell_pressed),
        ))
        .id();

    // キャラクター画像 子Entity
    world.spawn((
        Name::new("Shell-Image"),
        BitmapSource::new(SHELL_IMAGE_PATH),
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
fn create_balloon_window(world: &mut World, _shell_entity: Entity) -> Entity {
    let balloon_entity = world
        .spawn((
            Name::new("Balloon-Window"),
            BalloonWindowMarker,
            Window {
                title: "areka balloon".to_string(),
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(POINT {
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
        .map(|ft| ft.elapsed_secs())
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
/// 空行は Wait(0.3) に変換し、段落間のポーズを表現
fn build_typewriter_tokens(text: &str) -> Vec<TypewriterToken> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = text.split('\n').collect();

    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            // 空行 → 段落間ポーズ
            tokens.push(TypewriterToken::Wait(0.3));
        } else {
            // テキスト行
            if i > 0 && !lines[i - 1].is_empty() {
                // 前の行が空行でなければ改行テキストを入れる
                tokens.push(TypewriterToken::Text("\n".to_string()));
            }
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
        Phase::Bubble(_event) => {
            // シェルの現在位置を取得（wndprocレベルで既に更新済み）
            let shell_pos = world.get::<WindowPos>(entity).and_then(|wp| wp.position);

            let Some(pos) = shell_pos else {
                return false;
            };

            // バルーンのHWNDを取得してSetWindowPosCommandを発行
            let mut query = world.query_filtered::<&WindowHandle, With<BalloonWindowMarker>>();
            if let Some(handle) = query.iter(world).next() {
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

                // 全ウィンドウエンティティを収集
                let shells: Vec<Entity> = world
                    .query_filtered::<Entity, With<ShellWindowMarker>>()
                    .iter(world)
                    .collect();

                let balloons: Vec<Entity> = world
                    .query_filtered::<Entity, With<BalloonWindowMarker>>()
                    .iter(world)
                    .collect();

                // despawn（on_window_handle_remove → WM_CLOSE → PostQuitMessage）
                for e in shells.into_iter().chain(balloons) {
                    world.despawn(e);
                }

                return true;
            }
            false
        }
    }
}
