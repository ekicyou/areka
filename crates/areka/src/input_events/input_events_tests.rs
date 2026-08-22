use super::*;
use crate::emo2_boot::hit_region::HitRegion;
use crate::placement::spawn::{CharWindowMarker, GhostWindowMarker};
use areka_kanade::{KanadeMsg, MouseButton, MouseEventKind};
use std::sync::mpsc;
use wintf::ecs::Point;
use wintf::ecs::pointer::{DoubleClick, Phase, PointerState};

/// Bubble 相の合成 `PointerState`（client 物理 px・DoubleClick・Ctrl 押下）を組む。
fn bubble_pointer(
    x: i32,
    y: i32,
    double_click: DoubleClick,
    ctrl_down: bool,
) -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        double_click,
        ctrl_down,
        ..Default::default()
    })
}

/// `GhostWindowMarker` 窓の現数を数える。
fn ghost_count(world: &mut World) -> usize {
    world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .count()
}

/// Mock resolver＋注入 clock の `MouseWiring` を NonSend 挿入した World を組む。
fn world_with_wiring(
    region: fn(u32, i64, i64) -> HitRegion,
    clock: Box<dyn FnMut() -> u64>,
) -> (World, mpsc::Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let wiring = MouseWiring::with_clock(tx, RegionSource::Mock(region), clock);
    let mut world = World::new();
    world.insert_non_send(wiring);
    (world, rx)
}

/// 単調増加する注入 clock を作る（毎呼出で +step ms）。
fn stepping_clock(start: u64, step: u64) -> Box<dyn FnMut() -> u64> {
    let mut t = start;
    Box::new(move || {
        let now = t;
        t += step;
        now
    })
}

/// RED（mock-seam send・1.5）: Mock resolver＋注入 clock で単体から配信し、受信側で内容を観測する。
#[test]
fn mock_seam_plan_and_send_move_observed() {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let mut wiring = MouseWiring::with_clock(
        tx,
        RegionSource::Mock(|_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        }),
        stepping_clock(1000, 1000),
    );

    // Mock は presenter を無視して固定写像を返す。
    let hit = wiring.resolve_region(None, 0, 10, 20);
    assert_eq!(
        hit.region,
        Some("Head".to_string()),
        "Mock は固定 region を返す"
    );

    // 初回送出（moved=first_send）。恒等 mock ゆえ client px と surface px は同値。
    let sent = wiring.plan_and_send_move(0, (10, 20), hit.surface_point, hit.region.clone());
    assert!(sent, "初回移動は送出される");

    let msg = rx.try_recv().expect("KanadeMsg が届くべき");
    match msg {
        KanadeMsg::Mouse(m) => {
            assert_eq!(m.scope, 0);
            assert_eq!(m.x, 10);
            assert_eq!(m.y, 20);
            assert_eq!(m.region, Some("Head".to_string()));
            assert_eq!(m.kind, MouseEventKind::Move);
        }
        _ => panic!("Mouse(Move) を期待"),
    }
    assert!(rx.try_recv().is_err(), "1 件のみ送出されるべき");
}

/// 間引き統合（5.1）: 同一 pos 再送は hover 抑制で送出されない（moved=false）。
#[test]
fn throttle_suppresses_same_position_hover() {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let mut wiring = MouseWiring::with_clock(
        tx,
        RegionSource::Mock(|_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        }),
        // 大きく進む clock でも位置不変なら送出されないことを見る。
        stepping_clock(1000, 10_000),
    );

    // 初回は送出。
    assert!(wiring.plan_and_send_move(0, (10, 20), (10, 20), Some("Head".to_string())));
    rx.try_recv().expect("初回は届く");

    // 同一 pos（moved=false）: 間隔が幾ら経っても hover 抑制で送出しない。
    assert!(
        !wiring.plan_and_send_move(0, (10, 20), (10, 20), Some("Head".to_string())),
        "同一 pos は送出しない（hover 抑制）"
    );
    assert!(rx.try_recv().is_err(), "抑制時は何も届かない");
}

