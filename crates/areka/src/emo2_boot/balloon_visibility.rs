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
//! - 表示: ある scope の**可視グリフ数が増えた**フレームで、かつその scope が**現に不可視**のとき
//!   だけ表示する（Requirement 2.1 / 2.5）。起動時・会話開始時・scope 切替時に別条件を設けない。
//! - 非表示（会話開始側）: 可視グリフ数が**ゼロへ下降した**フレームで、かつ現に可視のときだけ
//!   非表示にする（Requirement 3.1）。会話がどの scope から始まるかを先読みしない（Requirement 3.6）。
//!
//! 改行・カーソル移動・待機・内容消去はいずれも可視グリフ数を増やさないため、表示の契機に
//! ならない（Requirement 2.3）。この一致は偶然ではなく、観測量に
//! `TextLayerState::visible_glyphs`（`crates/areka-emo-text/src/state.rs:440`）を採ったことの
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
use std::sync::OnceLock;

use tracing::{info, warn};

use super::talk_lifecycle::TalkLifecycleSignal;

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

/// 環境変数の値から短縮指定のミリ秒を読み取る純関数（環境変数へ触れない・単体テスト可能）。
///
/// - `None`／空／空白のみ → `None`（指定なし＝既定を使う。本番の既定経路なので無音）。
/// - 周辺の空白を落とした**正の**整数 → `Some(ms)`。
/// - `0`・負値・非数・`u64` の範囲超過 → `warn!` の上で `None`（既定へ縮退）。
///
/// `0` を受理しないのは、会話の表示終了と同時にバルーンが消えてしまい、実機サインオフで
/// 「既定時間の経過で消える」ことを観測できなくなるためである（design 決定 D6 は正の整数
/// ミリ秒と定めている）。`u64::from_str` が負号・小数点・非数字・範囲超過をいずれも `Err`
/// にするため、それらは自然に不正側へ落ちる。
pub(crate) fn parse_timeout_ms(value: Option<&str>) -> Option<u64> {
    let trimmed = value.map(str::trim)?;
    // 未設定と空白のみは同じ「指定なし」であり、誤りではないので警告を出さない。
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.parse::<u64>() {
        Ok(ms) if ms > 0 => Some(ms),
        _ => {
            warn!(
                env = TIMEOUT_ENV_KEY,
                value = trimmed,
                "[balloon-visibility] 待ち時間の指定が不正（正の整数ミリ秒を要する）→ 既定へ縮退"
            );
            None
        }
    }
}

/// 採用する待ち時間とその供給源を決め、1 行記録して返す（環境変数へ触れない）。
///
/// 記録は採用値・供給源・既定値の 3 つを同じ 1 行に載せる。短縮して満了を観測している最中
/// でも「短縮していない既定値が 30 秒であること」を同じ行から確かめられるようにするため
/// である（Requirement 9.5）。
fn resolve_timeout_secs(value: Option<&str>) -> (f64, TimeoutSource) {
    let (secs, source) = match parse_timeout_ms(value) {
        Some(ms) => (ms as f64 / 1000.0, TimeoutSource::Env),
        None => (DEFAULT_BALLOON_TIMEOUT_SECS, TimeoutSource::Default),
    };
    info!(
        env = TIMEOUT_ENV_KEY,
        timeout_secs = secs,
        source = source.as_str(),
        default_secs = DEFAULT_BALLOON_TIMEOUT_SECS,
        "[balloon-visibility] バルーン非表示までの待ち時間を確定"
    );
    (secs, source)
}

/// 採用する待ち時間（秒）を返す。環境変数はプロセスで**一度だけ**読む（design 決定 D6）。
///
/// 記録もこの初回の解決 1 回きりで、毎フレームの呼び出しでは何も起きない
/// （Requirement 8.6——常時出力でログを埋めない）。
pub(crate) fn configured_timeout_secs() -> f64 {
    static RESOLVED: OnceLock<f64> = OnceLock::new();
    *RESOLVED.get_or_init(|| resolve_timeout_secs(std::env::var(TIMEOUT_ENV_KEY).ok().as_deref()).0)
}

