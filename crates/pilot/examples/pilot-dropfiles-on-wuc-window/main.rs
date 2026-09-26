//! 先進坑 pilot-dropfiles-on-wuc-window（使い捨て）。
//!
//! 本番のゴースト窓と同じ条件（WUC 合成・クリック透過の付け外し）の窓に受け入れの宣言
//! `WS_EX_ACCEPTFILES` だけを足し、エクスプローラから落としたファイルが `WM_DROPFILES` で
//! 届くかを確かめる。一次記録は隣の `README.md`（3 幕）。
//!
//! 実行: `cargo run -p pilot --example pilot-dropfiles-on-wuc-window`
//! 上限時間: `AREKA_APP_SMOKE_EXIT_MS`（ミリ秒・空／非数値は既定 180 秒）。
//!
//! 終了コード: 0＝上限時間に到達・2＝初期化の失敗・3＝窓が外から閉じられた。
//!
//! 器は手本 `crates/pilot/examples/pilot-balloon-asset-swap/main.rs` から写した。

mod dropfiles;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_ACCEPTFILES, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};

use wintf::WinApp;
use wintf::ecs::clickthrough::ClickThroughRegistryHandle;
use wintf::ecs::layout::{
    BoxInset, BoxPosition, BoxSize, BoxStyle, Dimension, HitTest, LengthPercentageAuto, Rect,
};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::world::EcsWorldSelfRef;
use wintf::ecs::{ChildOf, FrameFinalize, Point, Window, WindowHandle, WindowPos, WindowStyle};

use dropfiles::{DROP_COUNT, Fix};

/// 上限時間の環境変数（本体 `areka` の smoke と同じ名前）。
const EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";
/// 上限時間の既定（design Key Decision 5）。
const DEFAULT_EXIT_MS: u64 = 180_000;
/// 窓の固定位置（物理 px・スクリーン座標）。
const WINDOW_POS: Point = Point { x: 160, y: 160 };
/// 窓の大きさ（論理 px・design Key Decision 2）。
const WINDOW_SIZE: f32 = 320.0;
/// 不透明な矩形の左上と一辺（論理 px・クライアント座標）。
const RECT_POS: f32 = 100.0;
const RECT_SIZE: f32 = 120.0;

// ---------------------------------------------------------------------------
// 終了の理由と終了コード（design Key Decision 9）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitReason {
    Deadline,
    InitFailure,
    ExternalClose,
}

fn exit_code(reason: ExitReason) -> i32 {
    match reason {
        ExitReason::Deadline => 0,
        ExitReason::InitFailure => 2,
        ExitReason::ExternalClose => 3,
    }
}

/// `AREKA_APP_SMOKE_EXIT_MS` → 上限時間（ms）。未設定・空は既定、非数値は `warn!` して既定。
fn exit_ms_from(value: Option<&str>) -> u64 {
    let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return DEFAULT_EXIT_MS;
    };
    v.parse::<u64>().unwrap_or_else(|_| {
        tracing::warn!(
            value = v,
            default_ms = DEFAULT_EXIT_MS,
            "[dropfiles] 上限時間の値が数でない — 既定を使う"
        );
        DEFAULT_EXIT_MS
    })
}

// ---------------------------------------------------------------------------
// World に置く状態
// ---------------------------------------------------------------------------

#[derive(Component)]
struct PilotWindow;

/// 走行の状態（上限時間と終了の理由）。`run()` の後に終了の理由を読む。
#[derive(Resource)]
struct Run {
    deadline: Instant,
    limit_ms: u64,
    exit: Option<ExitReason>,
}

/// 窓 1 枚（本番のゴースト窓の様式＋受け入れの宣言）と不透明な矩形 1 つを建てる。
fn spawn_window(world: &mut World) -> Entity {
    let px = |v: f32| Some(Dimension::Px(v));
    let window = world
        .spawn((
            Name::new("Pilot-DropFiles-Window"),
            PilotWindow,
            Window {
                title: "pilot-dropfiles-on-wuc-window".to_string(),
                ..Default::default()
            },
            // 本番 `crates/areka/src/placement/spawn.rs` の `window_style` に宣言だけを足す
            // （最前面にしない・ドラッグ移動なし）。
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_ACCEPTFILES,
            },
            WindowPos {
                position: Some(WINDOW_POS),
                ..Default::default()
            },
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: px(WINDOW_SIZE),
                    height: px(WINDOW_SIZE),
                }),
                ..Default::default()
            },
            // 窓自身は当たりなし（全面ヒットを避ける）。当たりは子の矩形だけ。
            HitTest::none(),
        ))
        .id();
    world.spawn((
        Name::new("Pilot-DropFiles-Rect"),
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.90,
            g: 0.20,
            b: 0.20,
            a: 1.0,
        }),
        BoxStyle {
            position: Some(BoxPosition::Absolute),
            inset: Some(BoxInset(Rect {
                left: LengthPercentageAuto::Px(RECT_POS),
                top: LengthPercentageAuto::Px(RECT_POS),
                right: LengthPercentageAuto::Auto,
                bottom: LengthPercentageAuto::Auto,
            })),
            size: Some(BoxSize {
                width: px(RECT_SIZE),
                height: px(RECT_SIZE),
            }),
            ..Default::default()
        },
        ChildOf(window),
    ));
    window
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// HWND が付いた直後に ① 素の拡張スタイルを残す → ② 透過機構へ登録 → ③ 手当て → ④ 受け口。
/// 失敗は `error!` を出して走行を続ける（その走行は README で無効と扱う）。
#[allow(clippy::type_complexity)] // bevy の Query の形そのもの
fn on_window_created(
    new_windows: Query<(Entity, &WindowHandle), (Added<WindowHandle>, With<PilotWindow>)>,
    registry: NonSend<ClickThroughRegistryHandle>,
    self_ref: NonSend<EcsWorldSelfRef>,
    fix: Res<Fix>,
) {
    for (entity, wh) in &new_windows {
        let hwnd = wh.hwnd;
        dropfiles::log_ex_style(hwnd, "created");
        registry.register(entity, hwnd);
        if let Err(e) = dropfiles::apply_fix(hwnd, *fix) {
            tracing::error!(error = %e, fix = ?*fix, "[dropfiles] 手当ての適用に失敗");
        }
        let ctx = dropfiles::Context {
            world: self_ref.0.clone(),
            window: entity,
        };
        match dropfiles::install(hwnd, ctx) {
            Ok(()) => tracing::info!(?entity, ?hwnd, "[dropfiles] 受け口を設置"),
            Err(e) => tracing::error!(error = %e, "[dropfiles] 受け口の設置に失敗"),
        }
    }
}

