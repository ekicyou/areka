#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Typewriter デモ
//!
//! タイプライター効果のデモンストレーション:
//! - 文字を一文字ずつ表示
//! - ウェイト制御
//! - pause/resume/skip 操作（skip は縦書きだけ）
//! - FireEvent による完了イベント（縦書きは skip した時点、横書きは最後まで進むと受け手に届き、ログに出る）

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::core::Result;
use wintf::ecs::layout::{
    BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension, LengthPercentageAuto,
};
use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::{
    TextDirection, Typewriter, TypewriterEvent, TypewriterEventKind, TypewriterLayoutCache,
    TypewriterTalk, TypewriterToken, deliver_typewriter_events,
};
use wintf::ecs::{FrameTime, Point, Update, Window, WindowPos};
use wintf::*;

#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct TypewriterDemoWindow;

/// 横書き Typewriter Entity を識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct HorizontalTypewriter;

/// 縦書き Typewriter Entity を識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct VerticalTypewriter;

/// 完了イベント受信用エンティティ
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct CompletionReceiver;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    world.borrow().spawn(|tx| async move {
        run_demo(tx).await;
    });

    println!("\nTypewriter デモ:");
    println!("  1. ウィンドウ作成とTypewriterエンティティ生成");
    println!("  2. Stage 1 IR でテキストを設定");
    println!("  3. 一文字ずつ表示（タイプライター効果）");
    println!("  4. 3秒後: pause()");
    println!("  5. 5秒後: resume()");
    println!("  6. 6秒後: 縦書きだけ skip()（全文即時表示・完了イベントもその場でログに出る）");
    println!("     横書きは最後まで進み、約9秒で完了イベントがログに出る");
    println!("  7. 10秒後: ウィンドウ終了");

    mgr.run()?;

    Ok(())
}

async fn run_demo(tx: CommandSender) {
    println!("[Async] 0s: Creating typewriter demo window");
    let _ = tx.send(Box::new(create_typewriter_demo_window));

    async_io::Timer::after(Duration::from_secs(1)).await;

    println!("[Async] 1s: Starting typewriter talk");
    let _ = tx.send(Box::new(start_typewriter_talk));

    async_io::Timer::after(Duration::from_secs(2)).await;

    println!("[Async] 3s: Pausing typewriter");
    let _ = tx.send(Box::new(pause_typewriter));

    async_io::Timer::after(Duration::from_secs(2)).await;

    println!("[Async] 5s: Resuming typewriter");
    let _ = tx.send(Box::new(resume_typewriter));

    async_io::Timer::after(Duration::from_secs(1)).await;

    println!("[Async] 6s: Skipping to end");
    let _ = tx.send(Box::new(skip_typewriter));

    async_io::Timer::after(Duration::from_secs(4)).await;

    println!("[Async] 10s: Closing window");
    let _ = tx.send(Box::new(close_window));
}

fn close_window(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<TypewriterDemoWindow>>();
    if let Some(window) = query.iter(world).next() {
        println!("[Test] Removing Window entity {:?}", window);
        world.despawn(window);
    }
}

