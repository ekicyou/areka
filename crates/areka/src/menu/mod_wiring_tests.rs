//! メニューの結線（[`wire_menu`]）と組込 2 項目の決定論テスト
//! （要件 2.2・4.3・5.1・6.2・6.6・9.6・11.4）。
//!
//! 確かめること: 結線の後に「説明書」「終了」が写しへ現れること・「説明書」の有効／無効が
//! 写しを取った時点のファイルの在否で決まること・「終了」の動作が終了指示をちょうど 1 件送る
//! こと（受信側で数える）・解放ハンドラがキャラクター窓にだけ付き枚数が記録されること・結線
//! そのものは窓に触れないこと・毎 tick の取り出しが入力の段へ登録されること・後続 spec 向けの
//! 登記の口が結線の有無で振る舞いを変えること。
//!
//! 本番では結線（`wire_menu`）が先で、窓はその後に作られて解放ハンドラが付く。窓を使うテストは
//! この順に組む。
//!
//! 「説明書」の動作は、説明書の結線が無い World でだけ呼ぶ。結線があると既定のアプリが実際に
//! 起動してしまうからである（ファイルが無くても OS の関数までは届く）。
//!
//! 期待値（項目名・リソース名・記録の綴り）はすべて書き下した値で、実装の表から導かない。

use std::sync::mpsc::{Receiver, channel};

use areka_kanade::CloseReason;
use bevy_ecs::schedule::Schedules;
use log_capture_kit::{LineFormat, capture_lines};
use temp_path_kit::TempPath;
use windows::Win32::Foundation::HINSTANCE;
use wintf::ecs::pointer::{ButtonReleased, OnPointerReleased, Phase, PointerState};
use wintf::ecs::{Input, Point, WindowHandle};

use super::*;
use crate::emo2_boot::hit_region::HitRegion;
use crate::input_events::{MouseWiring, RegionSource};
use crate::placement::spawn::CharWindowMarker;

/// 当たり判定の代役（このテストでは当たり判定を引かないので中身は使われない）。
fn no_region(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: None,
        surface_point: (x, y),
    }
}

/// 画面座標への写しの代役（実窓が無いので OS には聞けない）。
fn shifted_to_screen(_hwnd: HWND, x: i32, y: i32) -> Option<(i32, i32)> {
    Some((x + 1000, y + 2000))
}

/// スケジュールの入れ物だけを持つ World（結線は入力の段への登録を伴う）。
fn empty_world() -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world
}

/// メニューを結線した World と、運行（kanade）宛ての受信端。
fn wired_world() -> (World, Receiver<KanadeMsg>) {
    let (tx, rx) = channel::<KanadeMsg>();
    let mut world = empty_world();
    wire_menu(&mut world, tx);
    (world, rx)
}

/// 指定のスコープで写しを取る。
fn snapshot(world: &World, scope: u32) -> Vec<(Frame, MenuItem)> {
    world
        .get_non_send::<MenuWiring>()
        .expect("MenuWiring は結線で入る")
        .registry
        .snapshot(world, &MenuContext { scope })
}

/// 写しから指定の枠の項目を取り出す。
fn item_of(world: &World, frame: Frame) -> MenuItem {
    snapshot(world, 0)
        .into_iter()
        .find_map(|(f, item)| (f == frame).then_some(item))
        .expect("枠は登記済み")
}

/// 指定の枠の項目の動作を取り出す。
fn action_of(world: &World, frame: Frame) -> MenuAction {
    match item_of(world, frame).body {
        ItemBody::Action(action) => action,
        ItemBody::Submenu(_) => panic!("組込の項目は動作を持つ葉"),
    }
}

/// 入力の結線を World へ入れ、その送り先の受信端を返す。
fn insert_mouse_wiring(world: &mut World) -> Receiver<KanadeMsg> {
    let (tx, rx) = channel::<KanadeMsg>();
    world.insert_non_send(MouseWiring::new(tx, RegionSource::Mock(no_region)));
    rx
}