/// 間引き統合（5.1）: 移動＋同一 region＋間隔未経過は抑制され、何も送出されない。
#[test]
fn throttle_suppresses_move_same_region_within_interval() {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    // clock: 1回目=1000, 2回目=1050（+50ms < 100ms 間隔）。
    let mut wiring = MouseWiring::with_clock(
        tx,
        RegionSource::Mock(|_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        }),
        stepping_clock(1000, 50),
    );

    // 初回送出（now=1000）。
    assert!(wiring.plan_and_send_move(0, (10, 20), (10, 20), Some("Head".to_string())));
    rx.try_recv().expect("初回は届く");

    // 移動・同一 region・間隔未経過（now=1050, delta=50 < 100）: 抑制。
    assert!(
        !wiring.plan_and_send_move(0, (11, 20), (11, 20), Some("Head".to_string())),
        "移動＋同一 region＋間隔未経過は抑制"
    );
    assert!(rx.try_recv().is_err(), "抑制時は何も届かない");
}

/// 左ダブルクリックは間引きなしで無条件送出され、内容（kind=DoubleClick{Left}）が観測できる（1.2/3.3）。
#[test]
fn double_click_left_sends_unconditionally() {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let mut wiring = MouseWiring::with_clock(
        tx,
        RegionSource::Mock(|_, x, y| HitRegion {
            scope: 0,
            region: None,
            surface_point: (x, y),
        }),
        stepping_clock(1000, 1000),
    );

    wiring.send_double_click(0, (5, 6), Some("Head".to_string()), MouseButton::Left);
    match rx.try_recv().expect("dblclick が届くべき") {
        KanadeMsg::Mouse(m) => {
            assert_eq!(m.scope, 0);
            assert_eq!(m.x, 5);
            assert_eq!(m.y, 6);
            assert_eq!(m.region, Some("Head".to_string()));
            assert_eq!(
                m.kind,
                MouseEventKind::DoubleClick {
                    button: MouseButton::Left
                }
            );
        }
        _ => panic!("Mouse(DoubleClick) を期待"),
    }

    // クリックは throttle を通さない: 同一座標で連続送出しても届く。
    wiring.send_double_click(0, (5, 6), Some("Head".to_string()), MouseButton::Left);
    assert!(rx.try_recv().is_ok(), "クリックは間引かれず 2 回目も届く");
}

/// 右ダブルクリックも同様に無条件送出され、button=Right が観測できる（3.3）。
#[test]
fn double_click_right_sends_with_right_button() {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let mut wiring = MouseWiring::with_clock(
        tx,
        RegionSource::Mock(|_, x, y| HitRegion {
            scope: 0,
            region: None,
            surface_point: (x, y),
        }),
        stepping_clock(1000, 1000),
    );

    wiring.send_double_click(1, (7, 8), None, MouseButton::Right);
    match rx.try_recv().expect("dblclick が届くべき") {
        KanadeMsg::Mouse(m) => {
            assert_eq!(m.scope, 1);
            assert_eq!(m.x, 7);
            assert_eq!(m.y, 8);
            assert_eq!(m.region, None);
            assert_eq!(
                m.kind,
                MouseEventKind::DoubleClick {
                    button: MouseButton::Right
                }
            );
        }
        _ => panic!("Mouse(DoubleClick) を期待"),
    }
}

/// 配線挿入檻（3.1・DD-IE-9）: `wire_mouse_input` は fresh World へ `MouseWiring`
/// を NonSend 挿入する（Presenter region source・proven-wiring ゆえ存在確認で足る・
/// [[test-only-decision-branches-not-proven-wiring]]・窓ハンドラ登録は task 3.2）。
#[test]
fn wire_mouse_input_inserts_mouse_wiring_non_send() {
    let (tx, _rx) = mpsc::channel::<KanadeMsg>();
    let mut world = World::new();
    assert!(
        world.get_non_send::<MouseWiring>().is_none(),
        "挿入前は MouseWiring 不在"
    );
    wire_mouse_input(&mut world, tx);
    assert!(
        world.get_non_send::<MouseWiring>().is_some(),
        "wire_mouse_input 後は MouseWiring が NonSend 挿入されている"
    );
}

