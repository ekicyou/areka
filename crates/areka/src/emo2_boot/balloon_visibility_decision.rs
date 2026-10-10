//! balloon_visibility の子: 見える・隠すの判断（`decide` と、ライフサイクル信号・利用者の中断・内容の増減の判定）。
//! 足す予定の spec: なし。

use super::wait::decide_timeout;
use super::{
    BalloonVisibilityState, ContentDecisions, MeasurementDiscardReason, ScopeVisibility, TalkEnd,
    TalkLifecycleSignal, TalkTimeout, VisibilityAction, VisibilityDecision, VisibilityLogEvent,
    VisibilityObservations, VisibilityTrigger,
};

/// 本フレームの可視性遷移を決める（純関数・`World` / GPU / 時計に触れない）。
///
/// 判定は 4 段で、この順に依存する——⑴ 表示ライフサイクル信号の畳み込み（会話の開始・占有
/// 終端・待ち時間の指定・トークの終わり・利用者の中断）、⑵ 利用者の中断による非表示、⑶ 可視コンテンツ駆動の表示・非表示、
/// ⑷ タイムアウトの計測・抑止・満了。⑵ が ⑶ より前にあるのは、中断で隠した scope を ⑶ 以降が
/// 不可視として扱うためである。⑷ が最後にあるのは、満了で消す対象が「本フレームの発行を
/// 反映したあとに可視である scope」だからである。
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

    let broke = apply_lifecycle_signals(state, obs, now_talk_time, &mut logs);
    let broken = decide_user_break(state, obs, broke, &mut logs);
    let content = decide_content(state, obs, &broken, &mut logs);
    let (timed_out, timeout_notice) = decide_timeout(
        state,
        obs,
        now_talk_time,
        timeout_secs,
        &broken,
        &content,
        &mut logs,
    );

    // 並びは「中断の非表示 → 全消去の非表示 → 表示 → 満了の非表示」で固定する。中断で隠した
    // scope を同じフレームで表示し直すことはありうる（中断と次の会話の開始が同じフレームに
    // 届いた場合）ので、その 1 組だけは順序に意味がある——古い表示が消えてから新しい会話の
    // バルーンが出る。残りは別々の scope に限られる（1 つの scope が増加エッジとゼロ下降エッジを
    // 同時に満たすことはなく、本フレームに表示した scope は満了の対象から外してある）ため意味上の
    // 依存は無いが、出力の並びを入力から一意に決めるために順序を決め打つ。
    let mut actions: Vec<VisibilityAction> = Vec::new();
    if !broken.is_empty() {
        actions.push(VisibilityAction::HideScopes {
            scopes: broken,
            trigger: VisibilityTrigger::UserBreak,
        });
    }
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

    VisibilityDecision {
        actions,
        logs,
        timeout_notice,
    }
}

/// 隠す発行が箱（シェル内バルーン）の文字にも届くかを決める（areka-P0-shell-balloon 要件 6.10）。
///
/// - 文字が 0 に落ちたことによる非表示（`Clear`）は窓だけ——箱のあるサーフェスへ切り替えて
///   普通のバルーンの文字が 0 になっても、箱の文字は出し続ける（要件 5.1）。
/// - 時間切れ（`Timeout`）は窓と箱の両方。
/// - 利用者の中断（`UserBreak`）は、中断の掛け金が掛かったままなら箱も。同じ巡に次の台詞の
///   始まりが届いて掛け金が解けていれば、箱の文字は新しい台詞のものなので隠さない。
///
/// `break_latch` は信号を畳み込んだ後の値（`BalloonVisibilityState::break_latch`）を渡す。
/// 表示の契機（`Content`）と判断中核が作らない契機（`Explicit`）は隠す発行ではないので偽。
pub(super) fn hide_reaches_boxes(trigger: VisibilityTrigger, break_latch: bool) -> bool {
    match trigger {
        VisibilityTrigger::Timeout => true,
        VisibilityTrigger::UserBreak => break_latch,
        VisibilityTrigger::Clear | VisibilityTrigger::Content | VisibilityTrigger::Explicit => {
            false
        }
    }
}