/// 偽の窓ハンドルを持つ本体側のキャラクター窓を作る（OS へは渡らない）。
fn spawn_char_window(world: &mut World) -> Entity {
    world
        .spawn((
            CharWindowMarker { scope: 0 },
            WindowHandle {
                hwnd: HWND(0x1234 as *mut _),
                instance: HINSTANCE::default(),
            },
        ))
        .id()
}

/// クライアント座標 (30, 40) での右ボタンの解放（Bubble 相）。
fn right_release() -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x: 30, y: 40 },
        released: ButtonReleased {
            right: true,
            ..Default::default()
        },
        ..Default::default()
    })
}

/// 指定の出来事の行だけを拾う。
fn lines_of<'a>(lines: &'a [String], event: &str) -> Vec<&'a String> {
    lines.iter().filter(|l| l.contains(event)).collect()
}

/// 結線の後、写しには「説明書」と「終了」がこの順で現れる（要件 2.2）。
#[test]
fn wiring_registers_the_readme_and_close_items() {
    let (world, _rx) = wired_world();

    let items: Vec<(Frame, String, Option<&'static str>)> = snapshot(&world, 0)
        .into_iter()
        .map(|(frame, item)| (frame, item.label, item.caption_resource))
        .collect();
    assert_eq!(
        items,
        [
            (
                Frame::Readme,
                "説明書".to_string(),
                Some("readmebutton.caption")
            ),
            (
                Frame::Close,
                "終了".to_string(),
                Some("closebutton.caption")
            ),
        ]
    );

    let close = item_of(&world, Frame::Close);
    assert!(close.enabled, "終了は常に選べる");
    assert_eq!(close.checked, None);
    assert_eq!(item_of(&world, Frame::Readme).checked, None);
}

/// 「説明書」の有効／無効は、写しを取った時点のファイルの在否で決まる（要件 4.3・6.2・11.4）。
/// 結線し直さなくても、ファイルを置けば次の写しから有効になる。
#[test]
fn the_readme_item_follows_the_file_at_snapshot_time() {
    let (mut world, _rx) = wired_world();
    assert!(
        !item_of(&world, Frame::Readme).enabled,
        "説明書の結線が無ければ無効"
    );

    let dir = TempPath::new("menu-readme-item");
    let path = dir.child("readme.txt");
    let (_tx, requests) = channel::<crate::readme::ReadmeRequest>();
    crate::readme::wire_readme(&mut world, path.clone(), requests);
    assert!(
        !item_of(&world, Frame::Readme).enabled,
        "ファイルが無ければ無効"
    );

    std::fs::write(&path, "readme").expect("説明書を置く");
    assert!(
        item_of(&world, Frame::Readme).enabled,
        "ファイルを置いた後の写しは有効"
    );
}

/// 「終了」の動作は、メニューを出した窓のスコープを載せた終了指示をちょうど 1 件送る
/// （要件 5.1・6.6・9.6）。
#[test]
fn the_close_action_sends_exactly_one_close_request_with_the_window_scope() {
    for scope in [1, 0] {
        let (mut world, menu_rx) = wired_world();
        let mouse_rx = insert_mouse_wiring(&mut world);

        let action = action_of(&world, Frame::Close);
        action(&mut world, &MenuContext { scope });

        match mouse_rx.try_recv().expect("終了指示が 1 件届く") {
            KanadeMsg::CloseRequest {
                reason: CloseReason::User { scope: sent },
            } => assert_eq!(sent, scope),
            _ => panic!("CloseRequest{{User}} を期待"),
        }
        assert!(mouse_rx.try_recv().is_err(), "終了指示はちょうど 1 件");
        assert!(
            menu_rx.try_recv().is_err(),
            "メニュー自身の送り口からは何も送らない"
        );
    }
}

/// 入力の結線が無いときの「終了」は、記録して何もしない。
#[test]
fn the_close_action_without_the_mouse_wiring_warns_and_sends_nothing() {
    let (mut world, menu_rx) = wired_world();
    let action = action_of(&world, Frame::Close);

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        action(&mut world, &MenuContext { scope: 0 });
    });

    let warns = lines_of(&lines, "menu_close_no_mouse_wiring");
    assert_eq!(warns.len(), 1, "{lines:?}");
    assert!(warns[0].contains("level=WARN"), "{:?}", warns[0]);
    assert!(menu_rx.try_recv().is_err(), "何も送らない");
}

