//! 右ボタンの解放から計画ができるまでの流れ（[`handle_release`]・[`poll_once`]）の決定論テスト
//! （要件 1.8・1.9・1.10・3.2・3.4・3.7・6.2・7.4）。
//!
//! 合成した `PointerState` で解放ハンドラを直接呼び、運行（kanade）宛ての受信端と
//! [`MenuWiring`] の中身で結果を観測する。実窓も OS のメニューも使わない: 窓ハンドルは偽の値で、
//! 画面座標への写しはテスト用の関数を差し込む。時刻は引数で渡すので、どのテストも待たない。
//!
//! 期待値（名前の並び・項目名・座標・記録の綴り）はすべて書き下した値で、実装の表から導かない。

use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{KanadeMsg, MouseButton, MouseEventKind, MouseInput};
use log_capture_kit::{LineFormat, capture_lines};
use windows::Win32::Foundation::HINSTANCE;
use wintf::ecs::drag::{DragState, end_dragging, start_preparing, update_drag_state};
use wintf::ecs::pointer::ButtonReleased;
use wintf::ecs::{Point, WindowHandle};

use super::*;
use crate::emo2_boot::hit_region::HitRegion;
use crate::input_events::{MouseWiring, RegionSource};
use crate::menu::captions::QUERY_TIMEOUT;
use crate::menu::plan::PlanEntry;
use crate::menu::{ItemBody, MenuWiring};
use crate::placement::spawn::CharWindowMarker;

/// 当たり判定の代役（このテストでは当たり判定を引かないので中身は使われない）。
fn no_region(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: None,
        surface_point: (x, y),
    }
}

/// 画面座標への写しの代役。クライアント座標に (1000, 2000) を足す——恒等だと、クライアント
/// 座標をそのまま要求へ載せた実装と見分けが付かない。
fn shifted_to_screen(_hwnd: HWND, x: i32, y: i32) -> Option<(i32, i32)> {
    Some((x + 1000, y + 2000))
}

/// 写せなかったとき（窓が既に無い等）の代役。
fn failing_to_screen(_hwnd: HWND, _x: i32, _y: i32) -> Option<(i32, i32)> {
    None
}

/// 動作を持つ葉の項目を返す供給関数。
fn supplier(label: &'static str, resource: &'static str) -> crate::menu::Supplier {
    Rc::new(move |_, _| MenuItem {
        label: label.to_string(),
        caption_resource: Some(resource),
        enabled: true,
        checked: None,
        body: ItemBody::Action(Rc::new(|_, _| {})),
    })
}

/// テスト用の World 一式。
struct Fixture {
    world: World,
    /// 運行（kanade）宛ての受信端。照会も右ダブルクリックもここへ届く。
    kanade: Receiver<KanadeMsg>,
    /// 本体側（スコープ 0）のキャラクター窓。
    window: Entity,
}

/// 説明書と終了が登記済みで、結線が 2 つとも済んでいる World を組む。
fn fixture_with(hwnd: HWND, to_screen: crate::menu::ToScreen) -> Fixture {
    let (tx, kanade) = channel::<KanadeMsg>();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::new(tx.clone(), RegionSource::Mock(no_region)));
    let mut wiring = MenuWiring::with_to_screen(tx, to_screen);
    wiring
        .registry
        .register(Frame::Close, supplier("終了", "closebutton.caption"));
    wiring
        .registry
        .register(Frame::Readme, supplier("説明書", "readmebutton.caption"));
    world.insert_non_send(wiring);
    let window = world
        .spawn((
            CharWindowMarker { scope: 0 },
            WindowHandle {
                hwnd,
                instance: HINSTANCE::default(),
            },
        ))
        .id();
    Fixture {
        world,
        kanade,
        window,
    }
}

/// 偽の窓ハンドル（OS へは渡らない。要求へそのまま載ることを確かめるための値）。
fn fake_hwnd() -> HWND {
    HWND(0x1234 as *mut _)
}

