//! 本番の結線（`wire_menu`）を通した一気通貫の決定論テスト（要件 2.2・3.3・3.5・4.3・5.1・9.6）。
//!
//! 結線 → 窓を作る → 解放ハンドラを付ける（本番と同じ順）→ 右ボタンの解放 → 照会への返事 → 返事の取り出し → 計画 → 「終了」の動作、までを
//! 組込の 2 項目そのもので通す。OS のメニューは出さず、表示のタスクも起こさない。画面座標への
//! 写しだけは実窓が要るので、テスト用の関数を差し込んだ結線状態を渡す。
//!
//! 期待値（項目名・識別子・有効／無効）はすべて書き下した値で、実装の表から導かない。

use std::sync::mpsc::channel;
use std::time::Duration;

use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{CloseReason, KanadeMsg};
use bevy_ecs::schedule::Schedules;
use temp_path_kit::TempPath;
use windows::Win32::Foundation::HINSTANCE;
use wintf::ecs::Point;
use wintf::ecs::pointer::{ButtonReleased, OnPointerReleased};

use super::*;
use crate::emo2_boot::hit_region::HitRegion;
use crate::input_events::RegionSource;
use crate::menu::plan::PlanEntry;
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

/// 相方側の窓で右クリック → ゴーストが「説明書」の名前だけを返す → 計画は
/// 「説明書（無効）・区切り線・終了」になり、「終了」を選ぶと相方側のスコープの終了指示が
/// ちょうど 1 件送られる。
#[test]
fn a_right_click_on_the_wired_menu_ends_in_one_close_request() {
    let (menu_tx, menu_rx) = channel::<KanadeMsg>();
    let (mouse_tx, mouse_rx) = channel::<KanadeMsg>();
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(MouseWiring::new(mouse_tx, RegionSource::Mock(no_region)));
    // 説明書のファイルは置かない（「説明書」は無効で出る）。
    let dir = TempPath::new("menu-wired-flow");
    let (_readme_tx, readme_rx) = channel::<crate::readme::ReadmeRequest>();
    crate::readme::wire_readme(&mut world, dir.child("readme.txt"), readme_rx);
    // 本番と同じ順: 結線の時点で窓はまだ無く、窓を作った直後に解放ハンドラを付ける。
    crate::menu::wire_menu_with(
        &mut world,
        MenuWiring::with_to_screen(menu_tx, shifted_to_screen),
    );
    let kero = world
        .spawn((
            CharWindowMarker { scope: 1 },
            WindowHandle {
                hwnd: HWND(0x1234 as *mut _),
                instance: HINSTANCE::default(),
            },
        ))
        .id();
    crate::menu::attach_release_handlers(&mut world);
    assert!(
        world.get::<OnPointerReleased>(kero).is_some(),
        "窓を作った後の装着で付く"
    );

    let now = Instant::now();
    let release = PointerState {
        client_point: Point { x: 30, y: 40 },
        released: ButtonReleased {
            right: true,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(handle_release(&mut world, kero, &release, now));

    let reply = match menu_rx.try_recv().expect("照会が 1 件届く") {
        KanadeMsg::ResourceQuery { ids, reply } => {
            assert_eq!(
                ids,
                [
                    "kero.popupmenu.visible",
                    "readmebutton.caption",
                    "closebutton.caption"
                ]
            );
            reply
        }
        _ => panic!("ResourceQuery を期待"),
    };
    reply
        .send(vec![
            ("kero.popupmenu.visible", ResourceOutcome::NoContent),
            (
                "readmebutton.caption",
                ResourceOutcome::Value("取扱説明書(&R)".to_string()),
            ),
            ("closebutton.caption", ResourceOutcome::Value(String::new())),
        ])
        .expect("受け口は生きている");

    let ready = poll_once(&mut world, now + Duration::from_millis(16)).expect("計画ができる");
    assert_eq!(ready.request.scope, 1);
    assert_eq!(ready.request.screen_pos, (1030, 2040));
    assert_eq!(
        ready.plan.entries,
        [
            PlanEntry::Item {
                id: 1,
                label: "取扱説明書(&R)".to_string(),
                enabled: false,
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
    );

    let (frame, action) = ready.plan.action(2).expect("識別子 2 は終了");
    assert_eq!(frame, Frame::Close);
    action(
        &mut world,
        &MenuContext {
            scope: ready.request.scope,
        },
    );

    assert!(
        matches!(
            mouse_rx.try_recv().expect("終了指示が 1 件届く"),
            KanadeMsg::CloseRequest {
                reason: CloseReason::User { scope: 1 }
            }
        ),
        "メニューを出した窓（相方側）のスコープが載る"
    );
    assert!(mouse_rx.try_recv().is_err(), "終了指示はちょうど 1 件");
    assert!(
        menu_rx.try_recv().is_err(),
        "メニューの送り口には照会のほか何も流れない"
    );
}