/// 「説明書」の動作は説明書を開く関数へ届く。説明書の結線が無い World では、その関数が
/// 「結線が無い」を記録して終える（アプリは起動しない）。
#[test]
fn the_readme_action_routes_to_the_readme_opener() {
    let (mut world, _rx) = wired_world();
    let action = action_of(&world, Frame::Readme);

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        action(&mut world, &MenuContext { scope: 0 });
    });

    let warns = lines_of(&lines, "readme_open_no_wiring");
    assert_eq!(warns.len(), 1, "{lines:?}");
    assert!(warns[0].contains("level=WARN"), "{:?}", warns[0]);
}

/// 解放ハンドラはキャラクター窓にだけ付き、付けた枚数が記録される。2 度呼んでも変わらない。
#[test]
fn release_handlers_attach_to_character_windows_only() {
    let mut world = World::new();
    let sakura = world.spawn(CharWindowMarker { scope: 0 }).id();
    let kero = world.spawn(CharWindowMarker { scope: 1 }).id();
    let other = world.spawn_empty().id();

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        attach_release_handlers(&mut world);
    });
    attach_release_handlers(&mut world);

    assert!(world.get::<OnPointerReleased>(sakura).is_some());
    assert!(world.get::<OnPointerReleased>(kero).is_some());
    assert!(world.get::<OnPointerReleased>(other).is_none());
    let attached = lines_of(&lines, "menu_release_handlers_attached");
    assert_eq!(attached.len(), 1, "{lines:?}");
    assert!(attached[0].contains("level=DEBUG"), "{:?}", attached[0]);
    assert!(attached[0].contains("count=2"), "{:?}", attached[0]);
}

/// キャラクター窓が 1 枚も無いときに呼ぶと警告する（窓より先に呼んだ＝メニューが出ない）。
#[test]
fn attaching_to_zero_windows_warns() {
    let mut world = World::new();
    world.spawn_empty();

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        attach_release_handlers(&mut world);
    });

    let attached = lines_of(&lines, "menu_release_handlers_attached");
    assert_eq!(attached.len(), 1, "{lines:?}");
    assert!(attached[0].contains("level=WARN"), "{:?}", attached[0]);
    assert!(attached[0].contains("count=0"), "{:?}", attached[0]);
}

/// 結線そのものは窓に触れない。本番では結線の時点でキャラクター窓がまだ無く、窓は後から
/// 作られる——解放ハンドラを付けるのは、窓を作った側が呼ぶ [`attach_release_handlers`] だけである。
#[test]
fn wiring_alone_attaches_nothing_to_windows_spawned_before_or_after() {
    let (tx, _rx) = channel::<KanadeMsg>();
    let mut world = empty_world();
    let before = world.spawn(CharWindowMarker { scope: 0 }).id();

    let ((), lines) = capture_lines(LineFormat::LevelFields, || wire_menu(&mut world, tx));
    let after = world.spawn(CharWindowMarker { scope: 1 }).id();

    assert!(world.get_non_send::<MenuWiring>().is_some());
    assert!(world.get::<OnPointerReleased>(before).is_none());
    assert!(world.get::<OnPointerReleased>(after).is_none());
    assert!(
        lines_of(&lines, "menu_release_handlers_attached").is_empty(),
        "結線は解放ハンドラの装着を呼ばない: {lines:?}"
    );
}

