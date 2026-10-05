//! 窓なしの World のテスト（公開の口: 範囲の登録・差し替え）。

use super::*;
// 公開の型が crate::ecs の口から見えることも、ここで確かめる。
use crate::ecs::{
    OnTooltip, PointF, Rect, TooltipArea, TooltipEndReason, TooltipNotice, TooltipOsError,
    TooltipRange, TooltipRangeId, TooltipRegisterError, TooltipSupply, TooltipTurn,
    TooltipTurnToken, Window,
};

fn range(area: TooltipArea, text: Option<&str>) -> TooltipRange {
    TooltipRange {
        area,
        text: text.map(str::to_owned),
    }
}

fn window(world: &mut World) -> Entity {
    world.spawn(Window::default()).id()
}

fn table(world: &World, w: Entity) -> &ranges::TooltipRanges {
    world
        .get::<ranges::TooltipRanges>(w)
        .expect("範囲の表が窓に付いているはず")
}

#[test]
fn register_attaches_table_to_window() {
    let mut world = World::new();
    let w = window(&mut world);
    let r = range(TooltipArea::WholeWindow, Some("説明"));

    let id = register(&mut world, w, r.clone()).expect("窓があるので登録できるはず");

    assert_eq!(id.window(), w);
    assert_eq!(table(&world, w).get(id), Some(&r));
    // 2 つ目は同じ表に足される（表を付け直さない）。
    let id2 = register(&mut world, w, range(TooltipArea::WholeWindow, None)).unwrap();
    assert!(table(&world, w).get(id).is_some());
    assert!(table(&world, w).get(id2).is_some());
}

#[test]
fn register_on_missing_window_is_error() {
    let mut world = World::new();
    let gone = window(&mut world);
    world.despawn(gone);
    let not_window = world.spawn_empty().id();

    for e in [gone, not_window] {
        let got = register(&mut world, e, range(TooltipArea::WholeWindow, Some("x")));
        assert!(
            matches!(got, Err(TooltipRegisterError::NoSuchWindow)),
            "{got:?}"
        );
    }
    assert!(world.get::<ranges::TooltipRanges>(not_window).is_none());
}

#[test]
fn update_replaces_content_and_keeps_order() {
    let mut world = World::new();
    let w = window(&mut world);
    let a = register(&mut world, w, range(TooltipArea::WholeWindow, Some("a"))).unwrap();
    let b = register(&mut world, w, range(TooltipArea::WholeWindow, Some("b"))).unwrap();
    let p = PointF::new(5.0, 5.0);

    // 先に登録した a を差し替えても、重なった所では後の b が勝つまま。
    let a2 = range(TooltipArea::WholeWindow, Some("a2"));
    assert!(update(&mut world, a, a2.clone()));
    assert_eq!(table(&world, w).get(a), Some(&a2));
    assert_eq!(table(&world, w).hit(p).map(|(k, _)| k), Some(b));

    // b を点に当たらない矩形へ差し替えると、a が当たる。
    let off = TooltipArea::Rect(Rect {
        left: 100.0,
        top: 100.0,
        right: 200.0,
        bottom: 200.0,
    });
    assert!(update(&mut world, b, range(off, Some("b2"))));
    assert_eq!(table(&world, w).hit(p).map(|(k, _)| k), Some(a));
}

#[test]
fn update_missing_is_false() {
    let mut world = World::new();
    let w = window(&mut world);
    let a = register(&mut world, w, range(TooltipArea::WholeWindow, Some("a"))).unwrap();
    // 別の窓の持ち手（表の無い窓）・消した窓の持ち手は偽。
    let other = window(&mut world);
    let other_id = {
        let mut t = ranges::TooltipRanges::default();
        t.add(other, range(TooltipArea::WholeWindow, None))
    };
    assert!(!update(
        &mut world,
        other_id,
        range(TooltipArea::WholeWindow, Some("x"))
    ));
    world.despawn(w);
    assert!(!update(
        &mut world,
        a,
        range(TooltipArea::WholeWindow, Some("x"))
    ));
}

