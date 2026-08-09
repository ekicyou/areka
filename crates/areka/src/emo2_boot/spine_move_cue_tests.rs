use super::{
    run_move_drain_phase, Duration, Entity, GhostWindows, Instant, Point, SpineHarness,
    WindowHandle, WindowPos, World, HINSTANCE, HWND, SPIN_WAIT,
};

// ===========================================================================
// task 9.3 — move cue の決定論 spine e2e（cue→CueSheet→dispatch→broadcast→実 MoveCueSink→
// move channel→frame 相 drain→apply→実窓移動）。
//
// 9.1 が置いた throwaway `(_move_tx, move_rx)` を実 `MoveCueSink`（`SpineHarness::boot_with` の
// sinks 第 3 要素）へ差し替えた S-3 形（production `wire_emo2_boot` の 3-sink 構成）の上で、
// `\1\![move,...]` を含む OnBoot talk を実 sink 経路で流し、`MoveDirective` が move channel へ届き
// frame 相 drain（`run_move_drain_phase`＝task 9.2）で対象窓が fixture 検算位置へ即時移動することを
// 固定する。9.2 の frame 相配線が spine で end-to-end に生きていることの自動檻（headless・sleep 不使用・
// 注入 Tick のみ・手動実機確認は Task 11 に一本化）。
// ===========================================================================

/// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム・follow.rs/frame.rs の fake_handle 相当）。
fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// spine World の各キャラ／バルーン窓へ偽 WindowHandle を付与する（`enqueue_window_set_pos` が
/// WindowPos を書ける条件＝WindowHandle 実在。`spawn_ghost_windows` は実窓生成前ゆえ handle 未付与で、
/// これが無いと `move_window_to` は warn＋no-op に縮退し窓が動かない）。
fn attach_fake_window_handles(world: &mut World, gw: &GhostWindows) {
    let mut raw = 0x100usize;
    for scope in gw.scopes().collect::<Vec<_>>() {
        for e in [
            gw.char_window(scope).unwrap(),
            gw.balloon_window(scope).unwrap(),
        ] {
            world.entity_mut(e).insert(fake_handle(raw));
            raw += 0x10;
        }
    }
}

/// entity の WindowPos.position を読む（未設定は panic で検出）。
fn window_position(world: &World, e: Entity) -> Point {
    world
        .get::<WindowPos>(e)
        .expect("WindowPos があるはず")
        .position
        .expect("position があるはず")
}

