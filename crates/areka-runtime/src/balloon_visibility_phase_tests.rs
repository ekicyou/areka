// =============================================================================
// バルーン可視性の配線層（相関数）の決定論テスト
//
// 判断そのものの網羅表は親の `balloon_visibility_*_tests.rs` 群（判断中核 `decide`）が所有する。
// ここが押さえるのは**配線**だけである:
//   ⑴ 観測の収集（ドラッグ中・観測が得られない scope の見送り）
//   ⑵ 表示の発行が失敗したときの縮退
//   ⑶ 自分が発行していない可視性の変化（`trigger=explicit`）と滞在記録の掃除
//   ⑷ 表示ライフサイクル信号の取り出しと、グリフ数・ポインタ滞在の観測
//   ⑸ 非表示の発行（既存漏斗）
//   ⑹ ログの水準とフィールド
//
// GPU も実時間の待機も用いない。時刻は `FrameTime` 資源で注入する。
// 表示層は実 `EmoPresenter` を headless に用いる——`attach_target`
// は GPU 資源を要さず、可視状態の照会（`target_visible`）と非表示の適用（`apply(Hide)`）は
// そのまま通る。
// =============================================================================

use std::sync::mpsc::{self, Sender};

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::EmoWorld;
use dola::cue::{CueCommand, TalkCue};
use log_capture_kit::{CapturedEvent, ExpectField, capture};
use tracing::Level;

use super::super::ScopeVisibility;
use super::super::test_support::{SIGNAL_AT, ms};
use super::test_support::{attach_headless, runtime, visibility_lines, world_at};
use super::*;
use crate::frame::shared_test_support::wiring_with;
use crate::frame::test_support::synth_assets;
use dola::time::testing::at_ms;
use dola::{TimePoint, TimeSpan};

// ---------------------------------------------------------------------------
// 檻の組み立て
// ---------------------------------------------------------------------------

/// 檻の配線一式（相へ渡す `RuntimeWiring` と、その相手側の送信端・受信端）。
///
/// 送信端をここで保持し続けるのは、`mpsc` の受信端が「送信端が全て落ちたら切断」を返すためで
/// ある——切断の縮退を見たいテストだけが明示的に落とす。
struct Harness {
    wiring: RuntimeWiring,
    lifecycle_tx: Option<Sender<TalkLifecycleSignal>>,
}

impl Harness {
    /// headless 配線を組む（装着済みバルーンはゼロ件）。
    fn new() -> Self {
        // 待ち時間の確定は**プロセスで一度きり**の記録であり（相はその値を毎フレーム引くだけ）、
        // 最初に相を回したテストの捕捉窓へ紛れ込む。どのテストが先に走るかは並行実行の都合で
        // 決まるため、檻の組み立て時＝捕捉窓の外で先に確定させておく。
        let _ = super::super::configured_timeout();
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
        let mut wiring = wiring_with(synth_assets(&[]), runtime(), EmoPresenter::new());
        wiring.lifecycle_rx = lifecycle_rx;
        Harness {
            wiring,
            lifecycle_tx: Some(lifecycle_tx),
        }
    }

