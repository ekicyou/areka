// =============================================================================
// バルーン可視性の配線層（相関数）の決定論テスト（task 4.4）
//
// 判断そのものの網羅表は親の `balloon_visibility_tests.rs`（判断中核 `decide`）が所有する。
// ここが押さえるのは**配線**だけである:
//   ⑴ 観測の収集（可視状態・可視グリフ数・選択肢表示中・ポインタ滞在・ドラッグ中・信号）
//   ⑵ 判断が返した行動の発行（表示は専用入口・非表示は既存漏斗）と失敗時の縮退
//   ⑶ 判断が返した事象のログへの写し（design 決定 D10 の水準・フィールド）
//   ⑷ 自分が発行していない可視性変化の検出（`trigger=explicit`）と滞在記録の掃除
//
// GPU も実時間の待機も用いない。時刻は `FrameTime` 資源と `TalkClock` の epoch で注入する
// （Requirement 9.2 / 9.3）。表示層は実 `EmoPresenter` を headless に用いる——`attach_target`
// は GPU 資源を要さず、可視状態の照会（`target_visible`）と非表示の適用（`apply(Hide)`）は
// そのまま通る。
// =============================================================================

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
use bevy_ecs::prelude::Entity;
use dola::cue::{CueCommand, TalkCue};
use tracing::Level;

use super::super::{
    BalloonVisibilityState, MeasurementDiscardReason, ScopeVisibility, SuppressionKinds,
};
use super::*;
use crate::emo2_boot::assets::{BootAssets, LoopTables};
use crate::emo2_boot::talk_clock::TalkClock;
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::emo2_boot::target_map::balloon_target;
use crate::input_events::balloon::{BalloonWiring, ChoiceSelection};
use crate::placement::test_support::{LogEvent, capture_logs};
use wintf::ecs::FrameTime;

// ---------------------------------------------------------------------------
// 檻の組み立て
// ---------------------------------------------------------------------------

/// 表示資産を 1 つも持たない `BootAssets`（相は資産を読まない——読むのは `balloon_models` だけ）。
fn empty_assets() -> BootAssets {
    BootAssets {
        shells: Vec::new(),
        balloons: Vec::new(),
        resolver: SurfaceResolver::new(BTreeMap::new()),
        static_binds: BindSet::default(),
        bind_resolver: BindResolver::empty(),
        loop_tables: LoopTables {
            shell: AnimationTable::empty(),
            balloon: BTreeMap::new(),
        },
        shell_author_dpi: 96,
        balloon_author_dpi: 96,
    }
}

/// 檻の配線一式（相へ渡す `Emo2Wiring` と、その相手側の送信端・受信端）。
///
/// 送信端をここで保持し続けるのは、`mpsc` の受信端が「送信端が全て落ちたら切断」を返すためで
/// ある——切断の縮退を見たいテストだけが明示的に落とす。
struct Harness {
    wiring: Emo2Wiring,
    lifecycle_tx: Option<Sender<TalkLifecycleSignal>>,
    /// 選択確定の受け口（`BalloonWiring` の送信端を生かしておくためだけに保持する）。
    _choice_rx: Option<mpsc::Receiver<ChoiceSelection>>,
}

impl Harness {
    /// epoch 未確立の headless 配線を組む（装着済みバルーンはゼロ件）。
    fn new() -> Self {
        // 待ち時間の確定は**プロセスで一度きり**の記録であり（相はその値を毎フレーム引くだけ）、
        // 最初に相を回したテストの捕捉窓へ紛れ込む。どのテストが先に走るかは並行実行の都合で
        // 決まるため、檻の組み立て時＝捕捉窓の外で先に確定させておく。
        let _ = super::super::configured_timeout_secs();
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
        let wiring = Emo2Wiring::new(
            EmoPresenter::new(),
            mpsc::channel().1,
            mpsc::channel().1,
            lifecycle_rx,
            Rc::new(RefCell::new(TextLayerRuntime::new(
                TextLayerConfig::default(),
            ))),
            TalkClock::new(Arc::new(|| 0.0)),
            empty_assets(),
        );
        Harness {
            wiring,
            lifecycle_tx: Some(lifecycle_tx),
            _choice_rx: None,
        }
    }

    /// 装着済みバルーン scope を足す（相が対象 scope を引く母集合＝`balloon_models` のキー）。
    fn with_balloon_scope(mut self, scope: u32) -> Self {
        self.wiring
            .balloon_models
            .insert(scope, areka_parsers::balloon::parse_str("", None));
        self
    }

