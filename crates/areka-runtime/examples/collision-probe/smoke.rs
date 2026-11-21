use super::fixture::SMOKE_EXIT_ENV;
use super::{Entity, GhostWindowMarker, WinApp, With};

// ---------------------------------------------------------------------------
// Smoke auto close（env ゲート・hands-off のビルド/起動確認用）
// ---------------------------------------------------------------------------

/// [`SMOKE_EXIT_ENV`] から自動 close の遅延ミリ秒を読む（未設定・空・非数値・負値はゲート OFF）。
fn smoke_exit_ms() -> Option<u64> {
    let value = std::env::var(SMOKE_EXIT_ENV).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = trimmed.parse::<u64>().ok();
    if parsed.is_none() {
        // この probe の終わり方は自動終了だけ（読めない値で黙って切れると止まらなくなる）。
        tracing::warn!(
            env = SMOKE_EXIT_ENV,
            value = trimmed,
            "collision-probe: 自動終了の値が読めない（u64 のミリ秒のみ）— 自動終了は無効"
        );
    }
    parsed
}

/// env ゲート付き smoke 自動 close を結線する。
/// 指定 ms 後に全 [`GhostWindowMarker`] 窓を despawn → `WindowRegistry` 空遷移 → `run()` 正常復帰。
/// 窓が 1 枚も無いとき（準備の失敗）は空遷移が起きないので、`wintf::AppExit` へ終了を直に指示する。
pub(super) fn install_smoke_exit(app: &WinApp) {
    let Some(ms) = smoke_exit_ms() else {
        return;
    };
    let world_weak = std::rc::Rc::downgrade(&app.world());
    tracing::info!(
        env = SMOKE_EXIT_ENV,
        delay_ms = ms,
        "collision-probe: smoke 自動 close ゲート有効 — 全ゴースト窓を指定 ms 後に despawn します"
    );
    wintf::executor::spawn_local(async move {
        async_io::Timer::after(std::time::Duration::from_millis(ms)).await;
        // shutdown 済みなら strong 所有者は消えており upgrade は None。
        let Some(world) = world_weak.upgrade() else {
            tracing::debug!("collision-probe smoke 自動 close: world 既に drop 済み — no-op");
            return;
        };
        let mut ecs = world.borrow_mut();
        let w = ecs.world_mut();
        let targets: Vec<Entity> = w
            .query_filtered::<Entity, With<GhostWindowMarker>>()
            .iter(w)
            .collect();
        let count = targets.len();
        if count == 0 {
            tracing::info!(
                "collision-probe smoke 自動 close: ゴースト窓が無い — 終了を直に指示します"
            );
            if let Some(exit) = w.get_non_send::<wintf::AppExit>() {
                exit.request_exit();
            }
            return;
        }
        for e in targets {
            w.despawn(e);
        }
        tracing::info!(
            count,
            "collision-probe smoke 自動 close: ゴースト窓を despawn しました"
        );
    });
}