#[test]
fn empty_text_is_not_stored() {
    let mut world = World::new();
    let w = window(&mut world);
    let id = register(&mut world, w, range(TooltipArea::WholeWindow, Some(""))).unwrap();
    assert_eq!(table(&world, w).get(id).unwrap().text, None);

    assert!(update(
        &mut world,
        id,
        range(TooltipArea::WholeWindow, Some("x"))
    ));
    assert!(update(
        &mut world,
        id,
        range(TooltipArea::WholeWindow, Some(""))
    ));
    assert_eq!(table(&world, w).get(id).unwrap().text, None);
}

#[test]
fn notice_callback_component_attaches_to_window() {
    fn on(_: &mut World, _: &TooltipNotice) {}
    let mut world = World::new();
    let w = window(&mut world);
    world.entity_mut(w).insert(OnTooltip(on));
    assert!(world.get::<OnTooltip>(w).is_some());

    // 公開の型の名前が crate::ecs から引けること（中身の組み立ては後の段で確かめる）。
    let _: Option<(
        TooltipTurn,
        TooltipSupply,
        TooltipOsError,
        TooltipEndReason,
        TooltipTurnToken,
        TooltipRangeId,
    )> = None;
}

/// 窓なしの World のテスト（画面更新ごとの判定: 集める → 状態機械）。
///
/// 時刻は基準の時刻からのミリ秒で渡し、実際の時間は待たない。OS の見本・可視・待ち時間の
/// 設定は閉包で渡し、呼ばれた回数を数える。待ち時間の設定は 400 ミリ秒（待ちは 800 ミリ秒）。
mod decide_tests {
    use super::super::geometry::PointPx;
    use super::super::os::OsSample;
    use super::super::system::{TooltipSession, decide, note_button_press, notice};
    use super::super::turn::Effect;
    use super::*;
    use crate::ecs::{DPI, GlobalArrangement, Point, PointerState, SizeI, WindowPos};
    use std::cell::Cell;
    use std::time::Duration;
    use windows_numerics::Matrix3x2;

    /// 窓の左上（画面）。
    const ORIGIN: (i32, i32) = (100, 200);

    struct Rig {
        world: World,
        session: TooltipSession,
        t0: Instant,
        /// 見えていない窓。
        hidden: Vec<Entity>,
        samples: Cell<u32>,
        visibles: Cell<u32>,
        hovers: Cell<u32>,
    }

    impl Rig {
        fn new() -> Self {
            Self {
                world: World::new(),
                session: TooltipSession::default(),
                t0: Instant::now(),
                hidden: Vec::new(),
                samples: Cell::new(0),
                visibles: Cell::new(0),
                hovers: Cell::new(0),
            }
        }

        /// 窓（左上 ORIGIN・大きさ 300×150 の物理ピクセル）を置く。
        fn window(&mut self, dpi: u16) -> Entity {
            self.world
                .spawn((
                    Window::default(),
                    WindowPos {
                        position: Some(Point {
                            x: ORIGIN.0,
                            y: ORIGIN.1,
                        }),
                        size: Some(SizeI {
                            width: 300,
                            height: 150,
                        }),
                        ..Default::default()
                    },
                    DPI::from_dpi(dpi, dpi),
                ))
                .id()
        }

        /// 窓の中（物理ピクセル・窓の左上から）に当たり判定のある子を置く。
        fn hit_child(&mut self, window: Entity, l: i32, t: i32, r: i32, b: i32) -> Entity {
            let (x, y) = (ORIGIN.0 as f32, ORIGIN.1 as f32);
            let child = self
                .world
                .spawn(GlobalArrangement {
                    transform: Matrix3x2::translation(x + l as f32, y + t as f32),
                    bounds: Rect {
                        left: x + l as f32,
                        top: y + t as f32,
                        right: x + r as f32,
                        bottom: y + b as f32,
                    },
                })
                .id();
            self.world.entity_mut(window).add_children(&[child]);
            child
        }

