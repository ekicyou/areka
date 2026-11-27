use super::balloon::test_support::events_named;
use super::pointer_test_support::*;
use super::*;
use areka_kanade::{KanadeMsg, MouseButton, MouseEventKind};
use areka_placement::spawn::CharWindowMarker;
use areka_runtime::hit_region::HitRegion;
use std::sync::mpsc;
use wintf::ecs::Point;
use wintf::ecs::pointer::{DoubleClick, Phase, PointerState};

/// どの点も `Head`（scope 0）に当て、座標はそのまま返す偽の当たり判定。
fn head(_: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope: 0,
        region: Some("Head".to_string()),
        surface_point: (x, y),
    }
}

/// 受け口を落とした結線で、移動とダブルクリックの送出はそれぞれ `kind` つきの
/// `mouse_send_failed` を 1 行だけ出し、移動は送れなかったと返す。
#[test]
fn move_and_double_click_send_failures_record_one_line_each() {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    drop(rx);
    let mut wiring = MouseWiring::with_clock(
        tx,
        RegionSource::Mock(|_, x, y| HitRegion {
            scope: 0,
            region: None,
            surface_point: (x, y),
        }),
        stepping_clock(1000, 1000),
    );

    let (sent, moved) =
        log_capture_kit::capture(|| wiring.plan_and_send_move(0, (1, 2), (1, 2), None));
    assert!(!sent, "受け口が無ければ送れなかったと返す");
    let (_, clicked) =
        log_capture_kit::capture(|| wiring.send_double_click(0, (1, 2), None, MouseButton::Left));

    for (events, kind) in [(moved, "move"), (clicked, "double_click")] {
        let failed = events_named(&events, "mouse_send_failed");
        assert_eq!(failed.len(), 1, "{kind}: {events:?}");
        assert_eq!(failed[0].field_str("kind"), Some(kind), "{events:?}");
        assert_eq!(failed[0].target, "areka_input", "{events:?}");
        assert_eq!(failed[0].level, tracing::Level::WARN, "{events:?}");
    }
}

/// `wire_mouse_input` は fresh World へ `MouseWiring`
/// を NonSend 挿入する（Presenter region source・結線は存在確認で足る）。
#[test]
fn wire_mouse_input_inserts_mouse_wiring_non_send() {
    let (tx, _rx) = mpsc::channel::<KanadeMsg>();
    let mut world = World::new();
    wire_mouse_input(&mut world, tx);
    assert!(
        world.get_non_send::<MouseWiring>().is_some(),
        "wire_mouse_input 後は MouseWiring が NonSend 挿入されている"
    );
}

/// presenter 不在の正常縮退: `RegionSource::Presenter` で presenter=None なら
/// `region: None` を返し panic しない。
///
/// `surface_point` も併せて固定する——presenter が
/// 居なければ実適用 k を知る術が無いため等倍相当（縮約は恒等）で縮退し、受領した client 物理 px
/// がそのまま配信空間の値になる。
#[test]
fn presenter_absent_degrades_to_region_none() {
    let (tx, _rx) = mpsc::channel::<KanadeMsg>();
    let wiring = MouseWiring::with_clock(tx, RegionSource::Presenter, stepping_clock(1000, 1000));

    let hit = wiring.resolve_region(&World::new(), None, 3, 100, 200);
    assert_eq!(
        hit,
        HitRegion {
            scope: 3,
            region: None,
            surface_point: (100, 200),
        },
        "RuntimeWiring 不在は region None・座標は無変換へ正常縮退（scope はそのまま反映）"
    );
}

// -------------------------------------------------------------------------
// ポインタハンドラ
//
// 合成 `PointerState`／`Phase` でハンドラを直接呼び、mpsc で送出／非送出を観測する
// （GPU/実窓不要）。
// -------------------------------------------------------------------------

