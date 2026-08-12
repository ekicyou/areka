//! バルーン可視性の**配線層**（観測の収集・指令の発行・ログの出力）。
//!
//! 判断は親モジュールの純関数 [`decide`] が全て持つ。ここに置くのは「world と表示層から観測を
//! 集める」「判断が返した行動を発行する」「判断が返した事象をログへ写す」の 3 つだけで、
//! いつ出すか・いつ消すかの分岐は 1 つも持たない
//! （design「BalloonVisibilityController → Responsibilities & Constraints」）。
//!
//! # 1 フレームの手順
//!
//! ⑴ 表示ライフサイクル信号を全件取り出す → ⑵ scope ごとの観測を集める → ⑶ 自分が発行して
//! いない可視性の変化を前フレームとの差分で検出する → ⑷ [`decide`] を呼ぶ → ⑸ 返った行動を
//! 発行する → ⑹ 返った事象をログへ写す → ⑺ 不可視へ落ちた scope のポインタ滞在記録を掃除する。
//!
//! ⑶ が ⑷ より前にあるのは順序の都合ではなく必然である——[`decide`] は自分が積んだ遷移を
//! `prev_visible` へ書き込むため、その後で差分を取ると自分の発行が「外から来た変化」に化ける。
//!
//! # 縮退の向き（design「Error Handling → Error Strategy」）
//!
//! 観測が取れなければ**抑止なし・エッジなし**として扱い（Requirement 5.5——消えないまま固着
//! する側へ倒さない）、発行が失敗すれば**当該 scope だけ**を見送る（他 scope と会話を巻き込ま
//! ない）。いずれも記録の無い経路を作らない（Requirement 8.4）。

use std::cell::{Ref, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, TryRecvError};

use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::TextLayerRuntime;
use areka_sakura::ActorKey;
use bevy_ecs::prelude::{Entity, With};
use bevy_ecs::world::World;
use tracing::{error, info, warn};
use wintf::ecs::{FrameTime, WindowDragging};

use crate::input_events::balloon::BalloonWiring;
use crate::placement::spawn::BalloonWindowMarker;

use super::super::frame::{Emo2Wiring, resolve_talk_time};
use super::super::target_map::balloon_target;
use super::{
    BalloonVisibilityState, ScopeObservation, TalkLifecycleSignal, VisibilityAction,
    VisibilityLogEvent, VisibilityObservations, VisibilityTrigger, configured_timeout_secs, decide,
};

/// バルーン可視性の相（`emo2_frame_system` の相順から毎フレーム呼ばれる配線）。
///
/// # 相順の中での位置（design 決定 D5）
///
/// drain（本フレームの表示指令の適用）の**直後**、窓寸 reconcile の**直前**に置く。3 つの
/// 制約が同時に効いている:
///
/// - 本フレームの `\b` 指令をすべて適用し終えた**後**でなければ、判断が見る「現に可視か」が
///   1 フレーム古い状態になる（Requirement 3.5）。
/// - 表示が積む窓寸要求を同一フレームで消化できるよう、窓寸 reconcile の**前**に置く
///   （非表示から表示へ移った時に窓寸が 1 フレーム遅れないこと・Requirement 6.6）。
/// - 文字層の binding 再構築と描画より**上流**に置く（同じく Requirement 6.6）。
///
/// # 事前条件
///
/// `Emo2Wiring` を world から取り出した状態（`emo2_frame_system` の内側）で呼ぶ。装着が未了で
/// `wiring.balloon_models` が空なら対象 scope がゼロ件となり、信号の取り出しだけを行って
/// 何も発行しない。panic しない。
pub(in crate::emo2_boot) fn run_balloon_visibility_phase(
    wiring: &mut Emo2Wiring,
    world: &mut World,
) {
    // 文字層ランタイムは `Rc` を複製して受け取り、wiring の分解より前に借用を解く。
    let runtime = Rc::clone(wiring.runtime());
    let Emo2Wiring {
        presenter,
        lifecycle_rx,
        balloon_visibility: state,
        balloon_models,
        clock,
        ..
    } = wiring;

    // 対象 scope の母集合は装着済みバルーン（文字層スケール相と同一）。`HashMap` ゆえ列挙順は
    // 不定なので、観測・発行・ログの並びを決定論にするため昇順へ固定する。
    let mut scopes: Vec<u32> = balloon_models.keys().copied().collect();
    scopes.sort_unstable();

    // 注入時刻は text 相と**同一の**解決経路（override 無し＝本番）。
    let frame_now = world.get_resource::<FrameTime>().map(|ft| ft.0);
    let now_talk_time = resolve_talk_time(None, frame_now, clock);
    // 待ち時間はプロセスで一度だけ解決する（初回の呼び出しが起動時の 1 行を出す・design D6）。
    let timeout_secs = configured_timeout_secs();

    let lifecycle = drain_lifecycle(lifecycle_rx, state);
    let observations = collect_observations(
        presenter,
        &runtime,
        world,
        &scopes,
        now_talk_time,
        lifecycle,
        state,
    );

    // 外因の遷移は `decide` が `prev_visible` を上書きする**前**に読む（モジュール doc の ⑶）。
    let mut hidden = log_external_transitions(state, &observations);

    let decision = decide(state, &observations, now_talk_time, timeout_secs);
    let issued = issue_actions(presenter, world, state, &observations, &decision.actions);
    hidden.extend(issued.hidden.iter().copied());

    emit_visibility_logs(&decision.logs, &issued.not_shown);
    clear_hover_residency(world, &hidden);
}

