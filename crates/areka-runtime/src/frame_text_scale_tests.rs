use areka_placement::dpi_sync::DpiSyncHold;
use areka_placement::follow::{MonitorDpiEntry, MonitorDpiTable};
use areka_placement::resolver::RectPx;
use bevy_ecs::prelude::With;
use std::sync::mpsc;
use wintf::ecs::DPI;

use super::shared_test_support::wiring_with;
use super::test_support::{
    assert_no_write, capture_logs, count_level, dpi_world, gpu_attach_world, headless_wiring_with,
    zero_clock,
};
use super::*;

// ── 文字層 k 追従フェーズ（run_text_scale_phase） ──
//
// 本番関数をそのまま駆動する。`Some(view)` の適用は GPU 装着を要するため spine（in-crate GPU
// ハーネス）が受け持ち、ここでは GPU 不要で見える 2 点——(a) 走査対象が balloon 装着 scope に
// 限られること、(b) 表示未確立の縮退が **scope ごとに一度だけ** 鳴ること——を固定する。

/// 素の結線資源（GPU/資産不要・`run_text_scale_phase` の headless 駆動用）。
fn headless_wiring() -> RuntimeWiring {
    headless_wiring_with(mpsc::channel::<PresentCommand>().1, zero_clock())
}

/// 捕捉行のうち `level` かつ本文に `needle` を含む件数（他フェーズの警告と混ざらないよう絞る）。
fn count_level_containing(logs: &[String], level: &str, needle: &str) -> usize {
    let lv = format!("level={level}");
    logs.iter()
        .filter(|l| l.contains(&lv) && l.contains(needle))
        .count()
}

/// balloon 未装着（`balloon_models` 空）では走査対象がゼロ＝完全 no-op（警告も出さない）。
///
/// 「毎フレーム走査」が attach 前のフレームで鳴き続けないこと（起動直後の log 汚染の禁止）と、
/// shell しか無い状況で文字層へ触れないことを同時に固定する。
#[test]
fn text_scale_phase_without_balloon_models_is_silent_noop() {
    let mut wiring = headless_wiring();

    let logs = capture_logs(|| {
        assert!(
            run_text_scale_phase(&mut wiring, &mut World::new()).is_empty(),
            "balloon 未装着では再構築 scope なし"
        );
    });

    assert_eq!(
        count_level(&logs, "WARN"),
        0,
        "attach 前は何も鳴らさない: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "attach 前は何も鳴らさない: {logs:?}"
    );
}

/// 表示未確立の縮退: `text_slot_view` が `None` の scope は再追従せず skip し、**警告は scope
/// ごとに一度だけ**鳴る（毎フレーム走査ゆえ素朴な `warn!` は毎フレーム鳴ってしまう）。
#[test]
fn text_scale_phase_warns_once_per_scope_when_view_unavailable() {
    let mut wiring = headless_wiring();
    wiring
        .balloon_models
        .insert(0, areka_parsers::balloon::parse_str("", None));
    wiring
        .balloon_models
        .insert(1, areka_parsers::balloon::parse_str("", None));

    let first = capture_logs(|| {
        assert!(
            run_text_scale_phase(&mut wiring, &mut World::new()).is_empty(),
            "view None では再構築しない（縮退 skip・R8.6）"
        );
    });
    assert_eq!(
        count_level(&first, "WARN"),
        2,
        "初回は縮退した scope ごとに 1 回ずつ鳴る（0 と 1 の 2 件・R8.6 の観測可能性）: {first:?}"
    );

    // 2・3 フレーム目: 状態が変わっていない以上、同じ警告を鳴らし直さない（log spam の禁止）。
    let rest = capture_logs(|| {
        run_text_scale_phase(&mut wiring, &mut World::new());
        run_text_scale_phase(&mut wiring, &mut World::new());
    });
    assert_eq!(
        count_level(&rest, "WARN"),
        0,
        "同一状態が続く間は再度鳴らさない（エッジガード）: {rest:?}"
    );
    assert_eq!(
        count_level(&rest, "ERROR"),
        0,
        "縮退は失敗ではない: {rest:?}"
    );

    // 借用/poison を残さない（None 経路は runtime に触れない）。
    assert!(
        wiring.runtime.try_borrow_mut().is_ok(),
        "runtime を汚さない"
    );
}

