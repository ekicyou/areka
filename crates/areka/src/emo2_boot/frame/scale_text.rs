//! 文字層 k 追従フェーズ＋text フェーズ（[`run_text_scale_phase`]・[`run_text_phase`] ほか）。

use bevy_ecs::world::World;
use tracing::{debug, error, warn};

use areka_emo_text::actor::present_frame;
use areka_sakura::ActorKey;
use wintf::ecs::FrameTime;
use wintf::ecs::world::tick_wake;

use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
use crate::placement::dpi_sync::{self, HoldSite};
use crate::placement::spawn::GhostWindows;

use super::{
    Emo2Wiring, GhostWindowKind, ScaleReportSource, TalkClock, balloon_target,
    reconcile_window_size, shell_target,
};

// ---------------------------------------------------------------------------
// 文字層 k 追従フェーズ（areka-P0-emo-dpi-scaling task 7.2・design D11-3/D11-4・Req8.1/8.5/8.6）
// ---------------------------------------------------------------------------

/// 文字層 k 追従フェーズ（毎フレーム・R8.1/8.5/8.6・design D11-4）: 装着済み balloon scope の
/// 文字層 binding を presenter の**現適用 k** へ合わせ直す。戻り値は実際に binding を再構築した
/// scope（昇順・観測用。本番＝[`emo2_frame_system`] は捨てる）。
///
/// バルーンの**文字**は emo-text の binding（装着時の k を焼き付ける）に載るため、窓とバルーン画像
/// だけを再スケールすると文字だけが旧 k の寸法に取り残される（6.5 一次実走で実測した欠陥）。本
/// フェーズはその取り残しを構造的に消す。
///
/// # なぜ「イベント駆動」ではなく毎フレーム走査なのか（D11-4 の意図＝binding 変化の検出）
///
/// 素朴には「[`run_dpi_phase`] の `refresh_scale` が `Some` を返した balloon 窓へ伝搬する」と書け
/// るが、**`Some` と「適用 k が変わった」は同値ではない**——`refresh_scale` の doc が明記するとおり
/// 次の 2 つで乖離する:
///
/// - **不可視のとき**: `refresh_scale` は再表示せず `applied` も更新せずに `None` を返す。適用 k は
///   その後の `Show`（`apply_show`＝drain 相）で新 k へ跳ぶ——エッジは既に消費済みで二度と来ない
///   （`\b[-1]`→`\b[0]` は本番の通常列であり、バルーンは大半の時間が不可視である）。
/// - **k は変わったが丸め後の物理寸が同じとき**: `refresh_scale` は再表示に成功しても
///   `take_pending_resize` が `None` ゆえ `None` を返す。文字層の供給面は
///   `ceil(validrect 寸 × k)`（AC 8.2）と別の丸めで決まるため、こちらは寸が変わり得る。
///
/// ゆえに検出点は「**presenter の現状態から組み直した文字層 binding が、当該 actor の現 binding と
/// 食い違っているか**」であり、それを判定できる唯一の権威は
/// [`TextLayerRuntime::refresh_actor_binding`] である。本フェーズは判定を自前で複製せず（第 2 の
/// ガードは本家と乖離し得る）、毎フレーム [`TextLayerRuntime::refresh_actor_scale`] へ委ねる。
///
/// あちらの判定キーは **binding 全体（k・物理寸・image 原寸・slot・window）と、その image 原寸で
/// モデルから解き直した `ResolvedBalloonText`（文字描画領域を含む）の連言**であり、すべて同値なら
/// **再構築せず `false`** を返す（churn ガード・R4.5/R8.5）。未登録 actor も `false`（装着は
/// `register_actor_view` の領分）。**k が同値でも面実寸や当該 scope の `validrect` が違えば再構築
/// する**——k の同値のみを根拠に省略しない（R4.4。scope 別バルーン定義が当事者であり、旧契約
/// 「同値 k なら再構築しない」では相方側の領域変化を取りこぼす）。費用は balloon 1 枚あたり
/// `ResolvedBalloonText` の再解決 1 回と、binding／解決済み領域の 2 構造体比較。
///
/// # 呼ぶ位置（[`emo2_frame_system`] 内）
///
/// 適用 k の更新点は 1 フレームに 2 つ——[`run_dpi_phase`] の `refresh_scale` と
/// [`run_drain_phase`] の `apply_show`。本フェーズは**両者の下流**かつ [`run_text_phase`]
/// （`present_frame`）の**上流**に置く。こうすると、どちらの経路で k が跳ねても同一フレーム内で
/// binding が組み直され、その直後の描画が新 k の物理寸で走る（1 フレームの旧寸残りが生じない）。
///
/// # 縮退（R8.6・log-first だが log spam にしない）
///
/// `text_slot_view` が `None`（初回 `ShowSurface` が成立していない＝表示未確立）なら再追従できず
/// skip する。毎フレーム走査ゆえ素朴に `warn!` すると毎フレーム鳴るため、**scope ごとに一度だけ**
/// 警告し（`text_scale_warned`・emo-text の `unresolved_warned` と同型のエッジガード）、view が
/// 取れるようになった時点で再武装する（再度落ちれば再び 1 回鳴る）。なお `Hide` は
/// `text_slot_view` を `None` にしない（`apply_hide` は mount／chain／`applied`／`native_size` を
/// 保持する）ため、**不可視は本縮退経路に落ちない**——不可視の間は判定キーが同値のまま no-op が
/// 続き、`Show` で `applied` が跳ねた次の走査が再追従する。
///
/// [`BalloonModel`] は attach 時に記憶した per-scope の同一モデルを再利用する（再パースしない・
/// D11-3）。actor は attach と同一写像 `ActorKey::from(scope.to_string())`。shell target は emo2 で
/// 文字スロットを持たないため走査対象に入らない（`balloon_models` が balloon 装着 scope のみを持つ）。
/// panic しない。
pub fn run_text_scale_phase(wiring: &mut Emo2Wiring) -> Vec<u32> {
    // presenter（view 供給）／runtime（適用先）／balloon_models（再利用モデル）／warn ガードは
    // 互いに素なフィールドゆえ同時に借りられる。
    let Emo2Wiring {
        presenter,
        runtime,
        balloon_models,
        text_scale_warned,
        ..
    } = wiring;

    let mut refreshed = Vec::new();
    // BTreeMap ではなく HashMap ゆえ列挙順は不定。観測（戻り値）と warn 順を決定論にするため昇順化する。
    let mut scopes: Vec<u32> = balloon_models.keys().copied().collect();
    scopes.sort_unstable();

    for scope in scopes {
        let target = balloon_target(scope);
        // actor 引き当ては attach（`run_attach_phase` の `connect_balloon_text` 呼び）と**同一の写像**。
        // 別式で組むと存在しない actor を指し、7.1 の未登録 skip で静かに何も起きなくなる。
        let actor = ActorKey::from(scope.to_string());
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
        // 判定（k 変化・未登録）は 7.1 の権威へ委ねる（本フェーズは第 2 のガードを持たない・R8.5）。
        let model = &balloon_models[&scope];
        if runtime
            .borrow_mut()
            .refresh_actor_scale(&actor, &view, model)
        {
            refreshed.push(scope);
        }
    }
    refreshed
}