// ---------------------------------------------------------------------------
// ⑴ 表示ライフサイクル信号の取り出し
// ---------------------------------------------------------------------------

/// 受信端に溜まった信号を**全件**取り出す（design「観測の収集（配線層）」の「信号」）。
///
/// 受け取った信号が計測へ及ぼす作用（会話開始での破棄・占有終端の更新）は判断中核が担うため、
/// ここは運ぶだけである。切断は復旧しない片道の状態なので誤りレベルで 1 回だけ記録する
/// （毎フレーム鳴らすと、切断後のログが誤りレベルの行で埋まる）。切断済みチャネルでも
/// 保留中の値は先に返るため、**取り残しは生じない**。
fn drain_lifecycle(
    rx: &Receiver<TalkLifecycleSignal>,
    state: &mut BalloonVisibilityState,
) -> Vec<TalkLifecycleSignal> {
    let mut signals = Vec::new();
    loop {
        match rx.try_recv() {
            Ok(signal) => signals.push(signal),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                if !state.lifecycle_disconnect_logged {
                    state.lifecycle_disconnect_logged = true;
                    error!(
                        "[balloon-visibility] 表示ライフサイクル信号の受信端が切断された（以後の会話の占有終端を観測できない＝タイムアウト計測は始まらず表示を保持する）"
                    );
                }
                break;
            }
        }
    }
    signals
}

// ---------------------------------------------------------------------------
// ⑵ 観測の収集
// ---------------------------------------------------------------------------

/// 本フレームの観測スナップショットを組む（design「観測の収集（配線層）」）。
///
/// 観測が取れない軸は `None` として運ぶ——判断中核がそれを「エッジなし」「抑止なし」へ写す
/// （Requirement 5.5）。ここで既定値へ潰すと、観測できないことが判断の根拠に化ける。
fn collect_observations(
    presenter: &EmoPresenter,
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    world: &mut World,
    scopes: &[u32],
    now_talk_time: Option<f64>,
    lifecycle: Vec<TalkLifecycleSignal>,
    state: &mut BalloonVisibilityState,
) -> VisibilityObservations {
    if scopes.is_empty() {
        // 装着済みバルーンが 1 つも無い（attach 未了・またはバルーンを持たない構成）。観測する
        // 相手が居ないので観測の失敗も起こり得ない——ここで表示層や world を覗くと「起動直後の
        // 全フレームに観測失敗が 1 件出る」という嘘の記録になる。信号だけは畳み込みへ渡す。
        return VisibilityObservations {
            lifecycle,
            ..VisibilityObservations::default()
        };
    }

    // ドラッグ中の観測（world の問い合わせ）は可変借用を要するため、以降の共有借用より先に済ませる。
    let dragging = observe_dragging(world);
    let runtime = borrow_runtime(runtime, state);
    let hover_wiring = observe_hover_wiring(world, state);

    let mut observed = BTreeMap::new();
    for &scope in scopes {
        let target = balloon_target(scope);
        let Some(visible) = presenter.target_visible(target) else {
            // 判断の真実源が引けない scope（表示層に target が無い）。第 2 の帳簿で代用せず、
            // 理由を記録して**当該 scope だけ**を見送る。同じ縮退が続く間の記録は 1 回。
            if state
                .observation_failure_logged
                .unattached_scopes
                .insert(scope)
            {
                error!(
                    scope,
                    ?target,
                    "[balloon-visibility] 表示層に当該 scope の target が無く可視状態を観測できない → 本 scope の判断を見送る"
                );
            }
            continue;
        };
        // 観測が戻ったら次の縮退で再び 1 回鳴らせるよう武装し直す。
        state
            .observation_failure_logged
            .unattached_scopes
            .remove(&scope);

        let actor = ActorKey::from(scope.to_string());
        observed.insert(
            scope,
            ScopeObservation {
                // グリフ数は注入時刻の関数である。時刻が定まらないフレーム（talk 起点未確立）は
                // 観測できない＝エッジなしとして扱う（`decide` のタイムアウト評価も同じ理由で休む）。
                //
                // この対応づけには副次の効果がある。判断中核は「表示終了信号が 1 件も無いのに
                // 可視コンテンツが現れた」ことの記録（Requirement 4.8）を、現在時刻が分かる
                // フレームでしか作らない。だがグリフ数と現在時刻がここで同時に立つ以上、
                // 現在時刻が無いフレームでは表示のエッジ自体が立たない——記録の取りこぼしは
                // 構造的に起こらない。
                visible_glyphs: runtime
                    .as_ref()
                    .zip(now_talk_time)
                    .map(|(rt, t)| rt.state().visible_glyphs(&actor, t)),
                visible,
                hover: hover_wiring.map(|w| w.is_balloon_hovered(scope as usize)),
                choice_active: runtime.as_ref().map(|rt| rt.choice_active(&actor)),
            },
        );
    }

    VisibilityObservations {
        scopes: observed,
        lifecycle,
        dragging,
    }
}