fn fixture() -> Fixture {
    fixture_with(fake_hwnd(), shifted_to_screen)
}

/// クライアント座標 (30, 40) での解放。どのボタンが離されたかは呼び手が決める。
fn released(released: ButtonReleased) -> PointerState {
    PointerState {
        client_point: Point { x: 30, y: 40 },
        released,
        ..Default::default()
    }
}

fn right_released() -> PointerState {
    released(ButtonReleased {
        right: true,
        ..Default::default()
    })
}

/// 右ダブルクリックを 1 件預ける（押下ハンドラが置くのと同じ形）。
fn defer_double_click(world: &mut World) {
    world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は挿入済み")
        .defer_right_double_click(PendingDoubleClick {
            scope: 0,
            surface_pos: (7, 4),
            region: Some("Bust".to_string()),
        });
}

/// 預かりが残っているか（取り出すので、呼んだ後は必ず空になる）。
fn deferred_double_click_remains(world: &mut World) -> bool {
    world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は挿入済み")
        .take_pending_right_double_click()
        .is_some()
}

fn menu_wiring(world: &World) -> &MenuWiring {
    world
        .get_non_send::<MenuWiring>()
        .expect("MenuWiring は挿入済み")
}

fn in_flight(world: &World) -> bool {
    menu_wiring(world).in_flight.get()
}

/// `[menu]` の記録のうち、指定のレベルの行だけを拾う。
fn menu_lines<'a>(lines: &'a [String], level: &str) -> Vec<&'a String> {
    let needle = format!("level={level}");
    lines
        .iter()
        .filter(|l| l.contains(&needle) && l.contains("[menu]"))
        .collect()
}

/// 解放を無視したときの `trace!` がちょうど 1 行で、理由が載っていることを確かめる。
fn assert_ignored(lines: &[String], reason: &str) {
    let traces = menu_lines(lines, "TRACE");
    assert_eq!(traces.len(), 1, "{lines:?}");
    assert!(traces[0].contains("ignored release"), "{:?}", traces[0]);
    assert!(
        traces[0].contains(&format!("reason=\"{reason}\"")),
        "{:?}",
        traces[0]
    );
}

/// 届いた照会を取り出し、問い合わせた名前と返信端を返す。
fn take_query(
    kanade: &Receiver<KanadeMsg>,
) -> (Vec<&'static str>, areka_actor::ReplySender<QueryReply>) {
    match kanade.try_recv().expect("照会が 1 件届く") {
        KanadeMsg::ResourceQuery { ids, reply } => (ids, reply),
        _ => panic!("ResourceQuery を期待"),
    }
}

fn value(id: &'static str, body: &str) -> (&'static str, ResourceOutcome) {
    (id, ResourceOutcome::Value(body.to_string()))
}

/// 既定名のままの計画（説明書・区切り線・終了）。
fn default_entries() -> Vec<PlanEntry> {
    vec![
        PlanEntry::Item {
            id: 1,
            label: "説明書".to_string(),
            enabled: true,
            checked: false,
        },
        PlanEntry::Separator,
        PlanEntry::Item {
            id: 2,
            label: "終了".to_string(),
            enabled: true,
            checked: false,
        },
    ]
}