        fn pointer(&mut self, e: Entity) {
            self.world.entity_mut(e).insert(PointerState::default());
        }

        /// 1 回判定する。cursor は窓の左上からの物理ピクセル（None＝読めない）。
        fn run(&mut self, ms: u64, cursor: Option<(i32, i32)>, down: bool) -> Vec<Effect> {
            let now = self.t0 + Duration::from_millis(ms);
            let cursor = cursor.map(|(x, y)| PointPx {
                x: ORIGIN.0 + x,
                y: ORIGIN.1 + y,
            });
            let (samples, visibles, hovers, hidden) =
                (&self.samples, &self.visibles, &self.hovers, &self.hidden);
            decide(
                &mut self.world,
                &mut self.session,
                now,
                &mut || {
                    samples.set(samples.get() + 1);
                    OsSample {
                        cursor,
                        button_down: down,
                    }
                },
                &|_, e| {
                    visibles.set(visibles.get() + 1);
                    !hidden.contains(&e)
                },
                &mut || {
                    hovers.set(hovers.get() + 1);
                    Duration::from_millis(400)
                },
            )
        }

        fn at(&mut self, ms: u64, x: i32, y: i32) -> Vec<Effect> {
            self.run(ms, Some((x, y)), false)
        }
    }

    fn rect(l: f32, t: f32, r: f32, b: f32) -> TooltipArea {
        TooltipArea::Rect(Rect {
            left: l,
            top: t,
            right: r,
            bottom: b,
        })
    }

    fn reg(rig: &mut Rig, w: Entity, area: TooltipArea, text: Option<&str>) -> TooltipRangeId {
        register(
            &mut rig.world,
            w,
            TooltipRange {
                area,
                text: text.map(str::to_owned),
            },
        )
        .unwrap()
    }

    fn deadlines(fx: &[Effect]) -> Vec<Instant> {
        fx.iter()
            .filter_map(|e| match e {
                Effect::ArmDeadline(t) => Some(*t),
                _ => None,
            })
            .collect()
    }

    fn started(fx: &[Effect]) -> Option<TooltipTurn> {
        fx.iter().find_map(|e| match notice(e) {
            Some(TooltipNotice::TurnStarted(turn)) => Some(turn),
            _ => None,
        })
    }

    fn ended(fx: &[Effect]) -> Vec<TooltipEndReason> {
        fx.iter()
            .filter_map(|e| match notice(e) {
                Some(TooltipNotice::TurnEnded { reason, .. }) => Some(reason),
                _ => None,
            })
            .collect()
    }

    /// 窓（DPI 96）と、全体を覆う当たり判定の子（ポインタの状態つき）を置く。
    fn rig_with_window() -> (Rig, Entity, Entity) {
        let mut rig = Rig::new();
        let w = rig.window(96);
        let child = rig.hit_child(w, 0, 0, 300, 150);
        rig.pointer(child);
        (rig, w, child)
    }

    /// rig_with_window に範囲 (10,10)-(60,40) を足す。
    fn rig_with_range(text: Option<&str>) -> (Rig, Entity, Entity, TooltipRangeId) {
        let (mut rig, w, child) = rig_with_window();
        let id = reg(&mut rig, w, rect(10.0, 10.0, 60.0, 40.0), text);
        (rig, w, child, id)
    }

    /// (30,20) で入り、800 ミリ秒で出す番にする。
    fn start(rig: &mut Rig) -> TooltipTurn {
        rig.at(0, 30, 20);
        started(&rig.at(800, 30, 20)).expect("800 ミリ秒で来るはず")
    }