    /// talk の起点を確立する（`talk_time = frame_now − 0.0`）。
    fn with_talk_epoch(self) -> Self {
        self.wiring.clock.observe_cue(0.0);
        self
    }

    /// 本フレームの表示ライフサイクル信号を送る。
    fn signal(&self, signal: TalkLifecycleSignal) {
        self.lifecycle_tx
            .as_ref()
            .expect("送信端が生きている")
            .send(signal)
            .expect("受信端は wiring が保持している");
    }

    /// 信号の送信端を落とす（受信端から見て切断）。
    fn disconnect_lifecycle(&mut self) {
        self.lifecycle_tx = None;
    }

    /// 相を 1 フレーム回す。
    fn frame(&mut self, world: &mut World) {
        run_balloon_visibility_phase(&mut self.wiring, world);
    }
}

/// 表示層へ scope の balloon target を headless 装着する（可視状態は `Some(false)` から始まる）。
fn attach_balloon(harness: &mut Harness, world: &mut World, scope: u32) -> Entity {
    let window = world.spawn_empty().id();
    harness
        .wiring
        .presenter
        .attach_target(
            world,
            balloon_target(scope),
            window,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96,
        )
        .expect("headless 装着は成功する");
    window
}

/// 現在時刻（talk 相対秒）を注入した World を作る。
fn world_at(frame_now: f64) -> World {
    let mut world = World::new();
    world.insert_resource(FrameTime(frame_now));
    world
}

/// scope へ可視グリフを流し込む（`at=0.0` 起点・注入時刻 `t` で `len` 文字が可視になる）。
fn reveal_text(harness: &Harness, scope: u32, text: &str) {
    harness.wiring.runtime().borrow_mut().apply_cue(&TalkCue {
        at: 0.0,
        actor: areka_sakura::ActorKey::from(scope.to_string()),
        command: CueCommand::Text(text.into()),
        duration: text.chars().count() as f64 * 0.25,
    });
}

/// `[balloon-visibility]` を名乗る行だけを取り出す（他所のログは配線の関心事ではない）。
fn visibility_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains("[balloon-visibility]"))
        .collect()
}

/// 表示層に target を持たない scope（非表示の適用点が「未装着」側の腕を通る）。
const UNATTACHED_SCOPE: u32 = 5;

/// 較正: 捕捉窓が生きていることを同じ窓の中で示す（「ログが無い」の主張を恒真にしないため）。
fn calibrate() {
    tracing::info!("[balloon-visibility] 較正（捕捉窓の生存確認）");
}

// ---------------------------------------------------------------------------
// ⑴ 観測が得られない場合は理由を記録して当該 scope のみ見送る
// ---------------------------------------------------------------------------

/// 装着済みバルーンが 1 つも無いフレームは完全に無音である（Requirement 8.6）。
///
/// 起動直後の全フレームがこの形になるため、ここで 1 行でも出ると実機ログが埋まる。
#[test]
fn frame_without_attached_balloon_is_silent() {
    let mut harness = Harness::new();
    let mut world = world_at(0.0);

    let (_, events) = capture_logs(|| {
        harness.frame(&mut world);
        calibrate();
    });

    let lines = visibility_lines(&events);
    assert_eq!(
        lines.len(),
        1,
        "較正の 1 件だけが在るはず（装着ゼロのフレームは無音）: {lines:?}"
    );
}

