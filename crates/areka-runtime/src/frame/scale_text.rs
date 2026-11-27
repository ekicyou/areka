//! 文字の層を進める相・文字層 k 追従フェーズ・text フェーズ（[`run_text_advance_phase`]・[`run_text_scale_phase`]・[`run_text_phase`] ほか）。

use std::cell::RefCell;
use std::time::{Duration, Instant};

use bevy_ecs::world::World;
use tracing::{debug, error, info, warn};

use areka_emo_text::actor::{TextLayerRuntime, present_frame};
use areka_sakura::ActorKey;
use dola::{TimePoint, TimeSpan};
use wintf::ecs::FrameTime;
use wintf::ecs::world::tick_wake;

use areka_placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
use areka_placement::dpi_sync::{self, HoldSite};
use areka_placement::spawn::GhostWindows;

use crate::target_map::window_scope;

use super::{
    GhostWindowKind, RuntimeWiring, ScaleReportSource, balloon_target, reconcile_window_size,
    shell_target,
};

/// 文字の層を借りられなかった事を、相ごとに、縮みが続く間は 1 回だけ記録するための覚え。
#[derive(Debug, Default)]
pub(crate) struct TextLayerBusyLogged {
    advance: bool,
    scale: bool,
    text: bool,
}

/// 文字の層を借りられるか。借りられなければ（ほかの処理が持っている）、縮みの始まりだけ `error!` を
/// 出し、次の画面の更新を予約して `false` を返す。借りられれば覚えを倒して `true`。
///
/// 相の中で借りるのは自分だけなので、頭で 1 度確かめれば相の終わりまで借りられる。
fn text_layer_free(runtime: &RefCell<TextLayerRuntime>, logged: &mut bool, phase: &str) -> bool {
    if runtime.try_borrow_mut().is_ok() {
        *logged = false;
        return true;
    }
    if !std::mem::replace(logged, true) {
        error!(
            phase,
            "[text-layer] 文字の層がほかの処理に借りられていて、このフレームの仕事を見送る（次の画面の更新で再試行・同じ縮みの間は記録 1 回）"
        );
    }
    tick_wake::mark(tick_wake::REARM);
    false
}

/// 桁外れに遠い起こしの頭打ち（1 時間・可視性の相と同じ）。起きた相が預け直すので、見える物は変わらない。
const MAX_WAKE_WAIT: Duration = Duration::from_secs(3600);

/// `at` に次のフレームを起こすまでの待ち（純判断）。`at` が `frame_now` 以前なら `None`（次の画面の更新で足りる）。
pub(super) fn wake_after(at: TimePoint, frame_now: TimePoint) -> Option<Duration> {
    at.span_since(frame_now)
        .filter(|remaining| *remaining > TimeSpan::ZERO)
        .map(|remaining| remaining.to_std().min(MAX_WAKE_WAIT))
}

/// `at` に次のフレームを起こす予約を置く（預けられる期限は最も早い 1 つだけなので、相が走るたびに預け直す）。
fn arm_wake_at(at: TimePoint, frame_now: TimePoint) {
    match wake_after(at, frame_now) {
        Some(wait) => tick_wake::arm_deadline(Instant::now() + wait),
        None => tick_wake::mark(tick_wake::REARM),
    }
}

/// フレームの時刻（テストの値が在ればそれ、無ければ `FrameTime`、どちらも無ければ `None`）。
///
/// 進める相・可視性の相・文字の相がこの 1 本を呼ぶので、3 つの相は同じフレームの時刻で文字の層を読む。
pub(crate) fn resolve_frame_now(
    frame_now_override: Option<TimePoint>,
    world: &World,
) -> Option<TimePoint> {
    frame_now_override.or_else(|| world.get_resource::<FrameTime>().map(|ft| ft.0))
}