    /// 装着済みバルーン scope を足す（相が対象 scope を引く母集合＝`balloon_models` のキー）。
    fn with_balloon_scope(mut self, scope: u32) -> Self {
        self.wiring
            .balloon_models
            .insert(scope, areka_parsers::balloon::parse_str("", None));
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

/// scope へ可視グリフを流し込む（`at=0ms` から 1 文字 250 ms で `text` を見せる）。
fn reveal_text(harness: &Harness, scope: u32, text: &str) {
    harness
        .wiring
        .runtime()
        .borrow_mut()
        .apply_cue(&TalkCue::scheduled(
            TimePoint::ZERO,
            TimeSpan::ZERO,
            areka_sakura::ActorKey::from(scope.to_string()),
            CueCommand::Text(text.into()),
            ms(250 * text.chars().count() as u32),
        ));
}

/// 表示層に target を持たない scope（非表示の適用点が「未装着」側の腕を通る）。
const UNATTACHED_SCOPE: u32 = 5;

/// 較正: 捕捉窓が生きていることを同じ窓の中で示す（「ログが無い」の主張を恒真にしないため）。
fn calibrate() {
    tracing::info!("[balloon-visibility] 較正（捕捉窓の生存確認）");
}

// ---------------------------------------------------------------------------
// ⑴ 観測の収集（ドラッグ中・観測が得られない scope の見送り）
// ---------------------------------------------------------------------------

/// 装着済みバルーンが 1 つも無いフレームは完全に無音である。
///
/// 起動直後の全フレームがこの形になるため、ここで 1 行でも出ると実機ログが埋まる。
#[test]
fn frame_without_attached_balloon_is_silent() {
    let mut harness = Harness::new();
    let mut world = world_at(at_ms(0));

    let (_, events) = capture(|| {
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

/// 表示層に target が無い scope は、理由を誤りレベルで 1 回記録して**当該 scope だけ**見送る。
///
/// 見送りが scope 単位であることを示すため、観測できない scope 0 と健全な scope 1 を同じ
/// フレームへ並べる。両者に「前フレームは可視だった」記憶を置いておくと、健全な側だけが外因
/// 遷移として処理され、観測できない側は記録だけ残して何も導かれない——見送りが隣の scope を
/// 巻き込んでいないことと、見送った scope が黙って処理されてもいないことが同時に立つ。
/// 記録が毎フレーム続くと誤りレベルの行でログが埋まるため、同じ縮退が続く間は 1 回に抑える。
#[test]
fn scope_without_present_target_is_skipped_and_logged_once() {
    let mut harness = Harness::new().with_balloon_scope(0).with_balloon_scope(1);
    let mut world = world_at(at_ms(0));
    world.insert_non_send(BalloonHover::default());
    // scope 1 だけを表示層へ装着する（scope 0 は可視状態を引けない）。
    attach_headless(&mut harness.wiring.presenter, &mut world, 1);
    for scope in [0, 1] {
        harness.wiring.balloon_visibility.per_scope.insert(
            scope,
            ScopeVisibility {
                last_glyphs: 0,
                last_clear_count: 0,
                prev_visible: true,
            },
        );
    }

    let (_, events) = capture(|| {
        harness.frame(&mut world);
        harness.frame(&mut world);
    });

    let errors: Vec<&CapturedEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.level == Level::ERROR)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "観測できない scope の記録は 2 フレームで 1 件のはず: {errors:?}"
    );
    assert_eq!(
        errors[0].expect_field("scope"),
        "0",
        "scope が 1 行から確定する"
    );

    let explicit: Vec<&CapturedEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.field_str("trigger") == Some("explicit"))
        .collect();
    assert_eq!(
        explicit.len(),
        1,
        "健全な scope の処理は見送りに巻き込まれない（かつ見送った scope は処理されない）: {events:?}"
    );
    assert_eq!(
        explicit[0].expect_field("scope"),
        "1",
        "処理されたのは装着済みの scope のみ"
    );
    assert!(
        harness.wiring.balloon_visibility.per_scope[&0].prev_visible,
        "見送った scope の記憶は触られない（観測が無いのだから更新の根拠も無い）"
    );
}

/// ドラッグ中の観測はバルーン窓の標識とドラッグ標識の**連言**で成立する。
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
// ⑵ 表示の発行が失敗したときの縮退
// ---------------------------------------------------------------------------

/// 表示の発行が失敗した scope は、誤りレベルで記録したうえで**次フレームに偽の外因遷移を
/// 生まない**。
///
/// headless の表示層は「表示が一度も確立していない」ため `show_target` が必ず失敗する。判断中核は
/// `Show` を積む時点で `prev_visible` を真にするので、巻き戻しが無ければ次フレームの差分検出が
/// 「外から消された」と読んで `trigger=explicit` を出してしまう。
#[test]
fn failed_show_is_logged_and_rolled_back() {
    let mut harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(at_ms(1000));
    attach_headless(&mut harness.wiring.presenter, &mut world, 0);
    world.insert_non_send(BalloonHover::default());
    reveal_text(&harness, 0, "あい");

    let (_, first) = capture(|| harness.frame(&mut world));

    let errors: Vec<&CapturedEvent> = visibility_lines(&first)
        .into_iter()
        .filter(|e| e.level == Level::ERROR)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "表示の発行の失敗は誤りレベルで 1 件記録する: {first:?}"
    );
    assert_eq!(errors[0].expect_field("scope"), "0");
    assert!(
        visibility_lines(&first)
            .iter()
            .all(|e| e.field("visible") != Some("true")),
        "実らなかった表示を「遷移した」と名乗らない: {first:?}"
    );

    // 巻き戻しの直接観測（発行前の値＝不可視）。
    assert!(
        !harness.wiring.balloon_visibility.per_scope[&0].prev_visible,
        "失敗した表示の scope は発行前の可視状態へ巻き戻る"
    );

    // 次フレーム: 観測は変わらない（グリフ数も可視状態も同じ）ため、遷移も外因検出も起きない。
    let (_, second) = capture(|| {
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
// ⑶ 自分が発行していない可視性の変化
// ---------------------------------------------------------------------------

/// 前フレームとの差分で外因の非表示を検出し、`trigger=explicit` で記録して滞在記録を掃除する。
#[test]
fn external_hide_is_logged_as_explicit_and_clears_hover() {
    let mut harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(at_ms(1000));
    attach_headless(&mut harness.wiring.presenter, &mut world, 0);
    world.insert_non_send(BalloonHover::default());
    world
        .get_non_send_mut::<BalloonHover>()
        .expect("挿入済み")
        .set_balloon_hover(0);

    // 前フレームは可視だった、という記憶を置く（表示層の実可視は不可視のまま＝外因の非表示）。
    harness.wiring.balloon_visibility.per_scope.insert(
        0,
        ScopeVisibility {
            last_glyphs: 2,
            last_clear_count: 0,
            prev_visible: true,
        },
    );

    let (_, events) = capture(|| harness.frame(&mut world));

    let explicit: Vec<&CapturedEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.field_str("trigger") == Some("explicit"))
        .collect();
    assert_eq!(
        explicit.len(),
        1,
        "外因の可視性変化は 1 行で記録する: {events:?}"
    );
    assert_eq!(explicit[0].level, Level::INFO, "遷移の記録は情報レベル");
    assert_eq!(explicit[0].expect_field("scope"), "0");
    assert_eq!(explicit[0].expect_field("visible"), "false");

    assert!(
        !world
            .get_non_send::<BalloonHover>()
            .expect("挿入済み")
            .is_balloon_hovered(0),
        "不可視へ落ちた scope の滞在記録は掃除する（恒久抑止への固着を断つ）"
    );
}

// ---------------------------------------------------------------------------
// ⑷ 表示ライフサイクル信号の取り出しと、グリフ数・ポインタ滞在の観測
// ---------------------------------------------------------------------------

/// 信号は毎フレーム**全件**取り出し、占有終端の畳み込みへ渡す。
#[test]
fn lifecycle_signals_are_drained_in_full() {
    let mut harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(at_ms(1000));
    attach_headless(&mut harness.wiring.presenter, &mut world, 0);
    world.insert_non_send(BalloonHover::default());

    harness.signal(TalkLifecycleSignal::TalkStarted {
        at: SIGNAL_AT,
        talk_id: None,
    });
    harness.signal(TalkLifecycleSignal::DisplayEndAt {
        at: SIGNAL_AT,
        end: at_ms(2000),
    });
    harness.signal(TalkLifecycleSignal::DisplayEndAt {
        at: SIGNAL_AT,
        end: at_ms(5000),
    });
    harness.signal(TalkLifecycleSignal::DisplayEndAt {
        at: SIGNAL_AT,
        end: at_ms(3000),
    });

    harness.frame(&mut world);

    assert_eq!(
        harness.wiring.balloon_visibility.display_end,
        Some(at_ms(5000)),
        "1 フレームで全件を取り出し、占有終端の最大値まで畳み込む"
    );
    assert!(
        harness.wiring.lifecycle_rx.try_recv().is_err(),
        "受信端に取り残しが無い"
    );
}

/// 観測が取れない軸は `None` のまま運ぶ（既定値へ潰さない）。
///
/// 文字層ランタイムを外から借りたままにして借用失敗を作り、可視グリフ数が観測なしになること・
/// 記録が 1 回であること・借用が戻れば武装し直すことを見る。
#[test]
fn runtime_borrow_failure_yields_no_glyph_observation() {
    let harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(at_ms(1000));
    world.insert_non_send(BalloonHover::default());
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
    reveal_text(&harness, 0, "あい");
    let runtime = Rc::clone(harness.wiring.runtime());
    let mut state = BalloonVisibilityState::default();

    let (observations, events) = capture(|| {
        let held = runtime.borrow_mut(); // 相の外側が保持している状態を作る
        let first = collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(at_ms(1000)),
            Vec::new(),
            &mut state,
        );
        // 2 度目も失敗するが記録は増えない。
        let second = collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(at_ms(1000)),
            Vec::new(),
            &mut state,
        );
        drop(held);
        (first, second)
    });

    let observed = observations.0.scopes[&0];
    assert_eq!(observed.visible_glyphs, None, "グリフ数は観測なし");
    assert!(
        !observed.visible,
        "可視状態は表示層から引けている（借用失敗と無関係の軸）"
    );
    assert_eq!(observations.1.scopes[&0].visible_glyphs, None);
    let errors: Vec<&CapturedEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.level == Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "同じ縮退が続く間の記録は 1 件: {errors:?}");