/// 右ボタンの解放で、返事待ちがちょうど 1 件でき、照会が 1 件送られ、表示中の印が立つ。
#[test]
fn right_release_keeps_one_pending_query_and_sends_one_resource_query() {
    let mut f = fixture();
    let now = Instant::now();

    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));

    let (ids, _reply) = take_query(&f.kanade);
    assert_eq!(
        ids,
        [
            "sakura.popupmenu.visible",
            "readmebutton.caption",
            "closebutton.caption"
        ]
    );
    assert!(f.kanade.try_recv().is_err(), "照会はちょうど 1 件");

    assert!(in_flight(&f.world), "解放で表示中の印が立つ");
    let pending = menu_wiring(&f.world).pending.as_ref().expect("返事待ち");
    assert_eq!(pending.request.scope, 0);
    assert_eq!(pending.request.entity, f.window);
    assert_eq!(pending.request.hwnd, fake_hwnd());
    assert_eq!(pending.request.screen_pos, (1030, 2040));
    assert_eq!(pending.deadline, now + Duration::from_millis(1000));
    assert!(pending.rx.is_some(), "送れた照会は返事の受け口を持つ");
    let frames: Vec<Frame> = pending.snapshot.iter().map(|(frame, _)| *frame).collect();
    assert_eq!(
        frames,
        [Frame::Readme, Frame::Close],
        "右クリック時点の写し"
    );
}

/// 公開の入口は Bubble 相だけを処理する（Tunnel 相は伝播を続けるため常に `false`）。
#[test]
fn the_handler_only_acts_on_the_bubble_phase() {
    let mut f = fixture();
    let w = f.window;

    let tunnel = Phase::Tunnel(right_released());
    assert!(!on_char_pointer_released(&mut f.world, w, w, &tunnel));
    assert!(menu_wiring(&f.world).pending.is_none());

    let bubble = Phase::Bubble(right_released());
    assert!(on_char_pointer_released(&mut f.world, w, w, &bubble));
    assert!(menu_wiring(&f.world).pending.is_some());
}

/// 右以外のボタンの解放では何も起きない（預かりにも触れない）。
#[test]
fn a_release_of_another_button_does_nothing() {
    let mut f = fixture();
    defer_double_click(&mut f.world);
    let left = released(ButtonReleased {
        left: true,
        middle: true,
        ..Default::default()
    });

    let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
        handle_release(&mut f.world, f.window, &left, Instant::now())
    });

    assert!(!handled);
    assert!(menu_wiring(&f.world).pending.is_none());
    assert!(!in_flight(&f.world));
    assert!(f.kanade.try_recv().is_err(), "何も送らない");
    assert!(menu_lines(&lines, "TRACE").is_empty(), "{lines:?}");
    assert!(
        deferred_double_click_remains(&mut f.world),
        "右ボタンでない解放は預かりに触れない"
    );
}

/// 入力の結線が済む前（`MouseWiring` が無い）の右解放は、記録して何もしない（要件 1.8）。
#[test]
fn a_release_before_the_mouse_wiring_is_ignored() {
    let mut f = fixture();
    f.world.remove_non_send::<MouseWiring>();

    let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
        handle_release(&mut f.world, f.window, &right_released(), Instant::now())
    });

    assert!(!handled);
    assert_ignored(&lines, "not wired");
    assert!(menu_wiring(&f.world).pending.is_none());
    assert!(!in_flight(&f.world));
    assert!(f.kanade.try_recv().is_err(), "何も送らない");
}

/// メニューの結線が済む前（`MenuWiring` が無い）も同じ扱い（要件 1.8）。
#[test]
fn a_release_before_the_menu_wiring_is_ignored() {
    let mut f = fixture();
    f.world.remove_non_send::<MenuWiring>();

    let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
        handle_release(&mut f.world, f.window, &right_released(), Instant::now())
    });

    assert!(!handled);
    assert_ignored(&lines, "not wired");
    assert!(f.kanade.try_recv().is_err(), "何も送らない");
}

/// ドラッグの状態を待機へ戻す（テストが途中で落ちても必ず戻す）。
///
/// ドラッグの状態はスレッドごとの値で、テストの実行スレッドは使い回されることがある。
struct ResetDragState;

impl Drop for ResetDragState {
    fn drop(&mut self) {
        // 捕捉の持ち主は借用の外で手放す（wintf の `end_dragging` の規律）。
        end_dragging(Point { x: 0, y: 0 }, true);
        update_drag_state(|state| *state = DragState::Idle);
    }
}