/// 文字の層を進める相（フレームの頭で 1 回）: フレームの時刻までに時刻の来た物を、届いた順に当てる。
///
/// 列の頭の物が待っている時刻が在れば、その時刻に次のフレームを起こす予約を置く。フレームの時刻が無い
/// 世界と、文字の層を借りられないフレームでは進めず、次の画面の更新を予約する（列の物は残り、次の
/// フレームで当たる）。
pub fn run_text_advance_phase(
    wiring: &mut RuntimeWiring,
    world: &mut World,
    frame_now_override: Option<TimePoint>,
) {
    let Some(frame_now) = resolve_frame_now(frame_now_override, world) else {
        tick_wake::mark(tick_wake::REARM);
        return;
    };
    if !text_layer_free(
        &wiring.runtime,
        &mut wiring.text_busy_logged.advance,
        "advance",
    ) {
        return;
    }
    if let Some(waiting) = wiring.runtime.borrow_mut().advance(frame_now) {
        arm_wake_at(waiting, frame_now);
    }
}

/// 文字層 k 追従フェーズ（毎フレーム）: 装着済み balloon scope の文字層 binding を presenter の現適用 k へ合わせ直す。戻り値は実際に binding を再構築した scope（昇順・観測用。本番の [`frame_system`] は捨てる）。
///
/// バルーンの文字は emo-text の binding（装着時の k を焼き付ける）に載るため、窓とバルーン画像だけを再スケールすると文字だけが旧 k の寸法に取り残される。
///
/// # なぜイベント駆動でなく毎フレーム走査か
///
/// [`run_dpi_phase`] の `refresh_scale` が `Some` を返すことと「適用 k が変わった」ことは同値でない。
///
/// - 不可視のとき: `refresh_scale` は再表示も `applied` 更新もせず `None` を返す。適用 k は後の `Show`（`apply_show`＝drain 相）で跳び、エッジは消費済みで二度と来ない（バルーンは大半の時間が不可視）。
/// - k は変わったが丸め後の物理寸が同じとき: `take_pending_resize` が `None` ゆえ `None` を返す。文字層の供給面は `ceil(validrect 寸 × k)` と別の丸めで決まり、寸は変わり得る。
///
/// ゆえに検出点は「presenter の現状態から組み直した文字層 binding が当該 actor の現 binding と食い違うか」で、判定の権威は [`TextLayerRuntime::refresh_actor_binding`] だけである。判定を複製せず（第 2 のガードは本家と乖離し得る）、毎フレーム [`TextLayerRuntime::refresh_actor_scale`] へ委ねる。
///
/// 判定キーは binding 全体（k・物理寸・image 原寸・slot・window）と、その image 原寸でモデルから解き直した `ResolvedBalloonText` の連言で、すべて同値なら再構築せず `false`（churn ガード）。未登録 actor も `false`（装着は `register_actor_view` の領分）。k が同値でも面実寸や当該 scope の `validrect` が違えば再構築する。
///
/// # 呼ぶ位置（[`frame_system`] 内）
///
/// 適用 k の更新点は [`run_dpi_phase`] の `refresh_scale` と [`run_drain_phase`] の `apply_show` の 2 つ。本フェーズは両者の下流かつ [`run_text_phase`]（`present_frame`）の上流に置き、どちらの経路で k が跳ねても同一フレーム内で binding を組み直して、直後の描画を新 k の物理寸で走らせる。
///
/// # 縮退
///
/// 文字の層を借りられなければ（ほかの処理が持っている）、このフレームは何もせず、`error!`（同じ縮みの間は 1 回）を出して次の画面の更新を予約する。
///
/// `text_slot_view` が `None`（初回 `ShowSurface` が未成立）なら再追従できず skip する。毎フレーム走査ゆえ `warn!` は scope ごとに一度だけ（`text_scale_warned`・emo-text の `unresolved_warned` と同型のエッジガード）で、view が取れた時点で再武装する。`Hide` は `text_slot_view` を `None` にしない（`apply_hide` は mount／chain／`applied`／`native_size` を保持する）ので、不可視はこの経路に落ちない。
///
/// [`BalloonModel`] は attach 時に記憶した per-scope の同一モデルを再利用する（再パースしない）。actor は attach と同一写像 `ActorKey::from(scope.to_string())`。走査するのは `balloon_models` の持つ装着済みの scope。panic しない。
///
/// # 箱の同期
///
/// 同じ走査で各 scope のシェルの窓の `text_slot_view(shell_target(scope))` とその窓が表示している絵の番号（`current_surface_id`）を組にして集め、最後に [`TextLayerRuntime::sync_boxes`] を 1 回呼ぶ（拡大率・絵・シェルの切替に箱の面を合わせる）。シェルの窓が未確立の scope と、絵が非表示（`Hide`・全透明退化）で番号の無い scope は `None` を渡す（箱の登録を外す）。未確立は起動直後の正常な状態なので記録しない。`present_frame` の上流なので、合わせた登録は同じフレームの描画に効く。
pub fn run_text_scale_phase(wiring: &mut RuntimeWiring, world: &mut World) -> Vec<u32> {
    let RuntimeWiring {
        presenter,
        runtime,
        balloon_models,
        text_scale_warned,
        unconnected_text,
        text_busy_logged,
        ..
    } = wiring;

    let mut refreshed = Vec::new();
    if !text_layer_free(runtime, &mut text_busy_logged.scale, "text-scale") {
        return refreshed;
    }
    // BTreeMap ではなく HashMap ゆえ列挙順は不定。観測（戻り値）と warn 順を決定論にするため昇順化する。
    let mut scopes: Vec<u32> = balloon_models.keys().copied().collect();
    scopes.sort_unstable();

    let mut shell_views = Vec::with_capacity(scopes.len());
    for scope in scopes {
        let target = balloon_target(scope);
        // attach と同一の写像で組む。別式だと存在しない actor を指し、未登録 skip で静かに何も起きなくなる。
        let actor = ActorKey::from(scope.to_string());
        // 箱の同期の材料。バルーンの縮退 skip より前に集める。
        let shell = shell_target(scope);
        let shown = presenter
            .text_slot_view(shell)
            .zip(presenter.current_surface_id(shell));
        shell_views.push((actor.clone(), shown));
        let Some(view) = presenter.text_slot_view(target) else {
            // 表示未確立（初回 ShowSurface が成立していない）。毎フレーム走査ゆえ scope ごとに 1 回だけ鳴らす。
            if text_scale_warned.insert(scope) {
                warn!(
                    scope,
                    ?target,
                    actor = %actor.as_str(),
                    "text-scale: text_slot_view が None（表示未確立）→ 文字層 k 追従を skip し次機会へ委ねる（本 scope の警告は復帰まで抑止・R8.6）"
                );
            }
            continue;
        };
        // 復帰＝次に落ちたときは再び 1 回鳴らす（エッジの再武装）。
        text_scale_warned.remove(&scope);
        let model = &balloon_models[&scope];
        if let Some(background) = unconnected_text.remove(&scope) {
            super::attach::connect_balloon_text(runtime, Some(view), actor, model, background);
            info!(
                scope,
                "text-scale: 装着で繋げなかった文字層を、配置先ができたので繋ぐ"
            );
            refreshed.push(scope);
            continue;
        }
        if runtime
            .borrow_mut()
            .refresh_actor_scale(&actor, &view, model)
        {
            refreshed.push(scope);
        }
    }
    runtime.borrow_mut().sync_boxes(world, &shell_views);
    refreshed
}

