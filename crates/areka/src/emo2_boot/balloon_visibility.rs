//! バルーン可視性の**判断中核**（純関数 [`decide`]）とその状態モデル
//! （design.md「BalloonVisibilityController（`emo2_boot/balloon_visibility.rs`）」・
//! Requirements 2.1〜2.7 / 3.1 / 3.2 / 3.3 / 3.6 / 4.2 / 4.3 / 4.5 / 4.6 / 4.7 / 4.8 / 4.9 /
//! 5.1〜5.3 / 5.5 / 5.6 / 8.2 / 8.3 / 8.6 / 9.5）。
//!
//! # 何を決めるモジュールか
//!
//! バルーンを「いつ出すか・いつ消すか」を決める唯一の主体である。判断はここに集約し、
//! 観測の収集・presenter への発行・ログ出力といった配線は呼び手（フレームの相関数）に置く。
//! そのため [`decide`] は `World` も GPU も時計も触らず、**観測スナップショットと状態だけ**から
//! 遷移を導き、行動とログ用の事象を値として返す。
//!
//! # 表示・非表示の単一規則（Requirement 2.6）
//!
//! - 表示: ある scope の**可視グリフ数が前のフレームより増えた**フレームで（その scope の内容が
//!   消去された後は、空の状態（0）から増えたかで判定する）、かつその scope が**現に不可視**のとき
//!   だけ表示する（Requirement 2.1 / 2.5・`decide_content`）。消去されたかは
//!   `TextLayerState::clear_count` の変化で知る（areka-P0-balloon-reappear-short-talk）。
//!   起動時・会話開始時・scope 切替時に別条件を設けない。
//!   ただし利用者の中断で隠した後は、次のトークが始まるまで見送る（areka-P0-balloon-break 要件 4.8）。
//! - 非表示（会話開始側）: 可視グリフ数が**ゼロへ下降した**フレームで、かつ現に可視のときだけ
//!   非表示にする（Requirement 3.1）。会話がどの scope から始まるかを先読みしない（Requirement 3.6）。
//!
//! 改行・カーソル移動・待機・内容消去はいずれも可視グリフ数を増やさないため、表示の契機に
//! ならない（Requirement 2.3）。この一致は偶然ではなく、観測量に
//! `TextLayerState::visible_glyphs`（`areka-emo-text` の `state.rs`）を採ったことの
//! 帰結である——同関数はリビール済みのグリフのみを数える。
//!
//! # 会話終了後のタイムアウト（Requirement 4）
//!
//! 満了予定の起点は**会話の占有終端**（待機を含む台本の終わり）である。正典のカウント起点
//! 「スクリプトの表示が終わってから」に一致させるため、初期の確立式は
//! `満了予定 = 占有終端 + 既定時間` であって、「計測が成り立った最初のフレームの現在時刻 +
//! 既定時間」ではない——観測はフレーム単位で飛び飛びに入るため、後者では観測の遅れが
//! そのまま満了のずれになる。現在時刻を起点に取り直すのは抑止が解けた瞬間だけである
//! （Requirement 5.3）。
//!
//! 判断が迷う場面では常に**表示を保持する側**へ倒す——占有終端の信号が届かない間は計測を
//! 始めず（Requirement 4.8）、現在時刻が分からないフレームは計測に触れず、抑止中の超過は
//! 保留し続ける（Requirement 5.6）。唯一の例外が抑止の観測不能で、これは**抑止なし**として
//! 扱う（Requirement 5.5）——観測が取れないことを抑止と読むと、消えないまま固着する側へ
//! 倒れるためである。
//!
//! 待ち時間そのものの既定値（30 秒）は [`DEFAULT_BALLOON_TIMEOUT_SECS`] ただ 1 箇所で定義し
//! （Requirement 4.2）、実機サインオフで満了を観測するための短縮を環境変数
//! `AREKA_BALLOON_TIMEOUT_MS` で受ける（Requirement 9.5）。判断中核 [`decide`] は待ち時間を
//! 引数で受け取るだけで、環境変数にも既定値にも依存しない。
//!
//! # 可視かどうかの真実源
//!
//! 「現に可視か」は毎フレームの観測（本番では `EmoPresenter::target_visible`）が答える。
//! 本モジュールは第 2 の可視性帳簿を作らない。[`ScopeVisibility::prev_visible`] はエッジ検出
//! 専用であって、判断の根拠ではない。
//!
//! # 決定論（Requirement 9.1 の前提）
//!
//! 同一の観測列に対して常に同一の行動列・ログ列を返す。出力 [`Vec`] の並びを決めているのは
//! [`VisibilityObservations::scopes`] の走査順ただ 1 つで、これを [`BTreeMap`] にすることで
//! scope 昇順に固定し、ハッシュの反復順に左右されない形にしてある。
//! （[`BalloonVisibilityState`] の `per_scope` は scope をキーに引くだけで走査しないため出力順には
//! 効かないが、同じ写像の型を揃えてある。design の Data Models は `HashMap` と書いているが、
//! 行動とログの並びが観測可能である以上、決定論の不変条件「`decide` は同一入力に対し決定論」を
//! 満たす順序つきの写像を採る。）

