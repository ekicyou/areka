//! クリック透過「座標一致」切り分け検証台 — 1 窓・2 つのズレた矩形。
//!
//! 目的: areka のマスコット（BitmapSource＋αマスク＋WUC 描画）という多変数を排し、
//! **クリック透過機構と座標変換**を純粋に検証する最小台。1 つの DComp（WUC）窓の中に、
//! 対角にズラした 2 つの不透明矩形（赤=左上／青=右下・Bounds 判定）と、左下に透過 PNG
//! （キャラ画像・`HitTest::alpha_mask()` のピクセル単位α判定）を置く。要素が窓内の別位置に
//! あるため、判定領域（クリック透過の当たり）が表示とズレていれば一目で分かる。矩形＝座標一致の
//! 検証、画像＝αヒットテスト（不透明ピクセルだけ反応）の検証。
//!
//! # 期待動作（機構・座標が正しい場合）
//! - 赤／青の矩形上にカーソル → 窓は不透過（この窓がクリックを受領）。
//! - 矩形の外の透明な余白上にカーソル → 窓はクリック透過（背面プロセスへ通過）。
//! - 反応領域が色矩形とピクセル一致していれば座標変換は正しい。ズレていれば座標バグが残る。
//!
//! # 窓 Entity は `HitTest::none()`（重要）
//! `hit_test` は窓 Entity 配下を走査し、既定 `HitTestMode::Bounds` では**窓の全矩形が当たり**に
//! なる。窓 Entity を `HitTest::none()` にしないと窓全体が常にヒット＝永遠に不透過（透過しない）。
//! 本台では窓＝`None`・矩形＝`Bounds` とし、当たり＝2 矩形だけになるようにする。
//! （これは areka 本体で「マスコットが透過しない」場合の典型原因＝窓/コンテナが既定 Bounds の
//! ままで全面ヒットしている、の切り分けにもなる。）
//!
//! # 使い方
//! ```text
//! cargo run -p areka --example clickthrough_two_rects
//! # 反応領域を十字カーソルで可視化する場合（wintf の診断フラグ・既定 OFF）:
//! $env:WINTF_CROSSHAIR = "1"; cargo run -p areka --example clickthrough_two_rects
//! ```
//! 背面に別アプリ（ブラウザ・エディタ等）を置き、透明余白のクリックが背面へ抜けるか、
//! 矩形上のクリックが本窓に入るか（十字カーソル＝反応領域）を目視で確認する。
//! 終了: いずれかの矩形をダブルクリック。

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::Result;
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::drag::DragConfig;
use wintf::ecs::layout::{
    BoxInset, BoxPosition, BoxSize, BoxStyle, Dimension, HitTest, LengthPercentageAuto, Rect,
};
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::{BitmapSource, CommandSender};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::{
    ChildOf, CompositionMode, FrameFinalize, Point, Window, WindowHandle, WindowPos, WindowStyle,
};
use wintf::*;

/// 検証窓を識別するマーカー。
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
struct TwoRectsWindowMarker;

/// 窓の初期位置（screen physical）と論理サイズ。
const WIN_X: i32 = 400;
const WIN_Y: i32 = 200;
const WIN_W: f32 = 400.0;
const WIN_H: f32 = 400.0;

/// αヒットテスト検証用の透過 PNG（areka のキャラ画像を流用）。
const IMAGE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shell/base.png");

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,wintf::ecs::clickthrough=debug")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    world.borrow().spawn(|tx| async move {
        run_setup(tx).await;
    });

    // クリック透過機構への窓登録システム（areka と同一パターン）。
    world
        .borrow_mut()
        .add_systems(FrameFinalize, register_click_through_windows);

    println!();
    println!("クリック透過 切り分け検証台 — 2 つのズレた矩形");
    println!("================================================");
    println!("  赤(左上) / 青(右下) の矩形内 = 不透過（この窓が受領）");
    println!("  左下の画像 = αピクセル判定（不透明ピクセルだけ不透過・透明周辺は透過）");
    println!("  透明な余白 = クリック透過（背面プロセスへ通過）");
    println!("  反応領域が矩形/画像と一致していれば座標＋αヒットテストは正しい");
    println!("  ドラッグ移動: 矩形/画像を掴んで窓ごと移動（移動中は透過抑止=R5）");
    println!("  終了: 赤 or 青の矩形をダブルクリック");
    println!("  ヒント: 環境変数 WINTF_CROSSHAIR=1 で反応領域が十字カーソルになる");
    println!();

    mgr.run()?;
    Ok(())
}

/// クリック透過機構へ検証窓を登録する（`Added<WindowHandle>` で HWND 付与直後を捉える）。
fn register_click_through_windows(
    new_windows: Query<(Entity, &WindowHandle), (Added<WindowHandle>, With<TwoRectsWindowMarker>)>,
    handle: Option<NonSend<ClickThroughRegistryHandle>>,
) {
    let Some(handle) = handle else {
        return;
    };
    for (entity, wh) in new_windows.iter() {
        handle.register(entity, wh.hwnd);
        tracing::debug!(?entity, "クリック透過機構へ検証窓を登録しました");
    }
}