/// 表示層に target が無い scope は、理由を誤りレベルで 1 回記録して**当該 scope だけ**見送る
/// （Requirement 8.4・タスクの「当該 scope のみ見送る」）。
///
/// 見送りが scope 単位であることを示すため、観測できない scope 0 と健全な scope 1 を同じ
/// フレームへ並べる。両者に「前フレームは可視だった」記憶を置いておくと、健全な側だけが外因
/// 遷移として処理され、観測できない側は記録だけ残して何も導かれない——見送りが隣の scope を
/// 巻き込んでいないことと、見送った scope が黙って処理されてもいないことが同時に立つ。
/// 記録が毎フレーム続くと誤りレベルの行でログが埋まるため、同じ縮退が続く間は 1 回に抑える。
#[test]
fn scope_without_present_target_is_skipped_and_logged_once() {
    let mut harness = Harness::new().with_balloon_scope(0).with_balloon_scope(1);
    let mut world = world_at(0.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    // scope 1 だけを表示層へ装着する（scope 0 は可視状態を引けない）。
    attach_balloon(&mut harness, &mut world, 1);
    for scope in [0, 1] {
        harness.wiring.balloon_visibility.per_scope.insert(
            scope,
            ScopeVisibility {
                last_glyphs: 0,
                prev_visible: true,
            },
        );
    }

    let (_, events) = capture_logs(|| {
        harness.frame(&mut world);
        harness.frame(&mut world);
    });

    let errors: Vec<&LogEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.level == Level::ERROR)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "観測できない scope の記録は 2 フレームで 1 件のはず: {errors:?}"
    );
    assert_eq!(errors[0].field("scope"), "0", "scope が 1 行から確定する");

    let explicit: Vec<&LogEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.fields.get("trigger").map(String::as_str) == Some("\"explicit\""))
        .collect();
    assert_eq!(
        explicit.len(),
        1,
        "健全な scope の処理は見送りに巻き込まれない（かつ見送った scope は処理されない）: {events:?}"
    );
    assert_eq!(
        explicit[0].field("scope"),
        "1",
        "処理されたのは装着済みの scope のみ"
    );
    assert!(
        harness.wiring.balloon_visibility.per_scope[&0].prev_visible,
        "見送った scope の記憶は触られない（観測が無いのだから更新の根拠も無い）"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 表示の発行が失敗したときの縮退（Requirement 4.4・8.4）
// ---------------------------------------------------------------------------

/// 表示の発行が失敗した scope は、誤りレベルで記録したうえで**次フレームに偽の外因遷移を
/// 生まない**（tasks.md「タスク 4.4 の義務」＝発行前の値への巻き戻し）。
///
/// headless の表示層は「表示が一度も確立していない」ため `show_target` が必ず失敗する。判断中核は
/// `Show` を積む時点で `prev_visible` を真にするので、巻き戻しが無ければ次フレームの差分検出が
/// 「外から消された」と読んで `trigger=explicit` を出してしまう。
#[test]
fn failed_show_is_logged_and_rolled_back() {
    let mut harness = Harness::new().with_balloon_scope(0).with_talk_epoch();
    let mut world = world_at(1.0);
    attach_balloon(&mut harness, &mut world, 0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    reveal_text(&harness, 0, "あい");

    let (_, first) = capture_logs(|| harness.frame(&mut world));

    let errors: Vec<&LogEvent> = visibility_lines(&first)
        .into_iter()
        .filter(|e| e.level == Level::ERROR)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "表示の発行の失敗は誤りレベルで 1 件記録する: {first:?}"
    );
    assert_eq!(errors[0].field("scope"), "0");
    assert!(
        visibility_lines(&first)
            .iter()
            .all(|e| e.fields.get("visible").map(String::as_str) != Some("true")),
        "実らなかった表示を「遷移した」と名乗らない: {first:?}"
    );

    // 巻き戻しの直接観測（発行前の値＝不可視）。
    assert_eq!(
        harness.wiring.balloon_visibility.per_scope[&0].prev_visible, false,
        "失敗した表示の scope は発行前の可視状態へ巻き戻る"
    );

    // 次フレーム: 観測は変わらない（グリフ数も可視状態も同じ）ため、遷移も外因検出も起きない。
    let (_, second) = capture_logs(|| {
        harness.frame(&mut world);
        calibrate();
    });
    let lines = visibility_lines(&second);
    assert_eq!(
        lines.len(),
        1,
        "較正の 1 件だけが在るはず（偽の外因遷移も再発行も起きない）: {lines:?}"
    );
}

// ---------------------------------------------------------------------------
// ⑶ 自分が発行していない可視性の変化（Requirement 8.1 の「明示指令」）
// ---------------------------------------------------------------------------

/// 前フレームとの差分で外因の非表示を検出し、`trigger=explicit` で記録して滞在記録を掃除する。
#[test]
fn external_hide_is_logged_as_explicit_and_clears_hover() {
    let mut harness = Harness::new().with_balloon_scope(0).with_talk_epoch();
    let mut world = world_at(1.0);
    attach_balloon(&mut harness, &mut world, 0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    world
        .get_non_send_mut::<BalloonWiring>()
        .expect("挿入済み")
        .set_balloon_hover(0);

    // 前フレームは可視だった、という記憶を置く（表示層の実可視は不可視のまま＝外因の非表示）。
    harness.wiring.balloon_visibility.per_scope.insert(
        0,
        ScopeVisibility {
            last_glyphs: 2,
            prev_visible: true,
        },
    );

    let (_, events) = capture_logs(|| harness.frame(&mut world));

    let explicit: Vec<&LogEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.fields.get("trigger").map(String::as_str) == Some("\"explicit\""))
        .collect();
    assert_eq!(
        explicit.len(),
        1,
        "外因の可視性変化は 1 行で記録する: {events:?}"
    );
    assert_eq!(
        explicit[0].level,
        Level::INFO,
        "遷移の記録は情報レベル（D10）"
    );
    assert_eq!(explicit[0].field("scope"), "0");
    assert_eq!(explicit[0].field("visible"), "false");

    assert!(
        !world
            .get_non_send::<BalloonWiring>()
            .expect("挿入済み")
            .is_balloon_hovered(0),
        "不可視へ落ちた scope の滞在記録は掃除する（恒久抑止への固着を断つ）"
    );
}

// ---------------------------------------------------------------------------
// ⑷ 表示ライフサイクル信号の取り出し
// ---------------------------------------------------------------------------

/// 信号は毎フレーム**全件**取り出し、占有終端の畳み込みへ渡す（Requirements 4.1 / 4.5）。
#[test]
fn lifecycle_signals_are_drained_in_full() {
    let mut harness = Harness::new().with_balloon_scope(0).with_talk_epoch();
    let mut world = world_at(1.0);
    attach_balloon(&mut harness, &mut world, 0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));

    harness.signal(TalkLifecycleSignal::TalkStarted);
    harness.signal(TalkLifecycleSignal::DisplayEndAt(2.0));
    harness.signal(TalkLifecycleSignal::DisplayEndAt(5.0));
    harness.signal(TalkLifecycleSignal::DisplayEndAt(3.0));

    harness.frame(&mut world);

    assert_eq!(
        harness.wiring.balloon_visibility.display_end,
        Some(5.0),
        "1 フレームで全件を取り出し、占有終端の最大値まで畳み込む"
    );
    assert!(
        harness.wiring.lifecycle_rx.try_recv().is_err(),
        "受信端に取り残しが無い"
    );
}