    #[test]
    fn without_any_range_table_os_is_never_called() {
        let (mut rig, w, _) = rig_with_window();
        for ms in [0, 100, 1000] {
            assert!(rig.at(ms, 30, 20).is_empty());
        }
        assert_eq!(rig.samples.get(), 0, "OS の見本を呼んだ");
        assert_eq!(rig.visibles.get(), 0, "可視を読んだ");
        assert_eq!(rig.hovers.get(), 0, "待ち時間の設定を読んだ");

        // 較正: 範囲の表があれば、判定ごとに 1 回だけ見本を読む。
        reg(&mut rig, w, TooltipArea::WholeWindow, None);
        rig.at(1100, 30, 20);
        assert_eq!(rig.samples.get(), 1);
    }

    #[test]
    fn notice_carries_window_range_logical_position_token_and_text_flag() {
        let (mut rig, w, _) = rig_with_window();
        // 先に登録した窓の全体の範囲（文字なし）は、重なった所では後の矩形に負ける。
        let whole = reg(&mut rig, w, TooltipArea::WholeWindow, None);
        let id = reg(&mut rig, w, rect(10.0, 10.0, 60.0, 40.0), Some("説明"));

        let fx = rig.at(0, 30, 20);
        assert_eq!(
            deadlines(&fx),
            vec![rig.t0 + Duration::from_millis(800)],
            "{fx:?}"
        );
        assert!(started(&rig.at(799, 30, 20)).is_none());
        let fx = rig.at(800, 30, 20);
        let turn = started(&fx).expect("来るはず");
        let token = fx
            .iter()
            .find_map(|e| match e {
                Effect::TurnStarted { token, .. } => Some(*token),
                _ => None,
            })
            .unwrap();
        assert_eq!(turn.window, w);
        assert_eq!(turn.range, id);
        assert_eq!(turn.position, PointF::new(30.0, 20.0));
        assert_eq!(turn.token, token);
        assert!(turn.has_text);
        let note = rig.session.note.expect("出す番の控えがあるはず");
        assert_eq!(note.token, token);
        assert!(note.notify.is_none(), "知らせの関数の無い窓");

        // 矩形の外（全体の範囲だけ）へ出ると終わり、文字なしの範囲で次が来る。
        assert_eq!(
            ended(&rig.at(900, 200, 100)),
            vec![TooltipEndReason::LeftSafeZone]
        );
        let turn = started(&rig.at(2000, 200, 100)).expect("全体の範囲で来るはず");
        assert_eq!(turn.range, whole);
        assert_eq!(turn.position, PointF::new(200.0, 100.0));
        assert!(!turn.has_text);
        assert_ne!(turn.token, token);
    }

    #[test]
    fn dpi_144_hits_the_same_logical_rect() {
        let rig_at = |dpi: u16| {
            let mut rig = Rig::new();
            let w = rig.window(dpi);
            let child = rig.hit_child(w, 0, 0, 300, 150);
            rig.pointer(child);
            reg(&mut rig, w, rect(10.0, 10.0, 50.0, 30.0), Some("x"));
            rig
        };
        // DPI 96 では、物理 (60,30) は論理 (60,30) で、範囲 (10,10)-(50,30) の外。
        assert!(deadlines(&rig_at(96).at(0, 60, 30)).is_empty());

        // DPI 144 では、物理 (60,30) は論理 (40,20) で、同じ論理の矩形に当たる。
        let mut rig = rig_at(144);
        rig.at(0, 60, 30);
        let turn = started(&rig.at(800, 60, 30)).expect("来るはず");
        assert_eq!(turn.position, PointF::new(40.0, 20.0));

        // 出す番の範囲の画面の矩形も DPI で換算する（右の端は物理 75＝論理 50）。
        assert!(ended(&rig.at(900, 74, 30)).is_empty());
        assert_eq!(
            ended(&rig.at(1000, 76, 30)),
            vec![TooltipEndReason::LeftSafeZone]
        );
    }

