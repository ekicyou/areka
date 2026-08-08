use super::*;

/// FP 丸めに依存しない reveal 間隔（0.25 は 2 の冪＝正確表現）。duration 駆動 reveal では
/// `interval = duration / N` ゆえ、Text cue へ `N × REVEAL_INTERVAL` を焼き込むことで
/// interval=0.25 の決定論的リビール時刻列を得る（旧 char_wait=0.25 檻と機能等価・
/// 期待リビール時刻は実装と同一の `D/N` 算術で成立し、旧 0.05 リテラル由来値を使わない）。
pub(super) const REVEAL_INTERVAL: f64 = 0.25;

/// テスト用 cue 生成ヘルパ。Text cue には配送 duration = `N × REVEAL_INTERVAL` を焼き込み
/// （reveal interval=0.25）、他コマンドは瞬時（duration=0）とする。明示的な duration を
/// 与えたい縮退（D=0／空テキスト）・honor no-op 檻は [`cue_dur`] を使う。
pub(super) fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => t.chars().count() as f64 * REVEAL_INTERVAL,
        _ => 0.0,
    };
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration,
    }
}

/// 明示 duration 版の cue ヘルパ（D=0 の縮退・空テキストの 0 割り回避・honor no-op 檻用）。
pub(super) fn cue_dur(actor: &str, at: f64, duration: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration,
    }
}

/// actor の items を取得する（未生成なら panic ＝テスト失敗として扱う）。
pub(super) fn items_of<'a>(state: &'a TextLayerState, actor: &str) -> &'a [TextItem] {
    state
        .actor_state(&ActorKey::from(actor))
        .expect("actor state should exist")
        .items()
}

// ══ typewriter リビール進行（注入時刻駆動・R3／R7 系） ══
//
// reveal ペースは配送 duration 由来（`interval = duration / N`）。FP 誤差を排するため、
// 2 の冪で正確に表現できる間隔（0.25）を主に使い、Text cue へ `N × 0.25` の duration を
// 焼き込む（[`cue`] ヘルパが自動で行う）。期待リビール時刻は実装と同一の `D/N` 算術で
// 成立し、非 2 冪の間隔（≈0.05）は安全マージン付き時刻で観測する。

pub(super) fn reveal_times_of(state: &TextLayerState, actor: &str) -> Vec<f64> {
    state
        .actor_state(&ActorKey::from(actor))
        .expect("actor state should exist")
        .reveal()
        .times()
        .to_vec()
}
