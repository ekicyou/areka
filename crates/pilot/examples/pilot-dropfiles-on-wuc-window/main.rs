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

use std::time::{Duration, Instant};

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;

use wintf::WinApp;
use wintf::ecs::{FrameFinalize, Point, SizeI, Window, WindowPos};

/// 上限時間の環境変数（本体 `areka` の smoke と同じ名前）。
const EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";
/// 上限時間の既定（design Key Decision 5）。
const DEFAULT_EXIT_MS: u64 = 180_000;
/// 窓の固定位置（物理 px・スクリーン座標）。
const WINDOW_POS: Point = Point { x: 160, y: 160 };

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

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

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
    tracing::info!(limit_ms = run.limit_ms, "[dropfiles] 終了: 上限時間に到達");
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
    let app = WinApp::new().map_err(|e| format!("WinApp の初期化に失敗: {e}"))?;

    let world = app.world();
    {
        let mut w = world.borrow_mut();
        // 仮の窓（走行を生かすだけ）。本番と同じ様式・絵・受け口はタスク 3.1 で載せる。
        w.world_mut().spawn((
            Name::new("Pilot-DropFiles-Window"),
            PilotWindow,
            Window {
                title: "pilot-dropfiles-on-wuc-window".to_string(),
                ..Default::default()
            },
            WindowPos {
                position: Some(WINDOW_POS),
                size: Some(SizeI {
                    width: 640,
                    height: 640,
                }),
                ..Default::default()
            },
        ));
        w.world_mut().insert_resource(Run {
            deadline: Instant::now() + Duration::from_millis(limit_ms),
            limit_ms,
            exit: None,
        });
        w.add_systems(FrameFinalize, deadline_system);
    }

    app.run().map_err(|e| format!("run() が失敗: {e}"))?;

    let reason = world.borrow().world().resource::<Run>().exit;
    Ok(reason.unwrap_or_else(|| {
        tracing::warn!("[dropfiles] 終了: 窓が外から閉じられた");
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