/// presenter 不在の正常縮退（1.3・DD-IE-9）: `RegionSource::Presenter` で presenter=None なら
/// `region: None` を返し panic しない。
///
/// DPI追従（areka-P0-collision-dpi-hittest）以後は `surface_point` も併せて固定する——presenter が
/// 居なければ実適用 k を知る術が無いため等倍相当（縮約は恒等）で縮退し、受領した client 物理 px
/// がそのまま配信空間の値になる（要件 1.6 と同じ縮退規約）。
#[test]
fn presenter_absent_degrades_to_region_none() {
    let (tx, _rx) = mpsc::channel::<KanadeMsg>();
    let wiring = MouseWiring::with_clock(tx, RegionSource::Presenter, stepping_clock(1000, 1000));

    let hit = wiring.resolve_region(None, 3, 100, 200);
    assert_eq!(
        hit,
        HitRegion {
            scope: 3,
            region: None,
            surface_point: (100, 200),
        },
        "Emo2Wiring 不在は region None・座標は無変換へ正常縮退（scope はそのまま反映）"
    );
}

// -------------------------------------------------------------------------
// ポインタハンドラ檻（task 2.7・design Testing Strategy「配線存在檻/送出集合檻/暫定退避檻」）
//
// 合成 `PointerState`／`Phase` でハンドラを直接呼び、mpsc で送出／非送出を観測する
// （GPU/実窓不要・単一 pass/fail・[[areka-bin-crate-internal-tests-in-crate]]）。
// -------------------------------------------------------------------------

/// 配線存在檻（1.1・5.1）: Bubble 移動＋Mock region Some("Head")＋間引き通過で
/// `KanadeMsg::Mouse(Move)` を観測。同一位置の再移動は hover 抑制で送出されない。
#[test]
fn handler_move_sends_then_suppresses_same_position() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 10_000),
    );
    let e = world.spawn(CharWindowMarker { scope: 0 }).id();

    // 初回移動 → 送出（内容一致）
    let ev = bubble_pointer(10, 20, DoubleClick::None, false);
    assert!(
        on_char_pointer_moved(&mut world, e, e, &ev),
        "初回移動は送出"
    );
    match rx.try_recv().expect("KanadeMsg が届く") {
        KanadeMsg::Mouse(m) => {
            assert_eq!(m.scope, 0);
            assert_eq!(m.x, 10);
            assert_eq!(m.y, 20);
            assert_eq!(m.region, Some("Head".to_string()));
            assert_eq!(m.kind, MouseEventKind::Move);
        }
        _ => panic!("Mouse(Move) を期待"),
    }

    // 同一位置の再移動 → hover 抑制で送出なし（moved=false）
    let ev2 = bubble_pointer(10, 20, DoubleClick::None, false);
    assert!(
        !on_char_pointer_moved(&mut world, e, e, &ev2),
        "同一位置は抑制"
    );
    assert!(rx.try_recv().is_err(), "抑制時は何も届かない");
}

/// 送出集合檻（1.2・3.3）: 左／右ダブルクリック（Ctrl なし）は当たり判定を解決し
/// `KanadeMsg::Mouse(DoubleClick{button})` を送出する（Left→Left・Right→Right）。
#[test]
fn handler_double_click_left_and_right_send() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Bust".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 1000),
    );
    let e = world.spawn(CharWindowMarker { scope: 0 }).id();

    // 左ダブルクリック
    let ev = bubble_pointer(5, 6, DoubleClick::Left, false);
    assert!(on_char_pointer_pressed(&mut world, e, e, &ev));
    match rx.try_recv().expect("dblclick が届く") {
        KanadeMsg::Mouse(m) => {
            assert_eq!(m.scope, 0);
            assert_eq!(m.x, 5);
            assert_eq!(m.y, 6);
            assert_eq!(m.region, Some("Bust".to_string()));
            assert_eq!(
                m.kind,
                MouseEventKind::DoubleClick {
                    button: MouseButton::Left
                }
            );
        }
        _ => panic!("Mouse(DoubleClick Left) を期待"),
    }

    // 右ダブルクリック
    let ev = bubble_pointer(7, 8, DoubleClick::Right, false);
    assert!(on_char_pointer_pressed(&mut world, e, e, &ev));
    match rx.try_recv().expect("dblclick が届く") {
        KanadeMsg::Mouse(m) => {
            assert_eq!(
                m.kind,
                MouseEventKind::DoubleClick {
                    button: MouseButton::Right
                }
            );
        }
        _ => panic!("Mouse(DoubleClick Right) を期待"),
    }
}

