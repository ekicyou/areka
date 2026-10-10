//! balloon_visibility の子: 待ち時間の設定（既定値と環境変数での短縮）と、会話の後の時間切れの判定。
//! 足す予定の spec: なし。

use std::sync::OnceLock;

use tracing::{info, warn};

use areka_sakura::TalkId;

use super::decision::hide_reaches_boxes;

use super::{
    BalloonVisibilityState, ContentDecisions, DEFAULT_BALLOON_TIMEOUT_SECS,
    MeasurementDiscardReason, MeasurementOrigin, SuppressionKinds, TIMEOUT_ENV_KEY, TalkEnd,
    TalkTimeout, TimeoutSource, VisibilityLogEvent, VisibilityObservations, VisibilityTrigger,
};

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
pub(super) fn resolve_timeout_secs(value: Option<&str>) -> (f64, TimeoutSource) {
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

/// このトークの待ち時間（秒）を決める（areka-P0-balloon-lifecycle-events 要件 9.1〜9.3・9.9）。
///
/// 既定は引数で渡された値（30 秒か環境変数。決め方は変えない）。ミリ秒の指定は環境変数と同じ式
/// （1,000 で 1 回割る）で秒へ写し、丸めない。`None` は時間切れで隠さない。
fn talk_timeout_secs(timeout: TalkTimeout, default_secs: f64) -> Option<f64> {
    match timeout {
        TalkTimeout::Default => Some(default_secs),
        TalkTimeout::Millis(ms) => Some(ms as f64 / 1000.0),
        TalkTimeout::Never => None,
    }
}

/// 実効の表示終了と起点の採り方を決める（areka-P0-balloon-lifecycle-events 要件 5.1・5.3・5.6）。
///
/// トークの終わりが届くまでは `None`（計測を始めない）。届いていれば占有区間の終端と止まった
/// 時刻の早い方で、ちょうど同じなら終端を採る。時刻が無ければ終端。
fn effective_display_end(display_end: f64, talk_end: TalkEnd) -> Option<(f64, MeasurementOrigin)> {
    match talk_end {
        TalkEnd::NotYet => None,
        TalkEnd::At(stopped) if stopped < display_end => {
            Some((stopped, MeasurementOrigin::StoppedAt(stopped)))
        }
        TalkEnd::At(_) => Some((display_end, MeasurementOrigin::DisplayEnd)),
        TalkEnd::TimeUnknown => Some((display_end, MeasurementOrigin::StopTimeMissing)),
    }
}

/// タイムアウトの計測・抑止・満了を判定し、満了で非表示にする scope と、そのフレームの
/// 時間切れの知らせ（トークの番号）を返す
/// （Requirements 4.3 / 4.5 / 4.6 / 4.8 / 4.9 / 5.1〜5.3 / 5.5 / 5.6・
/// areka-P0-balloon-lifecycle-events 要件 2.1・2.4・2.5・5.1・5.3・5.5・5.6・7.3・9.1〜9.3・9.7）。
pub(super) fn decide_timeout(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    now_talk_time: Option<f64>,
    timeout_secs: f64,
    broken: &[u32],
    content: &ContentDecisions,
    logs: &mut Vec<VisibilityLogEvent>,
) -> (Vec<u32>, Option<TalkId>) {
    // 現在時刻が分からないフレームは計測に一切触れない（起点未確立）。抑止の持ち越しも
    // 止めるため、時刻が戻ったフレームで解除エッジが改めて立つ——表示を保持する側の縮退。
    let Some(now) = now_talk_time else {
        return (Vec::new(), None);
    };

    // 本フレームの発行を反映した可視 scope。真実源はあくまで観測値で、そこへ本フレームに
    // 発行した表示・非表示を重ねる（第 2 の可視性帳簿を作らない）。表示は最後の行動なので、
    // 中断や全消去で隠した直後に出し直した scope は可視として数える。
    //
    // 「可視」は窓が見えている、または箱に文字が出ていること（areka-P0-shell-balloon 要件 6.10）。
    // 箱の側は本フレームの隠す発行が箱に届いたときだけ消えたとみなす——全消去は窓だけを隠し、
    // 中断は掛け金が掛かったままのときだけ箱にも届く（`hide_reaches_boxes`）。
    let break_reaches_boxes = hide_reaches_boxes(VisibilityTrigger::UserBreak, state.break_latch);
    let visible: Vec<u32> = obs
        .scopes
        .iter()
        .filter(|(scope, observed)| {
            let window = if content.shown.contains(scope) {
                true
            } else if content.cleared.contains(scope) || broken.contains(scope) {
                false
            } else {
                observed.visible
            };
            let boxes = observed.box_showing && !(broken.contains(scope) && break_reaches_boxes);
            window || boxes
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

    // 計測が成り立つ条件: 占有終端が確立し、トークの終わりが届き、このトークの待ち時間が
    // 「なし」でなく、現在時刻が実効の表示終了に達し、消す対象が居ること。占有終端かトークの
    // 終わりが未着の間は計測を始めない＝表示を保持する（Requirement 4.8・
    // areka-P0-balloon-lifecycle-events 決定 D6）。
    let wait_secs = talk_timeout_secs(state.talk_timeout, timeout_secs);
    let eligible = match (state.display_end, wait_secs) {
        (Some(end), Some(wait)) if !visible.is_empty() => {
            effective_display_end(end, state.talk_end)
                .filter(|&(effective, _)| now >= effective)
                .map(|(effective, origin)| (end, effective, origin, wait))
        }
        _ => None,
    };

    let released = state.prev_suppressed && !suppressed;
    state.prev_suppressed = suppressed;

    if released {
        // 抑止が全て解けた。ここからはこのトークの待ち時間を**現在時刻起点で改めて**計り直す
        // （Requirement 5.3——抑止前の残り時間を再開しない・areka-P0-balloon-lifecycle-events 要件 9.7）。
        state.suppress_logged = false;
        if let Some((_, _, _, wait)) = eligible {
            let deadline = now + wait;
            state.deadline = Some(deadline);
            logs.push(VisibilityLogEvent::MeasurementRestarted { now, deadline });
        }
    } else if let Some((display_end, effective, origin, wait)) = eligible
        && state.deadline.is_none()
    {
        // 初期確立の起点は**実効の表示終了**＝占有終端と止まった時刻の早い方で、止まった時刻が
        // 届かなければ占有終端である（Requirement 4.1 の正典起点「スクリプトの表示が終わって
        // から」・areka-P0-balloon-lifecycle-events 要件 5.1・5.3・5.6）。完了 spec
        // areka-P0-balloon-visibility では起点は占有終端だけだったが、本 spec がこう改めた。
        // 「計測が成り立った最初のフレームの現在時刻 + 待ち時間」ではない——観測はフレーム単位で
        // 飛び飛びに入るため、そちらを採ると観測の遅れがそのまま満了のずれになる。中断で終わった
        // トーク（Requirement 4.6）は止まった時刻が占有終端より早いので、止まった時刻が起点になる。
        // 中断のみを理由とする即時非表示の経路はこの段には無い。利用者のダブルクリックによる
        // 中断だけは例外で、[`decide`] の別の段（[`decide_user_break`]）が計測を待たずに隠す
        // （areka-P0-balloon-break 要件 4.1）。起点の採り方は 1 件記録する（毎フレームは記録しない）。
        let deadline = effective + wait;
        state.deadline = Some(deadline);
        logs.push(VisibilityLogEvent::MeasurementStarted {
            origin,
            display_end,
            deadline,
        });
    }

    let Some(deadline) = state.deadline else {
        return (Vec::new(), None);
    };
    // 非数の現在時刻はどちらの比較も偽になり、満了しない側（表示を保持する側）へ倒れる。
    let expired = now >= deadline;
    if !expired {
        return (Vec::new(), None);
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
        return (Vec::new(), None);
    }

    // 満了。本フレームに表示したばかりの scope は対象から外す——出してすぐ消す行動の対を
    // 同じフレームで作らないための縮退で、表示を保持する側へ倒れる。外した scope は実効の表示終了が
    // 据え置かれたままなら次フレームで計測が立ち直り、そこで改めて満了する（消えないまま
    // 固着はしない）。なお本番では、コンテンツを運ぶ cue そのものが配信時点で占有終端を先へ
    // 押し出すため、この分岐は信号を失ったときにしか通らない。
    let targets: Vec<u32> = visible
        .into_iter()
        .filter(|scope| !content.shown.contains(scope))
        .collect();
    if targets.is_empty() {
        return (Vec::new(), None);
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
    // 隠す対象が居たフレームにだけ、いま出ている台詞のトークの番号を知らせに載せる
    // （areka-P0-balloon-lifecycle-events 要件 2.1）。番号が無ければ載せず警告の事象を積む。
    if state.talk_id.is_none() {
        logs.push(VisibilityLogEvent::TimeoutNoticeWithoutTalkId);
    }
    (targets, state.talk_id)
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