/// 表示・非表示が起きた契機の種別（ログの `trigger` フィールド・design 決定 D10）。
///
/// D10 の語彙 `content` / `clear` / `timeout` / `explicit` の 4 種をそのまま持つ。前 3 者は
/// 判断中核が導き、[`VisibilityTrigger::Explicit`] だけは配線層が前フレームとの差分から導く
/// （判断中核は自分が発行していない遷移を知らない）。
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
}

impl VisibilityTrigger {
    /// ログの `trigger` フィールドへ書く語（design 決定 D10 の語彙・実機サインオフの検索語）。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            VisibilityTrigger::Content => "content",
            VisibilityTrigger::Clear => "clear",
            VisibilityTrigger::Timeout => "timeout",
            VisibilityTrigger::Explicit => "explicit",
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
    /// 発行する行動。並びは「全消去の非表示 → 表示 → 満了の非表示」で、
    /// 各行動の中の scope は昇順で固定する。
    pub(crate) actions: Vec<VisibilityAction>,
    /// ログ用の事象。並びは「信号の畳み込み → 可視コンテンツ駆動の遷移（scope 昇順）→
    /// 計測と満了」で固定する。
    pub(crate) logs: Vec<VisibilityLogEvent>,
}

/// ある scope について本フレームに観測した値。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScopeObservation {
    /// 本フレームのリビール済み可視グリフ数。
    ///
    /// `None` は**観測が取れなかった**ことを表す（本番では文字層ランタイムの借用失敗）。
    /// その場合は増加も下降も判定せず、直前に観測できた値を次フレームの比較相手として
    /// 保持する——観測できないフレームを「ゼロへ下降した」と読んで消してしまわないため。
    pub(crate) visible_glyphs: Option<usize>,
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

/// 本フレームの可視性遷移を決める（純関数・`World` / GPU / 時計に触れない）。
///
/// 判定は 3 段で、この順に依存する——⑴ 表示ライフサイクル信号の畳み込み（会話の開始と占有
/// 終端）、⑵ 可視コンテンツ駆動の表示・非表示、⑶ タイムアウトの計測・抑止・満了。⑶ が ⑵ の
/// 後にあるのは、満了で消す対象が「本フレームの発行を反映したあとに可視である scope」だから
/// である。
///
/// `now_talk_time` は talk 相対秒の現在時刻（`resolve_talk_time` と同型で `None` は起点未確立）。
/// `None` のフレームではタイムアウトの評価そのものを行わない——時刻が分からないまま満了を
/// 名乗ると、表示を失う側へ倒れるためである。
pub(crate) fn decide(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    now_talk_time: Option<f64>,
    timeout_secs: f64,
) -> VisibilityDecision {
    let mut logs: Vec<VisibilityLogEvent> = Vec::new();

    apply_lifecycle_signals(state, obs, now_talk_time, &mut logs);
    let content = decide_content(state, obs, &mut logs);
    let timed_out = decide_timeout(state, obs, now_talk_time, timeout_secs, &content, &mut logs);

    // 並びは「全消去の非表示 → 表示 → 満了の非表示」で固定する。同一フレームで複数が立つのは
    // 別々の scope に限られる（1 つの scope が増加エッジとゼロ下降エッジを同時に満たすことは
    // なく、本フレームに表示した scope は満了の対象から外してある）ため意味上の依存は無いが、
    // 出力の並びを入力から一意に決めるために順序を決め打つ。
    let mut actions: Vec<VisibilityAction> = Vec::new();
    if !content.cleared.is_empty() {
        actions.push(VisibilityAction::HideScopes {
            scopes: content.cleared,
            trigger: VisibilityTrigger::Clear,
        });
    }
    actions.extend(
        content
            .shown
            .iter()
            .map(|&scope| VisibilityAction::Show { scope }),
    );
    if !timed_out.is_empty() {
        actions.push(VisibilityAction::HideScopes {
            scopes: timed_out,
            trigger: VisibilityTrigger::Timeout,
        });
    }

    VisibilityDecision { actions, logs }
}