/// 送出集合檻（7.1・7.3）: 中／拡張ボタンのダブルクリックと単発クリックはいずれも
/// 送出せず false を返す（OnMouseDoubleClickEx は M2・OnMouseClick 単発は不送出）。
#[test]
fn handler_middle_xbutton_and_single_click_do_not_send() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: None,
            surface_point: (x, y),
        },
        stepping_clock(1000, 1000),
    );
    let e = world.spawn(CharWindowMarker { scope: 0 }).id();

    for dc in [
        DoubleClick::Middle,
        DoubleClick::XButton1,
        DoubleClick::XButton2,
        DoubleClick::None,
    ] {
        let ev = bubble_pointer(1, 2, dc, false);
        assert!(
            !on_char_pointer_pressed(&mut world, e, e, &ev),
            "{dc:?} は送出しない"
        );
    }
    assert!(rx.try_recv().is_err(), "中/拡張/単発は何も送出しない");
}

/// 暫定退避檻（6.2/6.3・DD-IE-7）: Ctrl+左ダブルクリックで全 `GhostWindowMarker`
/// 窓を despawn し、SHIORI へは何も送らない。無関係 entity は残る。
#[test]
fn handler_ctrl_left_double_click_despawns_all_ghost_windows_without_sending() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 1000),
    );
    world.spawn(GhostWindowMarker);
    world.spawn(GhostWindowMarker);
    let w0 = world.spawn(GhostWindowMarker).id();
    let other = world.spawn_empty().id();
    assert_eq!(ghost_count(&mut world), 3);

    let ev = bubble_pointer(10, 20, DoubleClick::Left, true);
    assert!(on_char_pointer_pressed(&mut world, w0, w0, &ev));
    assert_eq!(ghost_count(&mut world), 0, "全ゴースト窓が despawn される");
    assert!(world.get_entity(other).is_ok(), "無関係 entity は残る");
    assert!(rx.try_recv().is_err(), "暫定退避は SHIORI へ送らない");
}

/// 暫定退避檻（6.1）: Ctrl なしの左ダブルクリックは窓を despawn せず、
/// 代わりに `DoubleClick` を送出する（退避と送出の分岐が排他であること）。
#[test]
fn handler_left_double_click_without_ctrl_does_not_despawn_and_sends() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: None,
            surface_point: (x, y),
        },
        stepping_clock(1000, 1000),
    );
    let w = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();

    let ev = bubble_pointer(3, 4, DoubleClick::Left, false);
    assert!(on_char_pointer_pressed(&mut world, w, w, &ev));
    assert_eq!(ghost_count(&mut world), 1, "Ctrl なしは despawn しない");
    assert!(
        matches!(
            rx.try_recv().expect("送出される"),
            KanadeMsg::Mouse(m) if m.kind == MouseEventKind::DoubleClick { button: MouseButton::Left }
        ),
        "Ctrl なし左ダブルクリックは DoubleClick を送出"
    );
}

/// self-gating（DD-IE-9）: `MouseWiring` 不在なら送出系ハンドラは no-op false。
#[test]
fn handlers_self_gate_when_mouse_wiring_absent() {
    let mut world = World::new();
    let e = world.spawn(CharWindowMarker { scope: 0 }).id();

    let moved = bubble_pointer(10, 20, DoubleClick::None, false);
    assert!(!on_char_pointer_moved(&mut world, e, e, &moved));
    let dbl = bubble_pointer(10, 20, DoubleClick::Left, false);
    assert!(!on_char_pointer_pressed(&mut world, e, e, &dbl));
    let mid = bubble_pointer(10, 20, DoubleClick::Middle, false);
    assert!(!on_char_pointer_pressed(&mut world, e, e, &mid));
}

/// 暫定退避は wiring 非依存（DD-IE-7）: `MouseWiring` 不在でも Ctrl+左で全窓 despawn。
#[test]
fn escape_works_without_mouse_wiring() {
    let mut world = World::new();
    world.spawn(GhostWindowMarker);
    let w = world.spawn(GhostWindowMarker).id();
    assert_eq!(ghost_count(&mut world), 2);

    let ev = bubble_pointer(10, 20, DoubleClick::Left, true);
    assert!(
        on_char_pointer_pressed(&mut world, w, w, &ev),
        "wiring 不在でも暫定退避は機能する"
    );
    assert_eq!(ghost_count(&mut world), 0, "全ゴースト窓 despawn");
}