/// 窓寸 reconcile の第 2 経路（状態照合・design Flow 2 キー決定 (d)／Flow 3 手順 5）。
///
/// [`GhostWindows`] の各 scope について shell／balloon 両 target の
/// [`ScaleReportSource::take_scale_report`] を引き、`Some(新物理寸)` を
/// [`reconcile_window_size`] で窓 client へ反映する（char＝アンカー保存／balloon＝位置維持）。
///
/// 報告は「表示が成立して物理寸が前回適用寸から変わった（**初回表示を含む**）」ことを表す状態で
/// あり、`Changed<DPI>` エッジの消費順序に依存しない。ゆえに (a) エッジが初回表示より前に消費
/// されても k₀ と実窓 DPI の差分は残置されず、(b) 既に [`run_dpi_phase`] が再表示して報告を消費
/// 済みなら取り出しは `None` となり二重に窓を書かない。
///
/// [`GhostWindows`] 未挿入（窓生成前）は no-op。窓 entity が引けない scope は `warn!`＋skip
/// （報告は既に取り出し済み＝次フレームへ持ち越さない——窓が無い以上反映先が無い）。panic しない。
///
/// # 破棄済み窓の打ち切り（要件 6.2/6.3・design D8 消費側）
///
/// 「登録は在るが**指す先の entity が既に despawn 済み**」は終了処理の**正常系**であり、
/// `debug!`（[`DESPAWNED_SKIP_TAG`]）で当該 target を打ち切って**他 scope の処理を継続**する
/// （警告以上を出さない＝要件 6.2）。上段の「登録が無い」`warn!` とは別事象である。
pub(super) fn reconcile_reported_sizes<S: ScaleReportSource>(source: &mut S, world: &mut World) {
    // GhostWindows は小さな Entity 写像（Clone）。target/窓の解決へ world の不変借用を跨がせない。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>().cloned() else {
        return;
    };
    for scope in ghost_windows.scopes() {
        let Ok(scope32) = u32::try_from(scope) else {
            error!(
                scope,
                "dpi reconcile: scope が u32 に収まらず target を採番できない → skip"
            );
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
            // 整合ゲート（設計 C5・要件 5.8）: 待ち札のある窓へはこの経路からも書かない。
            // **報告を取り出す前**に見送る——取り出すと消えるので、待ちが解けた後に反映する
            // 材料が失われる（次フレームへ持ち越すには presenter に残しておくほかない）。
            if let Some(window) = window
                && dpi_sync::defers_window_write(world, window, HoldSite::Reconcile)
            {
                continue;
            }
            // 報告が無い（＝物理寸が変わっていない／未表示／既に消費済み）なら何もしない。
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
            // 存在確認（要件 6.2/6.3・design D8 消費側）: レジストリが指す窓が既に
            // despawn 済み（終了処理でゴースト窓が破棄された後のフレーム）なら、**正常終了系**
            // として debug で打ち切り、**他の scope／target の処理は続ける**。報告は上で
            // 取り出し済みのまま持ち越さない（窓が無い以上、次フレームでも反映先は無い）。
            // 上の `None` 腕（レジストリ不整合＝warn）とは別物である——あちらは「登録が無い」、
            // こちらは「登録はあるが指す先が消えた」で、後者だけが終了処理の正常系。
            if world.get_entity(window).is_err() {
                debug!(
                    scope,
                    ?target,
                    entity = ?window,
                    "{DESPAWNED_SKIP_TAG} dpi reconcile: 窓 entity が破棄済み（despawn）→ 本 target を正常系として打ち切り（他 scope は継続）"
                );
                continue;
            }
            // 経路タグ: 本経路は「表示が成立して物理寸が変わった」状態に紐づき `Changed<DPI>`
            // に**依存しない**（初回表示の k₀ 補正もここで landing する）。DPI 由来と名乗らせ
            // ないため DpiReproject とは別語を貼る（Req 1.2・D13）。
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

/// `talk_time` 解決の純判断（override 優先→`clock.talk_time(frame_now)`→`None`）。
///
/// [`run_text_phase`] の分岐条件を GPU/時刻 I/O 抜きの決定論檻へ切り出した純関数:
/// - `override_` が `Some(t)`（テスト注入経路）→ `Some(t)`（`frame_now`／`clock` は無視・最優先）。
/// - `override_` が `None`（本番経路）→ `frame_now` が `Some(now)` なら `clock.talk_time(now)`、
///   `frame_now` が `None`（`FrameTime` 資源不在＝headless）なら `None`。
/// - いずれも `clock` の epoch 未確立（talk 未到達）なら `talk_time` が `None` を返すため `None`。
///
/// 戻り値 `None` は「今フレームは描くものがない（`present_frame` を呼ばない）」を意味する。
///
/// 可視性の相（`emo2_boot::balloon_visibility`）も同じ関数で `now_talk_time` を解決する
/// （`frame.rs` の再輸出経由）。ゆえに可視性が観測するグリフ数と text 相が描く文字は必ず同一の
/// 注入時刻に立つ。
pub(in crate::emo2_boot) fn resolve_talk_time(
    override_: Option<f64>,
    frame_now: Option<f64>,
    clock: &TalkClock,
) -> Option<f64> {
    match override_ {
        // テスト注入経路: override が最優先（frame_now／clock は無視）。
        Some(t) => Some(t),
        // 本番経路: FrameTime（frame_now）→ TalkClock。frame_now 不在／epoch 未確立は None。
        None => frame_now.and_then(|now| clock.talk_time(now)),
    }
}

/// 発話の文字がまだ現れ切っていないか（純判断・時計にも GPU にも触れない）。
///
/// 引数は actor ごとのリビール時刻列（`RevealSchedule::times()`）で、単調非減少ゆえ末尾が
/// 「その actor の最後の文字が現れる時刻」である。1 人でもそれが現在時刻より後なら、この発話は
/// **まだ進行中**——次の画面更新でも文字が増える。
///
/// 「時刻が引ける＝進行中」ではないことに注意する。`TalkClock` の起点は一度立つとプロセスの
/// 終わりまで残るため、`talk_time` が `Some` であることは発話が続いている証拠にならない。
/// 進行中かどうかを決めるのは**まだ現れていない文字が在るか**だけである。
pub(in crate::emo2_boot) fn reveal_pending<'a>(
    reveals: impl Iterator<Item = &'a [f64]>,
    talk_time: f64,
) -> bool {
    reveals
        .filter_map(|times| times.last())
        .any(|&last| last > talk_time)
}

/// フェーズ③（text・design「フェーズ③（text）」・R2.3）: `talk_time` が定まるフレームでのみ
/// `present_frame` を駆動する（`Err` は `error!`＋継続＝次フレーム再試行）。
///
/// `talk_time` の解決は [`resolve_talk_time`] に委ねる（`talk_time_override` が `Some` ならそれを、
/// なければ `FrameTime` 資源（`wintf::ecs::FrameTime`・`.0: f64`）を読んで
/// [`TalkClock::talk_time`]）。解決が `Some(t)` のときのみ
/// `present_frame(&mut runtime.borrow_mut(), world, t)` を呼ぶ。`Err(e)` は `error!`（`present_frame`
/// 側で失敗源を log 済み・first error 返却）＋継続で、他 actor を巻き込まず次フレーム再試行へ委ねる
/// （R2.3・emo-text 既存契約）。解決が `None`（epoch 未確立／`FrameTime` 不在かつ override なし）なら
/// `present_frame` を呼ばず skip する（描くものがない・hang しない）。
pub fn run_text_phase(wiring: &mut Emo2Wiring, world: &mut World, talk_time_override: Option<f64>) {
    // 本番の frame 時刻源（headless では不在）。override が Some なら resolve_talk_time が優先採用する。
    let frame_now = world.get_resource::<FrameTime>().map(|ft| ft.0);
    let Some(talk_time) = resolve_talk_time(talk_time_override, frame_now, &wiring.clock) else {
        // epoch 未確立（talk 未到達）または FrameTime 不在かつ override なし → 描くものがない・skip。
        return;
    };
    // present_frame は失敗源で log 済み（first error 返却）。frame は error!＋継続で、他 actor を
    // 巻き込まず次フレーム再試行へ委ねる（R2.3・emo-text 既存契約）。
    let mut runtime = wiring.runtime.borrow_mut();
    if let Err(e) = present_frame(&mut runtime, world, talk_time) {
        error!(
            error = %e,
            talk_time,
            "emo2 text: present_frame が失敗（他 actor 非破壊・次フレーム再試行・R2.3）"
        );
    }
    // 発話が進行中（まだ現れていない文字が在る）なら次の画面更新を予約する（設計 C16 の
    // `REARM`・要件 4.6）。タイプライタは 1 コマごとに文字が増えるので、進行中は毎画面更新
    // 回す必要がある。現れ切ったら予約を止める——放置中まで回り続けさせないためである。
    if reveal_pending(
        runtime
            .state()
            .actors()
            .map(|(_, state)| state.reveal().times()),
        talk_time,
    ) {
        tick_wake::mark(tick_wake::REARM);
    }
    // 実機サインオフ用 hover 注入導線（HoverInjectConduit・8.2/8.4/8.6）: present_frame の**後**に
    // 駆動し、`choice_active`／`choice_hit_rows` が当該フレームの提示を反映した状態で env ゲート
    // （`AREKA_CHOICE_HOVER_INJECT`）駆動の周期巡回注入を行う。env 未設定/無効なら完全 no-op
    // （`inject_choice_hover` を一度も呼ばない・本番既定）。`talk_time` は同じ frame clock 時刻源。
    super::hover_inject::drive(&mut runtime, talk_time);
}
