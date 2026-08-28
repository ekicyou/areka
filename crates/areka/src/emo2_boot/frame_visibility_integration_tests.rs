// =============================================================================
// フレーム統合の検証（task 6.6）
//
// ここが押さえるのは**相順そのもの**——本番の `emo2_frame_system` が design 決定 D5 の順で
// 各相を回したときにだけ成り立つ性質である。判断中核の分岐表・配線層の縮退・表示層の
// 可視性能力は、それぞれ `balloon_visibility_tests.rs` 群・`balloon_visibility_phase_tests.rs`・
// `areka-emo-present` の in-crate テストが所有しており、ここでは重ねない。
//
// 実 emo2 fixture ＋ 実 GPU（WARP 可・MTA）で本番の `emo2_frame_system` をそのまま駆動する。
// 表示層の可視化（`show_target`）は「表示が一度も確立していない」と必ず失敗するため、確立が
// 成立する実 attach を通さない限り、以下の性質はどれも観測できない。
//
// 時刻はすべて注入（`FrameTime` 資源＋`TalkClock` の epoch）で、実時間の待機・スリープ・
// 反復回数のみによる有界化は 1 つも用いない（Requirements 9.2 / 9.3）。
// =============================================================================

use std::path::PathBuf;
use std::sync::mpsc;

use areka_emo_text::state::TextLayerConfig;
use dola::cue::{CueCommand, TalkCue};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::Visual;
use wintf::ecs::layout::Arrangement;
use wintf::ecs::window::{ZOrderGroupSpec, ZOrderGroups};

use crate::emo2_boot::assets::build_boot_assets;
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::input_events::balloon::BalloonWiring;
use crate::placement::test_support::{ExpectField, LogEvent, capture_logs};

use super::test_support::{dpi_world, size_of, zero_clock};
use super::*;

// ---------------------------------------------------------------------------
// 檻の組み立て
// ---------------------------------------------------------------------------

/// emo2 fixture ルート（`CARGO_MANIFEST_DIR`＝`crates/areka` 相対・assets.rs テストと同一規約）。
fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// emo2 fixture のバルーンルート（assets.rs テストと同一規約）。
fn emo2_balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}

/// 相順の檻の World（2 スコープ窓＋偽 `WindowHandle`＋全窓 DPI 96＋実 GPU 資源）。
///
/// attach 相のゲートは `GhostWindows`＋`GraphicsCore`＋`WucGraphicsResource::is_valid()` の
/// 連言ゆえ、本番の相順を駆動するにはこの 3 点が要る。本番 UI スレッドと同じ MTA で COM を
/// 初期化する（記憶: areka WUC は MTA スレッドで動く）。WARP 可（`GraphicsCore::new()`）。
///
/// ポインタ配線（`BalloonWiring`）も本番同様に置く。不在でも「抑止なし」へ倒れるだけだが、
/// その縮退の誤りログが相順の観測窓へ毎回混ざるため、観測を濁さないために揃えておく。
fn gpu_frame_world() -> (World, crate::placement::spawn::GhostWindows) {
    // SAFETY: WIC デコード／D3D に要る COM の MTA 初期化
    // （既初期化の S_FALSE／RPC_E_CHANGED_MODE は無視——テストスレッド毎）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let (mut world, gw) = dpi_world();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
    world.insert_resource(core);
    world.insert_resource(wuc);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    (world, gw)
}

/// 実 emo2 資産で結線資源を組む（本番 `wire_emo2_boot` 手順 6 と同じ構築点）。
fn boot_wiring(
    rx: Receiver<PresentCommand>,
    lifecycle_rx: Receiver<TalkLifecycleSignal>,
) -> Emo2Wiring {
    let assets = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");
    Emo2Wiring::new(
        EmoPresenter::new(),
        rx,
        mpsc::channel::<MoveDirective>().1,
        lifecycle_rx,
        mpsc::channel::<crate::emo2_boot::zorder_cue::ZOrderDirective>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        zero_clock(),
        assets,
    )
}

/// 本番の相順を 1 フレーム分回す（`emo2_frame_system` の donor 手順をそのまま通す）。
///
/// 相を個別に呼ばないのが要点である——相順の性質を見たい檻が相を自分で並べたら、檻は自分の
/// 並べ方を検査するだけになる。
fn advance_frame(world: &mut World, wiring: Emo2Wiring) -> Emo2Wiring {
    world.insert_non_send(wiring);
    emo2_frame_system(world);
    world
        .remove_non_send::<Emo2Wiring>()
        .expect("emo2_frame_system は取り出した結線資源を必ず戻す")
}