fn create_typewriter_demo_window(world: &mut World) {
    let window_entity = world
        .spawn((
            Name::new("TypewriterDemo-Window"),
            TypewriterDemoWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(500.0)),
                    height: Some(Dimension::Px(500.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(Point { x: 100, y: 100 }),
                ..Default::default()
            },
            Window {
                title: "wintf - Typewriter Demo".to_string(),
                ..Default::default()
            },
        ))
        .id();

    let background = world
        .spawn((
            Name::new("Background"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.95,
                g: 0.95,
                b: 0.95,
                a: 1.0,
            }),
            BoxStyle {
                // size 100% + margin は親をはみ出すため、flex_grow を使用
                flex_grow: Some(1.0),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: LengthPercentageAuto::Px(20.0),
                    right: LengthPercentageAuto::Px(20.0),
                    top: LengthPercentageAuto::Px(20.0),
                    bottom: LengthPercentageAuto::Px(20.0),
                })),
                flex_direction: Some(wintf::ecs::layout::FlexDirection::Column),
                ..Default::default()
            },
            ChildOf(window_entity),
        ))
        .id();

    // 横書き用の内側ボックス（黒。1px の余白が枠に見える）
    let horizontal_box = world
        .spawn((
            Name::new("HorizontalBox"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
            BoxStyle {
                size: Some(BoxSize {
                    width: None,
                    height: Some(Dimension::Px(80.0)),
                }),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: LengthPercentageAuto::Px(10.0),
                    right: LengthPercentageAuto::Px(10.0),
                    top: LengthPercentageAuto::Px(10.0),
                    bottom: LengthPercentageAuto::Px(10.0),
                })),
                ..Default::default()
            },
            ChildOf(background),
        ))
        .id();

    // NOTE: Typewriterは描画コンテンツサイズを自己申告しないため、
    // flex_grow で親の残りを埋める
    world.spawn((
        Name::new("HorizontalTypewriter"),
        HorizontalTypewriter,
        Typewriter {
            font_family: "メイリオ".to_string(),
            font_size: 18.0,
            direction: TextDirection::HorizontalLeftToRight,
            default_char_wait: 0.15, // 150ms/文字（ゆっくり表示）
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
                r: 0.9,
                g: 0.9,
                b: 1.0,
                a: 1.0,
            },
        ),
        BoxStyle {
            // size 100% + margin は親をはみ出すため、flex_grow を使用
            flex_grow: Some(1.0),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(1.0),
                right: LengthPercentageAuto::Px(1.0),
                top: LengthPercentageAuto::Px(1.0),
                bottom: LengthPercentageAuto::Px(1.0),
            })),
            ..Default::default()
        },
        ChildOf(horizontal_box),
    ));

    // 縦書き用の内側ボックス（黒。1px の余白が枠に見える）
    let vertical_box = world
        .spawn((
            Name::new("VerticalBox"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(80.0)),
                    height: None,
                }),
                flex_grow: Some(1.0),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: LengthPercentageAuto::Px(10.0),
                    right: LengthPercentageAuto::Px(10.0),
                    top: LengthPercentageAuto::Px(10.0),
                    bottom: LengthPercentageAuto::Px(10.0),
                })),
                align_self: Some(wintf::ecs::layout::AlignSelf::FLEX_END), // 右寄せ
                ..Default::default()
            },
            ChildOf(background),
        ))
        .id();

    // NOTE: Typewriterは描画コンテンツサイズを自己申告しないため、
    // 幅は 85px に決め、高さは flex_grow で親の残りを埋める
    // NOTE: 空のTypewriterTalkを初期設定することで、会話開始前も背景が描画される
    world.spawn((
        Name::new("VerticalTypewriter"),
        VerticalTypewriter,
        Typewriter {
            font_family: "メイリオ".to_string(),
            font_size: 18.0,
            direction: TextDirection::VerticalRightToLeft,
            default_char_wait: 0.15, // 150ms/文字（ゆっくり表示）
            ..Default::default()
        },
        Brushes::with_colors(
            D2D1_COLOR_F {
                r: 0.1,
                g: 0.1,
                b: 0.5,
                a: 1.0,
            },
            D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 0.9,
                a: 1.0,
            },
        ),
        TypewriterTalk::new(vec![], dola::TimePoint::ZERO),
        BoxStyle {
            // size 100% + margin は親をはみ出すため、flex_grow を使用
            size: Some(BoxSize {
                width: Some(Dimension::Px(85.0)), // 3行分（テキスト幅81px + 余白）
                height: None,
            }),
            flex_grow: Some(1.0),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(1.0),
                right: LengthPercentageAuto::Px(1.0),
                top: LengthPercentageAuto::Px(1.0),
                bottom: LengthPercentageAuto::Px(1.0),
            })),
            ..Default::default()
        },
        ChildOf(vertical_box),
    ));

    world.spawn((
        Name::new("CompletionReceiver"),
        CompletionReceiver,
        TypewriterEvent::None,
    ));
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, log_typewriter_events);

    println!("[Test] Typewriter demo window created:");
    println!("  Window (root)");
    println!("  └─ Background (gray)");
    println!("     ├─ HorizontalTypewriter (横書き)");
    println!("     └─ VerticalTypewriter (縦書き)");
}