/// 観測が取れない軸は `None` のまま運ぶ（既定値へ潰さない・Requirement 5.5）。
///
/// 文字層ランタイムを外から借りたままにして借用失敗を作り、可視グリフ数と選択肢表示中の双方が
/// 観測なしになること・記録が 1 回であること・借用が戻れば武装し直すことを見る。
#[test]
fn runtime_borrow_failure_yields_no_glyph_and_no_choice_observation() {
    let harness = Harness::new().with_balloon_scope(0).with_talk_epoch();
    let mut world = world_at(1.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    let mut presenter = EmoPresenter::new();
    let window = world.spawn_empty().id();
    presenter
        .attach_target(
            &mut world,
            balloon_target(0),
            window,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96,
        )
        .expect("headless 装着は成功する");
    reveal_text(&harness, 0, "あい");
    let runtime = Rc::clone(harness.wiring.runtime());
    let mut state = BalloonVisibilityState::default();

    let (observations, events) = capture_logs(|| {
        let held = runtime.borrow_mut(); // 相の外側が保持している状態を作る
        let first = collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(1.0),
            Vec::new(),
            &mut state,
        );
        // 2 度目も失敗するが記録は増えない。
        let second = collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(1.0),
            Vec::new(),
            &mut state,
        );
        drop(held);
        (first, second)
    });

    let observed = observations.0.scopes[&0];
    assert_eq!(observed.visible_glyphs, None, "グリフ数は観測なし");
    assert_eq!(observed.choice_active, None, "選択肢表示中も観測なし");
    assert_eq!(
        observed.visible, false,
        "可視状態は表示層から引けている（借用失敗と無関係の軸）"
    );
    assert_eq!(observations.1.scopes[&0].visible_glyphs, None);
    let errors: Vec<&LogEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.level == Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "同じ縮退が続く間の記録は 1 件: {errors:?}");

    // 借用が戻れば観測が成立し、次の縮退で再び 1 回鳴らせるよう武装し直している。
    let (recovered, events) = capture_logs(|| {
        collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(1.0),
            Vec::new(),
            &mut state,
        )
    });
    assert_eq!(
        recovered.scopes[&0].visible_glyphs,
        Some(2),
        "注入時刻 1.0 では 2 文字ともリビール済み"
    );
    assert_eq!(
        recovered.scopes[&0].choice_active,
        Some(false),
        "選択肢スパンが無いので抑止しない"
    );
    assert!(
        visibility_lines(&events).is_empty(),
        "観測が成立したフレームは無音: {events:?}"
    );
    assert!(
        !state.observation_failure_logged.runtime,
        "記録の武装が戻っている"
    );
}