/// 左ボタンを 1 度押して離した後（ドラッグの状態は `JustEnded` で休む）の右解放はメニューへ進む。
///
/// 製品には状態を待機へ戻す呼び手が無いので、`JustEnded` を「ドラッグ中」と読むと、最初の
/// 左クリック以降メニューが二度と出なくなる（実機確認 9.3 で発覚）。押下→解放は製品と同じ関数で踏む。
#[test]
fn a_release_after_a_finished_left_click_still_opens_the_menu() {
    let mut f = fixture();
    let _reset = ResetDragState;
    start_preparing(f.window, Point { x: 0, y: 0 }, HWND::default());
    end_dragging(Point { x: 0, y: 0 }, false);

    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        Instant::now()
    ));
    assert!(menu_wiring(&f.world).pending.is_some());
}

/// ドラッグの途中（左ボタンを押したまま）の右解放は、預かりを捨てて記録し何もしない（要件 1.9）。
#[test]
fn a_release_while_dragging_is_ignored_and_drops_the_deferred_double_click() {
    let mut f = fixture();
    defer_double_click(&mut f.world);
    let _reset = ResetDragState;
    start_preparing(f.window, Point { x: 0, y: 0 }, HWND::default());

    let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
        handle_release(&mut f.world, f.window, &right_released(), Instant::now())
    });

    assert!(!handled);
    assert_ignored(&lines, "dragging");
    assert!(menu_wiring(&f.world).pending.is_none());
    assert!(!in_flight(&f.world));
    assert!(f.kanade.try_recv().is_err(), "何も送らない");
    assert!(
        !deferred_double_click_remains(&mut f.world),
        "預かりは捨てられている"
    );
}

/// 返事待ち・表示中の右解放は記録するだけで、先の返事待ちにも預かりにも触れない。
#[test]
fn a_release_while_a_menu_is_in_flight_is_ignored_and_keeps_the_query_and_the_double_click() {
    let mut f = fixture();
    let first = Instant::now();
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        first
    ));
    let (_ids, _reply) = take_query(&f.kanade);
    defer_double_click(&mut f.world);

    let later = first + Duration::from_millis(300);
    let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
        handle_release(&mut f.world, f.window, &right_released(), later)
    });

    assert!(!handled);
    assert_ignored(&lines, "menu in flight");
    assert!(
        menu_lines(&lines, "TRACE")[0].contains("dropped_double_click=false"),
        "{lines:?}"
    );
    assert!(f.kanade.try_recv().is_err(), "2 件目の照会は送らない");
    assert!(in_flight(&f.world), "先の要求の印は立ったまま");
    let pending = menu_wiring(&f.world)
        .pending
        .as_ref()
        .expect("先の返事待ち");
    assert_eq!(
        pending.deadline,
        first + QUERY_TIMEOUT,
        "期限は先の要求のまま"
    );
    assert!(
        deferred_double_click_remains(&mut f.world),
        "預かりは先の照会の決着のために残っている"
    );
}

/// 返事がダブルクリックの間隔より遅いときの一連: 解放 1（照会）→ 返事はまだ → 2 度目の押下で
/// 預かる → 解放 2（旗が立っているので無視）→ 指定の表示可否が返る → その tick の結果を返す。
fn right_double_click_answered_late(visible: &str) -> (Fixture, Option<ReadyMenu>) {
    let mut f = fixture();
    let now = Instant::now();
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));
    let (_ids, reply) = take_query(&f.kanade);
    assert!(
        poll_once(&mut f.world, now + Duration::from_millis(16)).is_none(),
        "返事はまだ来ていない"
    );

    defer_double_click(&mut f.world);
    let second_release = now + Duration::from_millis(200);
    assert!(!handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        second_release
    ));
    assert!(f.kanade.try_recv().is_err(), "解放 2 は何も送らない");

    reply
        .send(vec![value("sakura.popupmenu.visible", visible)])
        .expect("受け口は生きている");
    let ready = poll_once(&mut f.world, now + Duration::from_millis(400));
    (f, ready)
}