/// 配線層 per-scope 独立檻（5.x・DD-IE-5／DD-IE-9・task 4.4 NEW）: 1 つの `MouseWiring`
/// （`HashMap<u32, MouseMoveThrottle>` per-scope 保持）を通し、scope 0 が間引きで抑制されている
/// 最中でも scope 1 の初回移動は独立に送出される。scope の間引き状態が `HashMap` で別キーに
/// 保持されている（共有でない）ことの配線層証拠——throttle.rs の純関数 per-scope 檻
/// （別々の `MouseMoveThrottle` 値を使う）とは別レイヤ（同一 wiring の map を通す）。
#[test]
fn wiring_throttles_scopes_independently_via_hashmap() {
    // clock: 各ハンドラ呼出で +10ms（間隔 100ms 未満）。
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 10),
    );
    let e0 = world.spawn(CharWindowMarker { scope: 0 }).id();
    let e1 = world.spawn(CharWindowMarker { scope: 1 }).id();

    // scope 0 初回移動（now=1000）→ 送出。
    let ev = bubble_pointer(10, 20, DoubleClick::None, false);
    assert!(
        on_char_pointer_moved(&mut world, e0, e0, &ev),
        "scope 0 初回は送出"
    );
    match rx.try_recv().expect("scope 0 の Move が届く") {
        KanadeMsg::Mouse(m) => assert_eq!(m.scope, 0),
        _ => panic!("Mouse(Move) を期待"),
    }

    // scope 0 移動・同一 region・間隔未経過（now=1010, delta=10 < 100）→ 抑制。
    let ev = bubble_pointer(11, 20, DoubleClick::None, false);
    assert!(
        !on_char_pointer_moved(&mut world, e0, e0, &ev),
        "scope 0 は間引きで抑制される"
    );
    assert!(rx.try_recv().is_err(), "scope 0 の 2 回目は届かない");

    // scope 1 初回移動（now=1020）→ scope 0 の抑制状態と独立に送出される。
    // もし throttle 状態が scope 間で共有なら「同一 region・間隔未経過」で抑制されるはず。
    // HashMap が別キーで保持するため scope 1 は fresh（first_send）で送出される。
    let ev = bubble_pointer(10, 20, DoubleClick::None, false);
    assert!(
        on_char_pointer_moved(&mut world, e1, e1, &ev),
        "scope 1 は scope 0 の抑制と独立に初回送出される"
    );
    match rx.try_recv().expect("scope 1 の Move が届く") {
        KanadeMsg::Mouse(m) => assert_eq!(m.scope, 1, "送出された Move は scope 1"),
        _ => panic!("Mouse(Move) を期待"),
    }
    assert!(rx.try_recv().is_err(), "他に送出はない");
}

