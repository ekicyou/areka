// =============================================================================
// 判断中核テストの共有ヘルパ（観測スナップショットの組立て）
// 観測は「scope → 観測値（ScopeObservation）」の表でしかないため、ここに置くのは
// その表を組む最小の道具と、箱の名前を取る surfaces.txt の畳み込み（`shell_box_hover` の
// テストと共有する）だけにする。実時間の待機・スリープは一切用いない。
// =============================================================================

use std::collections::BTreeMap;

use areka_sakura::TalkId;
use dola::TimeSpan;
#[cfg(test)]
pub(super) use dola::time::testing::{delta_ms, ms};

use super::{
    BalloonVisibilityState, DEFAULT_BALLOON_TIMEOUT, GlyphObservation, ScopeObservation,
    TalkLifecycleSignal, VisibilityDecision, VisibilityObservations, decide,
};
#[cfg(test)]
use super::{VisibilityAction, VisibilityLogEvent, VisibilityTrigger};

/// タイムアウト時間。[`Frame::new`] の既定と同じ値。
#[cfg(test)]
pub(super) const TIMEOUT: TimeSpan = DEFAULT_BALLOON_TIMEOUT;

/// [`Frame::talk_started`] が運ぶトークの番号。
pub const TALK_ID: TalkId = TalkId(1);

/// 観測できた scope（可視グリフ数と実可視）。抑止条件はいずれも「観測できて不成立」。
/// 消去の回数は 0 で一定。
pub fn seen(visible_glyphs: usize, visible: bool) -> ScopeObservation {
    ScopeObservation {
        visible_glyphs: Some(GlyphObservation {
            count: visible_glyphs,
            clear_count: 0,
        }),
        visible,
        hover: Some(false),
        box_showing: false,
    }
}

/// グリフ数の観測が取れなかった scope（本番では文字層ランタイムの借用失敗）。
#[cfg(test)]
pub(super) fn unobserved(visible: bool) -> ScopeObservation {
    ScopeObservation {
        visible_glyphs: None,
        visible,
        hover: Some(false),
        box_showing: false,
    }
}

/// ポインタが滞在している scope にする。
#[cfg(test)]
pub(super) fn hovered(observation: ScopeObservation) -> ScopeObservation {
    ScopeObservation {
        hover: Some(true),
        ..observation
    }
}

/// 箱に文字が出ている scope にする。
#[cfg(test)]
pub(super) fn boxed(observation: ScopeObservation) -> ScopeObservation {
    ScopeObservation {
        box_showing: true,
        ..observation
    }
}