/// 装着で文字の配置先が無く繋げなかったスコープは、配置先が取れた最初のフレームの文字の拡大率の相で
/// 繋がる（遅らせない）。装着の相を配置先の無い状態で通すことは決定論で作れないため、実 GPU の装着の後に
/// 文字層を新しくして控えを入れ、「装着で繋げなかった後」の状態を組む。
#[test]
fn a_balloon_text_left_unconnected_at_attach_connects_on_the_first_frame_its_view_exists() {
    use crate::assets::build_boot_assets;
    use crate::sample_test_support::{emo2_balloon_root, emo2_root};

    use areka_emo_text::state::TextLayerConfig;

    let (_gpu, mut world, _gw) = gpu_attach_world();
    let assets = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
        .expect("emo2 fixture の BootAssets 組立は成功する");
    let background = assets.balloons[1].background_color;
    let fresh_runtime = || {
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )))
    };
    let mut wiring = wiring_with(assets, fresh_runtime(), EmoPresenter::new());
    run_attach_phase(&mut wiring, &mut world);
    assert!(wiring.attached, "前提: 装着が走る");

    wiring.runtime = fresh_runtime();
    wiring.unconnected_text.insert(1, background);

    let logs = capture_logs(|| {
        run_text_scale_phase(&mut wiring, &mut world);
    });

    let actor = ActorKey::from("1");
    assert_eq!(
        (
            wiring
                .runtime
                .borrow()
                .state()
                .actors()
                .any(|(key, _)| *key == actor),
            wiring.unconnected_text.is_empty(),
            count_level_containing(&logs, "INFO", "装着で繋げなかった文字層"),
        ),
        (true, true, 1),
        "(scope 1 の actor が登録された, 控えが空になった, 繋ぎ直しの info の行数): {logs:?}"
    );
}

/// **排他 system への組み込み**（call-site の檻）: [`frame_system`] は毎フレーム
/// [`run_text_scale_phase`] を駆動する。
///
/// 関数が正しくても system から呼ばれていなければ本番では何も起きない。未装着 presenter ＋
/// attach 相と同形の記憶済み [`BalloonModel`] を持つ World で 1 フレーム回すと縮退警告が
/// ちょうど 1 回鳴り、2 フレーム目は鳴らない（＝呼ばれている、かつエッジガードが system 越しに
/// 効いている）。
#[test]
fn frame_system_drives_text_scale_phase_every_frame() {
    let (mut world, _gw) = dpi_world();
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    let mut wiring = headless_wiring_with(rx, zero_clock());
    wiring
        .balloon_models
        .insert(0, areka_parsers::balloon::parse_str("", None));
    world.insert_non_send(wiring);

    let first = capture_logs(|| frame_system(&mut world));
    assert_eq!(
        count_level_containing(&first, "WARN", "text-scale"),
        1,
        "1 フレーム目で文字層 k 追従フェーズが駆動され縮退が 1 回鳴る（system 組み込みの証跡）: {first:?}"
    );

    let second = capture_logs(|| frame_system(&mut world));
    assert_eq!(
        count_level_containing(&second, "WARN", "text-scale"),
        0,
        "2 フレーム目は同一状態ゆえ鳴らない（エッジガードが system 越しに効く）: {second:?}"
    );
    assert!(
        world.get_non_send::<RuntimeWiring>().is_some(),
        "wiring は remove→insert で必ず戻る"
    );
}

/// **本番経路**での `SystemState` 永続性: `run_dpi_phase` は
/// 観測器を `RuntimeWiring.dpi_state` へ**保持**し、run を跨いで `last_run` を進める。
///
/// `dpi_phase_with` へテスト側の state を渡す檻では「本番が wiring のフィールドを使っている」
/// ことを一切見ないため、ここでは `run_dpi_phase(&mut wiring, ..)` だけを叩き、その**副作用**
/// （`wiring.dpi_state` の生成と、2 回目の run が拾う窓）を観測する。
///
/// 非空虚性の核: 毎 run 作り直す実装は 2 回目も全窓（4 窓）へマッチする churn になる。2 回目の前に
/// 1 窓だけ DPI を変え、整合待ちの札が付いた窓がその 1 窓だけであることで永続性を弁別する。
#[test]
fn run_dpi_phase_persists_system_state_across_frames_in_production_path() {
    let (mut world, gw) = dpi_world();
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    let mut wiring = headless_wiring_with(rx, zero_clock());
    assert!(
        wiring.dpi_state.is_none(),
        "前提: 観測器は初回 run まで未生成（SystemState::new は &mut World を要する）"
    );

    run_dpi_phase(&mut wiring, &mut world);
    assert!(
        wiring.dpi_state.is_some(),
        "run_dpi_phase は観測器を wiring へ保持しなければならない（毎 run 作り直せば churn）"
    );

    // 2 フレーム目（本番経路）: 1 窓だけ DPI を変え、モニタ表を窓の拡大率と食い違う値で置く。
    // 本番の観測器が拾った窓だけが整合ゲートで待ち札を付けられるので、札の付いた窓が拾われた窓である。
    let changed = gw.char_window(1).expect("char 窓がある");
    world.insert_resource(MonitorDpiTable {
        entries: vec![MonitorDpiEntry {
            bounds: RectPx {
                left: -100_000,
                top: -100_000,
                right: 100_000,
                bottom: 100_000,
            },
            dpi: 120,
        }],
    });
    world.entity_mut(changed).insert(DPI::from_dpi(144, 144));
    run_dpi_phase(&mut wiring, &mut world);

    let held: Vec<Entity> = world
        .query_filtered::<Entity, With<DpiSyncHold>>()
        .iter(&world)
        .collect();
    assert_eq!(
        held,
        vec![changed],
        "永続観測器は実際に DPI が変わった 1 窓だけを拾う（作り直せば全窓を拾う）"
    );
}