use std::collections::{BTreeMap, BTreeSet};

use super::talk_lifecycle::TalkLifecycleSignal;

#[path = "balloon_visibility_decision.rs"]
mod decision;
#[path = "balloon_visibility_wait.rs"]
mod wait;

pub(crate) use decision::decide;
use decision::hide_reaches_boxes;
pub(crate) use wait::configured_timeout_secs;
#[cfg(test)]
use wait::{parse_timeout_ms, resolve_timeout_secs};

/// バルーンを非表示にするまでの既定の待ち時間（秒）。**この値の定義箇所はここ 1 つだけ**
/// （Requirement 4.2——同じ数値を複数箇所へ散らさない）。
///
/// 30 秒は正典が沈黙する領域の areka 裁量であり、互換対応表へ記録する対象である
/// （Requirement 7.6）。
pub(crate) const DEFAULT_BALLOON_TIMEOUT_SECS: f64 = 30.0;

/// 既定の待ち時間を短縮するための環境変数名（`AREKA_` 冠規約・design 決定 D6）。
///
/// 実機サインオフで満了を観測するための手段であり（Requirement 9.5）、本番の既定経路では
/// 未設定である。値は**正の整数ミリ秒**。
const TIMEOUT_ENV_KEY: &str = "AREKA_BALLOON_TIMEOUT_MS";

/// 採用した待ち時間がどこから来たか（ログの `source` フィールド・design 決定 D6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeoutSource {
    /// [`DEFAULT_BALLOON_TIMEOUT_SECS`]（環境変数が未設定、または不正で縮退した場合）。
    Default,
    /// 環境変数 [`TIMEOUT_ENV_KEY`] による短縮指定。
    Env,
}

impl TimeoutSource {
    /// ログへ書く表記。実機サインオフの文字列検索に使う語である。
    fn as_str(self) -> &'static str {
        match self {
            TimeoutSource::Default => "default",
            TimeoutSource::Env => "env",
        }
    }
}

/// 表示・非表示が起きた契機の種別（ログの `trigger` フィールド・design 決定 D10）。
///
/// D10 の語彙 `content` / `clear` / `timeout` / `explicit` の 4 種に、利用者の中断
/// `user_break`（areka-P0-balloon-break 要件 4.4）を足した 5 種を持つ。
/// [`VisibilityTrigger::Explicit`] 以外は判断中核が導き、`Explicit` だけは配線層が前フレームとの
/// 差分から導く（判断中核は自分が発行していない遷移を知らない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisibilityTrigger {
    /// 可視コンテンツの配置（可視グリフ数の増加・Requirement 2.1 / 4.7）。
    Content,
    /// 内容の全消去（可視グリフ数のゼロへの下降・Requirement 3.1）。
    Clear,
    /// 会話の表示終了から既定時間が過ぎたこと（Requirement 4.3）。
    Timeout,
    /// 本制御が発行していない可視性の変化（`\b[-1]` の明示指令・面切替の全透明退化など）。
    ///
    /// 配線層が前フレームの [`ScopeVisibility::prev_visible`] と本フレームの観測との差分から
    /// 検出する（Requirement 8.1 の「明示指令」）。判断中核はこの契機を作らない。
    Explicit,
    /// 利用者がバルーンを左ダブルクリックして再生を中断した
    /// （areka-P0-balloon-break 要件 4.1・[`TalkLifecycleSignal::UserBreak`] の畳み込み）。
    UserBreak,
}