/// 文字層ランタイムを借りる（失敗は誤りレベルで 1 回・当該フレームは「グリフ観測なし・
/// 選択肢抑止なし」）。
fn borrow_runtime<'r>(
    runtime: &'r Rc<RefCell<TextLayerRuntime>>,
    state: &mut BalloonVisibilityState,
) -> Option<Ref<'r, TextLayerRuntime>> {
    match runtime.try_borrow() {
        Ok(borrowed) => {
            state.observation_failure_logged.runtime = false;
            Some(borrowed)
        }
        Err(_) => {
            if !state.observation_failure_logged.runtime {
                state.observation_failure_logged.runtime = true;
                error!(
                    "[balloon-visibility] 文字層ランタイムを借用できない → 本フレームは可視グリフ数と選択肢表示中を観測せず、表示状態を変えない"
                );
            }
            None
        }
    }
}

/// ポインタ滞在の照会先（`BalloonWiring`）を引く（不在は抑止なし・記録は 1 回）。
fn observe_hover_wiring<'w>(
    world: &'w World,
    state: &mut BalloonVisibilityState,
) -> Option<&'w BalloonWiring> {
    match world.get_non_send_resource::<BalloonWiring>() {
        Some(wiring) => {
            state.observation_failure_logged.hover_wiring = false;
            Some(wiring)
        }
        None => {
            if !state.observation_failure_logged.hover_wiring {
                state.observation_failure_logged.hover_wiring = true;
                error!(
                    "[balloon-visibility] ポインタ配線が world に無くバルーンへの滞在を観測できない → 抑止なしとして扱う"
                );
            }
            None
        }
    }
}

/// いずれかのバルーン窓がドラッグ中か（Requirement 5.1・design 決定 D8）。
///
/// `WindowDragging` はドラッグの全期間に載る窓 entity のマーカーで、バルーンではドラッグ対象が
/// 窓 entity そのものゆえ `BalloonWindowMarker` との連言で判定できる。借用や不在の資源を経由
/// しないため「観測できない」状態を持たない。
fn observe_dragging(world: &mut World) -> bool {
    world
        .query_filtered::<Entity, (With<BalloonWindowMarker>, With<WindowDragging>)>()
        .iter(world)
        .next()
        .is_some()
}

// ---------------------------------------------------------------------------
// ⑶ 自分が発行していない可視性の変化（Requirement 8.1 の「明示指令」）
// ---------------------------------------------------------------------------