/// 非同期タスクで検証窓（2 矩形）を生成する。
async fn run_setup(tx: CommandSender) {
    let _ = tx.send(Box::new(|world: &mut World| {
        create_test_window(world);
        tracing::info!("検証窓（2 つのズレた矩形）を生成しました");
    }));
}

/// 1 つの DComp 窓に、対角にズラした 2 矩形を子として置く。窓は `HitTest::none()`。
fn create_test_window(world: &mut World) -> Entity {
    let window = world
        .spawn((
            Name::new("TwoRects-Window"),
            TwoRectsWindowMarker,
            Window {
                title: "clickthrough two-rects".to_string(),
                composition_mode: CompositionMode::DComp,
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point { x: WIN_X, y: WIN_Y }),
                ..Default::default()
            },
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(WIN_W)),
                    height: Some(Dimension::Px(WIN_H)),
                }),
                ..Default::default()
            },
            // 窓自身はヒット対象外（全面ヒットを避ける）。当たりは子の矩形・画像だけ。
            HitTest::none(),
            // 矩形/画像を掴んで窓ごとドラッグ移動（move_window=true 既定）。ドラッグ中に
            // クリック透過が入らず、終了後に再収束する（R5 抑止）ことも本台で確認できる。
            DragConfig::default(),
        ))
        .id();

    // 赤い矩形（左上）: client (20,20)-(140,140)。
    spawn_rect(
        world,
        window,
        "Rect-A-red",
        20.0,
        20.0,
        120.0,
        120.0,
        D2D1_COLOR_F {
            r: 0.90,
            g: 0.20,
            b: 0.20,
            a: 1.0,
        },
    );

    // 青い矩形（右下・対角にズラす）: client (260,260)-(380,380)。
    spawn_rect(
        world,
        window,
        "Rect-B-blue",
        260.0,
        260.0,
        120.0,
        120.0,
        D2D1_COLOR_F {
            r: 0.20,
            g: 0.35,
            b: 0.90,
            a: 1.0,
        },
    );

    // 左下: 透過 PNG（キャラ画像）を αピクセル判定で置く。client (20,180)-(180,390)。
    //
    // 生成時に `HitTest::alpha_mask()` を付けるのが必須: `generate_alpha_mask_system` は
    // `Added<BitmapSourceResource>` ＋ `With<HitTest>` ＋ `HitTestMode::AlphaMask` の時だけ
    // αマスクを生成する（HitTest 無し＝既定 Bounds では全面矩形判定でαが効かない）。
    // 表示は `draw_bitmap` が bounds（BoxStyle サイズ）へスケールするため、表示と判定域は一致する。
    // 期待: キャラの不透明ピクセル上だけ十字カーソル ON・透明な周辺はクリック透過（背面へ）。
    world.spawn((
        Name::new("Image-alpha"),
        BitmapSource::new(IMAGE_PATH),
        HitTest::alpha_mask(),
        BoxStyle {
            position: Some(BoxPosition::Absolute),
            inset: Some(BoxInset(Rect {
                left: LengthPercentageAuto::Px(20.0),
                top: LengthPercentageAuto::Px(180.0),
                right: LengthPercentageAuto::Auto,
                bottom: LengthPercentageAuto::Auto,
            })),
            size: Some(BoxSize {
                width: Some(Dimension::Px(160.0)),
                height: Some(Dimension::Px(210.0)),
            }),
            ..Default::default()
        },
        ChildOf(window),
    ));

    window
}

/// 親窓の client 座標 (left, top) に size (w, h) の不透明矩形を絶対配置で置く。
/// `HitTest` 既定＝`Bounds`（矩形判定）ゆえ、当たり＝この矩形そのもの。
#[allow(clippy::too_many_arguments)]
fn spawn_rect(
    world: &mut World,
    parent: Entity,
    name: &'static str,
    left: f32,
    top: f32,
    w: f32,
    h: f32,
    color: D2D1_COLOR_F,
) {
    world.spawn((
        Name::new(name),
        Rectangle::new(),
        Brushes::with_foreground(color),
        BoxStyle {
            position: Some(BoxPosition::Absolute),
            inset: Some(BoxInset(Rect {
                left: LengthPercentageAuto::Px(left),
                top: LengthPercentageAuto::Px(top),
                right: LengthPercentageAuto::Auto,
                bottom: LengthPercentageAuto::Auto,
            })),
            size: Some(BoxSize {
                width: Some(Dimension::Px(w)),
                height: Some(Dimension::Px(h)),
            }),
            ..Default::default()
        },
        // 赤/青の矩形をダブルクリックで終了（ヒット当事者に直付けして確実に発火させる）。
        OnPointerPressed(on_pressed_quit),
        ChildOf(parent),
    ));
}

/// ダブルクリックで検証窓を despawn し終了する（areka と同一の終了経路）。
fn on_pressed_quit(
    world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(_) => false,
        Phase::Bubble(state) => {
            if state.double_click == DoubleClick::Left {
                tracing::info!("ダブルクリック検出 — 検証台を終了します");
                let windows: Vec<Entity> = world
                    .query_filtered::<Entity, With<TwoRectsWindowMarker>>()
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