/// 上限時間の検査（唯一の時計）。到達したら窓を消し、`WinApp` 既定の「最後の窓が閉じたら終了」で
/// `run()` を戻す。
fn deadline_system(
    mut run: ResMut<Run>,
    windows: Query<Entity, With<PilotWindow>>,
    mut commands: Commands,
) {
    if run.exit.is_some() || Instant::now() < run.deadline {
        return;
    }
    run.exit = Some(ExitReason::Deadline);
    tracing::info!(
        drops = DROP_COUNT.load(Ordering::Relaxed),
        limit_ms = run.limit_ms,
        "[dropfiles] 終了: 上限時間に到達"
    );
    for e in &windows {
        commands.entity(e).despawn();
    }
}

// ---------------------------------------------------------------------------
// Entry Point
// ---------------------------------------------------------------------------

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // 透過の切り替えの行（clickthrough の debug）を既定で見せる（要件 2.7）。
            EnvFilter::new("info,wintf::ecs::clickthrough=debug")
        }))
        .init();
    let reason = match boot_and_run() {
        Ok(reason) => reason,
        Err(e) => {
            tracing::error!(error = %e, "[dropfiles] 終了: 初期化に失敗（run() に入らない・run() 自体の失敗も含む）");
            ExitReason::InitFailure
        }
    };
    let code = exit_code(reason);
    tracing::info!(?reason, code, "[dropfiles] 終了コード");
    std::process::exit(code);
}

/// 起動（同期）→ `run()` → 終了の理由。`Err` は初期化の失敗（`run()` に入っていない）。
fn boot_and_run() -> Result<ExitReason, String> {
    let limit_ms = exit_ms_from(std::env::var(EXIT_ENV).ok().as_deref());
    let fix = Fix::from_env_value(std::env::var(Fix::ENV).ok().as_deref());
    let app = WinApp::new().map_err(|e| format!("WinApp の初期化に失敗: {e}"))?;
    tracing::info!(
        admin = dropfiles::is_admin(),
        fix = ?fix,
        limit_ms,
        "[dropfiles] 起動"
    );

    let world = app.world();
    {
        let mut w = world.borrow_mut();
        let window = spawn_window(w.world_mut());
        tracing::info!(
            ?window,
            x = WINDOW_POS.x,
            y = WINDOW_POS.y,
            "[dropfiles] 窓を生成"
        );
        w.world_mut().insert_resource(fix);
        w.world_mut().insert_resource(Run {
            deadline: Instant::now() + Duration::from_millis(limit_ms),
            limit_ms,
            exit: None,
        });
        w.add_systems(FrameFinalize, (on_window_created, deadline_system).chain());
    }

    app.run().map_err(|e| format!("run() が失敗: {e}"))?;

    let reason = world.borrow().world().resource::<Run>().exit;
    Ok(reason.unwrap_or_else(|| {
        tracing::warn!(
            drops = DROP_COUNT.load(Ordering::Relaxed),
            "[dropfiles] 終了: 窓が外から閉じられた"
        );
        ExitReason::ExternalClose
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_ms_falls_back_on_empty_or_non_numeric() {
        assert_eq!(DEFAULT_EXIT_MS, 180_000);
        assert_eq!(exit_ms_from(None), DEFAULT_EXIT_MS);
        assert_eq!(exit_ms_from(Some("")), DEFAULT_EXIT_MS);
        assert_eq!(exit_ms_from(Some("   ")), DEFAULT_EXIT_MS);
        assert_eq!(exit_ms_from(Some("abc")), DEFAULT_EXIT_MS);
        assert_eq!(exit_ms_from(Some("5s")), DEFAULT_EXIT_MS);
        assert_eq!(exit_ms_from(Some("5000")), 5000);
    }

    #[test]
    fn exit_codes_are_0_2_3() {
        assert_eq!(
            [
                exit_code(ExitReason::Deadline),
                exit_code(ExitReason::InitFailure),
                exit_code(ExitReason::ExternalClose),
            ],
            [0, 2, 3]
        );
    }
}