/// 前フレームの記憶と本フレームの観測の差分を `trigger=explicit` として記録し、不可視へ落ちた
/// scope を返す（返り値はポインタ滞在の掃除対象）。
///
/// `\b[-1]` の明示的な非表示や、面切替が全透明へ退化して確立が取り消された場合がここに現れる。
/// 明示指令のログを本コントローラが一元に出すのは design の指定であり、表示層側のログ水準・
/// 書式には手を入れない。初見の scope は比較相手が無いため何も出さない（起動直後に全 scope 分の
/// 「変化した」が出るのを避ける）。
fn log_external_transitions(
    state: &BalloonVisibilityState,
    observations: &VisibilityObservations,
) -> Vec<u32> {
    let mut hidden = Vec::new();
    for (&scope, observed) in &observations.scopes {
        let Some(previous) = state.per_scope.get(&scope) else {
            continue;
        };
        if previous.prev_visible == observed.visible {
            continue;
        }
        info!(
            scope,
            trigger = VisibilityTrigger::Explicit.as_str(),
            visible = observed.visible,
            "[balloon-visibility] 本制御が発行していない可視状態の変化を検出"
        );
        if !observed.visible {
            hidden.push(scope);
        }
    }
    hidden
}

// ---------------------------------------------------------------------------
// ⑸ 行動の発行
// ---------------------------------------------------------------------------

/// 発行の結果（後段のログと掃除が要る分だけ）。
struct IssueOutcome {
    /// 実際に不可視化した scope（ポインタ滞在の掃除対象）。
    hidden: Vec<u32>,
    /// 表示を発行したのに可視にならなかった scope（遷移として名乗らない・巻き戻し済み）。
    not_shown: Vec<u32>,
}

/// 判断が返した行動を表示層へ発行する。
///
/// 表示は分離された可視化の専用入口 `show_target`、非表示は既存の漏斗
/// `apply(PresentCommand::Hide)` を通す（非表示側の新しい入口は作らない・design の指定）。
/// 非表示が触るのはバルーン窓の可視性と現サーフェスだけで、内容の合成キャッシュ・会話状態・
/// キャラクター窓には触れない（Requirements 4.4／6.7）。
fn issue_actions(
    presenter: &mut EmoPresenter,
    world: &mut World,
    state: &mut BalloonVisibilityState,
    observations: &VisibilityObservations,
    actions: &[VisibilityAction],
) -> IssueOutcome {
    let mut hidden = Vec::new();
    let mut not_shown = Vec::new();
    for action in actions {
        match action {
            VisibilityAction::Show { scope } => {
                if !issue_show(presenter, world, *scope) {
                    roll_back_show(state, observations, *scope);
                    not_shown.push(*scope);
                }
            }
            VisibilityAction::HideScopes { scopes, .. } => {
                for &scope in scopes {
                    presenter.apply(
                        world,
                        PresentCommand::Hide {
                            target: balloon_target(scope),
                            reply: None,
                        },
                    );
                    hidden.push(scope);
                }
            }
        }
    }
    IssueOutcome { hidden, not_shown }
}

/// 1 scope の表示を発行し、**実際に可視になったか**を返す。
///
/// 成否を発行の戻り値だけで決めないのは、`show_target` が全透明への退化を正常な縮退として
/// `Ok` で返すためである（出す内容が無いので可視性を付与しない）。可視かどうかの真実源は
/// あくまで表示層の照会であり、ここでも第 2 の帳簿を作らない。
fn issue_show(presenter: &mut EmoPresenter, world: &mut World, scope: u32) -> bool {
    let target = balloon_target(scope);
    let issued = presenter.show_target(world, target);
    let visible = presenter.target_visible(target) == Some(true);
    match issued {
        Ok(()) if visible => true,
        Ok(()) => {
            // 発行そのものは通ったが可視になっていない（出す内容が無い＝全透明への退化）。
            // 失敗ではないので誤りレベルには上げず、遷移として名乗ることもしない。
            warn!(
                scope,
                ?target,
                trigger = VisibilityTrigger::Content.as_str(),
                "[balloon-visibility] 表示を発行したが可視にならなかった（出す内容が無い）→ 不可視のまま次のエッジを待つ"
            );
            false
        }
        Err(error) => {
            error!(
                scope,
                ?target,
                error = %error,
                trigger = VisibilityTrigger::Content.as_str(),
                "[balloon-visibility] 表示の発行に失敗 → 本 scope のみ見送る（他 scope と会話は巻き込まない）"
            );
            false
        }
    }
}

