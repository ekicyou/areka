#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # Tooltip Demo
//!
//! ツールチップ（`wintf::ecs::tooltip`）を実機で確かめるサンプル。
//!
//! ## 構成
//!
//! - **透過でいつも手前の窓**（左）— 絵のある所（色の付いた四角）だけマウスを受ける。
//!   四角ごとに範囲を登録してある:
//!   - 預けた文字: 1 行・複数行（LF・CRLF・CR の改行）・切れ目の無い長い URL・日本語の長い 1 行
//!   - 預けない範囲: 知らせの中ですぐ渡す・1 秒後に渡す（先に離れれば出ず `debug` に
//!     `[tooltip_supply_stale]` が残る）
//!   - 重なった 2 つの範囲（内側を後から登録＝重なった所では内側が勝つ）
//! - **もう 1 枚のいつも手前の窓**（右）— 窓の全体が 1 つの範囲。透過の窓のすぐ手前に居る
//!   宣言（`KeepDirectlyAbove`）を付け、キーで重なりを立て直す（`ReassertZOrder`）。
//!
//! キーはどの窓に入力先があっても効く（入力先を他のアプリに置いたまま試せる）:
//!
//! | キー | すること |
//! |---|---|
//! | F6 | 透過の窓の絵と当たり判定を消す／戻す（要件 2.9） |
//! | F7 | 透過の窓を OS の側で隠す／出す（試し S8） |
//! | F8 | 「1 行」の範囲を取り消す／登録し直す |
//! | F9 | もう 1 枚の窓の重なりを立て直す（試し S7） |
//! | F10 | 終わる |
//!
//! ## 使用法
//!
//! ```bash
//! RUST_LOG=info,wintf::ecs::tooltip=trace cargo run -p wintf --example tooltip_demo
//! ```
//!
//! `TOOLTIP_DEMO_EXIT_MS` にミリ秒を入れると、その時間の後に自動で終わる（動作の確かめ用）。
//! 今風の見た目（comctl32 の版 6）は `crates/wintf/build.rs` がサンプルの exe に埋めたマニフェストで出る。

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::{Duration, Instant};
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IsWindowVisible, SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP, WS_VISIBLE,
};
use windows::core::Result;
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::layout::{
    BoxInset, BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension, HitTest, LengthPercentageAuto,
};
use wintf::ecs::tooltip;
use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::label::Label;
use wintf::ecs::window::WindowStyle;
use wintf::ecs::world::tick_wake;
use wintf::ecs::{
    FrameFinalize, KeepDirectlyAbove, OnTooltip, Point, ReassertZOrder, Rect, TooltipArea,
    TooltipNotice, TooltipRange, TooltipRangeId, TooltipTurnToken, Update, Window, WindowHandle,
    WindowPos, ZOrderPairStrategy, apply_zorder_pair_maintenance, establish_owner_links,
};
use wintf::*;

/// 透過の窓の印（クリック透過の登録に使う）。
#[derive(Component)]
struct TransparentWindow;

/// サンプルの窓の印（終わるときに消す）。
#[derive(Component)]
struct DemoWindow;

/// 透過の窓に描く四角（論理の単位）。範囲もこの矩形で登録する。
struct Tile {
    label: &'static str,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    color: (f32, f32, f32),
}

impl Tile {
    fn rect(&self) -> Rect {
        Rect {
            left: self.left,
            top: self.top,
            right: self.left + self.width,
            bottom: self.top + self.height,
        }
    }
}

const fn tile(label: &'static str, pos: (f32, f32, f32, f32), color: (f32, f32, f32)) -> Tile {
    Tile {
        label,
        left: pos.0,
        top: pos.1,
        width: pos.2,
        height: pos.3,
        color,
    }
}