/// 「暫定退避操作でのみ全窓終了が起きる」統合檻（6.1/6.2/6.3・DD-IE-7・task 4.4 NEW）:
/// 移動・中ボタンダブルクリック・単発クリック・Ctrl なし左ダブルクリックの**いずれも**
/// `GhostWindowMarker` 窓を despawn しない（非退避操作）。Ctrl+左ダブルクリック**のみ**が
/// 全窓を despawn する。「でのみ」の排他性を単一 pass/fail で固定する（plain-left 単独の
/// `handler_left_double_click_without_ctrl_does_not_despawn_and_sends` に対し、非退避操作
/// 全集合の否定＋退避の肯定を 1 檻へ集約する）。
#[test]
fn only_escape_terminates_ghost_windows() {
    let (mut world, _rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 1000),
    );
    // 実キャラ窓は GhostWindowMarker かつ CharWindowMarker（ハンドラが scope を読める）。
    let w = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();
    world.spawn(GhostWindowMarker);
    world.spawn(GhostWindowMarker);
    assert_eq!(ghost_count(&mut world), 3);

    // 非退避操作はいずれも despawn しない。
    let non_escape = [
        bubble_pointer(10, 20, DoubleClick::None, false), // 移動（None）
        bubble_pointer(10, 21, DoubleClick::Middle, false), // 中ボタン dblclick
        bubble_pointer(10, 22, DoubleClick::XButton1, false), // 拡張ボタン dblclick
        bubble_pointer(10, 23, DoubleClick::None, false), // 単発クリック
        bubble_pointer(10, 24, DoubleClick::Left, false), // Ctrl なし左 dblclick
    ];
    // 移動は on_char_pointer_moved、他は on_char_pointer_pressed。
    assert!(on_char_pointer_moved(&mut world, w, w, &non_escape[0]));
    for ev in &non_escape[1..] {
        on_char_pointer_pressed(&mut world, w, w, ev);
    }
    assert_eq!(
        ghost_count(&mut world),
        3,
        "非退避操作（移動/中/拡張/単発/Ctrl なし左）は 1 つも despawn しない"
    );

    // 退避操作（Ctrl+左）のみが全窓を despawn する。
    let escape = bubble_pointer(10, 20, DoubleClick::Left, true);
    assert!(on_char_pointer_pressed(&mut world, w, w, &escape));
    assert_eq!(
        ghost_count(&mut world),
        0,
        "暫定退避操作でのみ全ゴースト窓が despawn される"
    );
}

// -------------------------------------------------------------------------
// 配信座標の空間切替檻（task 4.2・DD-4・design Testing Strategy「Unit Tests（areka bin）」項目 2）
//
// 恒等 mock（`surface_point: (x, y)`）では配信値が client 点と**数値的に区別できず**、切替の
// 前後で同じ色のままになる＝何も証明しない。ゆえに以下 2 本は **非恒等 mock**
// （surface = client ÷ 2）を用い、(i) 配信値が `surface_point` であること (ii) throttle の位置
// 比較が縮約前 client px のままであることを**両方向に割れる**形で固定する。
// -------------------------------------------------------------------------

/// 非恒等 mock で (i) 配信 `MouseInput{x,y}` が `surface_point` 値であること (ii) throttle が
/// 縮約前 client px を比較していることを同時に固定する（1.8・6.8・DD-4）。
///
/// 写像は `surface = client / 2`。client (10,20) と (11,20) は**同一の** surface (5,10) へ潰れる:
/// - 配信が client px のままなら (i) の期待値 (5,10) が外れる。
/// - throttle が surface px を比較していれば 2 回目は `moved=false` で hover 抑制され届かない。
///
/// clock は毎回 10s 進める＝間隔条件を常に満たすため、2 回目の送出可否は「位置が動いたか」
/// （＝どちらの空間で比較しているか）だけに依存する。
#[test]
fn move_delivers_surface_point_while_throttle_compares_client_px() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x / 2, y / 2),
        },
        stepping_clock(1000, 10_000),
    );
    let e = world.spawn(CharWindowMarker { scope: 0 }).id();

    // (i) client (10,20) → 配信は surface (5,10)。
    let ev = bubble_pointer(10, 20, DoubleClick::None, false);
    assert!(
        on_char_pointer_moved(&mut world, e, e, &ev),
        "初回移動は送出"
    );
    match rx.try_recv().expect("KanadeMsg が届く") {
        KanadeMsg::Mouse(m) => {
            assert_eq!(m.scope, 0);
            assert_eq!(
                (m.x, m.y),
                (5, 10),
                "配信座標は surface_point（client px の素通しではない）"
            );
            assert_eq!(m.region, Some("Head".to_string()));
            assert_eq!(m.kind, MouseEventKind::Move);
        }
        _ => panic!("Mouse(Move) を期待"),
    }

    // (ii) client (11,20) は surface では同一点 (5,10)。client px 比較なら moved=true で送出。
    let ev = bubble_pointer(11, 20, DoubleClick::None, false);
    assert!(
        on_char_pointer_moved(&mut world, e, e, &ev),
        "throttle は縮約前 client px を比較する（surface px 比較なら同一点で抑制されてしまう）"
    );
    match rx.try_recv().expect("2 回目の Move が届く") {
        KanadeMsg::Mouse(m) => assert_eq!(
            (m.x, m.y),
            (5, 10),
            "2 回目も配信値は surface_point（client px は配信されない）"
        ),
        _ => panic!("Mouse(Move) を期待"),
    }

    // 逆向きの固定: client 座標が真に同一なら hover 抑制で届かない（throttle は生きている）。
    let ev = bubble_pointer(11, 20, DoubleClick::None, false);
    assert!(
        !on_char_pointer_moved(&mut world, e, e, &ev),
        "同一 client px は hover 抑制"
    );
    assert!(rx.try_recv().is_err(), "抑制時は何も届かない");
}

