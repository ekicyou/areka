//! バルーン可視性の**判断中核**（純関数 [`decide`]）とその状態モデル
//! （design.md「BalloonVisibilityController（`emo2_boot/balloon_visibility.rs`）」・
//! Requirements 2.1〜2.7 / 3.1 / 3.2 / 3.3 / 3.6 / 4.7 / 8.6）。
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

use std::collections::BTreeMap;

/// 表示・非表示が起きた契機の種別（ログの `trigger` フィールド・design 決定 D10）。
///
/// D10 の語彙は `content` / `clear` / `timeout` / `explicit` の 4 種だが、ここに実在するのは
/// 本タスクが判断する 2 種のみである。`timeout` は task 3.2（タイムアウト計測）が、
/// `explicit` は task 4.4（明示指令による外因遷移の検出）が、それぞれ判断を実装する時に足す。
// 本番の消費者は task 4.4（ログの `trigger` フィールドへの書き出し）で着地する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisibilityTrigger {
    /// 可視コンテンツの配置（可視グリフ数の増加・Requirement 2.1 / 4.7）。
    Content,
    /// 内容の全消去（可視グリフ数のゼロへの下降・Requirement 3.1）。
    Clear,
}

/// 判断中核が生成するログ用の事象（配線層が `info!` へ写す・Requirement 8.1 / 8.6）。
///
/// **遷移が起きたフレームでしか生成しない**。毎フレームの判定そのものは 1 件も生成しない
/// （Requirement 8.6）。
///
/// 計測の開始・破棄・やり直し（Requirement 8.2）と抑止による見送り（同 8.3）の事象は
/// task 3.2 が variant として足す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisibilityLogEvent {
    /// 可視状態が遷移した（Requirement 8.1: 契機・対象 scope・遷移後の可視状態）。
    Transition {
        scope: u32,
        trigger: VisibilityTrigger,
        visible: bool,
    },
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
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct VisibilityDecision {
    /// 発行する行動。並びは「非表示 → 表示（scope 昇順）」で固定する。
    pub(crate) actions: Vec<VisibilityAction>,
    /// 遷移ログの事象。並びは scope 昇順で固定する。
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
}

/// 本フレームの観測スナップショット。
///
/// `scopes` の母集合は装着済みのバルーン scope（本番では装着済みバルーン資産のキー）。
/// 空（装着前）のときは [`decide`] が自然に何もしない。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct VisibilityObservations {
    /// scope 番号 → 観測値。
    pub(crate) scopes: BTreeMap<u32, ScopeObservation>,
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
    /// フレームで「外から表示された」と誤検出される。消費は task 4.4。
    ///
    /// **task 4.4 への申し送り（発行失敗時の巻き戻し）**: [`decide`] は `Show` を積む時点で
    /// この値を `true` にする（発行が成功する前提で先に反映する）。design の Error Strategy に
    /// 従い `show_target` が `Err` を返したフレームは当該 scope だけを飛ばすので、そのままだと
    /// `prev_visible=true` に対して実際の観測は `false` のまま次フレームを迎える。task 4.4 は
    /// **`show_target` が `Err` を返した scope について、この値を発行前の値へ巻き戻さなければ
    /// ならない**。巻き戻さないと次フレームの外因遷移検出が偽の `trigger=explicit`
    /// （Requirement 8.1）を出し、不要なポインタ滞在フラグの掃除まで走る。相関数は同一モジュール
    /// 内にあるため、この非公開フィールドへ直接書き戻せる。
    pub(crate) prev_visible: bool,
}

/// 可視性コントローラが持ち越す状態（UI スレッド専有・design の Data Models）。
///
/// scope ごとの状態と会話単位の状態を 1 つに束ねる。会話単位の 4 つのフィールドは本タスクで
/// 定義するのみで、駆動するのは task 3.2（タイムアウト計測と抑止）である。
#[allow(dead_code)] // 本番の構築点は task 4.1（`Emo2Wiring` への保持）。到達後は本 allow を外す
#[derive(Debug, Default)]
pub(crate) struct BalloonVisibilityState {
    /// scope ごとのエッジ検出状態。観測された scope の分だけ生える。
    per_scope: BTreeMap<u32, ScopeVisibility>,
    /// 現在の会話の占有終端（talk 相対秒・表示終了信号の最大値）。駆動は task 3.2。
    display_end: Option<f64>,
    /// 非表示の満了予定（talk 相対秒）。`None` は計測なし。駆動は task 3.2。
    deadline: Option<f64>,
    /// 前フレームの抑止の成否（抑止解除エッジの検出用）。駆動は task 3.2。
    prev_suppressed: bool,
    /// 抑止のログを当該エピソードで既に 1 回出したか。駆動は task 3.2。
    suppress_logged: bool,
    /// 表示終了信号の欠落の警告を当該会話で既に 1 回出したか。駆動は task 3.2。
    signal_gap_warned: bool,
}

/// 本フレームの可視性遷移を決める（純関数・`World` / GPU / 時計に触れない）。
///
/// 判定するのは可視コンテンツ駆動の表示と、内容の全消去に伴う非表示の 2 つだけである。
/// タイムアウトの計測・満了・抑止は task 3.2 がここへ加える——`now_talk_time` と
/// `timeout_secs` はその受け皿として design の署名どおり先に置いてあり、本タスクの判断は
/// 両者を読まない（読まないことが「時刻に依らず同じ遷移列を返す」ことの担保でもある）。
#[allow(dead_code)] // 本番の呼び手は task 4.4（`run_balloon_visibility_phase` の配線）
pub(crate) fn decide(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    // 消費は task 3.2（満了判定の現在時刻・`resolve_talk_time` と同型で `None` は epoch 未確立）
    _now_talk_time: Option<f64>,
    // 消費は task 3.2（満了予定＝占有終端 + 本値）
    _timeout_secs: f64,
) -> VisibilityDecision {
    let mut shows: Vec<VisibilityAction> = Vec::new();
    let mut cleared: Vec<u32> = Vec::new();
    let mut logs: Vec<VisibilityLogEvent> = Vec::new();

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
            shows.push(VisibilityAction::Show { scope });
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

    // 並びは「非表示 → 表示」で固定する。同一フレームで両者が立つのは別々の scope に限られる
    // （1 つの scope が増加エッジとゼロ下降エッジを同時に満たすことはない）ため意味上の依存は
    // 無いが、出力の並びを入力から一意に決めるために順序を決め打つ。
    let mut actions = Vec::with_capacity(shows.len() + usize::from(!cleared.is_empty()));
    if !cleared.is_empty() {
        actions.push(VisibilityAction::HideScopes {
            scopes: cleared,
            trigger: VisibilityTrigger::Clear,
        });
    }
    actions.extend(shows);

    VisibilityDecision { actions, logs }
}

#[cfg(test)]
#[path = "balloon_visibility_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "balloon_visibility_tests.rs"]
mod tests;