/// 表示ライフサイクル信号を会話単位の状態へ畳み込み、**本フレームに利用者の中断があったか**を
/// 返す（Requirements 4.1 / 4.5・areka-P0-balloon-break 要件 4.1 / 4.7・
/// areka-P0-balloon-lifecycle-events 要件 5.1・9.5・9.6）。
///
/// 占有終端だけでは計測は立たない——トークの終わり（`TalkEnded`）を畳み込んで初めて計測の
/// 成立条件がそろう（`decide_timeout`）。
///
/// 信号の受け取りは配線層の仕事だが、受け取った信号が計測へ及ぼす作用は判断である。
/// 畳み込みは**線の上の到着順どおり**に進める——同じフレームに中断と次の会話の開始が届いた
/// 場合、掛け金の最終値は後から届いた側で決まる。
fn apply_lifecycle_signals(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    now_talk_time: Option<f64>,
    logs: &mut Vec<VisibilityLogEvent>,
) -> bool {
    let mut broke = false;
    for signal in &obs.lifecycle {
        match *signal {
            TalkLifecycleSignal::TalkStarted { talk_id } => {
                // トークの番号・終わり・待ち時間を初めの値へ戻す
                // （areka-P0-balloon-lifecycle-events 要件 9.6）。
                state.talk_id = talk_id;
                state.talk_end = TalkEnd::NotYet;
                state.talk_timeout = TalkTimeout::Default;
                // 次の会話が始まった。進行中の計測は破棄する（Requirement 4.5）。
                if let Some(deadline) = state.deadline.take() {
                    logs.push(VisibilityLogEvent::MeasurementDiscarded {
                        reason: MeasurementDiscardReason::TalkStarted,
                        deadline,
                    });
                }
                state.display_end = None;
                state.signal_gap_warned = false;
                // 次の会話が始まったので、中断の掛け金を解く（areka-P0-balloon-break 要件 4.7）。
                state.break_latch = false;
                // `per_scope.last_glyphs` は**保持する**。直後に届く全消去の観測がゼロへの
                // 下降エッジとして読まれ、会話冒頭の全非表示を導く（design の Data Models）。
            }
            TalkLifecycleSignal::UserBreak => {
                // 利用者がバルーンを左ダブルクリックした。本フレームで出ているバルーンを
                // すべて隠し、次の会話が始まるまで内容では出し直さない
                // （areka-P0-balloon-break 要件 4.1 / 4.8）。
                broke = true;
                state.break_latch = true;
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
            TalkLifecycleSignal::BalloonTimeout(timeout) => {
                // 待ち時間の指定は到着順に上書きし、最後に届いた値が残る（要件 9.5）。計測は
                // トークの終わりの後にしか立たず、終わりの後には指定が届かないので、立っている
                // 計測を捨てる相手は構造上無い。
                state.talk_timeout = timeout;
            }
            TalkLifecycleSignal::TalkEnded { at } => {
                // 止まった時刻を丸めずに記す。時刻が無ければ「時刻なし」（要件 5.2・5.6）。
                state.talk_end = at.map_or(TalkEnd::TimeUnknown, TalkEnd::At);
            }
        }
    }
    broke
}

/// 利用者の中断で非表示にする scope を返す（areka-P0-balloon-break 要件 4.1 / 4.2 / 4.3）。
///
/// 対象は**現に出ているバルーンすべて**で、中断の合図が起きた scope だけではない。窓が
/// 見えている scope に加え、箱に文字が出ている scope も載せる（areka-P0-shell-balloon
/// 要件 6.10）。どちらも出ていない scope は載せない（隠す指示を重ねて出さない）。
/// 抑止（ドラッグ・ポインタの滞在・選択肢の表示中）は**見ない**——抑止はタイムアウトだけの規則であり、ダブルクリックした利用者は必ず
/// バルーンの上に居るためである。
fn decide_user_break(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    broke: bool,
    logs: &mut Vec<VisibilityLogEvent>,
) -> Vec<u32> {
    if !broke {
        return Vec::new();
    }

    let mut hidden: Vec<u32> = Vec::new();
    for (&scope, observed) in &obs.scopes {
        if !observed.visible && !observed.box_showing {
            continue;
        }
        // 初見の scope でも遷移は成立する（装着直後に外から出ているバルーンを隠す形）。
        state
            .per_scope
            .entry(scope)
            .or_insert(ScopeVisibility {
                last_glyphs: 0,
                last_clear_count: 0,
                prev_visible: false,
            })
            .prev_visible = false;
        hidden.push(scope);
        logs.push(VisibilityLogEvent::Transition {
            scope,
            trigger: VisibilityTrigger::UserBreak,
            visible: false,
        });
    }
    hidden
}

/// 可視コンテンツの増減から表示・非表示を導く（Requirements 2.1〜2.7 / 3.1 / 3.2 / 3.6）。
///
/// `broken` は本フレームに利用者の中断で隠した scope（scope 昇順）。ここから先は**隠した後の
/// 姿**で判断する——観測値に本フレームの発行を重ねる既存の流儀と同じである。
fn decide_content(
    state: &mut BalloonVisibilityState,
    obs: &VisibilityObservations,
    broken: &[u32],
    logs: &mut Vec<VisibilityLogEvent>,
) -> ContentDecisions {
    let mut shown: Vec<u32> = Vec::new();
    let mut cleared: Vec<u32> = Vec::new();

    // 走査は scope 昇順（`BTreeMap`）。行動とログの並びが観測可能である以上、走査順そのものを
    // 決定論の一部として固定する。
    for (&scope, observed) in &obs.scopes {
        // 中断で隠した scope は、この先すべて不可視として扱う。
        let visible = observed.visible && !broken.contains(&scope);

        // 初見の scope は「まだ 1 文字も置かれていない」ところから始める。装着直後の観測が
        // ゼロなら以後もエッジは立たず、そのまま不可視で据え置かれる（Requirement 1.1 と整合）。
        let previous = state.per_scope.entry(scope).or_insert(ScopeVisibility {
            last_glyphs: 0,
            last_clear_count: 0,
            prev_visible: visible,
        });

        let Some(observed_glyphs) = observed.visible_glyphs else {
            // 観測が取れなかったフレーム。増加とも下降とも読まず、`last_glyphs`・
            // `last_clear_count` も据え置く——観測できないことを「消えた」と読むと表示を失う側へ倒れる。
            previous.prev_visible = visible;
            continue;
        };
        let glyphs = observed_glyphs.count;

        let last_glyphs = previous.last_glyphs;
        // 表示の比べる相手: 前に見た後でこの scope の内容が消去されていれば、消去の後に置かれた
        // 文字は空の状態（0）からの増加として数える（同じフレームの全消去＋1 文字でも縁が立つ）。
        let show_baseline = if observed_glyphs.clear_count == previous.last_clear_count {
            last_glyphs
        } else {
            0
        };
        previous.last_glyphs = glyphs;
        previous.last_clear_count = observed_glyphs.clear_count;

        if glyphs > show_baseline && !visible && !state.break_latch {
            // 表示: 可視グリフ数の増加エッジ、かつ現に不可視のときだけ（Requirement 2.1 / 2.5）。
            // 中断の掛け金が掛かっている間は見送る——止めた台本の文字は再生を止めた後も時刻の
            // 進行だけで増えうるため（areka-P0-balloon-break 要件 4.8）。見送りでは記録を 1 件も
            // 作らず（毎フレームの判定は無音という既存規律）、`last_glyphs` は上で更新済みである。
            previous.prev_visible = true;
            shown.push(scope);
            logs.push(VisibilityLogEvent::Transition {
                scope,
                trigger: VisibilityTrigger::Content,
                visible: true,
            });
        } else if glyphs == 0 && last_glyphs > 0 && visible {
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
            previous.prev_visible = visible;
        }
    }

    ContentDecisions { shown, cleared }
}