/// 両方のTypewriterにトークを開始
fn start_typewriter_talk(world: &mut World) {
    let mut receiver_query = world.query_filtered::<Entity, With<CompletionReceiver>>();
    let Some(receiver_entity) = receiver_query.iter(world).next() else {
        println!("[Error] CompletionReceiver entity not found");
        return;
    };

    let current_time = {
        let Some(frame_time) = world.get_resource::<FrameTime>() else {
            println!("[Error] FrameTime not found");
            return;
        };
        frame_time.0
    };

    let tokens = vec![
        TypewriterToken::Text("こんにちは、".to_string()),
        TypewriterToken::Wait(0.3),
        TypewriterToken::Text("今日もいい天気ですね。".to_string()),
        TypewriterToken::Wait(0.5),
        TypewriterToken::Text("タイプライター効果のデモです！".to_string()),
        TypewriterToken::FireEvent {
            target: receiver_entity,
            event: TypewriterEventKind::Complete,
        },
    ];

    let mut h_query = world.query_filtered::<Entity, With<HorizontalTypewriter>>();
    if let Some(entity) = h_query.iter(world).next() {
        let talk = TypewriterTalk::new(tokens.clone(), current_time);
        world.entity_mut(entity).insert(talk);
        println!("[Test] HorizontalTypewriter talk started");
    }

    let mut v_query = world.query_filtered::<Entity, With<VerticalTypewriter>>();
    if let Some(entity) = v_query.iter(world).next() {
        let talk = TypewriterTalk::new(tokens, current_time);
        world.entity_mut(entity).insert(talk);
        println!("[Test] VerticalTypewriter talk started");
    }

    println!("[Test] TypewriterTalk started with text:");
    println!(
        "  \"こんにちは、（wait 300ms）今日もいい天気ですね。（wait 500ms）タイプライター効果のデモです！\""
    );
}

fn pause_typewriter(world: &mut World) {
    let Some(frame_time) = world.get_resource::<FrameTime>() else {
        println!("[Error] FrameTime not found");
        return;
    };
    let current_time = frame_time.0;

    let mut query = world.query::<&mut TypewriterTalk>();
    for mut talk in query.iter_mut(world) {
        let progress_before = talk.progress();
        talk.pause(current_time);
        println!(
            "[Test] Typewriter paused at progress: {:.1}%",
            progress_before * 100.0
        );
    }
}

fn resume_typewriter(world: &mut World) {
    let Some(frame_time) = world.get_resource::<FrameTime>() else {
        println!("[Error] FrameTime not found");
        return;
    };
    let current_time = frame_time.0;

    let mut query = world.query::<&mut TypewriterTalk>();
    for mut talk in query.iter_mut(world) {
        talk.resume(current_time);
        println!(
            "[Test] Typewriter resumed at progress: {:.1}%",
            talk.progress() * 100.0
        );
    }
}

/// 縦書き Typewriter をスキップ（残りの FireEvent もその場で受け手へ届ける）
fn skip_typewriter(world: &mut World) {
    let mut query = world.query_filtered::<
        (Entity, &mut TypewriterTalk, &TypewriterLayoutCache),
        With<VerticalTypewriter>,
    >();
    let mut fired = Vec::new();
    for (entity, mut talk, layout_cache) in query.iter_mut(world) {
        fired.push((entity, talk.skip(layout_cache.timeline())));
        println!("[Test] VerticalTypewriter skipped to end (progress: 100%)");
    }
    let mut commands = world.commands();
    for (entity, events) in fired {
        deliver_typewriter_events(&mut commands, entity, events);
    }
    world.flush();
}

/// 受け手に届いた TypewriterEvent をログに出す
fn log_typewriter_events(query: Query<(&Name, &TypewriterEvent), Changed<TypewriterEvent>>) {
    for (name, event) in &query {
        if *event != TypewriterEvent::None {
            info!(receiver = %name, event = ?event, "[Typewriter] Event received");
        }
    }
}