const ONE_LINE: Tile = tile("1 行", (20.0, 20.0, 160.0, 60.0), (0.85, 0.35, 0.35));
const MULTI_LINE: Tile = tile("複数行", (200.0, 20.0, 160.0, 60.0), (0.85, 0.6, 0.25));
const LONG_URL: Tile = tile("長い URL", (380.0, 20.0, 160.0, 60.0), (0.75, 0.75, 0.2));
const LONG_JA: Tile = tile(
    "日本語の長い 1 行",
    (20.0, 100.0, 160.0, 60.0),
    (0.35, 0.7, 0.35),
);
const IMMEDIATE: Tile = tile("すぐ渡す", (200.0, 100.0, 160.0, 60.0), (0.25, 0.65, 0.75));
const DELAYED: Tile = tile(
    "1 秒後に渡す",
    (380.0, 100.0, 160.0, 60.0),
    (0.35, 0.45, 0.85),
);
const OUTER: Tile = tile(
    "重なり（外側）",
    (20.0, 180.0, 340.0, 160.0),
    (0.6, 0.4, 0.8),
);
const INNER: Tile = tile(
    "重なり（内側）",
    (110.0, 240.0, 160.0, 60.0),
    (0.85, 0.45, 0.7),
);

/// 描く順（後の四角が手前）。
const TILES: [&Tile; 8] = [
    &ONE_LINE,
    &MULTI_LINE,
    &LONG_URL,
    &LONG_JA,
    &IMMEDIATE,
    &DELAYED,
    &OUTER,
    &INNER,
];

const ONE_LINE_TEXT: &str = "1 行の説明";
const MULTI_LINE_TEXT: &str =
    "1 行目（LF の改行）\n2 行目（CRLF の改行）\r\n3 行目（CR の改行）\r4 行目";
const LONG_URL_TEXT: &str = "https://example.com/areka/wintf/tooltip/a-very-long-path-without-any-break-points/0123456789abcdefghijklmnopqrstuvwxyz/0123456789abcdefghijklmnopqrstuvwxyz?query=value&another=value";
const LONG_JA_TEXT: &str = "これは日本語の長い一行の説明です。ツールチップの最大の幅を越えると、語の切れ目でなくても幅で折り返されるはずです。折り返しの位置と行の高さを目で確かめてください。";

/// サンプルの状態。
#[derive(Resource)]
struct Demo {
    tip_window: Entity,
    other_window: Entity,
    /// 「1 行」の範囲（F8 で取り消すと `None`）。
    one_line: Option<TooltipRangeId>,
    immediate: TooltipRangeId,
    delayed: TooltipRangeId,
    /// 透過の窓の絵（F6 で消すと空）。
    picture: Vec<Entity>,
    /// 1 秒後に渡す出す番の印と、渡す時刻。先に終わっても取り下げない（終わった印へ渡すと
    /// `StaleTurn` が返り `debug` に `[tooltip_supply_stale]` が残るのを確かめるため）。
    pending: Option<(TooltipTurnToken, Instant)>,
}

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
        let _ = tx.send(Box::new(setup));
        tick_wake::arm_deadline(Instant::now());
        poll_keys(tx).await;
    });
    if let Some(ms) = std::env::var("TOOLTIP_DEMO_EXIT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
    {
        world.borrow().spawn(move |tx| async move {
            async_io::Timer::after(Duration::from_millis(ms)).await;
            info!("[tooltip_demo] TOOLTIP_DEMO_EXIT_MS={ms} に達したので終わる");
            let _ = tx.send(Box::new(close_all));
            tick_wake::arm_deadline(Instant::now());
        });
    }

    println!("\nTooltip Demo:");
    println!("  左: 透過でいつも手前の窓（色の付いた四角の上だけマウスを受ける）");
    println!("  右: もう 1 枚のいつも手前の窓（窓の全体が範囲）");
    println!("\nキー（入力先がどの窓でも効く）:");
    println!("  F6  透過の窓の絵と当たり判定を消す／戻す（要件 2.9）");
    println!("  F7  透過の窓を OS の側で隠す／出す（試し S8）");
    println!("  F8  「1 行」の範囲を取り消す／登録し直す");
    println!("  F9  もう 1 枚の窓の重なりを立て直す（試し S7）");
    println!("  F10 終わる");
    println!("\n判定の分岐を見るには RUST_LOG=info,wintf::ecs::tooltip=trace で起動する。");

    mgr.run()?;

    Ok(())
}

// ============================================================================
// 窓と範囲
// ============================================================================

