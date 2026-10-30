//! シェルの切り替えでキャラクターの窓のカーソルの表が新しいシェルのものへ替わる決定論テスト
//! （shell-mouse-cursor 要件 6.8・tasks.md 4.3）。
//!
//! 土台は親（`switch_finish_tests.rs`）の頼んだ段の組み立てをそのまま使う。

use super::*;

/// スコープ 0 の当たり判定 `Head` に乗る形だけを `shape` にしたシェルのカーソル。
fn head_cursor(shape: crate::emo2_boot::shell_cursor_table::SystemCursor) -> ShellCursorAssets {
    use crate::emo2_boot::shell_cursor_table::{CursorScene, Shape};
    let mut assets = ShellCursorAssets::default();
    assets
        .rows
        .rows
        .insert((0, "Head".into(), CursorScene::Up), Shape::System(shape));
    assets
}

/// どこでも当たり判定 `Head`（スコープはそのまま）。
fn all_head(scope: u32, x: i64, y: i64) -> crate::emo2_boot::hit_region::HitRegion {
    crate::emo2_boot::hit_region::HitRegion {
        scope,
        region: Some("Head".to_owned()),
        surface_point: (x, y),
    }
}

/// シェルの切替の後のキャラクターの窓の形は新しいシェルのカーソルの表から選ばれ、前の表の形は
/// 同じ点・同じ状態のままでも返らない（shell-mouse-cursor 要件 6.8・tasks.md 4.3）。
///
/// # 非空虚性
/// 後始末がカーソルを `hand_to` しないと、前の表の help が返って赤。
#[test]
fn shell_switch_hands_the_new_cursor_table_to_the_char_window_answer() {
    use crate::emo2_boot::shell_cursor_table::SystemCursor;
    use windows::Win32::UI::WindowsAndMessaging::{IDC_CROSS, IDC_HELP};
    use wintf::ecs::cursor::{CursorAsk, CursorQuery, CursorReply, CursorTrigger, stock_cursor};

    let mut world = World::new();
    // 答えの関数は時計を読まないので、本番の構築子でよい。
    world.insert_non_send(crate::input_events::MouseWiring::new(
        mpsc::channel().0,
        crate::input_events::RegionSource::Mock(all_head),
    ));
    crate::input_events::char_watch::cursor_answer::register_char_cursor(&mut world);
    let window = world
        .spawn((
            wintf::ecs::Window::default(),
            crate::placement::spawn::CharWindowMarker { scope: 0 },
        ))
        .id();
    let SwapFinish::Shell {
        source,
        balloon,
        restored,
        boxes,
        ..
    } = shell_finish()
    else {
        unreachable!("shell_finish はシェルの残り")
    };
    let finish = SwapFinish::Shell {
        source,
        balloon,
        restored,
        boxes,
        cursors: head_cursor(SystemCursor::Cross),
    };
    let mut rig = rig_on(world, SkinKind::Shell, false, finish);
    // 起動の装着の相が入れた前のシェルのカーソル。
    head_cursor(SystemCursor::Help).hand_to(&mut rig.world);
    let shown = |world: &World| {
        let ask = CursorAsk {
            trigger: CursorTrigger::Move,
            client: (10, 10),
            see_through: false,
            buttons_down: false,
            dpi: 96,
        };
        match (world.resource::<CursorQuery>().0)(world, window, &ask) {
            Some(CursorReply::Show(h)) => h,
            other => panic!("Show でない: {other:?}"),
        }
    };
    let before = shown(&rig.world);

    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    frame(&mut rig);

    assert_eq!(
        (before, shown(&rig.world), stage(&rig.world)),
        (
            stock_cursor(IDC_HELP).unwrap(),
            stock_cursor(IDC_CROSS).unwrap(),
            None
        ),
        "(切替の前, 切替の後, 印)"
    );
}