impl VisibilityTrigger {
    /// ログの `trigger` フィールドへ書く語（design 決定 D10 の語彙・実機サインオフの検索語）。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            VisibilityTrigger::Content => "content",
            VisibilityTrigger::Clear => "clear",
            VisibilityTrigger::Timeout => "timeout",
            VisibilityTrigger::Explicit => "explicit",
            VisibilityTrigger::UserBreak => "user_break",
        }
    }
}

/// 成立している抑止条件の内訳（Requirement 8.3 のログ用）。
///
/// 抑止 = ドラッグ中 ∨ 可視 scope へのポインタ滞在 ∨ 装着 scope の選択肢表示中
/// （design「可視性の判断フロー」の抑止の式）。観測が取れなかった条件は成立しない側へ倒す
/// （Requirement 5.5）。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SuppressionKinds {
    /// いずれかのバルーン窓がドラッグ中（Requirement 5.1）。
    pub(crate) dragging: bool,
    /// いずれかの**可視**バルーンの上にポインタが滞在中（Requirement 5.2）。
    pub(crate) hover: bool,
    /// いずれかの装着 scope で選択肢が表示中（Requirement 5.4・外部所有の照会）。
    pub(crate) choice: bool,
}

impl SuppressionKinds {
    /// いずれかの抑止条件が成立しているか。
    fn any(self) -> bool {
        self.dragging || self.hover || self.choice
    }
}

/// タイムアウト計測を破棄した理由（Requirement 8.2 のログ用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MeasurementDiscardReason {
    /// 次の会話が始まった（Requirement 4.5）。
    TalkStarted,
    /// 占有終端が現在時刻より未来へ更新された（選択肢バリア解除後の cue 到着）。
    DisplayEndAdvanced,
    /// 可視のバルーンが 1 つも無くなった（消す対象が無い）。
    NoVisibleScope,
}

impl MeasurementDiscardReason {
    /// ログの `reason` フィールドへ書く語（実機サインオフの検索語）。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            MeasurementDiscardReason::TalkStarted => "talk_started",
            MeasurementDiscardReason::DisplayEndAdvanced => "display_end_advanced",
            MeasurementDiscardReason::NoVisibleScope => "no_visible_scope",
        }
    }
}

/// 判断中核が生成するログ用の事象（配線層が `info!` へ写す・Requirement 8.1 / 8.6）。
///
/// **遷移や計測の状態が動いたフレームでしか生成しない**。毎フレームの判定そのものは 1 件も
/// 生成しない（Requirement 8.6）。
///
/// 観測そのものが取れなかったこと（文字層ランタイムの借用失敗・ポインタ配線の不在）は
/// ここでは扱わない——design「Error Handling → Error Strategy」がその記録を失敗の検出点
/// である配線層（Requirement 8.4 の `error!`）へ割り当てているためで、判断中核は
/// 「観測が無い」を入力の一形態として受け取るだけである。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum VisibilityLogEvent {
    /// 可視状態が遷移した（Requirement 8.1: 契機・対象 scope・遷移後の可視状態）。
    Transition {
        scope: u32,
        trigger: VisibilityTrigger,
        visible: bool,
    },
    /// タイムアウト計測を開始した（Requirement 8.2: 起点＝占有終端・満了予定）。
    MeasurementStarted { display_end: f64, deadline: f64 },
    /// タイムアウト計測を破棄した（Requirement 8.2: 理由と破棄した満了予定）。
    MeasurementDiscarded {
        reason: MeasurementDiscardReason,
        deadline: f64,
    },
    /// 抑止の解除に伴い計測をやり直した（Requirement 8.2 / 5.3: 起点＝現在時刻・満了予定）。
    MeasurementRestarted { now: f64, deadline: f64 },
    /// 抑止が成立しているため非表示を見送った（Requirement 8.3・抑止 1 回につき 1 件）。
    TimeoutSuppressed {
        kinds: SuppressionKinds,
        deadline: f64,
    },
    /// 可視コンテンツが現れたのに会話の表示終了信号が 1 件も届いていない
    /// （Requirement 4.8・会話 1 回。表示は保持したまま記録だけ残す）。
    DisplayEndSignalMissing,
}

/// 判断中核が配線層へ依頼する行動。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VisibilityAction {
    /// 当該 scope のバルーンを可視化する。
    Show { scope: u32 },
    /// 列挙した scope のバルーンをまとめて不可視にする（`scopes` は scope 昇順）。
    HideScopes {
        scopes: Vec<u32>,
        trigger: VisibilityTrigger,
    },
}

