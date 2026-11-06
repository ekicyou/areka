use super::*;
use areka_sakura::contract::FONT_TAG_CARRIER;

/// FP 丸めに依存しない reveal 間隔（0.25 は 2 の冪＝正確表現）。duration 駆動 reveal では
/// `interval = duration / N` ゆえ、Text cue へ `N × REVEAL_INTERVAL` を焼き込むことで
/// interval=0.25 の決定論的リビール時刻列を得る（期待リビール時刻は実装と同一の `D/N` 算術で成立する）。
pub(super) const REVEAL_INTERVAL: f64 = 0.25;

/// テスト用 cue 生成ヘルパ。Text cue には配送 duration = `N × REVEAL_INTERVAL` を焼き込み
/// （reveal interval=0.25）、他コマンドは瞬時（duration=0）とする。明示的な duration を
/// 与えたい縮退（D=0／空テキスト）・honor no-op 檻は [`cue_dur`] を使う。
pub(super) fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => areka_sakura::cluster::cluster_count(t) as f64 * REVEAL_INTERVAL,
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

// ══ 状態のテストで共通の組み立て（書体・文字・適用・区間・位置） ══

/// `\f[トークン…]` を運ぶ汎用キャリアの cue（再生時間 0）。
pub(crate) fn font(actor: &str, tokens: &[&str]) -> TalkCue {
    cue(
        actor,
        0.0,
        CueCommand::command_carrier(
            FONT_TAG_CARRIER,
            tokens.iter().map(|t| (*t).to_owned()).collect(),
        ),
    )
}

/// 文字の cue（時刻 0）。
pub(crate) fn text(actor: &str, s: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Text(s.into()))
}

/// 文字の cue を今の行き先へ書く。
pub(crate) fn write(state: &mut TextLayerState, actor: &str, s: &str) {
    state.apply_cue(&text(actor, s));
}

/// 空の状態へ cue の列を順に当てた状態。
pub(crate) fn applied(cues: &[TalkCue]) -> TextLayerState {
    let mut state = TextLayerState::default();
    for c in cues {
        state.apply_cue(c);
    }
    state
}

/// スコープの普通のバルーンの場所の範囲の列（場所が無ければ空）。
pub(crate) fn spans(state: &TextLayerState, actor: &str) -> Vec<ChoiceSpan> {
    state
        .actor_state(&ActorKey::from(actor))
        .map(|s| s.choices().to_vec())
        .unwrap_or_default()
}

/// スコープと場所の組の鍵。
pub(crate) fn at(actor: &str, place: TextPlace) -> PlaceKey {
    PlaceKey {
        actor: ActorKey::from(actor),
        place,
    }
}
