use super::*;
use areka_emo_compose::BindSet;
use areka_sakura::ActorKey;
use log_capture_kit::{LineFormat, capture_lines};
use std::collections::BTreeMap;

/// テスト用の TalkCue（Shell 系 Emote・at/actor 込み）を組む。
pub(super) fn emote_cue(at: f64, scope: &str, key: &str) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command: CueCommand::Emote { key: key.into() },
        duration: 0.0, // 表情切替は瞬時（明示的 0）。
    }
}

/// 同期 `handle_message` 用の小さな解決層（"通常"→2100 の 1 件のみ）。
pub(super) fn tiny_resolver() -> SurfaceResolver {
    let mut aliases: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    aliases.insert("通常".to_string(), vec![2100]);
    SurfaceResolver::new(aliases)
}

/// 非空の静的 bind 集合を持つ空スコープ状態。
pub(super) fn fresh_states() -> ScopeStates {
    ScopeStates::new(BindSet::from_ids([1100, 1207]))
}

/// 不活性なループ統括器（空表＋ダミー乱数）。cue/bind/balloon の同期 `handle_message` 檻で
/// tick 経路を触らない既存挙動を保つための足場（`disabled()` は on_tick 常時空・on_surface_changed
/// は空 playback への no-op ゆえ、既存の発行/ログ挙動と byte 同値）。
pub(super) fn inert_runtime() -> LoopRuntime {
    LoopRuntime::new(SerikoLoopConfig::disabled())
}
/// `capture_logs` の変種: `f` の戻り値も併せて返す（同期 handler の `ControlFlow` 表明用）。
///
/// 捕捉層は `capture_logs` と同一（共有機構 `log-capture-kit` へ委譲）で、`f` が発火した
/// log 文字列と `f` の戻り値を組で返す。重複ハーネスを作らない。
pub(super) fn capture_logs_flow<T, F: FnOnce() -> T>(f: F) -> (String, T) {
    let (ret, lines) = capture_lines(LineFormat::LevelTargetFields, f);
    (lines.join("\n"), ret)
}

/// テスト専用 tracing 捕捉ハーネス（硬化機構の唯一の定義元 `log-capture-kit` へ委譲）。
/// 1 イベント 1 行へ level／target／各フィールド（`name=value`）を整形し、改行連結で返す。
///
/// **「`with_default` はスレッドローカルだから並行実行でも干渉しない」は誤り**である。
/// 差し替わるのはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める
/// callsite の interest キャッシュはプロセス全体で 1 つしかなく、その発行点をプロセス内で
/// 最初に踏んだスレッドの判定が焼き付く。捕捉窓を持たないスレッド（既定は `NoSubscriber`）が
/// 先に踏むと `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。
/// 共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 窓の内側での interest 再計算 ⑶ 番兵イベントに
/// よる空振り検出（捕捉できなければ panic）の 3 点でこれを塞ぐ。機序の逐条解説と
/// `tracing-core` の実コード引用は `log_capture_kit` の crate doc および同 crate の
/// `src/probe.rs` にある。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> String {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines.join("\n")
}