/// テストの中に持つ surfaces.txt の文面を畳んだ箱の配置。箱の名前は公開の構築口を持たないので
/// ここから取る（検体は読まない）。
#[cfg(test)]
pub(crate) fn box_layout(source: &str) -> areka_emo_compose::BoxLayout {
    use areka_parsers::shell::{parse, parse_boxes};
    let world = areka_emo_compose::EmoWorld::build(&parse(source));
    let (layout, report) =
        areka_emo_compose::fold_boxes(&parse_boxes(source), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    layout
}

/// 期待する表示行動。
#[cfg(test)]
pub(super) fn show(scope: u32) -> VisibilityAction {
    VisibilityAction::Show { scope }
}

/// 期待する表示ログ。
#[cfg(test)]
pub(super) fn shown_log(scope: u32) -> VisibilityLogEvent {
    VisibilityLogEvent::Transition {
        scope,
        trigger: VisibilityTrigger::Content,
        visible: true,
    }
}

/// 期待する非表示行動。
#[cfg(test)]
pub(super) fn hide(scopes: &[u32], trigger: VisibilityTrigger) -> VisibilityAction {
    VisibilityAction::HideScopes {
        scopes: scopes.to_vec(),
        trigger,
    }
}

/// 期待する非表示ログ。
#[cfg(test)]
pub(super) fn hidden_log(scope: u32, trigger: VisibilityTrigger) -> VisibilityLogEvent {
    VisibilityLogEvent::Transition {
        scope,
        trigger,
        visible: false,
    }
}

/// 抑止条件の観測が 1 つも取れなかった scope にする。
#[cfg(test)]
pub(super) fn suppression_unobserved(observation: ScopeObservation) -> ScopeObservation {
    ScopeObservation {
        hover: None,
        ..observation
    }
}

/// 観測スナップショットを組む。引数の並び順は結果に影響しない（走査は scope 昇順）。
pub(super) fn observations(entries: &[(u32, ScopeObservation)]) -> VisibilityObservations {
    VisibilityObservations {
        scopes: entries.iter().copied().collect::<BTreeMap<_, _>>(),
        ..VisibilityObservations::default()
    }
}

/// 1 フレーム分の判断を進める（時刻を与えないため、タイムアウトの評価は行われない）。
///
/// 時刻 `None` は本番の「talk の起点が未確立」に対応する。コンテンツ駆動の判定だけを見たい
/// テストはこの入口を使う——タイムアウト側の判断が時刻無しで動き出せば、この呼び方をしている
/// テストのどれかが必ず落ちる。
#[cfg(test)]
pub(super) fn step(
    state: &mut BalloonVisibilityState,
    entries: &[(u32, ScopeObservation)],
) -> VisibilityDecision {
    Frame::new(entries).run(state)
}

/// タイムアウト計測まで含めた 1 フレームの入力を組み立てる。
///
/// 実時間の待機は一切用いず、時刻は [`Frame::at`] で注入する。注入する時刻は観測したい時点
/// （満了境界の直前・直後など）そのものに限り、そこで頭打ちにする。
pub struct Frame {
    obs: VisibilityObservations,
    now: Option<TimeSpan>,
    timeout: TimeSpan,
}

impl Frame {
    /// 観測から 1 フレームを組む（既定は時刻なし・タイムアウトは既定時間・抑止なし・信号なし）。
    ///
    /// タイムアウトの既定は [`DEFAULT_BALLOON_TIMEOUT`] から引く（同じ数値を複数箇所へ散らさない）。
    pub fn new(entries: &[(u32, ScopeObservation)]) -> Self {
        Self {
            obs: observations(entries),
            now: None,
            timeout: DEFAULT_BALLOON_TIMEOUT,
        }
    }

    /// 現在時刻（台本の時刻）を注入する。
    pub fn at(mut self, now: TimeSpan) -> Self {
        self.now = Some(now);
        self
    }

    /// タイムアウト時間を指定する。
    pub fn timeout(mut self, timeout: TimeSpan) -> Self {
        self.timeout = timeout;
        self
    }

    /// 会話開始の信号を本フレームに届ける。番号は本番と同じく配送が付けた [`TALK_ID`]。
    pub fn talk_started(mut self) -> Self {
        self.obs.lifecycle.push(TalkLifecycleSignal::TalkStarted {
            talk_id: Some(TALK_ID),
        });
        self
    }

    /// 占有終端の信号を本フレームに届ける。
    pub fn display_end(mut self, end: TimeSpan) -> Self {
        self.obs
            .lifecycle
            .push(TalkLifecycleSignal::DisplayEndAt(end));
        self
    }

    /// 占有終端とトークの終わり（止まった時刻＝占有終端）を本フレームに一緒に届ける。
    ///
    /// 時間切れの計測はトークの終わりが届いてから始まる。止まった時刻は終端と同じにするので、
    /// 起点は終端のままで満了の時刻は変わらない。
    pub fn ended_at(self, end: TimeSpan) -> Self {
        let mut frame = self.display_end(end);
        frame
            .obs
            .lifecycle
            .push(TalkLifecycleSignal::TalkEnded { at: Some(end) });
        frame
    }

    /// 任意の合図を本フレームに届ける（到着順に積む）。
    #[cfg(test)]
    pub(super) fn signal(mut self, signal: TalkLifecycleSignal) -> Self {
        self.obs.lifecycle.push(signal);
        self
    }

    /// いずれかのバルーン窓がドラッグ中である状態にする。
    #[cfg(test)]
    pub(super) fn dragging(mut self) -> Self {
        self.obs.dragging = true;
        self
    }

    /// 判断を 1 フレーム進める。
    pub fn run(self, state: &mut BalloonVisibilityState) -> VisibilityDecision {
        decide(state, &self.obs, self.now, self.timeout)
    }
}

/// `t` から `by` だけ前の時刻（満了の直前などを指す）。
#[cfg(test)]
pub(super) fn earlier(t: TimeSpan, by: TimeSpan) -> TimeSpan {
    t.checked_sub(by).expect("固定した時刻は 0 以上")
}