/// 結線の無い World（boot に失敗した起動）で付いた解放ハンドラは、右解放を記録して無視する。
#[test]
fn an_attached_handler_without_any_wiring_ignores_the_release() {
    let mut world = World::new();
    let window = spawn_char_window(&mut world);
    attach_release_handlers(&mut world);
    let handler = world.get::<OnPointerReleased>(window).expect("装着済み").0;

    let (handled, lines) = capture_lines(LineFormat::LevelFields, || {
        handler(&mut world, window, window, &right_release())
    });

    assert!(!handled);
    let ignored = lines_of(&lines, "menu_release_ignored");
    assert_eq!(ignored.len(), 1, "{lines:?}");
    assert!(
        ignored[0].contains("reason=\"not wired\""),
        "{:?}",
        ignored[0]
    );
}

/// 結線は入力の段へ system をちょうど 1 つ登録する。
#[test]
fn wiring_registers_one_system_into_the_input_schedule() {
    let (world, _rx) = wired_world();

    let schedules = world.resource::<Schedules>();
    let input = schedules.get(Input).expect("入力の段がある");
    assert_eq!(input.systems_len(), 1);
}

/// 登録された system は返事待ちを覗くものである。順序は本番と同じ: 結線（窓はまだ無い）→ 窓を
/// 作る → 解放ハンドラを付ける → 右解放。付いたハンドラで返事待ちを作り、返信端を捨ててから
/// 入力の段を 1 回回すと、返事待ちが片付いて表示中の印が降りる。この World には外側の World への
/// 参照が無いので、表示のタスクは起こされず、その旨が 1 行記録される。
#[test]
fn the_registered_system_polls_the_pending_query() {
    let (tx, rx) = channel::<KanadeMsg>();
    let mut world = empty_world();
    let _mouse_rx = insert_mouse_wiring(&mut world);
    wire_menu_with(
        &mut world,
        MenuWiring::with_to_screen(tx, shifted_to_screen),
    );
    let window = spawn_char_window(&mut world);
    attach_release_handlers(&mut world);

    let handler = world
        .get::<OnPointerReleased>(window)
        .expect("窓を作った後の装着で付く")
        .0;
    assert!(handler(&mut world, window, window, &right_release()));
    assert!(world.get_non_send::<MenuWiring>().unwrap().in_flight.get());
    // 照会を取り出して返信端ごと捨てる（返事は来ないと次の tick で分かる）。
    drop(rx.try_recv().expect("照会が 1 件届く"));

    let ((), lines) = capture_lines(LineFormat::LevelFields, || world.run_schedule(Input));

    let wiring = world.get_non_send::<MenuWiring>().unwrap();
    assert!(wiring.pending.is_none(), "返事待ちは片付いている");
    assert!(!wiring.in_flight.get(), "表示中の印は降りている");
    assert_eq!(
        lines_of(&lines, "menu_outer_world_ref_missing").len(),
        1,
        "{lines:?}"
    );
}

/// サブメニューの見出しを返す供給関数（後続 spec が登記する形）。
fn shell_supplier() -> Supplier {
    Rc::new(|_, _| MenuItem {
        label: "シェル".to_string(),
        caption_resource: None,
        enabled: true,
        checked: None,
        body: ItemBody::Submenu(Vec::new()),
    })
}

/// 後続 spec 向けの登記の口は、結線の前なら記録して何もしない。
#[test]
fn registering_before_the_wiring_warns_and_does_nothing() {
    let mut world = World::new();

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        register(&mut world, Frame::Shell, shell_supplier());
    });

    let warns = lines_of(&lines, "menu_register_no_wiring");
    assert_eq!(warns.len(), 1, "{lines:?}");
    assert!(warns[0].contains("level=WARN"), "{:?}", warns[0]);
    assert!(world.get_non_send::<MenuWiring>().is_none());
}

/// 結線の後に登記した枠は、枠の並び順の位置で写しに現れる。
#[test]
fn registering_after_the_wiring_places_the_frame_in_order() {
    let (mut world, _rx) = wired_world();

    register(&mut world, Frame::Shell, shell_supplier());

    let frames: Vec<Frame> = snapshot(&world, 0).into_iter().map(|(f, _)| f).collect();
    assert_eq!(frames, [Frame::Shell, Frame::Readme, Frame::Close]);
}
