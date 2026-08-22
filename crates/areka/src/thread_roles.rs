//! アクタースレッドの役割名を宣言する 1 箇所（areka-P0-draw-load-parity 要件 2.3）。
//!
//! # 何をするか
//!
//! `areka_actor::install_thread_start_hook` へフックを 1 度導入する。以後、`spawn_actor` で
//! 起こされたスレッドは走り始めに（body より前に）自分の役割名を宣言し、wintf のスレッド
//! 名簿へ載る。載った項目は複製ハンドルを持つので、報告器がスレッド 1 本ごとの CPU 時間を
//! 役割名つきで読める。
//!
//! # なぜ実行体（areka）に置くのか
//!
//! 役割名の写像は「アクター名 → wintf の固定語彙」であり、両側を知る者にしか書けない。
//! ところが `areka-actor` も `areka-ghost` も wintf に依存しておらず、依存を足すこともでき
//! ない（`Cargo.toml` 非接触＝要件 8.6）。実行体 `areka` は wintf と `areka-actor` の双方に
//! 依存する唯一の場所なので、宣言はここに 1 箇所だけ置く。`areka-actor` 側は「呼ばれ口」
//! （`ThreadStartHook`）を持つだけで相手を知らないままである。
//!
//! この形の副作用として、**ティッカーの生成点（`areka-ghost/src/ticker.rs`）には手を入れて
//! いない**。2 系統ともアクター名（`ticker`／`loop-ticker`）で `spawn_actor` を通るため、
//! ここの写像だけで役割名が決まる。名前が変われば `thread_roles_tests.rs` の実物起動テスト
//! が落ちる（黙って `actor:` へ落ちない）。
//!
//! # 費用
//!
//! フックはアクタースレッド 1 本につき 1 度。未導入時は `OnceLock::get` 1 回で、既存の
//! 挙動は一切変わらない。

use tracing::{error, info};

use wintf::ecs::world::thread_registry::{
    self, ROLE_TICKER_DISPATCHER_KANADE, ROLE_TICKER_LOOP, role_actor,
};

/// アクター名（＝スレッド名）から、名簿へ宣言する役割名を決める純関数。
///
/// ティッカー 2 系統だけは固定語彙の専用名を持ち（報告行で 1 行として読みたいため）、
/// それ以外のアクターは `actor:<name>` へ落とす。
pub fn role_for_actor_name(name: &str) -> String {
    match name {
        // areka-ghost `spawn_ticker`（dispatcher 50ms ＋ kanade 1000ms の 2 周期を 1 本で配る）。
        "ticker" => ROLE_TICKER_DISPATCHER_KANADE.to_owned(),
        // areka-ghost `spawn_loop_ticker`（SERIKO ループ評価専用の単発レーン）。
        "loop-ticker" => ROLE_TICKER_LOOP.to_owned(),
        other => role_actor(other),
    }
}

/// スレッド開始フック本体。生成されたアクタースレッドの中で 1 度だけ呼ばれる。
///
/// 登録の失敗は握り潰さない（`error!`）が、アクターは止めない——名簿に載らなかった分の
/// CPU は報告器が `unregistered_rest` の差として 1 行で出すので、観測が黙って消えることは
/// ない。
fn on_actor_thread_start(name: &str) {
    let role = role_for_actor_name(name);
    if let Err(e) = thread_registry::register_current_thread(role.clone()) {
        error!(
            actor = %name,
            role = %role,
            error = %e,
            "[thread_roles] スレッド名簿への登録に失敗した（CPU は unregistered_rest へ回る・アクターは継続）"
        );
    }
}

/// フックを導入する。導入できたら `true`、既に導入済みなら `false`。
///
/// 起動のできるだけ早い段階（`main` の tracing 初期化直後）で 1 度呼ぶ。これより前に
/// 起きたアクタースレッドは対象外になる。複数回呼んでも安全で、2 度目以降は何もしない
/// （`areka-actor` 側が `warn!` を残す）。
pub fn install() -> bool {
    match areka_actor::install_thread_start_hook(on_actor_thread_start) {
        Ok(()) => {
            info!("[thread_roles] アクタースレッドの役割宣言フックを導入した");
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
#[path = "thread_roles_tests.rs"]
mod tests;