/// [`decide`] の返り値。行動とログを分けて返し、発行もログ出力も配線層に委ねる。
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct VisibilityDecision {
    /// 発行する行動。並びは「中断の非表示 → 全消去の非表示 → 表示 → 満了の非表示」で、
    /// 各行動の中の scope は昇順で固定する。
    pub(crate) actions: Vec<VisibilityAction>,
    /// ログ用の事象。並びは「信号の畳み込み → 中断の非表示（scope 昇順）→ 可視コンテンツ駆動の
    /// 遷移（scope 昇順）→ 計測と満了」で固定する。
    pub(crate) logs: Vec<VisibilityLogEvent>,
}

/// 1 scope・1 フレームの「可視の文字」の観測（同じ借用・同じ注入時刻で読んだ組）。
///
/// 数と回数を組として型で縛るのは、片方だけが更新される記憶（回数だけ進み数が古いまま）を
/// 作れなくするため。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GlyphObservation {
    /// リビール済みの可視グリフ数（`TextLayerState::visible_glyphs`）。
    pub(crate) count: usize,
    /// その scope の内容が消された回数（`TextLayerState::clear_count`）。
    pub(crate) clear_count: u64,
}

/// ある scope について本フレームに観測した値。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopeObservation {
    /// 本フレームのリビール済み可視グリフ数と消去の回数の組。
    ///
    /// `None` は**観測が取れなかった**ことを表す（本番では文字層ランタイムの借用失敗・注入時刻なし）。
    /// その場合は増加も下降も判定せず、直前に観測できた値を次フレームの比較相手として
    /// 保持する——観測できないフレームを「ゼロへ下降した」と読んで消してしまわないため。
    pub(crate) visible_glyphs: Option<GlyphObservation>,
    /// 現に可視か（本番では `EmoPresenter::target_visible`）。判断の真実源。
    pub(crate) visible: bool,
    /// この scope のバルーンの上にポインタが滞在しているか（本番では
    /// `BalloonWiring::is_balloon_hovered`・Requirement 5.2）。
    ///
    /// `None` は**観測が取れなかった**ことを表す（本番ではポインタ配線そのものの不在）。
    /// その場合は抑止しない側として扱う（Requirement 5.5——消えないまま固着する側へ倒さない）。
    pub(crate) hover: Option<bool>,
    /// この scope で選択肢が表示中か（本番では `TextLayerRuntime::choice_active`・
    /// Requirement 5.4 の外部所有の照会）。
    ///
    /// `None` は観測が取れなかったことを表し、`hover` と同じく抑止しない側へ倒す
    /// （Requirement 5.5）。
    pub(crate) choice_active: Option<bool>,
    /// この scope の箱（シェル内バルーン）のどれかに文字が出ているか（areka-P0-shell-balloon
    /// 要件 6.10・6.11）。既定は偽で、偽なら判断は箱を足す前と同じになる。
    ///
    /// 中断と時間切れの対象は「窓が見えている、または箱に文字が出ている」scope である。
    /// 窓を出す判断（`decide_content`）はこの欄を読まない。
    pub(crate) box_showing: bool,
}

/// 本フレームの観測スナップショット。
///
/// `scopes` の母集合は装着済みのバルーン scope（本番では装着済みバルーン資産のキー）。
/// 空（装着前）のときは [`decide`] が自然に何もしない。
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct VisibilityObservations {
    /// scope 番号 → 観測値。
    pub(crate) scopes: BTreeMap<u32, ScopeObservation>,
    /// 本フレームに受け取った表示ライフサイクル信号（到着順＝FIFO）。
    ///
    /// 本番では `Emo2Wiring` の受信端を配線層が全件取り出して渡す。受け取りは配線層の仕事だが、
    /// 受け取った信号が計測へ及ぼす作用（会話開始での破棄・占有終端の更新）は判断であり、
    /// ここで扱う。
    pub(crate) lifecycle: Vec<TalkLifecycleSignal>,
    /// いずれかのバルーン窓がドラッグ中か（Requirement 5.1）。
    ///
    /// 本番の観測は world の問い合わせ（ドラッグ標識つきバルーン窓の有無）であり、失敗しうる
    /// 借用や不在の資源を経由しないため、`hover` / `choice_active` と違って「観測できない」
    /// 状態を持たない。
    pub(crate) dragging: bool,
}

