use super::*;
use areka_sakura::contract::{FONT_TAG_CARRIER, TimePoint};
use dola::time::testing::ms;

/// 1 字の reveal 間隔。Text cue へ `N × REVEAL_INTERVAL` の duration を焼き込むと、字の時刻が
/// この間隔のちょうどの倍数になる（100ns で割り切れる）。
pub(super) const REVEAL_INTERVAL: TimeSpan = ms(250);

/// テスト用 cue 生成ヘルパ。Text cue には配送 duration = `N × REVEAL_INTERVAL` を焼き込み
/// （reveal 間隔 250ms）、他コマンドは瞬時（duration=0）とする。明示的な duration を
/// 与えたい縮退（D=0／空テキスト）・honor no-op 檻は [`cue_dur`] を使う。
pub(super) fn cue(actor: &str, at: TimeSpan, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => {
            REVEAL_INTERVAL.mul_div(areka_sakura::cluster::cluster_count(t) as u64, 1)
        }
        _ => TimeSpan::ZERO,
    };
    cue_dur(actor, at, duration, command)
}

/// 明示 duration 版の cue ヘルパ（D=0 の縮退・空テキストの 0 割り回避・honor no-op 檻用）。
pub(super) fn cue_dur(
    actor: &str,
    at: TimeSpan,
    duration: TimeSpan,
    command: CueCommand,
) -> TalkCue {
    TalkCue::scheduled(
        TimePoint::ZERO,
        at,
        ActorKey::from(actor),
        command,
        duration,
    )
}

/// actor の items を取得する（未生成なら panic ＝テスト失敗として扱う）。
pub(super) fn items_of<'a>(state: &'a TextLayerState, actor: &str) -> &'a [TextItem] {
    state
        .actor_state(&ActorKey::from(actor))
        .expect("actor state should exist")
        .items()
}

// ══ typewriter リビール進行（注入時刻駆動） ══

pub(super) fn reveal_times_of(state: &TextLayerState, actor: &str) -> Vec<TimePoint> {
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
        TimeSpan::ZERO,
        CueCommand::command_carrier(
            FONT_TAG_CARRIER,
            tokens.iter().map(|t| (*t).to_owned()).collect(),
        ),
    )
}

/// 文字の cue（時刻 0）。
pub(crate) fn text(actor: &str, s: &str) -> TalkCue {
    cue(actor, TimeSpan::ZERO, CueCommand::Text(s.into()))
}

/// アンカーを開く合図（時刻 0・再生時間 0）。
pub(crate) fn open(actor: &str, id: &str, references: &[&str]) -> TalkCue {
    cue(
        actor,
        TimeSpan::ZERO,
        CueCommand::AnchorBegin {
            id: id.to_owned(),
            references: references.iter().map(|r| (*r).to_owned()).collect(),
        },
    )
}

/// アンカーを閉じる合図（時刻 0・再生時間 0）。
pub(crate) fn close(actor: &str) -> TalkCue {
    cue(actor, TimeSpan::ZERO, CueCommand::AnchorEnd)
}

/// 選択肢の合図（時刻 0・再生時間 0・参照なし）。
pub(crate) fn choice(actor: &str, id: &str, s: &str) -> TalkCue {
    cue(
        actor,
        TimeSpan::ZERO,
        CueCommand::Choice {
            id: id.to_owned(),
            text: s.to_owned(),
            references: Vec::new(),
        },
    )
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