/// 結線の疎通（run_dpi_phase／frame_system）: 実 `EmoPresenter`（未装着）と `Changed<DPI>`
/// のある World で `frame_system` を回しても、報告源が何も返さないため窓書込はゼロで
/// panic しない。フェーズが排他 system へ組み込まれていること自体を固定する。
#[test]
fn frame_system_runs_dpi_phase_without_writes_when_unattached() {
    let (mut world, gw) = dpi_world();
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    world.insert_non_send(headless_wiring_with(rx, zero_clock()));

    world
        .entity_mut(gw.char_window(0).unwrap())
        .insert(DPI::from_dpi(192, 192));
    frame_system(&mut world);
    frame_system(&mut world);

    assert!(
        world.get_non_send::<RuntimeWiring>().is_some(),
        "wiring は remove→insert で必ず戻る"
    );
    for scope in [0usize, 1] {
        assert_no_write(&world, gw.char_window(scope).unwrap(), "未装着 presenter");
        assert_no_write(
            &world,
            gw.balloon_window(scope).unwrap(),
            "未装着 presenter",
        );
    }
}

// ── 発話が進行中かの純判断（reveal_pending） ──
//
// 「次の画面更新でも文字が増えるか」だけを決める引き算である。時計にも表示層にも触れない。
// どこで呼ぶか（text 相の末尾）は字面検査が受け持つ。

use dola::TimeSpan;
use dola::time::testing::ms;

use super::scale_text::reveal_pending;

/// 描く相手が 1 人も居なければ進行中ではない。
#[test]
fn no_actor_is_not_in_progress() {
    let none: [&[TimeSpan]; 0] = [];
    assert!(!reveal_pending(none.iter().copied(), ms(0)));
}

/// 文字が現れ切っていれば進行中ではない——時刻起点が残っていても予約しない。
#[test]
fn fully_revealed_text_is_not_in_progress() {
    let times: [&[TimeSpan]; 1] = [&[ms(0), ms(100), ms(200)]];
    assert!(
        !reveal_pending(times.iter().copied(), ms(300)),
        "最後の文字より後の時刻では進行中ではない"
    );
    assert!(
        !reveal_pending(times.iter().copied(), ms(200)),
        "ちょうど最後の文字が現れる時刻は「現れ済み」側（可視数の数え方と同じ向き）"
    );
}

/// まだ現れていない文字が 1 つでもあれば進行中である。
#[test]
fn pending_glyph_makes_it_in_progress() {
    let times: [&[TimeSpan]; 1] = [&[ms(0), ms(100), ms(200)]];
    assert!(reveal_pending(times.iter().copied(), ms(190)));
    assert!(reveal_pending(times.iter().copied(), ms(0)));
}

/// 相手が複数居るときは 1 人でも進行中なら進行中（他が現れ切っていても止めない）。
#[test]
fn any_actor_still_revealing_counts() {
    let times: [&[TimeSpan]; 2] = [&[ms(0), ms(100)], &[ms(0), ms(5000)]];
    assert!(
        reveal_pending(times.iter().copied(), ms(1000)),
        "片方が現れ切っていても、もう片方が残っていれば進行中"
    );
    assert!(
        !reveal_pending(times.iter().copied(), ms(5000)),
        "全員が現れ切ったら止まる"
    );
}

/// 時刻列が空の相手（cue 未着・`Clear` 直後）は判断に加わらない。
#[test]
fn empty_schedule_is_ignored() {
    let times: [&[TimeSpan]; 2] = [&[], &[]];
    assert!(!reveal_pending(times.iter().copied(), ms(0)));
}