/// 窓 2 枚を作り、範囲と知らせの関数と系を登録する。
fn setup(world: &mut World) {
    // 重なりの立て直し（`ReassertZOrder`）を消費する系。wintf は自分では載せないので使う側が載せる。
    world.insert_resource(ZOrderPairStrategy::default());
    world.resource_mut::<Schedules>().add_systems(
        FrameFinalize,
        (establish_owner_links, apply_zorder_pair_maintenance).chain(),
    );
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, (deliver_delayed, register_click_through));

    let topmost = WindowStyle {
        style: WS_POPUP | WS_VISIBLE,
        ex_style: WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
    };

    // 透過の窓。窓そのものは当たり判定を持たない（絵の無い所はマウスを受けない）。
    let tip_window = world
        .spawn((
            Name::new("TooltipDemo-Transparent"),
            DemoWindow,
            TransparentWindow,
            HitTest::none(),
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(560.0)),
                    height: Some(Dimension::Px(360.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(Point { x: 200, y: 200 }),
                ..Default::default()
            },
            Window {
                title: "Tooltip Demo (transparent)".to_string(),
                ..Default::default()
            },
            topmost,
            OnTooltip(on_tooltip),
        ))
        .id();
    let picture = spawn_picture(world, tip_window);

    // もう 1 枚の窓（不透明）。透過の窓のすぐ手前に居る宣言を付ける。
    let other_window = world
        .spawn((
            Name::new("TooltipDemo-Other"),
            DemoWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(320.0)),
                    height: Some(Dimension::Px(200.0)),
                }),
                ..Default::default()
            },
            // 位置は物理のピクセル。透過の窓（幅 560 の論理）が 200% の画面でも重ならない所。
            WindowPos {
                position: Some(Point { x: 1500, y: 200 }),
                ..Default::default()
            },
            Window {
                title: "Tooltip Demo (other)".to_string(),
                ..Default::default()
            },
            topmost,
            KeepDirectlyAbove { peer: tip_window },
            OnTooltip(on_tooltip),
        ))
        .id();
    let background = world
        .spawn((
            Name::new("TooltipDemo-Other-Background"),
            Rectangle::new(),
            Brushes::with_foreground(color((0.9, 0.9, 0.9))),
            BoxStyle {
                flex_grow: Some(1.0),
                ..Default::default()
            },
            ChildOf(other_window),
        ))
        .id();
    spawn_label(world, background, "もう 1 枚の窓（窓の全体が範囲）");

    let static_range = |tile: &Tile, text: &str| TooltipRange {
        area: TooltipArea::Rect(tile.rect()),
        text: Some(text.to_string()),
    };
    let dynamic_range = |tile: &Tile| TooltipRange {
        area: TooltipArea::Rect(tile.rect()),
        text: None,
    };
    let reg = |world: &mut World, window: Entity, range: TooltipRange| {
        tooltip::register(world, window, range).expect("窓のエンティティはある")
    };
    let one_line = reg(world, tip_window, static_range(&ONE_LINE, ONE_LINE_TEXT));
    reg(
        world,
        tip_window,
        static_range(&MULTI_LINE, MULTI_LINE_TEXT),
    );
    reg(world, tip_window, static_range(&LONG_URL, LONG_URL_TEXT));
    reg(world, tip_window, static_range(&LONG_JA, LONG_JA_TEXT));
    let immediate = reg(world, tip_window, dynamic_range(&IMMEDIATE));
    let delayed = reg(world, tip_window, dynamic_range(&DELAYED));
    reg(
        world,
        tip_window,
        static_range(&OUTER, "重なり（外側・先に登録）"),
    );
    reg(
        world,
        tip_window,
        static_range(&INNER, "重なり（内側・後から登録＝重なった所で勝つ）"),
    );
    reg(
        world,
        other_window,
        TooltipRange {
            area: TooltipArea::WholeWindow,
            text: Some("もう 1 枚のいつも手前の窓（窓の全体が範囲）".to_string()),
        },
    );

    world.insert_resource(Demo {
        tip_window,
        other_window,
        one_line: Some(one_line),
        immediate,
        delayed,
        picture,
        pending: None,
    });
    info!(
        ?tip_window,
        ?other_window,
        "[tooltip_demo] 窓と範囲を登録した"
    );
}