/// 可視コンテンツ駆動の判定が本フレームに導いた遷移（いずれも scope 昇順）。
struct ContentDecisions {
    /// 表示する scope。
    shown: Vec<u32>,
    /// 内容の全消去に伴い非表示にする scope。
    cleared: Vec<u32>,
}

/// 表示ライフサイクル信号を会話単位の状態へ畳み込む（Requirements 4.1 / 4.5）。
///
/// 信号の受け取りは配線層の仕事だが、受け取った信号が計測へ及ぼす作用は判断である。
fn apply_lifecycle_signals(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    now_talk_time: Option<f64>,
    logs: &mut Vec<VisibilityLogEvent>,
) {
    for signal in &obs.lifecycle {
        match *signal {
            TalkLifecycleSignal::TalkStarted => {
                // 次の会話が始まった。進行中の計測は破棄する（Requirement 4.5）。
                if let Some(deadline) = state.deadline.take() {
                    logs.push(VisibilityLogEvent::MeasurementDiscarded {
                        reason: MeasurementDiscardReason::TalkStarted,
                        deadline,
                    });
                }
                state.display_end = None;
                state.signal_gap_warned = false;
                // `per_scope.last_glyphs` は**保持する**。直後に届く全消去の観測がゼロへの
                // 下降エッジとして読まれ、会話冒頭の全非表示を導く（design の Data Models）。
            }
            TalkLifecycleSignal::DisplayEndAt(end) => {
                // 送出側の畳み込み種は負の無限大であり（`talk_lifecycle.rs:89`）、非有限値は
                // 送出側で弾かれるが負の有限値は通る。下限 0.0 で丸めて、起点が負へ回った分だけ
                // 早く消える側へ倒れる乖離を断つ（非数もこの丸めで 0.0 になる）。
                let end = end.max(0.0);
                let raised = state.display_end.is_none_or(|known| end > known);
                if !raised {
                    // 既知最大以下の値は状態を動かさない（重複・後退に対して単調）。
                    continue;
                }
                state.display_end = Some(end);

                // 占有終端が現在時刻より未来へ動いたなら、立っている満了予定はもう根拠を失う
                // （選択肢のバリア解除後に cue が届いた形）。破棄して立て直させる。
                if let Some(now) = now_talk_time
                    && end > now
                    && let Some(deadline) = state.deadline.take()
                {
                    logs.push(VisibilityLogEvent::MeasurementDiscarded {
                        reason: MeasurementDiscardReason::DisplayEndAdvanced,
                        deadline,
                    });
                }
            }
        }
    }
}

/// 可視コンテンツの増減から表示・非表示を導く（Requirements 2.1〜2.7 / 3.1 / 3.2 / 3.6）。
fn decide_content(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    logs: &mut Vec<VisibilityLogEvent>,
) -> ContentDecisions {
    let mut shown: Vec<u32> = Vec::new();
    let mut cleared: Vec<u32> = Vec::new();

    // 走査は scope 昇順（`BTreeMap`）。行動とログの並びが観測可能である以上、走査順そのものを
    // 決定論の一部として固定する。
    for (&scope, observed) in &obs.scopes {
        // 初見の scope は「まだ 1 文字も置かれていない」ところから始める。装着直後の観測が
        // ゼロなら以後もエッジは立たず、そのまま不可視で据え置かれる（Requirement 1.1 と整合）。
        let previous = state.per_scope.entry(scope).or_insert(ScopeVisibility {
            last_glyphs: 0,
            prev_visible: observed.visible,
        });

        let Some(glyphs) = observed.visible_glyphs else {
            // 観測が取れなかったフレーム。増加とも下降とも読まず、`last_glyphs` も据え置く
            // ——観測できないことを「消えた」と読むと表示を失う側へ倒れる。
            previous.prev_visible = observed.visible;
            continue;
        };

        let last_glyphs = previous.last_glyphs;
        previous.last_glyphs = glyphs;

        if glyphs > last_glyphs && !observed.visible {
            // 表示: 可視グリフ数の増加エッジ、かつ現に不可視のときだけ（Requirement 2.1 / 2.5）。
            previous.prev_visible = true;
            shown.push(scope);
            logs.push(VisibilityLogEvent::Transition {
                scope,
                trigger: VisibilityTrigger::Content,
                visible: true,
            });
        } else if glyphs == 0 && last_glyphs > 0 && observed.visible {
            // 非表示: ゼロへの下降エッジ、かつ現に可視のときだけ（Requirement 3.1）。
            // ゼロ以外への下降（部分消去）は契機にしない。
            previous.prev_visible = false;
            cleared.push(scope);
            logs.push(VisibilityLogEvent::Transition {
                scope,
                trigger: VisibilityTrigger::Clear,
                visible: false,
            });
        } else {
            // 遷移なし。ここで何も積まないことが Requirement 8.6（毎フレームの判定は無音）を成す。
            previous.prev_visible = observed.visible;
        }
    }

    ContentDecisions { shown, cleared }
}

