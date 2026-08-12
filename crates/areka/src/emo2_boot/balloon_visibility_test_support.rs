// =============================================================================
// 判断中核テストの共有ヘルパ（観測スナップショットの組立て）
//
// design.md「File Structure Plan」の `balloon_visibility_test_support.rs`。
// 観測は「scope → (可視グリフ数, 現に可視か)」の表でしかないため、ここに置くのは
// その表を組む最小の道具だけにする。実時間の待機・スリープは一切用いない。
// =============================================================================

use std::collections::BTreeMap;

use super::{
    BalloonVisibilityState, ScopeObservation, TalkLifecycleSignal, VisibilityDecision,
    VisibilityObservations, decide,
};

/// 観測できた scope（可視グリフ数と実可視）。抑止条件はいずれも「観測できて不成立」。
pub(crate) fn seen(visible_glyphs: usize, visible: bool) -> ScopeObservation {
    ScopeObservation {
        visible_glyphs: Some(visible_glyphs),
        visible,
        hover: Some(false),
        choice_active: Some(false),
    }
}

/// グリフ数の観測が取れなかった scope（本番では文字層ランタイムの借用失敗）。
pub(crate) fn unobserved(visible: bool) -> ScopeObservation {
    ScopeObservation {
        visible_glyphs: None,
        visible,
        hover: Some(false),
        choice_active: Some(false),
    }
}

/// ポインタが滞在している scope にする（Requirement 5.2）。
pub(crate) fn hovered(observation: ScopeObservation) -> ScopeObservation {
    ScopeObservation {
        hover: Some(true),
        ..observation
    }
}

/// 選択肢を表示中の scope にする（Requirement 5.4）。
pub(crate) fn choosing(observation: ScopeObservation) -> ScopeObservation {
    ScopeObservation {
        choice_active: Some(true),
        ..observation
    }
}

/// 抑止条件の観測が 1 つも取れなかった scope にする（Requirement 5.5）。
pub(crate) fn suppression_unobserved(observation: ScopeObservation) -> ScopeObservation {
    ScopeObservation {
        hover: None,
        choice_active: None,
        ..observation
    }
}

/// 観測スナップショットを組む。引数の並び順は結果に影響しない（走査は scope 昇順）。
pub(crate) fn observations(entries: &[(u32, ScopeObservation)]) -> VisibilityObservations {
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
pub(crate) fn step(
    state: &mut BalloonVisibilityState,
    entries: &[(u32, ScopeObservation)],
) -> VisibilityDecision {
    Frame::new(entries).run(state)
}

/// タイムアウト計測まで含めた 1 フレームの入力を組み立てる。
///
/// 実時間の待機は一切用いず、時刻は [`Frame::at`] で注入する。注入する時刻は観測したい時点
/// （満了境界の直前・直後など）そのものに限り、そこで頭打ちにする。
pub(crate) struct Frame {
    obs: VisibilityObservations,
    now: Option<f64>,
    timeout_secs: f64,
}

impl Frame {
    /// 観測から 1 フレームを組む（既定は時刻なし・タイムアウト 30 秒・抑止なし・信号なし）。
    pub(crate) fn new(entries: &[(u32, ScopeObservation)]) -> Self {
        Self {
            obs: observations(entries),
            now: None,
            timeout_secs: 30.0,
        }
    }

    /// 現在時刻（talk 相対秒）を注入する。
    pub(crate) fn at(mut self, now: f64) -> Self {
        self.now = Some(now);
        self
    }

    /// タイムアウト時間（秒）を指定する。
    pub(crate) fn timeout(mut self, timeout_secs: f64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// 会話開始の信号を本フレームに届ける。
    pub(crate) fn talk_started(mut self) -> Self {
        self.obs.lifecycle.push(TalkLifecycleSignal::TalkStarted);
        self
    }

    /// 占有終端の信号を本フレームに届ける。
    pub(crate) fn display_end(mut self, end: f64) -> Self {
        self.obs
            .lifecycle
            .push(TalkLifecycleSignal::DisplayEndAt(end));
        self
    }

    /// いずれかのバルーン窓がドラッグ中である状態にする（Requirement 5.1）。
    pub(crate) fn dragging(mut self) -> Self {
        self.obs.dragging = true;
        self
    }

    /// 判断を 1 フレーム進める。
    pub(crate) fn run(self, state: &mut BalloonVisibilityState) -> VisibilityDecision {
        decide(state, &self.obs, self.now, self.timeout_secs)
    }
}
