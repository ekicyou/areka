//! 自動 close の仕掛け（env ゲート付き・CI smoke・task 2.3・R4.1）。
//!
//! `main.rs` の 1,000 行の上限のため外へ出した（areka-P0-startup-options-core 要件 2.6・中身は変えない）。

use crate::app_exit;
use wintf::WinApp;

/// env ゲート付き自動 close 機構（CI smoke・task 2.3・R4.1）。
///
/// `AREKA_APP_SMOKE_EXIT_MS` が有効なミリ秒値のときだけ、VSync relay と同じ
/// `wintf::executor::spawn_local`＋world `Weak` 作法で **一発の** async タスクを投入する
/// （ECS システムではない）。投入は 1 度目の窓の準備が通ったとき（呼び口は `main`）だけ
/// （areka-P0-ghost-restart-unit 要件 3.5）。env 未設定・不正なら
/// 発火せず、ゴースト窓はメニューの「終了」か OS の閉鎖要求を待ち続ける。
pub(crate) fn arm(app: &WinApp) {
    if let Some(ms) = smoke_exit_ms() {
        // WinApp が strong 所有者を保持するため、この Weak は shutdown まで upgrade 可能。
        let world_weak = std::rc::Rc::downgrade(&app.world());
        tracing::info!(
            env = SMOKE_EXIT_ENV,
            delay_ms = ms,
            "smoke 自動 close ゲート有効 — ゴースト窓を指定 ms 後に despawn します"
        );
        wintf::executor::spawn_local(async move {
            // 指定 ms を async スリープ（async-io は既存依存・tokio 不要）。
            async_io::Timer::after(std::time::Duration::from_millis(ms)).await;
            // shutdown 済みなら strong 所有者は消えており upgrade は None ＝ no-op。
            let Some(world) = world_weak.upgrade() else {
                tracing::debug!("smoke 自動 close: world 既に drop 済み（shutdown）— no-op");
                return;
            };
            // await を跨いで borrow を保持しない TIGHT スコープで全窓を閉じ、終了を指示する。
            {
                let mut ecs = world.borrow_mut();
                let w = ecs.world_mut();
                let count = app_exit::quit_app(w, app_exit::ExitOrigin::Smoke);
                tracing::info!(count, "smoke 自動 close: ゴースト窓を despawn しました");
            }
        });
    }
}

/// 自動 close ゲートを有効化する環境変数名（`AREKA_` 冠規約・記憶 areka-runtime-env-naming）。
const SMOKE_EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";

/// 与えられた値から自動 close の遅延ミリ秒を解釈する純粋ヘルパ（env アクセスなし・単体テスト可能）。
///
/// - `None`／空／空白のみ／非数値／負値／`u64` 溢れ → `None`（ゲート OFF＝タスク不投入）。
/// - 周辺空白をトリムした非負整数 → `Some(ms)`（`"0"` は 0ms＝即時発火として受理）。
///
/// `u64::from_str` は負号・小数点・非数字を弾き、範囲外を `Err` にするため、負値・溢れは
/// 自然に `None` へ落ちる（不正入力はゲート OFF に倒す）。
fn smoke_exit_ms_from(value: Option<&str>) -> Option<u64> {
    let trimmed = value.map(str::trim)?;
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u64>().ok()
}

/// 環境変数 [`SMOKE_EXIT_ENV`] から自動 close の遅延ミリ秒を読む。
fn smoke_exit_ms() -> Option<u64> {
    smoke_exit_ms_from(std::env::var(SMOKE_EXIT_ENV).ok().as_deref())
}

/// smoke の自動終了ゲート（task 2.3）の headless 単体テスト。
///
/// ランタイム結線（`fn main` の自動終了の仕掛け）は生きた `WinApp` を要し headless では駆動できないため、
/// 判断を持つ `smoke_exit_ms_from` だけを純粋・env 非依存で検証する（結線は実プロセス smoke が担う）。
#[cfg(test)]
#[path = "smoke_exit_tests.rs"]
mod tests;