/// scope ごとの前フレーム状態（エッジ検出のための記憶）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopeVisibility {
    /// 直近に**観測できた**可視グリフ数。観測が取れなかったフレームでは更新しない。
    pub(crate) last_glyphs: usize,
    /// 直近に観測できた消去の回数（`last_glyphs` と同じフレームでだけ更新する）。
    pub(crate) last_clear_count: u64,
    /// 前フレーム終了時点の可視状態（本判断が同フレームで発行した遷移を反映した値）。
    ///
    /// 用途は**自分が発行していない可視性遷移の検出**（`trigger=explicit` のログと、非表示へ
    /// 落ちた scope のポインタ滞在フラグの掃除）だけで、可視かどうかの判断には使わない。
    /// 発行分を反映するのはそのためで、観測値をそのまま覚えると、自分が出した表示が次の
    /// フレームで「外から表示された」と誤検出される。読み手は配線層の
    /// `log_external_transitions`（`balloon_visibility_phase.rs`）である。
    ///
    /// **発行が実らなかったときの巻き戻し**: [`decide`] は `Show` を積む時点でこの値を `true` に
    /// する（発行が成功する前提で先に反映する）。design の Error Strategy に従って配線層は失敗した
    /// scope だけを飛ばすので、そのままだと `prev_visible=true` に対して実際の観測は `false` の
    /// まま次フレームを迎え、外因遷移の検出が偽の `trigger=explicit`（Requirement 8.1）を出して
    /// 不要なポインタ滞在の掃除まで走る。配線層はこれを避けるため、表示が可視に至らなかった
    /// scope についてこの値を発行前の観測値へ戻す（`roll_back_show`）。相関数は本モジュールの
    /// 子モジュールにあるため、この非公開フィールドへ直接書き戻せる。
    pub(crate) prev_visible: bool,
}

/// 観測が取れなかったことを既に 1 回記録したか（配線層専用・Requirement 8.4 のログ抑制）。
///
/// 観測の失敗（design「Error Handling → Error Strategy」の「観測の失敗」）は毎フレーム同じ形で
/// 続きうるため、素朴に記録すると誤りレベルの行でログを埋める（Requirement 8.6 が禁じる常時出力と
/// 同じ害）。ここで「1 回鳴らして、観測が戻ったら武装し直す」を持つ——`text_scale_warned`
/// （`frame/wiring.rs:94`）と同型のエッジガードである。
///
/// 判断中核 [`decide`] はこの値を読み書きしない（縮退の方向は観測値そのものが表す）。
#[derive(Debug, Default)]
struct ObservationFailureLogged {
    /// 文字層ランタイムの借用に失敗した（グリフ数と選択肢表示中が観測できない）。
    runtime: bool,
    /// ポインタ配線（`BalloonWiring`）が world に無い（滞在が観測できない）。
    hover_wiring: bool,
    /// 箱の上の滞在の記録（`ShellBoxHover`）が world に無く、文字の出ている箱への滞在が
    /// 観測できない（areka-P0-shell-balloon 要件 6.11）。
    box_hover: bool,
    /// 表示層に当該 scope の target が無く、現に可視かが観測できない scope の集合。
    unattached_scopes: BTreeSet<u32>,
}