/// 実らなかった表示の `prev_visible` を発行前の値へ戻す（tasks.md「タスク 4.4 の義務」）。
///
/// 判断中核は `Show` を積む時点で `prev_visible` を真にする（相順では観測が発行より前に採られる
/// ため、観測値をそのまま覚えると自分が出した表示が毎フレーム外因と誤検出される）。表示が実ら
/// なかった scope をそのままにすると、今度は逆に次フレームで「外から消された」という偽の
/// `trigger=explicit` と、不要なポインタ滞在の掃除が走る。戻す先は本フレームの観測値＝
/// 「発行しなかった場合に判断中核が書いたはずの値」である。
fn roll_back_show(
    state: &mut BalloonVisibilityState,
    observations: &VisibilityObservations,
    scope: u32,
) {
    let observed_visible = observations
        .scopes
        .get(&scope)
        .is_some_and(|observed| observed.visible);
    if let Some(previous) = state.per_scope.get_mut(&scope) {
        previous.prev_visible = observed_visible;
    }
}

// ---------------------------------------------------------------------------
// ⑹ ログ（design 決定 D10）
// ---------------------------------------------------------------------------

/// 判断が返した事象をログへ写す（遷移・計測は情報レベル・信号の欠落は警告）。
///
/// 判断中核は**状態が動いたフレームでしか**事象を作らないため、ここが毎フレーム鳴ることはない
/// （Requirement 8.6）。1 行から scope と契機が確定できるよう、構造化フィールドは D10 の語彙
/// （`scope`／`trigger`／`visible`／`deadline`／`talk_time`）で揃える（Requirement 8.5）。
///
/// `not_shown` の scope については表示への遷移を出さない——発行が実らなかった以上、遷移は
/// 起きていない（起きていない遷移を記録すると、実機サインオフの合否判定が嘘をつく）。
fn emit_visibility_logs(logs: &[VisibilityLogEvent], not_shown: &[u32]) {
    for event in logs {
        match *event {
            VisibilityLogEvent::Transition {
                scope,
                trigger,
                visible,
            } => {
                if visible && not_shown.contains(&scope) {
                    continue;
                }
                info!(
                    scope,
                    trigger = trigger.as_str(),
                    visible,
                    "[balloon-visibility] バルーンの可視状態が遷移した"
                );
            }
            VisibilityLogEvent::MeasurementStarted {
                display_end,
                deadline,
            } => info!(
                display_end,
                deadline, "[balloon-visibility] タイムアウト計測を開始（起点＝会話の占有終端）"
            ),
            VisibilityLogEvent::MeasurementDiscarded { reason, deadline } => info!(
                reason = reason.as_str(),
                deadline, "[balloon-visibility] タイムアウト計測を破棄"
            ),
            VisibilityLogEvent::MeasurementRestarted { now, deadline } => info!(
                talk_time = now,
                deadline, "[balloon-visibility] 抑止が解けたのでタイムアウト計測をやり直した"
            ),
            VisibilityLogEvent::TimeoutSuppressed { kinds, deadline } => info!(
                dragging = kinds.dragging,
                hover = kinds.hover,
                choice = kinds.choice,
                deadline,
                "[balloon-visibility] 抑止が成立しているためタイムアウトによる非表示を見送った"
            ),
            VisibilityLogEvent::DisplayEndSignalMissing => warn!(
                "[balloon-visibility] 可視コンテンツが現れたのに会話の表示終了信号が 1 件も届いていない → 計測を始めず表示を保持する"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// ⑺ ポインタ滞在記録の掃除
// ---------------------------------------------------------------------------

/// 不可視へ落ちた scope のポインタ滞在の記録を落とす（Requirement 5.2／5.5）。
///
/// 不可視の間はポインタの離脱通知が届かないため、滞在の記録を残したままにすると抑止が真のまま
/// 固着して二度と消えないバルーンが生まれる。`BalloonWiring` の不在は掃除する相手が無いだけで
/// あり、観測できなかったことは既に収集時に記録済みなのでここでは黙って戻る。
fn clear_hover_residency(world: &mut World, scopes: &[u32]) {
    if scopes.is_empty() {
        return;
    }
    let Some(mut wiring) = world.get_non_send_resource_mut::<BalloonWiring>() else {
        return;
    };
    for &scope in scopes {
        wiring.clear_balloon_hover(scope as usize);
    }
}

#[cfg(test)]
#[path = "balloon_visibility_phase_tests.rs"]
mod tests;