/// 左ダブルクリック（Ctrl なし）は当たり判定を解決して即
/// `KanadeMsg::Mouse(DoubleClick{Left})` を送出する。右ダブルクリックは押下の時点では送らずに
/// 預かり、預かりを送ると `DoubleClick{Right}` が同じ形（スコープ・座標・当たり判定名）で届く。
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

    // 右ダブルクリック: 押下では届かず、預かりを送って初めて届く。
    let ev = bubble_pointer(7, 8, DoubleClick::Right, false);
    assert!(on_char_pointer_pressed(&mut world, e, e, &ev));
    assert!(rx.try_recv().is_err(), "右は押下の時点では送らない");
    let pending = take_pending(&mut world).expect("右ダブルクリックは預かりとして残る");
    send_pending(&mut world, pending);
    match rx.try_recv().expect("dblclick が届く") {
        KanadeMsg::Mouse(m) => {
            assert_eq!((m.scope, m.x, m.y), (0, 7, 8));
            assert_eq!(m.region, Some("Bust".to_string()));
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

/// 中／拡張ボタンのダブルクリックと単発クリックはいずれも
/// 送出せず false を返す（押下の受け手は `OnMouseDoubleClickEx` を送らない・単発のクリックは解放の受け手が送る）。
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

/// self-gating: `MouseWiring` 不在なら送出系ハンドラは no-op false。
#[test]
fn handlers_self_gate_when_mouse_wiring_absent() {
    let mut world = World::new();
    let e = world.spawn(CharWindowMarker { scope: 0 }).id();

    let moved = bubble_pointer(10, 20, DoubleClick::None, false);
    assert!(!on_char_pointer_moved(&mut world, e, e, &moved));
    let dbl = bubble_pointer(10, 20, DoubleClick::Left, false);
    assert!(!on_char_pointer_pressed(&mut world, e, e, &dbl));
}

/// scope ごとに独立: 1 つの `MouseWiring`
/// （`HashMap<u32, MouseMoveThrottle>` per-scope 保持）を通し、scope 0 が間引きで抑制されている
/// 最中でも scope 1 の初回移動は独立に送出される。scope の間引き状態が `HashMap` で別キーに
/// 保持されている（共有でない）ことの配線層証拠。
#[test]
fn wiring_throttles_scopes_independently_via_hashmap() {
    // clock: 各ハンドラ呼出で +10ms（間隔 100ms 未満）。
    let (mut world, rx) = world_with_wiring(head, stepping_clock(1000, 10));
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
    // HashMap が別キーで保持するため scope 1 は fresh（初回）で送出される。
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

// -------------------------------------------------------------------------
// 配信座標の空間切替
//
// 恒等 mock（`surface_point: (x, y)`）では配信値が client 点と**数値的に区別できず**、切替の
// 前後で同じ色のままになる＝何も証明しない。ゆえに以下 2 本は **非恒等 mock**
// （surface = client ÷ 2）を用い、(i) 配信値が `surface_point` であること (ii) throttle の位置
// 比較が縮約前 client px のままであることを**両方向に割れる**形で固定する。
// -------------------------------------------------------------------------

/// 非恒等 mock で (i) 配信 `MouseInput{x,y}` が `surface_point` 値であること (ii) throttle が
/// 縮約前 client px を比較していることを同時に固定する。
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

/// 非恒等 mock で double-click の配信座標も `surface_point` であることを固定する。
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

/// Tunnel 相は両ハンドラとも no-op（Bubble のみ処理）: 退避も送出も起きない。
#[test]
fn handlers_ignore_tunnel_phase() {
    let (mut world, rx) = world_with_wiring(head, stepping_clock(1000, 1000));
    let w = world.spawn(CharWindowMarker { scope: 0 }).id();

    let tunnel_move = Phase::Tunnel(PointerState {
        client_point: Point { x: 10, y: 20 },
        ..Default::default()
    });
    assert!(!on_char_pointer_moved(&mut world, w, w, &tunnel_move));

    // Tunnel の Ctrl+Shift+左でも退避しない（Bubble 相のみ処理）
    let tunnel_ctrl_left = Phase::Tunnel(PointerState {
        client_point: Point { x: 10, y: 20 },
        double_click: DoubleClick::Left,
        ctrl_down: true,
        shift_down: true,
        ..Default::default()
    });
    assert!(!on_char_pointer_pressed(
        &mut world,
        w,
        w,
        &tunnel_ctrl_left
    ));
    assert!(!exit_requested(&world), "Tunnel 相では退避しない");
    assert!(rx.try_recv().is_err(), "Tunnel 相は何も送出しない");
}
