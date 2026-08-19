use std::sync::mpsc;
use wintf::ecs::DPI;

use super::*;
use super::test_support::{
    assert_no_write,
    capture_logs,
    count_level,
    dpi_world,
    headless_wiring_with,
    zero_clock,
};

// ── task 7.2: 文字層 k 追従フェーズ（run_text_scale_phase・D11-3/D11-4・R8.1/8.5/8.6） ──
//
// 本番関数をそのまま駆動する（シームを噛ませない）。`Some(view)` の適用そのもの（binding
// 再構築・供給面破棄・churn ガード）は GPU 装着を要するため spine（in-crate GPU ハーネス）が
// 実経路で檻に入れる。ここでは GPU 不要で観測できる 2 点——(a) 走査対象が balloon 装着 scope に
// 限られること、(b) 表示未確立の縮退が **scope ごとに一度だけ** 鳴ること——を固定する。

/// 素の結線資源（GPU/資産不要・`run_text_scale_phase` の headless 駆動用）。
fn headless_wiring() -> Emo2Wiring {
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
            run_text_scale_phase(&mut wiring).is_empty(),
            "balloon 未装着では再構築 scope なし"
        );
    });

    assert_eq!(count_level(&logs, "WARN"), 0, "attach 前は何も鳴らさない: {logs:?}");
    assert_eq!(count_level(&logs, "ERROR"), 0, "attach 前は何も鳴らさない: {logs:?}");
}