/// 注入時刻が定まらないフレーム（talk 起点未確立）は可視グリフ数を観測しない。
///
/// グリフ数は注入時刻の関数なので、時刻抜きに「0 文字」と読むと会話開始前の全 scope が
/// ゼロ下降エッジの手前に置かれてしまう。観測なし＝エッジなしへ倒す。
#[test]
fn glyphs_are_not_observed_without_injected_time() {
    let harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(1.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    let mut presenter = EmoPresenter::new();
    let window = world.spawn_empty().id();
    presenter
        .attach_target(
            &mut world,
            balloon_target(0),
            window,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96,
        )
        .expect("headless 装着は成功する");
    reveal_text(&harness, 0, "あい");
    let runtime = Rc::clone(harness.wiring.runtime());
    let mut state = BalloonVisibilityState::default();

    let observations = collect_observations(
        &presenter,
        &runtime,
        &mut world,
        &[0],
        None,
        Vec::new(),
        &mut state,
    );

    assert_eq!(observations.scopes[&0].visible_glyphs, None);
    assert_eq!(
        observations.scopes[&0].choice_active,
        Some(false),
        "選択肢表示中は時刻に依らず観測できる"
    );
}

/// ポインタ配線が world に無ければ滞在は観測なし＝抑止なしへ倒し、記録は 1 回（Requirement 5.5）。
#[test]
fn missing_pointer_wiring_is_logged_once_and_suppresses_nothing() {
    let harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(0.0);
    let mut presenter = EmoPresenter::new();
    let window = world.spawn_empty().id();
    presenter
        .attach_target(
            &mut world,
            balloon_target(0),
            window,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96,
        )
        .expect("headless 装着は成功する");
    let runtime = Rc::clone(harness.wiring.runtime());
    let mut state = BalloonVisibilityState::default();

    let (observations, events) = capture_logs(|| {
        let first = collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(0.0),
            Vec::new(),
            &mut state,
        );
        collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(0.0),
            Vec::new(),
            &mut state,
        );
        first
    });

    assert_eq!(observations.scopes[&0].hover, None, "滞在は観測なし");
    assert_eq!(
        visibility_lines(&events)
            .into_iter()
            .filter(|e| e.level == Level::ERROR)
            .count(),
        1,
        "記録は 2 フレームで 1 件: {events:?}"
    );
}