/// タイムアウトの計測・抑止・満了を判定し、満了で非表示にする scope を返す
/// （Requirements 4.3 / 4.5 / 4.6 / 4.8 / 4.9 / 5.1〜5.3 / 5.5 / 5.6）。
fn decide_timeout(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    now_talk_time: Option<f64>,
    timeout_secs: f64,
    content: &ContentDecisions,
    logs: &mut Vec<VisibilityLogEvent>,
) -> Vec<u32> {
    // 現在時刻が分からないフレームは計測に一切触れない（起点未確立）。抑止の持ち越しも
    // 止めるため、時刻が戻ったフレームで解除エッジが改めて立つ——表示を保持する側の縮退。
    let Some(now) = now_talk_time else {
        return Vec::new();
    };

    // 本フレームの発行を反映した可視 scope。真実源はあくまで観測値で、そこへ本フレームに
    // 発行した表示・非表示を重ねる（第 2 の可視性帳簿を作らない）。
    let visible: Vec<u32> = obs
        .scopes
        .iter()
        .filter(|(scope, observed)| {
            if content.cleared.contains(scope) {
                false
            } else {
                content.shown.contains(scope) || observed.visible
            }
        })
        .map(|(&scope, _)| scope)
        .collect();

    let suppression = observe_suppression(obs, &visible);
    let suppressed = suppression.any();

    // 表示終了信号の欠落（Requirement 4.8）: 可視コンテンツが現れたのに信号が 1 件も無い。
    // 本番では占有終端の信号が当の cue の配信時点で送られるため、コンテンツより先に届く
    // （`talk_lifecycle.rs` の Ordering 契約）。ここへ来るのは信号が失われた場合である。
    if state.display_end.is_none() && !content.shown.is_empty() && !state.signal_gap_warned {
        state.signal_gap_warned = true;
        logs.push(VisibilityLogEvent::DisplayEndSignalMissing);
    }

    // 消す対象が 1 つも無くなったら計測は意味を失う（満了で消した直後もここを通る）。
    if visible.is_empty()
        && let Some(deadline) = state.deadline.take()
    {
        logs.push(VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::NoVisibleScope,
            deadline,
        });
    }

    // 計測が成り立つ条件: 占有終端が確立し、現在時刻がそこに達し、消す対象が居ること。
    // 占有終端が未確立の間は計測を始めない＝表示を保持する（Requirement 4.8）。
    let eligible_display_end = match state.display_end {
        Some(end) if now >= end && !visible.is_empty() => Some(end),
        _ => None,
    };

    let released = state.prev_suppressed && !suppressed;
    state.prev_suppressed = suppressed;

    if released {
        // 抑止が全て解けた。ここからは既定時間を**現在時刻起点で改めて**計り直す
        // （Requirement 5.3——抑止前の残り時間を再開しない）。
        state.suppress_logged = false;
        if eligible_display_end.is_some() {
            let deadline = now + timeout_secs;
            state.deadline = Some(deadline);
            logs.push(VisibilityLogEvent::MeasurementRestarted { now, deadline });
        }
    } else if let Some(display_end) = eligible_display_end
        && state.deadline.is_none()
    {
        // 初期確立の起点は**占有終端**である（Requirement 4.1 の正典起点「スクリプトの表示が
        // 終わってから」）。「計測が成り立った最初のフレームの現在時刻 + 既定時間」ではない
        // ——観測はフレーム単位で飛び飛びに入るため、そちらを採ると観測の遅れがそのまま
        // 満了のずれになる。中断による終了（Requirement 4.6）も占有終端が起点で、中断のみを
        // 理由とする即時非表示の経路はここに存在しない。
        let deadline = display_end + timeout_secs;
        state.deadline = Some(deadline);
        logs.push(VisibilityLogEvent::MeasurementStarted {
            display_end,
            deadline,
        });
    }

    let Some(deadline) = state.deadline else {
        return Vec::new();
    };
    // 非数の現在時刻はどちらの比較も偽になり、満了しない側（表示を保持する側）へ倒れる。
    let expired = now >= deadline;
    if !expired {
        return Vec::new();
    }

    if suppressed {
        // 抑止中の超過は保留し続ける（Requirement 5.6）。満了予定は消さず、解除エッジで
        // 取り直す。記録は 1 回の抑止につき 1 件（Requirement 8.3）。
        if !state.suppress_logged {
            state.suppress_logged = true;
            logs.push(VisibilityLogEvent::TimeoutSuppressed {
                kinds: suppression,
                deadline,
            });
        }
        return Vec::new();
    }

    // 満了。本フレームに表示したばかりの scope は対象から外す——出してすぐ消す行動の対を
    // 同じフレームで作らないための縮退で、表示を保持する側へ倒れる。外した scope は占有終端が
    // 据え置かれたままなら次フレームで計測が立ち直り、そこで改めて満了する（消えないまま
    // 固着はしない）。なお本番では、コンテンツを運ぶ cue そのものが配信時点で占有終端を先へ
    // 押し出すため、この分岐は信号を失ったときにしか通らない。
    let targets: Vec<u32> = visible
        .into_iter()
        .filter(|scope| !content.shown.contains(scope))
        .collect();
    if targets.is_empty() {
        return Vec::new();
    }

    state.deadline = None;
    for &scope in &targets {
        if let Some(previous) = state.per_scope.get_mut(&scope) {
            previous.prev_visible = false;
        }
        logs.push(VisibilityLogEvent::Transition {
            scope,
            trigger: VisibilityTrigger::Timeout,
            visible: false,
        });
    }
    targets
}

/// 本フレームに成立している抑止条件を数える（design「可視性の判断フロー」の抑止の式）。
///
/// 観測が取れなかった条件（`None`）は成立しない側へ倒す（Requirement 5.5——消えないまま
/// 固着する側へ倒さない）。ポインタの滞在は**可視である scope に限って**効かせる——不可視の
/// 間は離脱の通知が届かず、滞在の記録が残ったままだと恒久的な抑止に固着するためである。
fn observe_suppression(obs: &VisibilityObservations, visible: &[u32]) -> SuppressionKinds {
    let mut kinds = SuppressionKinds {
        dragging: obs.dragging,
        ..SuppressionKinds::default()
    };
    for (scope, observed) in &obs.scopes {
        if observed.hover == Some(true) && visible.contains(scope) {
            kinds.hover = true;
        }
        if observed.choice_active == Some(true) {
            kinds.choice = true;
        }
    }
    kinds
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

// 会話終了観測から判断中核までの端から端（task 6.6）。実台本の再生から占有終端が計測起点に
// なるところまでを 1 本で通す。判断中核の私有状態を読むため親の内側に置く。
#[cfg(test)]
#[path = "balloon_visibility_lifecycle_e2e_tests.rs"]
mod lifecycle_e2e_tests;