/// 可視性コントローラが持ち越す状態（UI スレッド専有・design の Data Models）。
///
/// scope ごとの状態と会話単位の状態を 1 つに束ねる。
#[derive(Debug, Default)]
pub(crate) struct BalloonVisibilityState {
    /// scope ごとのエッジ検出状態。観測された scope の分だけ生える。
    per_scope: BTreeMap<u32, ScopeVisibility>,
    /// 現在の会話の占有終端（talk 相対秒・表示終了信号の最大値）。`None` は信号未着。
    display_end: Option<f64>,
    /// 非表示の満了予定（talk 相対秒）。`None` は計測なし。
    deadline: Option<f64>,
    /// 利用者の中断でバルーンを隠したので、内容の増加では出し直さない
    /// （areka-P0-balloon-break 要件 4.8）。次の会話開始の信号で解く。
    ///
    /// 止めた台本の文字は、再生を止めた後も時刻の進行だけで見える数が増えうる
    /// （`crates/areka-emo-text/src/state.rs` の `RevealSchedule` が先の時刻まで決まっているため）。
    /// この掛け金が無いと、文の途中で止めたときだけ消えたバルーンが次の描画で戻ってくる。
    break_latch: bool,
    /// 前フレームの抑止の成否（抑止解除エッジの検出用・Requirement 5.3）。
    prev_suppressed: bool,
    /// 抑止による見送りのログを今回の抑止で既に 1 回出したか（Requirement 8.3）。
    /// 抑止の解除で武装し直す。
    suppress_logged: bool,
    /// 表示終了信号の欠落の記録を当該会話で既に 1 回出したか（Requirement 4.8）。
    /// 会話開始の通知で武装し直す。
    signal_gap_warned: bool,
    /// 観測の失敗を既に記録したか（配線層専用のエッジガード）。
    observation_failure_logged: ObservationFailureLogged,
    /// 表示ライフサイクル信号の受信端が切断されたことを既に記録したか
    /// （design「観測の収集（配線層）」の「切断検出は error! 1 回」・配線層専用）。
    ///
    /// 切断は復旧しない片道の状態なので、武装し直しの経路を持たない。
    lifecycle_disconnect_logged: bool,
}

impl BalloonVisibilityState {
    /// バルーンの差し替えで、その scope の可視の記憶を消し、計測を止める
    /// （areka-P0-shell-balloon-switch 要件 3.3・design「SwitchPhase」の完了の後始末）。
    ///
    /// 古い装着は消え、新しいバルーンは外部所有のまま隠れているので、可視の記憶を偽へ倒す
    /// （消えた古いバルーンを「外から隠された」と読まない）。見えるバルーンが無くなるので計測も
    /// 止める。文字の数の記憶は保つ——消すと、表示の済んだ文字が次のフレームで増加の縁に化け、
    /// 新しいバルーンが台詞の外で出てしまう（次の台詞で今日どおり現れる＝要件 3.3）。
    pub(crate) fn forget_scope(&mut self, scope: u32) {
        if let Some(previous) = self.per_scope.get_mut(&scope) {
            previous.prev_visible = false;
        }
        self.deadline = None;
    }
}

/// 可視コンテンツ駆動の判定が本フレームに導いた遷移（いずれも scope 昇順）。
struct ContentDecisions {
    /// 表示する scope。
    shown: Vec<u32>,
    /// 内容の全消去に伴い非表示にする scope。
    cleared: Vec<u32>,
}

// ---------------------------------------------------------------------------
// 相関数（フレームの相順から呼ばれる配線層）
// ---------------------------------------------------------------------------

// 配線層は子モジュールへ置く。本ファイルは判断中核だけで既に 700 行を超えており、観測の収集・
// 発行・ログを同居させるとリポジトリの 1 ファイル 1,000 行の上限を越えるためである
// （design の File Structure Plan が「テーマ超過時はさらに分割」と既に想定している形）。
// 子モジュールは親の非公開項目（[`BalloonVisibilityState::per_scope`] 等）へ到達できるため、
// 「相関数は判断中核と同一の可視性の内側に置く」という設計上の前提は保たれる。
#[path = "balloon_visibility_phase.rs"]
mod phase;

pub(super) use phase::run_balloon_visibility_phase;

#[cfg(test)]
#[path = "balloon_visibility_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "balloon_visibility_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "balloon_visibility_timeout_config_tests.rs"]
mod timeout_config_tests;

#[cfg(test)]
#[path = "balloon_visibility_content_tests.rs"]
mod content_tests;

#[cfg(test)]
#[path = "balloon_visibility_timeout_suppression_tests.rs"]
mod timeout_suppression_tests;

#[cfg(test)]
#[path = "balloon_visibility_user_break_tests.rs"]
mod user_break_tests;

#[cfg(test)]
#[path = "balloon_visibility_forget_tests.rs"]
mod forget_tests;

#[cfg(test)]
#[path = "balloon_visibility_box_tests.rs"]
mod box_tests;

// 会話終了観測から判断中核までの端から端（task 6.6）。実台本の再生から占有終端が計測起点に
// なるところまでを 1 本で通す。判断中核の私有状態を読むため親の内側に置く。
#[cfg(test)]
#[path = "balloon_visibility_lifecycle_e2e_tests.rs"]
mod lifecycle_e2e_tests;