/// ポインタ配線が在れば scope ごとの滞在をそのまま観測する（Requirement 5.2）。
#[test]
fn pointer_residency_is_observed_per_scope() {
    let harness = Harness::new().with_balloon_scope(0).with_balloon_scope(1);
    let mut world = world_at(0.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    world
        .get_non_send_mut::<BalloonWiring>()
        .expect("挿入済み")
        .set_balloon_hover(1);
    let mut presenter = EmoPresenter::new();
    for scope in [0, 1] {
        let window = world.spawn_empty().id();
        presenter
            .attach_target(
                &mut world,
                balloon_target(scope),
                window,
                EmoWorld::build(&areka_parsers::shell::parse("")),
                AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
                96,
            )
            .expect("headless 装着は成功する");
    }
    let runtime = Rc::clone(harness.wiring.runtime());
    let mut state = BalloonVisibilityState::default();

    let observations = collect_observations(
        &presenter,
        &runtime,
        &mut world,
        &[0, 1],
        Some(0.0),
        Vec::new(),
        &mut state,
    );

    assert_eq!(observations.scopes[&0].hover, Some(false));
    assert_eq!(observations.scopes[&1].hover, Some(true));
}

/// ドラッグ中の観測はバルーン窓の標識とドラッグ標識の**連言**で成立する（design 決定 D8）。
#[test]
fn dragging_requires_both_balloon_window_and_drag_marker() {
    let mut world = World::new();
    assert!(!observe_dragging(&mut world), "標識が無ければ成立しない");

    let balloon = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    assert!(
        !observe_dragging(&mut world),
        "バルーン窓だけではドラッグ中ではない"
    );

    let other = world.spawn(WindowDragging).id();
    assert!(
        !observe_dragging(&mut world),
        "バルーン窓でない窓のドラッグは対象外"
    );

    world.entity_mut(balloon).insert(WindowDragging);
    assert!(observe_dragging(&mut world), "両方揃えば成立する");

    world.entity_mut(balloon).remove::<WindowDragging>();
    world.despawn(other);
    assert!(!observe_dragging(&mut world), "ドラッグ終了で戻る");
}

// ---------------------------------------------------------------------------
// ⑸ 非表示の発行（既存漏斗・Requirements 4.4 / 6.7）
// ---------------------------------------------------------------------------

/// 非表示はバルーン target の可視性だけを変え、キャラクター窓の target には触れない
/// （Requirements 6.7・4.4）。
#[test]
fn hide_touches_only_the_balloon_target() {
    let mut world = world_at(0.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    let mut presenter = EmoPresenter::new();
    for target in [
        balloon_target(0),
        crate::emo2_boot::target_map::shell_target(0),
    ] {
        let window = world.spawn_empty().id();
        presenter
            .attach_target(
                &mut world,
                target,
                window,
                EmoWorld::build(&areka_parsers::shell::parse("")),
                AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
                96,
            )
            .expect("headless 装着は成功する");
    }
    let mut state = BalloonVisibilityState::default();
    let observations = VisibilityObservations::default();

    // 発行の観測は表示層が非表示の適用点で出す 1 行（`presenter/hub.rs:125`）で行う。可視状態の
    // 照会は使えない——装着直後の target は既に不可視であり、発行を丸ごと取り除いても同じ値を
    // 返す（＝どんな実装でも通る空虚な主張になる）。
    //
    // 同じ捕捉窓へ**未装着 scope への非表示**を 1 件混ぜる。表示層は未装着 target に対しても
    // 非表示の適用点で 1 行出す（同 `:117`）ため、「キャラクター窓の target が現れない」という
    // 否定側の主張が、窓が何も捕まえていないことによって恒真になっていないことを同時に示せる。
    let (outcome, events) = capture_logs(|| {
        issue_actions(
            &mut presenter,
            &mut world,
            &mut state,
            &observations,
            &[VisibilityAction::HideScopes {
                scopes: vec![0, UNATTACHED_SCOPE],
                trigger: VisibilityTrigger::Timeout,
            }],
        )
    });

    assert_eq!(
        outcome.hidden,
        vec![0, UNATTACHED_SCOPE],
        "掃除対象として scope を返す"
    );

    let applied: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.message().contains("apply(Hide): 非表示へ"))
        .collect();
    assert_eq!(
        applied.len(),
        1,
        "非表示は装着済みの 1 target へちょうど 1 回適用される: {events:?}"
    );
    assert_eq!(
        applied[0].field("target_id"),
        format!("{:?}", balloon_target(0)),
        "適用先はバルーンの target"
    );

    let shell = format!("{:?}", crate::emo2_boot::target_map::shell_target(0));
    assert!(
        events
            .iter()
            .all(|e| e.fields.get("target_id") != Some(&shell)),
        "キャラクター窓の target へは指令が届かない（可視性へ波及しない・Requirement 6.7）: {events:?}"
    );

    // 較正: 未装着 scope の分がこの窓で確かに見えている（否定側の主張が空虚でない）。
    assert_eq!(
        events
            .iter()
            .filter(|e| e.message().contains("apply(Hide): 未装着ターゲット"))
            .filter(|e| e.field("target_id") == format!("{:?}", balloon_target(UNATTACHED_SCOPE)))
            .count(),
        1,
        "未装着 scope への非表示も同じ窓で 1 行として見えている: {events:?}"
    );
}

/// 不可視へ落ちた scope の滞在記録だけを掃除する（他 scope は残す）。
#[test]
fn hover_residency_is_cleared_for_hidden_scopes_only() {
    let mut world = World::new();
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    {
        let mut wiring = world.get_non_send_mut::<BalloonWiring>().expect("挿入済み");
        wiring.set_balloon_hover(0);
        wiring.set_balloon_hover(1);
    }

    clear_hover_residency(&mut world, &[0]);

    let wiring = world.get_non_send::<BalloonWiring>().expect("挿入済み");
    assert!(!wiring.is_balloon_hovered(0), "不可視へ落ちた scope は掃除");
    assert!(wiring.is_balloon_hovered(1), "他 scope の滞在は残る");
}

// ---------------------------------------------------------------------------
// ⑹ ログの水準とフィールド（design 決定 D10・Requirements 8.1〜8.5）
// ---------------------------------------------------------------------------

/// 判断が返した事象は D10 の水準と語彙で 1 行ずつ出る。
#[test]
fn decision_events_are_written_with_the_agreed_level_and_fields() {
    let logs = vec![
        VisibilityLogEvent::Transition {
            scope: 1,
            trigger: VisibilityTrigger::Content,
            visible: true,
        },
        VisibilityLogEvent::Transition {
            scope: 0,
            trigger: VisibilityTrigger::Clear,
            visible: false,
        },
        VisibilityLogEvent::MeasurementStarted {
            display_end: 4.0,
            deadline: 34.0,
        },
        VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::TalkStarted,
            deadline: 34.0,
        },
        VisibilityLogEvent::MeasurementRestarted {
            now: 40.0,
            deadline: 70.0,
        },
        VisibilityLogEvent::TimeoutSuppressed {
            kinds: SuppressionKinds {
                dragging: false,
                hover: true,
                choice: false,
            },
            deadline: 34.0,
        },
        VisibilityLogEvent::DisplayEndSignalMissing,
    ];

    let (_, events) = capture_logs(|| emit_visibility_logs(&logs, &[]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 7, "事象 1 件につき 1 行: {events:?}");

    assert_eq!(lines[0].level, Level::INFO);
    assert_eq!(lines[0].field("scope"), "1");
    assert_eq!(lines[0].field("trigger"), "\"content\"");
    assert_eq!(lines[0].field("visible"), "true");

    assert_eq!(lines[1].field("trigger"), "\"clear\"");
    assert_eq!(lines[1].field("visible"), "false");

    assert_eq!(lines[2].level, Level::INFO);
    assert_eq!(lines[2].field("display_end"), "4.0");
    assert_eq!(lines[2].field("deadline"), "34.0");

    assert_eq!(lines[3].field("reason"), "\"talk_started\"");

    assert_eq!(lines[4].field("talk_time"), "40.0");
    assert_eq!(lines[4].field("deadline"), "70.0");

    assert_eq!(lines[5].level, Level::INFO);
    assert_eq!(lines[5].field("hover"), "true");
    assert_eq!(lines[5].field("dragging"), "false");
    assert_eq!(lines[5].field("choice"), "false");

    assert_eq!(
        lines[6].level,
        Level::WARN,
        "表示終了信号の欠落は縮退（表示は保持する）"
    );
}

/// 発行が実らなかった scope の「表示へ遷移した」は名乗らない（起きていない遷移を記録しない）。
#[test]
fn transition_of_a_show_that_did_not_take_is_not_written() {
    let logs = vec![
        VisibilityLogEvent::Transition {
            scope: 0,
            trigger: VisibilityTrigger::Content,
            visible: true,
        },
        VisibilityLogEvent::Transition {
            scope: 1,
            trigger: VisibilityTrigger::Content,
            visible: true,
        },
        VisibilityLogEvent::Transition {
            scope: 0,
            trigger: VisibilityTrigger::Clear,
            visible: false,
        },
    ];

    let (_, events) = capture_logs(|| emit_visibility_logs(&logs, &[0]));

    let lines = visibility_lines(&events);
    assert_eq!(lines.len(), 2, "scope 0 の表示だけが落ちる: {events:?}");
    assert_eq!(lines[0].field("scope"), "1");
    assert_eq!(lines[1].field("scope"), "0");
    assert_eq!(
        lines[1].field("visible"),
        "false",
        "非表示の遷移は実際に起きるので落とさない"
    );
}

/// 信号の受信端が切断されたら誤りレベルで 1 回だけ記録する（design「観測の収集（配線層）」）。
#[test]
fn disconnected_lifecycle_channel_is_logged_once() {
    let mut harness = Harness::new().with_balloon_scope(0).with_talk_epoch();
    let mut world = world_at(1.0);
    attach_balloon(&mut harness, &mut world, 0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    harness.disconnect_lifecycle();

    let (_, events) = capture_logs(|| {
        harness.frame(&mut world);
        harness.frame(&mut world);
    });

    let errors: Vec<&LogEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.level == Level::ERROR)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "切断の記録は 2 フレームで 1 件のはず: {errors:?}"
    );
}