/// `[balloon-visibility]` を名乗る行だけを取り出す（他所のログは相順の関心事ではない）。
fn visibility_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains("[balloon-visibility]"))
        .collect()
}

/// 本制御が発行していない可視性変化として記録された行だけを取り出す。
fn explicit_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
    visibility_lines(events)
        .into_iter()
        .filter(|e| e.field("trigger") == Some("\"explicit\""))
        .collect()
}

/// 当該 scope へ可視グリフを流し込む（`at=0.0` 起点・注入時刻が進むほど文字がリビールされる）。
fn reveal_text(wiring: &Emo2Wiring, scope: u32, text: &str) {
    wiring.runtime.borrow_mut().apply_cue(&TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope.to_string()),
        command: CueCommand::Text(text.into()),
        duration: text.chars().count() as f64 * 0.25,
    });
}

/// 窓 entity の `DPI` を現在値と必ず異なる実機水準（96／192）へ差し替え、その新 DPI を返す。
///
/// 固定値を書き込むと「たまたま同値＝拡大率不変」で檻が空虚化するため、現在値を読んでから
/// 別の値を選ぶ（`spine_test_support.rs:80-83` の `bump_window_dpi` と同流儀）。
fn bump_window_dpi(world: &mut World, window: Entity) -> u16 {
    let current = world.get::<DPI>(window).map(|d| d.dpi_x);
    let next = if current == Some(192) { 96 } else { 192 };
    world.entity_mut(window).insert(DPI::from_dpi(next, next));
    next
}

/// 装着済みバルーンの「確立はしているが不可視」を 1 scope 分主張する。
///
/// 不可視だけを見る主張は「そもそも何も確立していない」場合にも真になる（`target_visible` は
/// `attach_target` 直後から `Some(false)` を返す）。確立の証跡——面 0 の記録・文字の配置先・
/// 文字スロット entity の実値——を先に要求してから不可視を主張することで、確立が起きていない
/// 縮退では不可視の主張へ到達する前に落ちる。
fn assert_established_but_invisible(world: &World, wiring: &Emo2Wiring, scope: u32, at: &str) {
    let target = balloon_target(scope);
    assert_eq!(
        wiring.presenter.current_surface_id(target),
        Some(0),
        "{at} scope{scope}: 面 0 が確立している（撤去ではなく不可視のままの確立）"
    );
    let view = wiring
        .presenter
        .text_slot_view(target)
        .unwrap_or_else(|| panic!("{at} scope{scope}: 文字の配置先が確立している"));
    assert_eq!(
        wiring.presenter.target_visible(target),
        Some(false),
        "{at} scope{scope}: バルーンは不可視のまま（Requirement 1.1／1.2・全 scope 適用の 1.6）"
    );
    assert_eq!(
        world.get::<Visual>(view.slot()).map(|v| v.is_visible),
        Some(false),
        "{at} scope{scope}: 文字スロット entity の可視状態も偽のまま（presenter の帳簿ではなく実値）"
    );
}

// ---------------------------------------------------------------------------
// ⑴ 判断は「本フレームの表示指令を適用し終えた実状態」で行われる（Requirement 3.5）
// ---------------------------------------------------------------------------