/// 返事が遅くても、メニューを抑止するゴーストへ右ダブルクリックがちょうど 1 件届く（要件 1.10）。
#[test]
fn a_late_suppressing_reply_still_delivers_the_right_double_click() {
    let (mut f, ready) = right_double_click_answered_late("0");

    assert!(ready.is_none(), "抑止なので計画はできない");
    match f.kanade.try_recv().expect("右ダブルクリックが 1 件届く") {
        KanadeMsg::Mouse(m) => assert_eq!(
            m,
            MouseInput {
                scope: 0,
                x: 7,
                y: 4,
                region: Some("Bust".to_string()),
                kind: MouseEventKind::DoubleClick {
                    button: MouseButton::Right
                },
            }
        ),
        _ => panic!("Mouse(DoubleClick) を期待"),
    }
    assert!(f.kanade.try_recv().is_err(), "ちょうど 1 件");
    assert!(!in_flight(&f.world));
    assert!(!deferred_double_click_remains(&mut f.world));
}

/// 同じ一連でメニューを出すなら、右ダブルクリックは送らずに捨て、計画を作る。
#[test]
fn a_late_showing_reply_drops_the_right_double_click_and_builds_the_plan() {
    let (mut f, ready) = right_double_click_answered_late("1");

    let ready = ready.expect("計画ができる");
    assert_eq!(ready.plan.entries, default_entries());
    assert!(f.kanade.try_recv().is_err(), "右ダブルクリックは送らない");
    assert!(
        !deferred_double_click_remains(&mut f.world),
        "預かりは残らない"
    );
}

/// 画面座標へ写せなければ要求を捨てて記録する（預かりも捨てる）。
#[test]
fn a_release_whose_position_cannot_be_mapped_is_ignored() {
    let mut f = fixture_with(fake_hwnd(), failing_to_screen);
    defer_double_click(&mut f.world);

    let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
        handle_release(&mut f.world, f.window, &right_released(), Instant::now())
    });

    assert!(!handled);
    assert_ignored(&lines, "client_to_screen failed");
    assert!(menu_wiring(&f.world).pending.is_none());
    assert!(!in_flight(&f.world));
    assert!(f.kanade.try_recv().is_err(), "何も送らない");
    assert!(!deferred_double_click_remains(&mut f.world));
}

/// キャラクター窓の印や窓ハンドルを持たない entity での右解放も、記録して何もしない。
#[test]
fn a_release_on_an_entity_that_is_not_a_live_character_window_is_ignored() {
    let mut f = fixture();
    let unmarked = f.world.spawn_empty().id();
    let not_created_yet = f.world.spawn(CharWindowMarker { scope: 1 }).id();

    for (entity, reason) in [
        (unmarked, "not a character window"),
        (not_created_yet, "no window handle"),
    ] {
        defer_double_click(&mut f.world);
        let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
            handle_release(&mut f.world, entity, &right_released(), Instant::now())
        });

        assert!(!handled, "{reason}");
        assert_ignored(&lines, reason);
        assert!(menu_wiring(&f.world).pending.is_none(), "{reason}");
        assert!(!in_flight(&f.world), "{reason}");
        assert!(f.kanade.try_recv().is_err(), "何も送らない: {reason}");
        assert!(!deferred_double_click_remains(&mut f.world), "{reason}");
    }
}

/// 返事待ちが無い tick は何もしない。
#[test]
fn polling_without_a_pending_query_does_nothing() {
    let mut f = fixture();
    let (ready, lines) = capture_lines(LineFormat::LevelFields, || {
        poll_once(&mut f.world, Instant::now())
    });
    assert!(ready.is_none());
    assert!(lines.iter().all(|l| !l.contains("[menu]")), "{lines:?}");
    assert!(!in_flight(&f.world));
}