/// 窓寸 reconcile の第 2 経路（状態照合）。
///
/// [`GhostWindows`] の各 scope の shell／balloon 両 target について [`ScaleReportSource::take_scale_report`] を引き、`Some(新物理寸)` を [`reconcile_window_size`] で窓 client へ反映する（char＝アンカー保存／balloon＝位置維持）。
///
/// 報告は「表示が成立して物理寸が前回適用寸から変わった（初回表示を含む）」状態で、`Changed<DPI>` エッジの消費順序に依存しない。エッジが初回表示より前に消費されても k₀ と実窓 DPI の差分は残らず、[`run_dpi_phase`] が既に報告を消費していれば `None` で二重に書かない。
///
/// [`GhostWindows`] 未挿入（窓生成前）は no-op。窓 entity が引けない scope は `warn!`＋skip（報告は取り出し済みで持ち越さない）。panic しない。
///
/// # 破棄済み窓の打ち切り
///
/// 登録は在るが指す entity が despawn 済みは終了処理の正常系なので、`debug!`（[`DESPAWNED_SKIP_TAG`]）でその target を打ち切り、他 scope は継続する。「登録が無い」`warn!` とは別事象。
pub(super) fn reconcile_reported_sizes<S: ScaleReportSource>(source: &mut S, world: &mut World) {
    // GhostWindows は小さな Entity 写像（Clone）。target/窓の解決へ world の不変借用を跨がせない。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>().cloned() else {
        return;
    };
    for scope in ghost_windows.scopes() {
        let Some(scope32) = window_scope(scope) else {
            continue;
        };
        for (target, window, kind) in [
            (
                shell_target(scope32),
                ghost_windows.char_window(scope),
                GhostWindowKind::Char,
            ),
            (
                balloon_target(scope32),
                ghost_windows.balloon_window(scope),
                GhostWindowKind::Balloon,
            ),
        ] {
            // 整合ゲート: 待ち札のある窓へはこの経路からも書かない。報告を取り出す前に見送る（取り出すと消え、待ちが解けた後に反映する材料が失われる）。
            if let Some(window) = window
                && dpi_sync::defers_window_write(world, window, HoldSite::Reconcile)
            {
                continue;
            }
            let Some(new_size) = source.take_scale_report(target) else {
                continue;
            };
            let Some(window) = window else {
                warn!(
                    scope,
                    ?target,
                    ?new_size,
                    "dpi reconcile: 窓 entity が無い（GhostWindows 不整合）→ 反映先が無く skip"
                );
                continue;
            };
            // 登録は在るが窓が despawn 済み（終了処理後のフレーム）は正常系。debug で打ち切り他の target は続ける。上の `None` 腕（登録が無い＝warn）とは別事象。
            if world.get_entity(window).is_err() {
                debug!(
                    scope,
                    ?target,
                    entity = ?window,
                    "{DESPAWNED_SKIP_TAG} dpi reconcile: 窓 entity が破棄済み（despawn）→ 本 target を正常系として打ち切り（他 scope は継続）"
                );
                continue;
            }
            // DPI 由来と名乗らせないため DpiReproject とは別語を貼る（初回表示の k₀ 補正もこの経路で landing する）。
            reconcile_window_size(
                world,
                window,
                kind,
                new_size,
                PlacementRoute::ReportedSizeReconcile,
            );
        }
    }
}