/// 透過の窓の絵（四角とその名前）を描く。返すのは四角のエンティティ（消すと名前も消える）。
fn spawn_picture(world: &mut World, window: Entity) -> Vec<Entity> {
    TILES
        .iter()
        .map(|t| {
            let e = world
                .spawn((
                    Name::new(format!("TooltipDemo-Tile-{}", t.label)),
                    Rectangle::new(),
                    Brushes::with_foreground(color(t.color)),
                    BoxStyle {
                        position: Some(BoxPosition::Absolute),
                        inset: Some(BoxInset(wintf::ecs::layout::Rect {
                            left: LengthPercentageAuto::Px(t.left),
                            top: LengthPercentageAuto::Px(t.top),
                            right: LengthPercentageAuto::Auto,
                            bottom: LengthPercentageAuto::Auto,
                        })),
                        size: Some(BoxSize {
                            width: Some(Dimension::Px(t.width)),
                            height: Some(Dimension::Px(t.height)),
                        }),
                        ..Default::default()
                    },
                    ChildOf(window),
                ))
                .id();
            spawn_label(world, e, t.label);
            e
        })
        .collect()
}

fn spawn_label(world: &mut World, parent: Entity, text: &str) {
    world.spawn((
        Name::new(format!("TooltipDemo-Label-{text}")),
        Label {
            text: text.to_string(),
            font_family: "Yu Gothic UI".to_string(),
            font_size: 13.0,
            ..Default::default()
        },
        Brushes::with_foreground(color((0.05, 0.05, 0.05))),
        BoxStyle {
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(6.0),
                right: LengthPercentageAuto::Auto,
                top: LengthPercentageAuto::Px(4.0),
                bottom: LengthPercentageAuto::Auto,
            })),
            ..Default::default()
        },
        ChildOf(parent),
    ));
}

fn color((r, g, b): (f32, f32, f32)) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r, g, b, a: 1.0 }
}

// ============================================================================
// 知らせと、後から渡す文字
// ============================================================================

/// 2 枚の窓に付ける知らせの関数。預けない範囲に文字を渡す。
fn on_tooltip(world: &mut World, notice: &TooltipNotice) {
    info!(?notice, "[tooltip_demo] 知らせ");
    let TooltipNotice::TurnStarted(turn) = notice else {
        return;
    };
    let (immediate, delayed) = {
        let demo = world.resource::<Demo>();
        (demo.immediate, demo.delayed)
    };
    if turn.range == immediate {
        let text = format!(
            "知らせの中ですぐ渡した説明\nx = {:.0}, y = {:.0}",
            turn.position.x, turn.position.y
        );
        let result = tooltip::supply_text(world, turn.token, &text);
        info!(?result, "[tooltip_demo] すぐ渡した");
    } else if turn.range == delayed {
        let ready_at = Instant::now() + Duration::from_secs(1);
        world.resource_mut::<Demo>().pending = Some((turn.token, ready_at));
        // マウスが止まっていても、その時刻に画面更新が回るよう期限を預ける。
        tick_wake::arm_deadline(ready_at);
    }
}

/// 画面更新ごとに回す。1 秒経っていれば渡す（先に終わっていれば `StaleTurn`）。
fn deliver_delayed(world: &mut World) {
    let Some((token, ready_at)) = world.get_resource::<Demo>().and_then(|d| d.pending) else {
        return;
    };
    if Instant::now() < ready_at {
        return;
    }
    world.resource_mut::<Demo>().pending = None;
    let result = tooltip::supply_text(world, token, "1 秒後に届いた説明");
    info!(?result, "[tooltip_demo] 1 秒後に渡した");
}