/// 非恒等 mock で double-click の配信座標も `surface_point` であることを固定する（1.8・DD-4）。
///
/// クリック経路は throttle を通らない＝この経路に client px は現れない。
#[test]
fn double_click_delivers_surface_point() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Bust".to_string()),
            surface_point: (x / 2, y / 2),
        },
        stepping_clock(1000, 1000),
    );
    let e = world.spawn(CharWindowMarker { scope: 0 }).id();

    let ev = bubble_pointer(10, 20, DoubleClick::Left, false);
    assert!(on_char_pointer_pressed(&mut world, e, e, &ev));
    match rx.try_recv().expect("dblclick が届く") {
        KanadeMsg::Mouse(m) => {
            assert_eq!(
                (m.x, m.y),
                (5, 10),
                "dblclick の配信座標も surface_point（client px ではない）"
            );
            assert_eq!(m.region, Some("Bust".to_string()));
            assert_eq!(
                m.kind,
                MouseEventKind::DoubleClick {
                    button: MouseButton::Left
                }
            );
        }
        _ => panic!("Mouse(DoubleClick) を期待"),
    }
}

/// no-op 保存檻（1.9）: k=1.0 相当（恒等 mock）では move／double-click の配信値が
/// 受領 client px と完全に一致する＝DPI追従導入前の配信値を変更しない。
///
/// 上 2 本（非恒等 mock）が「空間が切り替わったこと」を固定するのに対し、本檻は
/// 「等倍では何も変わらないこと」を固定する対（両者が揃って初めて R1.8/R1.9 が閉じる）。
#[test]
fn identity_scale_delivers_unchanged_client_coords() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 10_000),
    );
    let e = world.spawn(CharWindowMarker { scope: 0 }).id();

    let ev = bubble_pointer(42, 77, DoubleClick::None, false);
    assert!(on_char_pointer_moved(&mut world, e, e, &ev));
    match rx.try_recv().expect("Move が届く") {
        KanadeMsg::Mouse(m) => assert_eq!(
            (m.x, m.y),
            (42, 77),
            "k=1.0 では配信値＝受領 client px（従前と同一）"
        ),
        _ => panic!("Mouse(Move) を期待"),
    }

    let ev = bubble_pointer(13, 99, DoubleClick::Right, false);
    assert!(on_char_pointer_pressed(&mut world, e, e, &ev));
    match rx.try_recv().expect("dblclick が届く") {
        KanadeMsg::Mouse(m) => assert_eq!(
            (m.x, m.y),
            (13, 99),
            "k=1.0 では dblclick の配信値も従前どおり client px と同値"
        ),
        _ => panic!("Mouse(DoubleClick) を期待"),
    }
}

/// Tunnel 相は両ハンドラとも no-op（Bubble のみ処理）: 退避も送出も起きない。
#[test]
fn handlers_ignore_tunnel_phase() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 1000),
    );
    let w = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();

    let tunnel_move = Phase::Tunnel(PointerState {
        client_point: Point { x: 10, y: 20 },
        ..Default::default()
    });
    assert!(!on_char_pointer_moved(&mut world, w, w, &tunnel_move));

    // Tunnel の Ctrl+左でも退避しない（Bubble 相のみ処理）
    let tunnel_ctrl_left = Phase::Tunnel(PointerState {
        client_point: Point { x: 10, y: 20 },
        double_click: DoubleClick::Left,
        ctrl_down: true,
        ..Default::default()
    });
    assert!(!on_char_pointer_pressed(
        &mut world,
        w,
        w,
        &tunnel_ctrl_left
    ));
    assert_eq!(ghost_count(&mut world), 1, "Tunnel 相では退避しない");
    assert!(rx.try_recv().is_err(), "Tunnel 相は何も送出しない");
}