/// spine move e2e（R5.1/R8.1・DD・task 9.3）: fixture 形の move script を含む OnBoot talk を実 sink 経路
/// （ghost→sakura compile→CueSheet→dispatch→broadcast→**実 MoveCueSink**→move channel）で流し、frame 相
/// drain（`run_move_drain_phase`＝task 9.2）が `MoveDirective` を drain して対象キャラ窓を検算位置へ即時
/// 移動させることを固定する。9.1 が置いた throwaway 送出端を実 MoveCueSink（sinks 第 3 要素・S-3 形）へ
/// 差し替えた配線が spine で end-to-end に生きていることの自動檻（headless・sleep 不使用・注入 Tick のみ・
/// R8.3/8.4/8.6）。窓が実際に動く＝`MoveDirective` が channel へ届き drain→apply された唯一の経路ゆえ、
/// 移動観測が「channel 到達＋frame 相 drain の live」を同時に証跡する。
///
/// # `\1` は正典どおり scope1（エモ＝相方）へ切替（観測 scope は 1・R4.4）
///
/// fixture は `\1\![move,-353,,,0,base,base]`（kero=scope1 を sakura=scope0 基準で動かす意図）で、
/// **bare `\1` は正典どおり sakura compile で `SpeakerScope{1}` へ写像される**（Task 12.1 で
/// `decode.rs`／lexer が `\0`/`\1` を SpeakerScope へ正規化・以前の `Raw` passthrough 縮退は解消済み）。
/// ゆえに move cue の scope は切替後の 1 として発火し（`cue.actor == "1"` → `MoveDirective.scope == 1`）、
/// base は `0`＝**scope0（むらさき＝話者）を基準にした scope1 の移動**として反映される（対象＝scope1 char 窓・
/// 基準＝scope0 char 窓）。実 channel 到達 directive は
/// `MoveDirective{ scope:1, x:Px(-353), y:Fix, base:Scope(0) }`。この e2e が `\1` の正典スコープ切替を
/// parse→compile→cue.actor→MoveDirective.scope→対象窓解決まで end-to-end に固定する。
///
/// # 検算（`two_scope_placements`・全て物理 px・R-6）
///
/// 対象＝scope1 pos(1049,1063) size(278,357)・基準＝scope0 pos(1483,733) size(434,687)・x=Px(-353)・y=Fix。
/// `CanonDefaultBasepos`（x=幅÷2）で
/// x' = base_pos.x + basepos(base窓).x + dx − basepos(対象窓).x
///    = 1483 + 434/2 − 353 − 278/2 = 1483 + 217 − 353 − 139 = 1208・
/// y は Fix ゆえ対象窓（scope1）の現状維持 1063。移動先 (1208,1063) は move cue が channel→drain→apply を
/// 通ったことの非空虚な証跡（RED では窓不動）。
///
/// # RED（実 MoveCueSink 未配線時）
///
/// 9.1 の throwaway `(_move_tx, move_rx)`（送出端即 drop・sinks に MoveCueSink なし）では move cue は
/// seriko/text sink へのみ broadcast され両者が良性スキップ→move channel は空のまま→窓は不動（moved=false
/// で FAIL・実測済み）。実 MoveCueSink を 3rd sink へ配線して初めて窓が動く（GREEN）。
#[test]
fn spine_move_cue_drives_window_move_end_to_end() {
    // fixture 形の move script（`\1` は正典どおり scope1 へ切替・doc 参照）。bare `\1` は Task 12.1 で
    // SpeakerScope へ写像されるため実 SHIORI 由来の現実的入力＝正典スコープ切替を e2e 検証する。
    let mut harness = SpineHarness::boot(r"\1\![move,-353,,,0,base,base]\e");

    // GhostWindows（`boot_with` が spawn_ghost_windows で資源挿入）から対象 char 窓を引き、実窓生成前ゆえ
    // 未付与の WindowHandle を偽装付与する（move_window_to の反映口 enqueue_window_set_pos の成立条件）。
    let gw = harness
        .world
        .get_resource::<GhostWindows>()
        .expect("spine World には GhostWindows が挿入済み")
        .clone();
    attach_fake_window_handles(&mut harness.world, &gw);
    // 観測 scope は 1（`\1` が正典どおり scope1 へ切替・doc 参照）。対象＝scope1（エモ）char 窓・基準＝scope0。
    let target = gw.char_window(1).expect("scope1（エモ＝相方）の char 窓");

    // 移動前の初期位置（two_scope_placements の scope1 char_pos）。
    let baseline = window_position(&harness.world, target);
    assert_eq!(
        baseline,
        Point { x: 1049, y: 1063 },
        "前提: 移動前の scope1 初期位置（two_scope_placements）"
    );

    // OnBoot talk を Tick 注入で駆動し、各反復で実 frame 相 move drain を回す。move cue は at=0.0 ゆえ talk
    // 起動後の最初の有効 Tick で発火するが、boot→compile→dispatch→broadcast はスレッド群を跨いで非同期に
    // 流れるため、窓が動く（channel→drain→apply 完了）まで有界待機する。有界性は壁時計 deadline（[`SPIN_WAIT`]）
    // ＋200µs poll-backoff sleep（R7.9・根拠は `drive_shell_shown` の doc）。観測「初期位置からの変位」は
    // move cue が 1 発のみ＝以降動かない＝ラッチであり、台本は `\w`／`\c` なし（全 cue `at=0.0`）ゆえ
    // 注入時刻の前進が観測を壊すクラス（R7.8）ではない。
    let mut moved = false;
    let deadline = Instant::now() + SPIN_WAIT;
    let mut now = 0u64;
    loop {
        now += 1;
        harness.inject_dispatcher_tick(now);
        // 実 frame 相 drain（task 9.2）: move channel を try_iter し apply_move_directive で即時反映。
        run_move_drain_phase(&harness.wiring, &mut harness.world);
        if window_position(&harness.world, target) != baseline {
            moved = true;
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_micros(200));
    }
    assert!(
        moved,
        "move cue が有界内に channel→frame drain→apply を通って対象窓を動かさない（実 MoveCueSink 配線が死んでいる？）"
    );

    // 検算位置（scope0 基準・CanonDefaultBasepos）へ即時移動＝MoveDirective が channel へ届き drain→apply された
    // 非空虚な証跡（R5.1・9.2 の frame 相配線が spine で生きている）。
    assert_eq!(
        window_position(&harness.world, target),
        Point { x: 1208, y: 1063 },
        "x'=1483+217−353−139=1208（base=scope0 基準・CanonDefaultBasepos）・y=Fix は現状維持 1063（cue→channel→frame drain→apply→実窓移動）"
    );

    // 二重適用なし: move cue は 1 発ゆえ、追加 drain で窓はさらに動かない（channel は drain 済みで空）。
    run_move_drain_phase(&harness.wiring, &mut harness.world);
    assert_eq!(
        window_position(&harness.world, target),
        Point { x: 1208, y: 1063 },
        "move channel は drain 済みで空（二重適用なし・FIFO 全件消費）"
    );

    harness.shutdown_bounded();
}