    #[test]
    fn where_the_hit_test_misses_is_not_entered() {
        // 当たり判定は窓の左半分だけ（右半分は透過で抜ける）。範囲は窓の全体。
        let mut rig = Rig::new();
        let w = rig.window(96);
        let child = rig.hit_child(w, 0, 0, 150, 150);
        rig.pointer(child);
        reg(&mut rig, w, TooltipArea::WholeWindow, Some("x"));
        for ms in [0, 800, 2000] {
            let fx = rig.at(ms, 200, 50);
            assert!(fx.is_empty(), "{ms}: {fx:?}");
        }
        // 較正: 当たる所なら入る。
        assert!(!deadlines(&rig.at(2100, 50, 50)).is_empty());

        // ポインタの状態が配下に無い窓も、足元の候補にしない。
        let (mut rig, _, child, _) = rig_with_range(Some("x"));
        rig.world.entity_mut(child).remove::<PointerState>();
        for ms in [0, 800] {
            assert!(rig.at(ms, 30, 20).is_empty());
        }
    }

    #[test]
    fn hidden_window_is_not_entered() {
        // 当たり判定・ポインタの状態・範囲はあるが、窓が見えていない。
        let (mut rig, w, _, _) = rig_with_range(Some("x"));
        rig.hidden.push(w);
        for ms in [0, 800, 2000] {
            let fx = rig.at(ms, 30, 20);
            assert!(fx.is_empty(), "{ms}: {fx:?}");
        }
        // 較正: 見えるようになれば入る。
        rig.hidden.clear();
        assert!(!deadlines(&rig.at(2100, 30, 20)).is_empty());
    }

    #[test]
    fn despawning_the_window_ends_with_destroyed() {
        fn on(_: &mut World, _: &TooltipNotice) {}
        let (mut rig, w, _, _) = rig_with_range(Some("x"));
        rig.world.entity_mut(w).insert(OnTooltip(on));
        start(&mut rig);
        assert!(rig.session.note.and_then(|n| n.notify).is_some());

        rig.world.despawn(w);
        assert_eq!(
            ended(&rig.at(900, 30, 20)),
            vec![TooltipEndReason::WindowDestroyed]
        );
        // 窓が消えても、知らせの関数の写しは控えに残る（終わりの知らせを呼べる）。
        assert!(rig.session.note.and_then(|n| n.notify).is_some());
    }

    #[test]
    fn removing_the_registration_ends_with_unregistered() {
        let (mut rig, w, _, id) = rig_with_range(Some("x"));
        start(&mut rig);
        assert!(
            rig.world
                .get_mut::<ranges::TooltipRanges>(w)
                .unwrap()
                .remove(id)
        );
        assert_eq!(
            ended(&rig.at(900, 30, 20)),
            vec![TooltipEndReason::RangeUnregistered]
        );
    }

    #[test]
    fn losing_the_hit_child_or_visibility_ends_with_hidden() {
        // 当たり判定の子を消す（ポインタの状態は窓に付いたまま・マウスは動かさない）。
        let mut rig = Rig::new();
        let w = rig.window(96);
        let child = rig.hit_child(w, 0, 0, 300, 150);
        rig.pointer(w);
        reg(&mut rig, w, rect(10.0, 10.0, 60.0, 40.0), Some("x"));
        start(&mut rig);
        assert!(ended(&rig.at(900, 30, 20)).is_empty(), "較正: まだ続く");
        rig.world.despawn(child);
        assert_eq!(
            ended(&rig.at(1000, 30, 20)),
            vec![TooltipEndReason::WindowHidden]
        );

        // 窓が見えなくなる。
        let (mut rig, w, _, _) = rig_with_range(Some("x"));
        start(&mut rig);
        rig.hidden.push(w);
        assert_eq!(
            ended(&rig.at(900, 30, 20)),
            vec![TooltipEndReason::WindowHidden]
        );
    }