    // 借用が戻れば観測が成立し、次の縮退で再び 1 回鳴らせるよう武装し直している。
    let (recovered, events) = capture(|| {
        collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(at_ms(1000)),
            Vec::new(),
            &mut state,
        )
    });
    assert_eq!(
        recovered.scopes[&0].visible_glyphs.map(|g| g.count),
        Some(2),
        "注入時刻 1000ms では 2 文字ともリビール済み"
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

/// フレームの時刻が無いフレームは可視グリフ数を観測しない。
///
/// グリフ数は注入時刻の関数なので、時刻抜きに「0 文字」と読むと会話開始前の全 scope が
/// ゼロ下降エッジの手前に置かれてしまう。観測なし＝エッジなしへ倒す。
#[test]
fn glyphs_are_not_observed_without_injected_time() {
    let harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(at_ms(1000));
    world.insert_non_send(BalloonHover::default());
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
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
}

/// ポインタ配線が world に無ければ滞在は観測なし＝抑止なしへ倒し、記録は 1 回。
#[test]
fn missing_pointer_wiring_is_logged_once_and_suppresses_nothing() {
    let harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(at_ms(0));
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
    let runtime = Rc::clone(harness.wiring.runtime());
    let mut state = BalloonVisibilityState::default();

    let (observations, events) = capture(|| {
        let first = collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(at_ms(0)),
            Vec::new(),
            &mut state,
        );
        collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(at_ms(0)),
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

/// ポインタ配線が在れば scope ごとの滞在をそのまま観測する。
#[test]
fn pointer_residency_is_observed_per_scope() {
    let harness = Harness::new().with_balloon_scope(0).with_balloon_scope(1);
    let mut world = world_at(at_ms(0));
    world.insert_non_send(BalloonHover::default());
    world
        .get_non_send_mut::<BalloonHover>()
        .expect("挿入済み")
        .set_balloon_hover(1);
    let mut presenter = EmoPresenter::new();
    for scope in [0, 1] {
        attach_headless(&mut presenter, &mut world, scope);
    }
    let runtime = Rc::clone(harness.wiring.runtime());
    let mut state = BalloonVisibilityState::default();

    let observations = collect_observations(
        &presenter,
        &runtime,
        &mut world,
        &[0, 1],
        Some(at_ms(0)),
        Vec::new(),
        &mut state,
    );

    assert_eq!(observations.scopes[&0].hover, Some(false));
    assert_eq!(observations.scopes[&1].hover, Some(true));
}

// ---------------------------------------------------------------------------
// ⑸ 非表示の発行（既存漏斗）
// ---------------------------------------------------------------------------

/// 非表示はバルーン target の可視性だけを変え、キャラクター窓の target には触れない。
#[test]
fn hide_touches_only_the_balloon_target() {
    let mut world = world_at(at_ms(0));
    world.insert_non_send(BalloonHover::default());
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
    // キャラクター窓の target は balloon target でないので共有の道具を通さず直に装着する。
    let shell_window = world.spawn_empty().id();
    presenter.attach_target(
        crate::target_map::shell_target(0),
        shell_window,
        EmoWorld::build(&areka_parsers::shell::parse("")),
        AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
        96,
    );
    let mut state = BalloonVisibilityState::default();
    let observations = VisibilityObservations::default();

    // 発行の観測は表示層が非表示の適用点で出す 1 行（`presenter/hub.rs`）で行う。可視状態の
    // 照会は使えない——装着直後の target は既に不可視であり、発行を丸ごと取り除いても同じ値を
    // 返す（＝どんな実装でも通る空虚な主張になる）。
    //
    // 同じ捕捉窓へ**未装着 scope への非表示**を 1 件混ぜる。表示層は未装着 target に対しても
    // 非表示の適用点で 1 行出すため、「キャラクター窓の target が現れない」という
    // 否定側の主張が、窓が何も捕まえていないことによって恒真になっていないことを同時に示せる。
    let (outcome, events) = capture(|| {
        issue_actions(
            &mut presenter,
            &mut world,
            &runtime(),
            &mut state,
            &observations,
            &[super::super::test_support::hide(
                &[0, UNATTACHED_SCOPE],
                VisibilityTrigger::Timeout,
            )],
        )
    });

    assert_eq!(
        outcome.hidden,
        vec![0, UNATTACHED_SCOPE],
        "掃除対象として scope を返す"
    );

    let applied: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains("apply(Hide): 非表示へ"))
        .collect();
    assert_eq!(
        applied.len(),
        1,
        "非表示は装着済みの 1 target へちょうど 1 回適用される: {events:?}"
    );
    assert_eq!(
        applied[0].expect_field("target_id"),
        format!("{:?}", balloon_target(0)),
        "適用先はバルーンの target"
    );

    let shell = format!("{:?}", crate::target_map::shell_target(0));
    assert!(
        events
            .iter()
            .all(|e| e.field("target_id") != Some(shell.as_str())),
        "キャラクター窓の target へは指令が届かない（可視性へ波及しない）: {events:?}"
    );

    // 較正: 未装着 scope の分がこの窓で確かに見えている（否定側の主張が空虚でない）。
    assert_eq!(
        events
            .iter()
            .filter(|e| e.message().contains("apply(Hide): 未装着ターゲット"))
            .filter(|e| e.expect_field("target_id")
                == format!("{:?}", balloon_target(UNATTACHED_SCOPE)))
            .count(),
        1,
        "未装着 scope への非表示も同じ窓で 1 行として見えている: {events:?}"
    );
}

/// 不可視へ落ちた scope の滞在記録だけを掃除する（他 scope は残す）。
#[test]
fn hover_residency_is_cleared_for_hidden_scopes_only() {
    let mut world = World::new();
    world.insert_non_send(BalloonHover::default());
    {
        let mut wiring = world.get_non_send_mut::<BalloonHover>().expect("挿入済み");
        wiring.set_balloon_hover(0);
        wiring.set_balloon_hover(1);
    }

    clear_hover_residency(&mut world, &[0]);

    let wiring = world.get_non_send::<BalloonHover>().expect("挿入済み");
    assert!(!wiring.is_balloon_hovered(0), "不可視へ落ちた scope は掃除");
    assert!(wiring.is_balloon_hovered(1), "他 scope の滞在は残る");
}

// ---------------------------------------------------------------------------
// ⑹ ログの水準とフィールド
// ---------------------------------------------------------------------------

/// 信号の受信端が切断されたら誤りレベルで 1 回だけ記録する。
#[test]
fn disconnected_lifecycle_channel_is_logged_once() {
    let mut harness = Harness::new().with_balloon_scope(0);
    let mut world = world_at(at_ms(1000));
    attach_headless(&mut harness.wiring.presenter, &mut world, 0);
    world.insert_non_send(BalloonHover::default());
    harness.disconnect_lifecycle();

    let (_, events) = capture(|| {
        harness.frame(&mut world);
        harness.frame(&mut world);
    });

    let errors: Vec<&CapturedEvent> = visibility_lines(&events)
        .into_iter()
        .filter(|e| e.level == Level::ERROR)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "切断の記録は 2 フレームで 1 件のはず: {errors:?}"
    );
}