/// 返事が届いた tick で、項目名を写した計画ができる。表示中の印は計画を手放すまで立っている。
#[test]
fn the_tick_that_sees_the_reply_builds_the_plan_with_the_captions() {
    let mut f = fixture();
    let now = Instant::now();
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));
    let (_ids, reply) = take_query(&f.kanade);
    reply
        .send(vec![
            value("sakura.popupmenu.visible", "1"),
            value("readmebutton.caption", "よんでね(&R)"),
            (
                "closebutton.caption",
                ResourceOutcome::Value("おわる".to_string()),
            ),
        ])
        .expect("受け口は生きている");

    let ready = poll_once(&mut f.world, now + Duration::from_millis(16)).expect("計画ができる");

    assert_eq!(
        ready.plan.entries,
        [
            PlanEntry::Item {
                id: 1,
                label: "よんでね(&R)".to_string(),
                enabled: true,
                checked: false,
            },
            PlanEntry::Separator,
            PlanEntry::Item {
                id: 2,
                label: "おわる".to_string(),
                enabled: true,
                checked: false,
            },
        ]
    );
    assert_eq!(ready.request.screen_pos, (1030, 2040));
    assert_eq!(ready.request.entity, f.window);
    assert!(menu_wiring(&f.world).pending.is_none(), "返事待ちは片付く");
    assert!(in_flight(&f.world), "計画を持っている間は印が立っている");
    drop(ready);
    assert!(!in_flight(&f.world), "計画を手放すと印が降りる");
}

/// 表示可否が `0` の tick では計画を作らず、預かっていた右ダブルクリックをちょうど 1 件送る。
#[test]
fn a_suppressed_tick_builds_no_plan_and_sends_the_deferred_double_click() {
    let mut f = fixture();
    let now = Instant::now();
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));
    let (_ids, reply) = take_query(&f.kanade);
    defer_double_click(&mut f.world);
    reply
        .send(vec![
            value("sakura.popupmenu.visible", "0"),
            value("closebutton.caption", "おわる"),
        ])
        .expect("受け口は生きている");

    let (ready, lines) = capture_lines(LineFormat::LevelFields, || poll_once(&mut f.world, now));

    assert!(ready.is_none(), "抑止の tick では計画ができない");
    assert_eq!(
        menu_lines(&lines, "INFO").len(),
        1,
        "抑止の記録は 1 行: {lines:?}"
    );
    assert!(menu_wiring(&f.world).pending.is_none());
    assert!(!in_flight(&f.world), "抑止で終わったら印が降りる");
    match f.kanade.try_recv().expect("右ダブルクリックが 1 件届く") {
        KanadeMsg::Mouse(m) => assert_eq!(
            m,
            MouseInput {
                scope: 0,
                x: 7,
                y: 4,
                region: Some("Bust".to_string()),
                kind: MouseEventKind::DoubleClick {
                    button: MouseButton::Right
                },
            }
        ),
        _ => panic!("Mouse(DoubleClick) を期待"),
    }
    assert!(f.kanade.try_recv().is_err(), "ちょうど 1 件");
    assert!(!deferred_double_click_remains(&mut f.world));
}

/// 抑止されても、預かりが無ければ何も送らない。
#[test]
fn a_suppressed_tick_without_a_deferred_double_click_sends_nothing() {
    let mut f = fixture();
    let now = Instant::now();
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));
    let (_ids, reply) = take_query(&f.kanade);
    reply
        .send(vec![value("sakura.popupmenu.visible", "0")])
        .expect("受け口は生きている");

    assert!(poll_once(&mut f.world, now).is_none());

    assert!(f.kanade.try_recv().is_err(), "何も送らない");
    assert!(!in_flight(&f.world));
}