    #[test]
    fn pointer_state_moving_to_an_overlapping_window_ends_with_left_safe_zone() {
        let run = |transfer: bool| {
            let (mut rig, _, child, _) = rig_with_range(Some("x"));
            // 同じ所に重なった別の窓（範囲なし）。
            let other = rig.window(96);
            let other_child = rig.hit_child(other, 0, 0, 300, 150);
            start(&mut rig);
            if transfer {
                rig.world.entity_mut(child).remove::<PointerState>();
                rig.pointer(other_child);
            }
            // 範囲の矩形の中で動かす。
            ended(&rig.at(900, 35, 22))
        };
        assert_eq!(run(true), vec![TooltipEndReason::LeftSafeZone]);
        assert!(run(false).is_empty(), "較正: 移らなければ続く");
    }

    #[test]
    fn press_mark_while_no_ranges_is_not_carried_over() {
        let (mut rig, w, _) = rig_with_window();
        note_button_press();
        assert!(rig.at(0, 30, 20).is_empty());
        assert_eq!(rig.samples.get(), 0);

        reg(&mut rig, w, rect(10.0, 10.0, 60.0, 40.0), Some("x"));
        assert!(
            !deadlines(&rig.at(100, 30, 20)).is_empty(),
            "古い押下で休みにならず、待ちに入るはず"
        );

        // 較正: 範囲がある間に立てた印は、次の判定で押下として効く（待ちが取り消される）。
        note_button_press();
        assert!(deadlines(&rig.at(200, 30, 20)).is_empty());
        assert!(started(&rig.at(1000, 30, 20)).is_none());
        // 読んだら倒す（出て入り直せば待ちに入る）。
        rig.at(1100, 200, 100);
        assert!(!deadlines(&rig.at(1200, 30, 20)).is_empty());
    }

    #[test]
    fn replacing_during_a_turn_takes_effect_from_the_next_turn() {
        let (mut rig, _, _, id) = rig_with_range(Some("x"));
        start(&mut rig);
        // 範囲を (10,10)-(20,20) へ縮める。
        assert!(update(
            &mut rig.world,
            id,
            TooltipRange {
                area: rect(10.0, 10.0, 20.0, 20.0),
                text: Some("y".into()),
            },
        ));
        // 続いている出す番は始まりの矩形のまま: (50,30) は古い矩形の中なので続く。
        assert!(ended(&rig.at(900, 50, 30)).is_empty());
        assert_eq!(
            ended(&rig.at(1000, 70, 30)),
            vec![TooltipEndReason::LeftSafeZone]
        );
        // 次の出す番は新しい矩形: (50,30) では入らず、(15,15) で入る。
        assert!(deadlines(&rig.at(1100, 50, 30)).is_empty());
        assert!(!deadlines(&rig.at(1200, 15, 15)).is_empty());
    }

    #[test]
    fn unreadable_cursor_neither_enters_nor_ends() {
        let (mut rig, _, _, _) = rig_with_range(Some("x"));
        assert!(rig.run(0, None, false).is_empty());
        start(&mut rig);
        assert!(ended(&rig.run(900, None, false)).is_empty());
    }

    #[test]
    fn trace_carries_wait_time_and_reason() {
        let level = |e: &&log_capture_kit::CapturedEvent| e.level == tracing::Level::TRACE;
        let (mut rig, w, _, _) = rig_with_range(Some("x"));

        let (_, events) = log_capture_kit::capture(|| rig.at(0, 30, 20));
        assert!(
            events.iter().filter(level).any(
                |e| e.message().contains("[tooltip_turn]") && e.field("wait_ms") == Some("800")
            ),
            "{events:?}"
        );

        rig.at(800, 30, 20);
        rig.hidden.push(w);
        let (_, events) = log_capture_kit::capture(|| rig.at(900, 30, 20));
        assert!(
            events
                .iter()
                .filter(level)
                .any(|e| e.message().contains("[tooltip_turn]")
                    && e.field("reason") == Some("WindowHidden")),
            "{events:?}"
        );
    }
}