/// R8.6 縮退（log-first だが log spam にしない）: `text_slot_view` が `None`（表示未確立）の
/// scope は再追従せず skip し、**警告は scope ごとに一度だけ**鳴る（毎フレーム走査ゆえ素朴な
/// `warn!` は毎フレーム鳴ってしまう）。
///
/// 実源（何も装着していない `EmoPresenter`＝`text_slot_view` が常に `None`）へ、attach 相と同じ
/// 形で per-scope の [`BalloonModel`] を記憶させた状態を作り、3 フレーム相当を走らせる。
#[test]
fn text_scale_phase_warns_once_per_scope_when_view_unavailable() {
    let mut wiring = headless_wiring();
    // attach 相が記憶するのと同じ形（scope→model）。presenter は未装着ゆえ view は常に None。
    wiring
        .balloon_models
        .insert(0, areka_parsers::balloon::parse_str("", None));
    wiring
        .balloon_models
        .insert(1, areka_parsers::balloon::parse_str("", None));

    let first = capture_logs(|| {
        assert!(
            run_text_scale_phase(&mut wiring).is_empty(),
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
        run_text_scale_phase(&mut wiring);
        run_text_scale_phase(&mut wiring);
    });
    assert_eq!(
        count_level(&rest, "WARN"),
        0,
        "同一状態が続く間は再度鳴らさない（エッジガード）: {rest:?}"
    );
    assert_eq!(count_level(&rest, "ERROR"), 0, "縮退は失敗ではない: {rest:?}");

    // 借用/poison を残さない（None 経路は runtime に触れない）。
    assert!(wiring.runtime.try_borrow_mut().is_ok(), "runtime を汚さない");
}

/// **排他 system への組み込み**（call-site の檻）: [`emo2_frame_system`] は毎フレーム
/// [`run_text_scale_phase`] を駆動する。
///
/// 関数が正しくても system から呼ばれていなければ本番では何も起きない——その 1 行の欠落を
/// 検出する。未装着 presenter（`text_slot_view` が常に `None`）＋ attach 相と同形の記憶済み
/// [`BalloonModel`] を持つ World で 1 フレーム回すと R8.6 の縮退警告がちょうど 1 回鳴り、
/// 2 フレーム目は鳴らない（＝呼ばれている、かつエッジガードが system 越しに効いている）。
#[test]
fn emo2_frame_system_drives_text_scale_phase_every_frame() {
    let (mut world, _gw) = dpi_world();
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    let mut wiring = headless_wiring_with(rx, zero_clock());
    // attach 相が記憶するのと同じ形（scope→model）。GPU 資源が無いため attach 相自体は空回りする。
    wiring
        .balloon_models
        .insert(0, areka_parsers::balloon::parse_str("", None));
    world.insert_non_send_resource(wiring);

    let first = capture_logs(|| emo2_frame_system(&mut world));
    assert_eq!(
        count_level_containing(&first, "WARN", "text-scale"),
        1,
        "1 フレーム目で文字層 k 追従フェーズが駆動され縮退が 1 回鳴る（system 組み込みの証跡）: {first:?}"
    );

    let second = capture_logs(|| emo2_frame_system(&mut world));
    assert_eq!(
        count_level_containing(&second, "WARN", "text-scale"),
        0,
        "2 フレーム目は同一状態ゆえ鳴らない（エッジガードが system 越しに効く）: {second:?}"
    );
    assert!(
        world.get_non_send_resource::<Emo2Wiring>().is_some(),
        "wiring は remove→insert で必ず戻る"
    );
}

/// **本番経路**での `SystemState` 永続性（Flow 2 キー決定 (b)・churn 禁止）: `run_dpi_phase` は
/// 観測器を `Emo2Wiring.dpi_state` へ**保持**し、run を跨いで `last_run` を進める。
///
/// `dpi_phase_with` へテスト側の state を渡す檻では「本番が wiring のフィールドを使っている」
/// ことを一切見ないため、ここでは `run_dpi_phase(&mut wiring, ..)` だけを叩き、その**副作用**
/// （`wiring.dpi_state` の生成と `last_run` の前進）を private フィールド越しに観測する。
///
/// 非空虚性の核: 同一 World で**新規** `SystemState` を作ると全窓（4 窓）へマッチする——
/// すなわち「毎 run 作り直す実装」は毎フレーム全窓を refresh する churn になる。本番の
/// 永続観測器が 0 件であることと対にして、永続性が実際に効いていることを弁別する。
#[test]
fn run_dpi_phase_persists_system_state_across_frames_in_production_path() {
    let (mut world, gw) = dpi_world();
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    let mut wiring = headless_wiring_with(rx, zero_clock());
    assert!(
        wiring.dpi_state.is_none(),
        "前提: 観測器は初回 run まで未生成（SystemState::new は &mut World を要する）"
    );

    // 1 フレーム目（本番経路）: 初回 run の全窓マッチをここで消費する。
    run_dpi_phase(&mut wiring, &mut world);
    assert!(
        wiring.dpi_state.is_some(),
        "run_dpi_phase は観測器を wiring へ保持しなければならない（毎 run 作り直せば churn）"
    );

    // Bevy 0.19ではSystemState::get自体が観測ティックを進めるため、まず直前フェーズ内の
    // 変更を同期してから次回観測を行う。次フレーム相当のマッチは0件でなければならない。
    let dpi_state = wiring.dpi_state.as_mut().expect("生成済み");
    let _ = dpi_state
        .get(&world)
        .expect("DPI changed query validation should succeed")
        .iter()
        .count();
    let matched_after_first = dpi_state
        .get(&world)
        .expect("DPI changed query validation should succeed")
        .iter()
        .count();
    assert_eq!(
        matched_after_first, 0,
        "永続観測器は初回 run で Changed を消費済み＝以降はマッチしない"
    );

    // 非空虚性: 同一 World で新規 SystemState を作ると変更済み窓へマッチする（＝作り直し実装の churn）。
    let mut fresh: SystemState<DpiChangedQuery> = SystemState::new(&mut world);
    assert!(
        fresh
            .get(&world)
            .expect("DPI changed query validation should succeed")
            .iter()
            .count()
            > 0,
        "新規 SystemState は変更済み窓へマッチする（この差が永続性の効果そのもの）"
    );

    // 永続観測器は「変化しなくなった」のではなく、実変化はきちんと拾う（恒久的な盲目でない）。
    world
        .entity_mut(gw.char_window(1).expect("char 窓がある"))
        .insert(DPI::from_dpi(144, 144));
    let matched_after_change = wiring
        .dpi_state
        .as_mut()
        .expect("生成済み")
        .get(&world)
        .iter()
        .count();
    assert_eq!(
        matched_after_change, 1,
        "実際に DPI が変わった 1 窓だけを拾う（検知が生きている）"
    );
}

/// 結線の疎通（run_dpi_phase／emo2_frame_system）: 実 `EmoPresenter`（未装着）と `Changed<DPI>`
/// のある World で `emo2_frame_system` を回しても、報告源が何も返さないため窓書込はゼロで
/// panic しない。フェーズが排他 system へ組み込まれていること自体を固定する。
#[test]
fn emo2_frame_system_runs_dpi_phase_without_writes_when_unattached() {
    let (mut world, gw) = dpi_world();
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    world.insert_non_send_resource(headless_wiring_with(rx, zero_clock()));

    // DPI 差替（Changed 発火）→ 排他 system を 2 フレーム回す。
    world
        .entity_mut(gw.char_window(0).unwrap())
        .insert(DPI::from_dpi(192, 192));
    emo2_frame_system(&mut world);
    emo2_frame_system(&mut world);

    assert!(
        world.get_non_send_resource::<Emo2Wiring>().is_some(),
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