/// メニューを出すなら、預かっていた右ダブルクリックは送らずに捨てる（要件 1.10）。
#[test]
fn a_shown_menu_drops_the_deferred_double_click_without_sending_it() {
    let mut f = fixture();
    let now = Instant::now();
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));
    let (_ids, reply) = take_query(&f.kanade);
    defer_double_click(&mut f.world);
    reply
        .send(vec![value("sakura.popupmenu.visible", "1")])
        .expect("受け口は生きている");

    let ready = poll_once(&mut f.world, now);

    assert!(ready.is_some());
    assert!(f.kanade.try_recv().is_err(), "右ダブルクリックは送らない");
    assert!(
        !deferred_double_click_remains(&mut f.world),
        "預かりは残らない"
    );
}

/// 返事が来ないまま期限内なら待ち続け、期限を過ぎたら全件を既定名にして計画を作る（要件 3.4）。
#[test]
fn an_unanswered_query_waits_until_the_deadline_then_falls_back_to_the_default_labels() {
    let mut f = fixture();
    let now = Instant::now();
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));
    let (_ids, _reply) = take_query(&f.kanade);

    let (ready, lines) = capture_lines(LineFormat::LevelFields, || {
        poll_once(&mut f.world, now + Duration::from_millis(999))
    });
    assert!(ready.is_none(), "期限内は待つ");
    assert!(menu_wiring(&f.world).pending.is_some(), "返事待ちは残る");
    assert!(in_flight(&f.world), "印は立ったまま");
    assert!(lines.iter().all(|l| !l.contains("[menu]")), "{lines:?}");

    let (ready, lines) = capture_lines(LineFormat::LevelFields, || {
        poll_once(&mut f.world, now + Duration::from_millis(1001))
    });
    let ready = ready.expect("期限を過ぎたら計画ができる");
    assert_eq!(ready.plan.entries, default_entries());
    let warns = menu_lines(&lines, "WARN");
    assert_eq!(warns.len(), 1, "警告は 1 行: {lines:?}");
    assert!(warns[0].contains("timeout"), "{:?}", warns[0]);
    assert!(menu_wiring(&f.world).pending.is_none());
}

/// 照会を送れなかったら返事の受け口を持たない返事待ちを置き、次の tick で全件を既定名にする。
#[test]
fn a_query_that_cannot_be_sent_falls_back_on_the_next_tick() {
    let Fixture {
        mut world,
        kanade,
        window,
    } = fixture();
    drop(kanade);
    let now = Instant::now();

    let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
        handle_release(&mut world, window, &right_released(), now)
    });
    assert!(handled, "送れなくても要求は預かる");
    assert!(
        menu_lines(&lines, "WARN").is_empty(),
        "警告は次の tick の 1 行だけ: {lines:?}"
    );
    assert!(in_flight(&world));
    let pending = menu_wiring(&world).pending.as_ref().expect("返事待ち");
    assert!(pending.rx.is_none(), "送れなかった照会は受け口を持たない");

    let (ready, lines) = capture_lines(LineFormat::LevelFields, || poll_once(&mut world, now));
    let ready = ready.expect("次の tick で計画ができる");
    assert_eq!(ready.plan.entries, default_entries());
    let warns = menu_lines(&lines, "WARN");
    assert_eq!(warns.len(), 1, "警告は 1 行: {lines:?}");
    assert!(warns[0].contains("send_failed"), "{:?}", warns[0]);
}