/// 次に現れる字の時刻（全部の場所の、`frame_now` より後の現れる時刻の最も早い物・純判断）。
///
/// 引数は場所ごとの字の現れる時刻の列（`RevealSchedule::times()`・減らない並び）。字が現れ切っていれば `None`。
pub(super) fn next_reveal<'a>(
    reveals: impl Iterator<Item = &'a [TimePoint]>,
    frame_now: TimePoint,
) -> Option<TimePoint> {
    reveals
        .filter_map(|times| times.get(times.partition_point(|&t| t <= frame_now)))
        .copied()
        .min()
}

/// 文字の相: フレームの時刻で `present_frame` を駆動する（`Err` は `error!`＋継続＝次フレーム再試行）。
///
/// フレームの時刻が無ければ描かない。文字の層を借りられなければ、そのフレームは描かず、次の画面の更新を
/// 予約する。まだ現れていない字が在れば、次に現れる字の時刻に次のフレームを起こす予約を置く。
pub fn run_text_phase(
    wiring: &mut RuntimeWiring,
    world: &mut World,
    frame_now_override: Option<TimePoint>,
) {
    let Some(frame_now) = resolve_frame_now(frame_now_override, world) else {
        return;
    };
    if !text_layer_free(&wiring.runtime, &mut wiring.text_busy_logged.text, "text") {
        return;
    }
    let mut runtime = wiring.runtime.borrow_mut();
    if let Err(e) = present_frame(&mut runtime, world, frame_now) {
        error!(
            error = %e,
            frame_now = %frame_now,
            "runtime text: present_frame が失敗（他 actor 非破壊・次フレーム再試行・R2.3）"
        );
    }
    if let Some(next) = next_reveal(
        runtime
            .state()
            .places()
            .map(|(_, state)| state.reveal().times()),
        frame_now,
    ) {
        arm_wake_at(next, frame_now);
    }
    // 実機サインオフ用の hover 注入。present_frame の後に駆動し、choice_active／choice_hit_rows が当該フレームの提示を反映した状態で巡回注入する。env（`AREKA_CHOICE_HOVER_INJECT`）未設定なら no-op。
    super::hover_inject::drive(&mut runtime, frame_now);
}