/// ハンドルが付いたばかりの透過の窓。
type NewTransparentWindows<'w, 's> =
    Query<'w, 's, (Entity, &'static WindowHandle), (With<TransparentWindow>, Added<WindowHandle>)>;

/// 透過の窓をクリック透過の仕組みに登録する（絵の無い所のクリックを下の窓へ通す）。
fn register_click_through(
    new_windows: NewTransparentWindows,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    let Some(handle) = handle else {
        return;
    };
    for (window, h) in &new_windows {
        handle.register(window, h.hwnd);
    }
}

// ============================================================================
// キー
// ============================================================================

/// キーを見回り、押された瞬間に UI スレッドへ指令を送る（入力先がどこでも効くよう、
/// 窓のメッセージでなくキーの状態を読む）。
async fn poll_keys(tx: CommandSender) {
    /// キーと、押されたときに UI スレッドで行うこと。
    type KeyCommand = (VIRTUAL_KEY, fn(&mut World));
    const KEYS: [KeyCommand; 5] = [
        (VK_F6, toggle_picture),
        (VK_F7, toggle_shown),
        (VK_F8, toggle_one_line),
        (VK_F9, reassert_other),
        (VK_F10, close_all),
    ];
    let mut was_down = [false; KEYS.len()];
    loop {
        async_io::Timer::after(Duration::from_millis(30)).await;
        for ((vk, command), was) in KEYS.iter().zip(was_down.iter_mut()) {
            // SAFETY: 読み取りだけの API。
            let down = unsafe { GetAsyncKeyState(i32::from(vk.0)) } < 0;
            if down && !*was {
                if tx.send(Box::new(*command)).is_err() {
                    return;
                }
                tick_wake::arm_deadline(Instant::now());
            }
            *was = down;
        }
    }
}

/// F6: 透過の窓の絵と当たり判定を消す／戻す。
fn toggle_picture(world: &mut World) {
    let Some(mut demo) = world.get_resource_mut::<Demo>() else {
        return;
    };
    let picture = std::mem::take(&mut demo.picture);
    let window = demo.tip_window;
    if picture.is_empty() {
        let picture = spawn_picture(world, window);
        world.resource_mut::<Demo>().picture = picture;
        info!("[tooltip_demo] F6: 絵と当たり判定を戻した");
    } else {
        for e in picture {
            world.despawn(e);
        }
        info!("[tooltip_demo] F6: 絵と当たり判定を消した");
    }
}

/// F7: 透過の窓を OS の側で隠す／出す。
fn toggle_shown(world: &mut World) {
    let Some(demo) = world.get_resource::<Demo>() else {
        return;
    };
    let Some(hwnd) = world.get::<WindowHandle>(demo.tip_window).map(|h| h.hwnd) else {
        return;
    };
    // SAFETY: UI スレッドで、生きている自分の窓のハンドルに対して呼ぶ。
    unsafe {
        if IsWindowVisible(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_HIDE);
            info!("[tooltip_demo] F7: 透過の窓を隠した");
        } else {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            info!("[tooltip_demo] F7: 透過の窓を出した");
        }
    }
}

/// F8: 「1 行」の範囲を取り消す／登録し直す。
fn toggle_one_line(world: &mut World) {
    let Some(mut demo) = world.get_resource_mut::<Demo>() else {
        return;
    };
    let window = demo.tip_window;
    match demo.one_line.take() {
        Some(id) => {
            let removed = tooltip::unregister(world, id);
            info!(removed, "[tooltip_demo] F8: 「1 行」の範囲を取り消した");
        }
        None => {
            let range = TooltipRange {
                area: TooltipArea::Rect(ONE_LINE.rect()),
                text: Some(ONE_LINE_TEXT.to_string()),
            };
            let id = tooltip::register(world, window, range).ok();
            world.resource_mut::<Demo>().one_line = id;
            info!(?id, "[tooltip_demo] F8: 「1 行」の範囲を登録し直した");
        }
    }
}

/// F9: もう 1 枚の窓の重なりを立て直す。
fn reassert_other(world: &mut World) {
    let Some(other) = world.get_resource::<Demo>().map(|d| d.other_window) else {
        return;
    };
    world.entity_mut(other).insert(ReassertZOrder::default());
    info!("[tooltip_demo] F9: もう 1 枚の窓の重なりを立て直した");
}

/// F10・自動終了: サンプルの窓を全部消す（窓が無くなるとアプリが終わる）。
fn close_all(world: &mut World) {
    let windows: Vec<Entity> = world
        .query_filtered::<Entity, With<DemoWindow>>()
        .iter(world)
        .collect();
    for window in windows {
        world.despawn(window);
    }
    info!("[tooltip_demo] 窓を閉じた");
}