/// 可視性の相は drain の**後**に置かれている——本フレームに届いた `\b[-1]` 相当の非表示を
/// 適用し終えた実状態を見て判断する（design 決定 D5・Requirement 3.5）。
///
/// # 観測の仕組み
///
/// 配線層は「前フレームの記憶」と「本フレームの観測」の差分を `trigger=explicit` として
/// 記録する。ゆえに「前フレームは可視・本フレームの drain が非表示を適用」という状況を作れば、
/// 相順が正しいときだけ当該フレームに 1 行が現れる。相を drain の前へ移すと、判断が見るのは
/// 1 フレーム古い可視状態（＝まだ可視）となり、差分が立たず 1 行も出ない。
///
/// 可視の実状態は表示層へ直接発行して作る。ここで見たいのはコンテンツ判定ではなく
/// **相順が何を観測するか**であり、可視化の契機まで本檻に持ち込むと観測点が二重になる。
#[test]
fn visibility_phase_judges_the_state_left_by_this_frames_display_commands() {
    let (mut world, _gw) = gpu_frame_world();
    let (present_tx, present_rx) = mpsc::channel::<PresentCommand>();
    // 送出端は檻が保持し続ける（drop すると受信端が切断され、相が誤りレベルで 1 行鳴らす）。
    let (_lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
    let mut wiring = boot_wiring(present_rx, lifecycle_rx);
    let target = balloon_target(0);

    // フレーム 1: 装着（不可視のままの確立）。
    wiring = advance_frame(&mut world, wiring);
    assert_established_but_invisible(&world, &wiring, 0, "フレーム 1 終了時点:");

    // 実状態を可視にする（本番でこれを発行するのは可視性の相だが、本檻の対象は相順ゆえ直接発行する）。
    wiring
        .presenter
        .show_target(&mut world, target)
        .expect("確立済みバルーンの可視化は成功する");
    assert_eq!(
        wiring.presenter.target_visible(target),
        Some(true),
        "前提: 表示層の実状態が可視になっている"
    );

    // フレーム 2: 外から可視になったことを相が検出し、以後の比較相手が「可視」に揃う。
    let (next, events) = capture_logs(|| advance_frame(&mut world, wiring));
    wiring = next;
    let explicit = explicit_lines(&events);
    assert_eq!(
        explicit.len(),
        1,
        "外因の可視化が 1 行で記録される（次フレームの比較相手が可視に揃った証跡）: {events:?}"
    );
    assert_eq!(explicit[0].expect_field("scope"), "0");
    assert_eq!(explicit[0].expect_field("visible"), "true");

    // フレーム 3: 本フレームの drain が `\b[-1]` 相当の非表示を適用する。
    present_tx
        .send(PresentCommand::Hide {
            target,
            reply: None,
        })
        .expect("受信端は結線資源が保持している");
    let (next, events) = capture_logs(|| advance_frame(&mut world, wiring));
    wiring = next;

    let explicit = explicit_lines(&events);
    assert_eq!(
        explicit.len(),
        1,
        "指令適用後の実状態で判断していれば、同一フレームで非表示への変化を検出する（相を drain の前へ移すと 0 件になる）: {events:?}"
    );
    assert_eq!(explicit[0].expect_field("scope"), "0");
    assert_eq!(explicit[0].expect_field("visible"), "false");
    assert_eq!(
        wiring.presenter.target_visible(target),
        Some(false),
        "フレーム終了時点の実状態も不可視"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 表示へ遷移したフレームで窓寸が同一フレームに届く（Requirement 6.6）
// ---------------------------------------------------------------------------

/// 可視性の相は窓寸の反映より**前**に置かれている——表示へ遷移したフレームで、その表示が
/// 積んだ窓寸要求が同一フレームのうちに窓へ届く（design 決定 D5・Requirement 6.6）。
///
/// # 非空虚性の作り方（不可視の間に溜めた拡大率の差）
///
/// 不可視のバルーンは dpi 相で拡大率を拾わない（`refresh_scale` の可視ゲート・
/// `areka-emo-present/src/presenter/refresh.rs:74-80`）。ゆえに不可視のまま表示倍率を変えると
/// 差は保留され、可視化の際に `show_target` が単一漏斗を再通過して**現在の**倍率から拡大率を
/// 導出し直したときに初めて物理寸が跳ぶ。この跳びが同一フレームで窓へ届くかどうかが、
/// 相が窓寸の反映より前に居るかどうかそのものである。相を反映の後ろへ移すと、窓寸は当該
/// フレームでは旧寸のまま据え置かれて落ちる。
#[test]
fn show_transition_lands_the_new_window_size_within_the_same_frame() {
    let (mut world, gw) = gpu_frame_world();
    let (_present_tx, present_rx) = mpsc::channel::<PresentCommand>();
    let (_lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
    let mut wiring = boot_wiring(present_rx, lifecycle_rx);
    let target = balloon_target(0);
    let window = gw.balloon_window(0).expect("balloon 窓がある");

    // フレーム 1: 装着（不可視のままの確立）。確立が積んだ初回の窓寸要求も同一フレームで届く。
    wiring = advance_frame(&mut world, wiring);
    let established = wiring
        .presenter
        .target_physical_size(target)
        .expect("確立済みなら物理寸が引ける");
    assert_eq!(
        size_of(&world, window),
        Some(SizeI::new(established.0 as i32, established.1 as i32)),
        "前提: 確立が積んだ窓寸要求は装着フレームのうちに窓へ届いている"
    );

    // 不可視のまま表示倍率が変わる。
    let bumped_dpi = bump_window_dpi(&mut world, window);

    // フレーム 2: 不可視のバルーンは dpi 相で拡大率を拾わない＝窓寸も動かない。
    wiring = advance_frame(&mut world, wiring);
    assert_eq!(
        wiring.presenter.target_visible(target),
        Some(false),
        "可視コンテンツがまだ無いので不可視のまま"
    );
    assert_eq!(
        size_of(&world, window),
        Some(SizeI::new(established.0 as i32, established.1 as i32)),
        "不可視の間は倍率の変化が保留される（この保留が次の主張を非空虚にする）"
    );

    // 可視コンテンツを置く（表示の唯一の契機）。時刻は注入のみ。
    wiring.clock.observe_cue(0.0);
    world.insert_resource(FrameTime(1.0));
    reveal_text(&wiring, 0, "あい");

    // フレーム 3: 表示へ遷移し、その表示が積んだ窓寸要求が同一フレームで窓へ届く。
    wiring = advance_frame(&mut world, wiring);
    assert_eq!(
        wiring.presenter.target_visible(target),
        Some(true),
        "可視コンテンツの配置で表示へ遷移する"
    );
    let shown = wiring
        .presenter
        .target_physical_size(target)
        .expect("表示成立後も物理寸が引ける");
    assert_ne!(
        shown, established,
        "前提: 表示倍率 {bumped_dpi} では物理寸が実際に変わる（同値なら本檻は何も弁別しない）"
    );
    assert_eq!(
        size_of(&world, window),
        Some(SizeI::new(shown.0 as i32, shown.1 as i32)),
        "表示へ遷移したフレームで窓寸が同一フレームに届く（相を窓寸の反映の後ろへ移すと旧寸のまま落ちる）"
    );
}

// ---------------------------------------------------------------------------
// ⑶ 起動から最初の可視コンテンツまで全フレームで不可視（Requirements 1.1 / 1.2 / 1.6）
// ---------------------------------------------------------------------------

/// 起動から最初の可視コンテンツまで、**全フレーム**でバルーンが可視にならない。
///
/// 既存の 2 本（`frame_attach_tests.rs::attach_establishes_balloons_invisible_with_slot_and_surface`
/// と `spine_display_tests.rs::spine_boot_leaves_all_balloons_invisible_with_established_slot_and_surface`）
/// はいずれも**装着時点**の 1 点だけを見る。本番は毎フレーム 9 つの相が回るため、装着後の
/// どこかの相が可視化してしまう欠陥は 1 点観測では捕まらない——ここが全フレームの掃引を担う。
///
/// 時刻は最初のフレームから注入しておく（`FrameTime`＋epoch）。時刻が定まらない間だけ不可視、
/// という当たり判定の緩い緑を避けるためである。
///
/// # 非空虚性
///
/// 掃引の後に可視コンテンツを置き、当該 scope が実際に可視へ遷移することを主張する。
/// 「そもそも何をしても可視にならない」実装では、この最後の主張が落ちる。
#[test]
fn every_frame_from_boot_to_the_first_visible_content_keeps_balloons_invisible() {
    let (mut world, _gw) = gpu_frame_world();
    let (_present_tx, present_rx) = mpsc::channel::<PresentCommand>();
    let (_lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
    let mut wiring = boot_wiring(present_rx, lifecycle_rx);

    // 起点から時刻を注入しておく（可視グリフ数が観測できるフレームでも不可視であることを見る）。
    wiring.clock.observe_cue(0.0);
    world.insert_resource(FrameTime(0.5));

    for frame in 1..=4u32 {
        wiring = advance_frame(&mut world, wiring);
        let scopes = wiring.balloon_model_scopes();
        assert_eq!(
            scopes,
            vec![0u32, 1],
            "前提: 両 scope の balloon 装着が成立している（片方でも欠けると掃引が空虚になる）"
        );
        for scope in scopes {
            assert_established_but_invisible(
                &world,
                &wiring,
                scope,
                &format!("フレーム {frame} 終了時点:"),
            );
        }
    }

    // 非空虚性: 可視コンテンツを置けば当該 scope **だけ**が可視になる。
    reveal_text(&wiring, 0, "あい");
    world.insert_resource(FrameTime(1.0));
    wiring = advance_frame(&mut world, wiring);
    assert_eq!(
        wiring.presenter.target_visible(balloon_target(0)),
        Some(true),
        "可視コンテンツを置いた scope は表示へ遷移する（掃引が「何も可視にならない」ゆえの緑ではない）"
    );
    assert_eq!(
        wiring.presenter.target_visible(balloon_target(1)),
        Some(false),
        "喋っていない scope は表示しない（Requirement 2.2）"
    );
}

/// 文字の配置先を接続した後も、文字スロットの可視状態は偽のままである。
///
/// 文字層は最初の提示フレームで配置先 entity へ供給面を装着し、`VisualGraphics` と
/// `Arrangement` を**挿入**する（`areka-emo-text/src/surface.rs:249-262`）。`Visual` の既定は
/// 可視（`wintf/src/ecs/graphics/visual.rs:26` の構造体・既定値は `is_visible: true`）なので、
/// この挿入がもし `Visual` まで含めば、装着で不可視に作った文字スロットが可視既定へ戻される。
/// 枠だけが不可視で文字が画面に残る状態そのものであり、Requirement 1.7 が禁じている形である。
///
/// # 可視コンテンツを生まない cue で観測する
///
/// 可視グリフを生む cue を使うと同一フレームの可視性の相がバルーンを可視化してしまい、
/// 「接続後も不可視」を観測できない。改行だけを流すと、文字層は配置先へ供給面を装着する一方で
/// 可視グリフ数は 0 のままとなり、装着と不可視が同時に成り立つフレームが作れる。
#[test]
fn text_slot_stays_invisible_after_the_text_layer_attaches_its_surface() {
    let (mut world, _gw) = gpu_frame_world();
    let (_present_tx, present_rx) = mpsc::channel::<PresentCommand>();
    let (_lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
    let mut wiring = boot_wiring(present_rx, lifecycle_rx);
    let target = balloon_target(1);

    // フレーム 1: 装着（不可視のままの確立）。
    wiring = advance_frame(&mut world, wiring);
    let slot = wiring
        .presenter
        .text_slot_view(target)
        .expect("前提: 文字の配置先が確立している")
        .slot();
    assert_eq!(
        world.get::<Arrangement>(slot).map(|a| a.size.width),
        Some(0.0),
        "前提: 文字層が触る前の配置先は寸ゼロ（この後の寸が装着の証跡になる）"
    );

    // 可視グリフを生まない cue（改行）だけを置く。
    wiring.runtime.borrow_mut().apply_cue(&TalkCue {
        at: 0.0,
        actor: ActorKey::from("1"),
        command: CueCommand::NewLine { ratio: 1.0 },
        duration: 0.0,
    });
    wiring.clock.observe_cue(0.0);
    world.insert_resource(FrameTime(1.0));

    // フレーム 2: 文字層が配置先へ供給面を装着する。
    wiring = advance_frame(&mut world, wiring);

    assert!(
        world
            .get::<Arrangement>(slot)
            .is_some_and(|a| a.size.width > 0.0 && a.size.height > 0.0),
        "前提: 文字層が配置先へ供給面を装着した（装着が起きていなければ以下の主張は空虚）"
    );
    assert_eq!(
        world.get::<Visual>(slot).map(|v| v.is_visible),
        Some(false),
        "文字の配置先を接続した後も文字スロットは不可視のまま（外部からの挿入で可視既定へ戻されない）"
    );
    assert_eq!(
        wiring.presenter.target_visible(target),
        Some(false),
        "改行だけでは可視コンテンツにならず、バルーンも不可視のまま（Requirement 2.3）"
    );
}

// ---------------------------------------------------------------------------
// ⑸ 再表示がグループ機構の追随トリガになる（areka-P0-scope-zorder-pinning 要件 7.3）
// ---------------------------------------------------------------------------

/// バルーンが実際に表示へ遷移したフレームで、重なりのグループ機構へ「是正が要るかもしれない」
/// の印が立つ（areka-P0-scope-zorder-pinning 要件 7.3・task 5.2 → 6.1 の引受け）。
///
/// # なぜここに置くのか
///
/// トリガの実装点（`balloon_visibility_phase.rs` の `note_balloon_shown`）を持つ spec の
/// 決定論テストは、**表示が実際に成立した巡を本番経路で通せない**——headless の表示層は
/// 「表示が一度も確立していない」ため `show_target` が必ず失敗し、`shown` を真にできない
/// （`balloon_visibility_phase_tests.rs` の同旨の注記）。あちらは純判断・World の状態・
/// 結線の字面で受け入れており、**肯定側だけが本番経路で未観測**のまま残っていた。
///
/// 本ハーネスは実 GPU ＋実 emo2 fixture で本番の `emo2_frame_system` をそのまま回し、
/// 実際に `target_visible == Some(true)` を成立させられる唯一の場所である（⑵と同じ助走）。
/// よってここが肯定側の引受先になる。
///
/// # 両側から挟む
///
/// 表示が成立していないフレーム（装着だけの 1・可視コンテンツの無い 2）では印が立たない
/// ことを先に要求する。肯定側だけを見ると「印が最初から立っている」「毎フレーム立てて
/// いる」形が素通りする。
///
/// # 本ハーネスで印を動かせるのは再表示のトリガだけである
///
/// 維持系（印を降ろす⑤）は `emo2_frame_system` に結線されていない。指令の取り出しの相は
/// task 6.2 で結線されたが、**この檻では印を 1 度も動かさない**——台帳に据えた宣言と受け口へ
/// 置いた射影を最初から一致させてあるので、相は毎フレーム「何も動いていない」と判断して
/// 書込も印立ても行わない（要件 6.1）。よって観測された印の上がりは、他の経路の混入では
/// なく再表示のトリガそのものである。
///
/// 台帳を空のまま受け口だけへ宣言を植えると、相が受け口を**台帳の内容で上書きする**
/// （正本は台帳ひとつ・design「Data Models」）。それは本番では起こり得ない食い違いであり、
/// 檻の側が二重帳簿を作っていることを意味する。ゆえに宣言は台帳から作る。
#[test]
fn a_show_transition_raises_the_zorder_group_mark_on_the_production_frame_path() {
    let (mut world, gw) = gpu_frame_world();
    let (_present_tx, present_rx) = mpsc::channel::<PresentCommand>();
    let (_lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
    let mut wiring = boot_wiring(present_rx, lifecycle_rx);
    let target = balloon_target(0);

    // グループを 1 本だけ載せる（トリガは「宣言が 1 本でもあるか」を見る）。正本は台帳なので
    // 台帳へ据え、受け口にはその射影と**同じ内容**を置く——本番で取り出しの相が到達する
    // 定常状態そのものであり、相はこの状態を毎フレーム素通りする（書込も印立ても無い）。
    let (members, _) = crate::placement::zorder_group_ledger::parse_zorder_tokens(&["1", "0"])
        .expect("2 スコープの数値モードは受理される");
    let group_id = wiring
        .zorder_ledger
        .try_add_tag_group(members)
        .expect("空の台帳への最初の追加は受理される");
    let mut groups = ZOrderGroups::default();
    groups.groups.push(ZOrderGroupSpec {
        id: group_id,
        members: vec![
            gw.balloon_window(1).expect("scope 1 の balloon 窓がある"),
            gw.char_window(1).expect("scope 1 の char 窓がある"),
            gw.balloon_window(0).expect("scope 0 の balloon 窓がある"),
            gw.char_window(0).expect("scope 0 の char 窓がある"),
        ],
    });
    world.insert_resource(groups);

    // フレーム 1: 装着（不可視のままの確立）。
    wiring = advance_frame(&mut world, wiring);
    assert_eq!(
        wiring.presenter.target_visible(target),
        Some(false),
        "前提: 装着だけのフレームは不可視"
    );
    assert!(
        !world.resource::<ZOrderGroups>().pending,
        "表示へ遷移していないフレームで印が立っている（毎フレーム立てる形は要件 6.4 を崩す）"
    );

    // フレーム 2: 可視コンテンツがまだ無い。
    wiring = advance_frame(&mut world, wiring);
    assert!(
        !world.resource::<ZOrderGroups>().pending,
        "可視コンテンツの無いフレームで印が立っている"
    );

    // 可視コンテンツを置く（表示の唯一の契機）。時刻は注入のみ。
    wiring.clock.observe_cue(0.0);
    world.insert_resource(FrameTime(1.0));
    reveal_text(&wiring, 0, "あい");

    // フレーム 3: 表示へ遷移する。
    wiring = advance_frame(&mut world, wiring);
    assert_eq!(
        wiring.presenter.target_visible(target),
        Some(true),
        "前提: 可視コンテンツの配置で表示へ遷移する（遷移していなければ以下の主張は空虚）"
    );
    assert!(
        world.resource::<ZOrderGroups>().pending,
        "表示へ遷移したフレームでグループ機構の印が立っていない（要件 7.3 の肯定側）"
    );
}