/// 返事を待っている間に窓が消えていたら、記録して捨てる（計画を作らない・預かりも捨てる）。
#[test]
fn a_window_that_disappeared_while_waiting_drops_the_request() {
    // この窓は途中で消す。消すと wintf が窓ハンドルへ `WM_CLOSE` を投函するので、実在の窓を
    // 指しえない空のハンドルにしておく（空のハンドル宛ての投函は呼んだスレッド自身の
    // メッセージ待ち行列に入るだけで、テストのスレッドはそれを汲まない）。
    let mut f = fixture_with(HWND(std::ptr::null_mut()), shifted_to_screen);
    let now = Instant::now();
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));
    let (_ids, reply) = take_query(&f.kanade);
    reply
        .send(vec![value("sakura.popupmenu.visible", "1")])
        .expect("受け口は生きている");
    defer_double_click(&mut f.world);
    f.world.despawn(f.window);

    let (ready, lines) = capture_lines(LineFormat::LevelFields, || poll_once(&mut f.world, now));

    assert!(ready.is_none(), "窓が無ければ計画を作らない");
    let debugs = menu_lines(&lines, "DEBUG");
    assert_eq!(debugs.len(), 1, "{lines:?}");
    assert!(debugs[0].contains("window gone"), "{:?}", debugs[0]);
    assert!(menu_wiring(&f.world).pending.is_none());
    assert!(!in_flight(&f.world), "捨てたら印が降りる");
    assert!(f.kanade.try_recv().is_err(), "何も送らない");
    assert!(!deferred_double_click_remains(&mut f.world));
}

/// 返事が速いとき（ふつうの場合）の右ダブルクリック: 解放 1 の照会は 2 度目の押下より前に
/// 決着し（預かりはまだ無いので何も送らない）、2 度目の押下で預かり、解放 2 が 2 件目の要求として
/// 受け付けられ、その照会の決着が預かりを送る（要件 1.10）。
///
/// 受け付けた解放が預かりに触れると、2 件目の決着の時点で送る材料が無くなり、最後の表明が落ちる。
#[test]
fn a_fast_suppressing_reply_still_delivers_the_right_double_click_through_the_second_request() {
    let mut f = fixture();
    let now = Instant::now();

    // 解放 1: 照会はすぐ決着する。預かりはまだ無いので何も送らない。
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now
    ));
    let (_ids, reply) = take_query(&f.kanade);
    reply
        .send(vec![value("sakura.popupmenu.visible", "0")])
        .expect("受け口は生きている");
    assert!(poll_once(&mut f.world, now + Duration::from_millis(16)).is_none());
    assert!(f.kanade.try_recv().is_err(), "預かりが無いので何も送らない");
    assert!(!in_flight(&f.world), "1 件目が終わって印が降りている");

    // 2 度目の押下で預かり、解放 2 は 2 件目の要求として受け付けられる。
    defer_double_click(&mut f.world);
    assert!(handle_release(
        &mut f.world,
        f.window,
        &right_released(),
        now + Duration::from_millis(200)
    ));
    let (_ids, reply) = take_query(&f.kanade);
    assert!(f.kanade.try_recv().is_err(), "2 件目の照会もちょうど 1 件");
    let deferred = f
        .world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は挿入済み")
        .take_pending_right_double_click()
        .expect("受け付けた解放は預かりに触れない");
    // 確かめるために取り出したので、同じものを戻す。
    f.world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は挿入済み")
        .defer_right_double_click(deferred);

    // 2 件目の決着が預かりを送る。
    reply
        .send(vec![value("sakura.popupmenu.visible", "0")])
        .expect("受け口は生きている");
    assert!(poll_once(&mut f.world, now + Duration::from_millis(216)).is_none());
    match f.kanade.try_recv().expect("右ダブルクリックが 1 件届く") {
        KanadeMsg::Mouse(m) => assert_eq!(
            m,
            MouseInput {
                scope: 0,
                x: 7,
                y: 4,
                region: Some("Bust".to_string()),
                kind: MouseEventKind::DoubleClick {
                    button: MouseButton::Right
                },
            }
        ),
        _ => panic!("Mouse(DoubleClick) を期待"),
    }
    assert!(f.kanade.try_recv().is_err(), "ちょうど 1 件");
    assert!(!in_flight(&f.world));
    assert!(!deferred_double_click_remains(&mut f.world));
}
