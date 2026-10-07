//! `turn` の決定論テスト（待ち時間・来た・取り消し・安全地帯・終わり・入り直し・別の範囲への切り替え・
//! 期限の預け直し・印の照合）。
//!
//! 時刻は基準の時刻からのミリ秒で渡し、実際の時間は待たない。窓も作らない。
//! 画面の位置と論理の位置は同じ（96 DPI）として組む。

use super::*;
use crate::ecs::Rect;
use crate::ecs::tooltip::ranges::{TooltipRange, TooltipRanges};
use bevy_ecs::world::World;

const SETTING: Duration = Duration::from_millis(400);

/// 範囲 A（0..100）と範囲 B（200..300）、後から登録した A の内側の範囲 C と、A と TIP の間の
/// 通り道の上の範囲 D を持つ窓と、状態機械の組。
struct Rig {
    m: TurnMachine,
    t0: Instant,
    window: Entity,
    a: TooltipRangeId,
    b: TooltipRangeId,
    c: TooltipRangeId,
    d: TooltipRangeId,
    has_text: bool,
    setting: Duration,
    hover_reads: usize,
}

const RECT_A: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 100,
    bottom: 100,
};
const RECT_B: RectPx = RectPx {
    left: 200,
    top: 0,
    right: 300,
    bottom: 100,
};
/// A の内側（右下の隅）。後から登録したので、重なった所では C が勝つ。
const RECT_C: RectPx = RectPx {
    left: 70,
    top: 70,
    right: 90,
    bottom: 90,
};
/// A と TIP の間の通り道の上。
const RECT_D: RectPx = RectPx {
    left: 25,
    top: -25,
    right: 35,
    bottom: -15,
};
/// 範囲 A の真上に出たことにするツールチップ。
const TIP: RectPx = RectPx {
    left: 20,
    top: -60,
    right: 80,
    bottom: -30,
};

fn area(rc: RectPx) -> TooltipArea {
    TooltipArea::Rect(Rect {
        left: rc.left as f32,
        top: rc.top as f32,
        right: rc.right as f32,
        bottom: rc.bottom as f32,
    })
}

impl Rig {
    fn new() -> Self {
        let window = World::new().spawn_empty().id();
        let mut table = TooltipRanges::default();
        let mut add = |rc| {
            table.add(
                window,
                TooltipRange {
                    area: area(rc),
                    text: Some("説明".into()),
                },
            )
        };
        let (a, b, c, d) = (add(RECT_A), add(RECT_B), add(RECT_C), add(RECT_D));
        Self {
            m: TurnMachine::default(),
            t0: Instant::now(),
            window,
            a,
            b,
            c,
            d,
            has_text: true,
            setting: SETTING,
            hover_reads: 0,
        }
    }

    fn t(&self, ms: u64) -> Instant {
        self.t0 + Duration::from_millis(ms)
    }

    fn rect_of(&self, id: TooltipRangeId) -> RectPx {
        [(self.a, RECT_A), (self.b, RECT_B), (self.c, RECT_C)]
            .into_iter()
            .find_map(|(k, rc)| (k == id).then_some(rc))
            .unwrap_or(RECT_D)
    }

    /// 足元の範囲（位置から決める。重なった所では後から登録した範囲）。
    fn over_at(&self, x: i32, y: i32) -> Option<Over> {
        let p = PointPx { x, y };
        [self.d, self.c, self.b, self.a]
            .into_iter()
            .find(|&id| in_safe_zone(p, self.rect_of(id), None))
            .map(|range| self.over_of(range, x, y))
    }

    /// 範囲 range に入っている足元（殻はツールチップの下の範囲もこの形で渡す）。
    fn over_of(&self, range: TooltipRangeId, x: i32, y: i32) -> Over {
        Over {
            window: self.window,
            range,
            area_logical: area(self.rect_of(range)),
            has_text: self.has_text,
            pos_logical: PointF::new(x as f32, y as f32),
        }
    }

    /// 追っている範囲があれば、窓が受けている様子で渡す。
    fn present(&self, receives: bool) -> Option<Tracked> {
        self.m.tracked_range().map(|(_, id)| Tracked::Present {
            range_px: self.rect_of(id),
            receives,
        })
    }

    fn step(
        &mut self,
        ms: u64,
        cursor: Option<(i32, i32)>,
        button_down: bool,
        over: Option<Over>,
        tracked: Option<Tracked>,
    ) -> Vec<Effect> {
        let input = StepInput {
            now: self.t(ms),
            cursor: cursor.map(|(x, y)| PointPx { x, y }),
            button_down,
            over,
            tracked,
        };
        let setting = self.setting;
        let reads = &mut self.hover_reads;
        self.m.step(&input, &mut || {
            *reads += 1;
            setting
        })
    }

    /// ふつうの 1 回: 位置から足元を決め、窓は受けている。
    fn at(&mut self, ms: u64, x: i32, y: i32) -> Vec<Effect> {
        let (over, tracked) = (self.over_at(x, y), self.present(true));
        self.step(ms, Some((x, y)), false, over, tracked)
    }

    /// 押した 1 回。
    fn press(&mut self, ms: u64, x: i32, y: i32) -> Vec<Effect> {
        let (over, tracked) = (self.over_at(x, y), self.present(true));
        self.step(ms, Some((x, y)), true, over, tracked)
    }

    /// 追っている範囲が消えた・隠れた 1 回（足元は無い）。
    fn gone(&mut self, ms: u64, x: i32, y: i32, tracked: Tracked) -> Vec<Effect> {
        self.step(ms, Some((x, y)), false, None, Some(tracked))
    }

    /// 動かないまま、窓がそこで受けなくなった 1 回（足元は無い）。
    fn not_receiving(&mut self, ms: u64, x: i32, y: i32) -> Vec<Effect> {
        let tracked = self.present(false);
        self.step(ms, Some((x, y)), false, None, tracked)
    }

    /// A に入って 800 ミリ秒で来たところまで進める。来た印を返す。
    fn active_on_a(&mut self) -> TooltipTurnToken {
        self.at(0, 50, 50);
        started(&self.at(800, 50, 50)).expect("800 ミリ秒で来る").0
    }
}

/// 来たの中身（印・範囲・預けた文字の有無）。
fn started(fx: &[Effect]) -> Option<(TooltipTurnToken, TooltipRangeId, bool)> {
    fx.iter().find_map(|e| match *e {
        Effect::TurnStarted {
            token,
            range,
            stored,
            ..
        } => Some((token, range, stored)),
        _ => None,
    })
}

/// 終わりの中身（印・理由）。
fn ended(fx: &[Effect]) -> Vec<(TooltipTurnToken, TooltipEndReason)> {
    fx.iter()
        .filter_map(|e| match *e {
            Effect::TurnEnded { token, reason, .. } => Some((token, reason)),
            _ => None,
        })
        .collect()
}

fn hides(fx: &[Effect]) -> Vec<HideReason> {
    fx.iter()
        .filter_map(|e| match *e {
            Effect::Hide { reason, .. } => Some(reason),
            _ => None,
        })
        .collect()
}

/// 知らせ（来た・終わった）も消すも無い。
fn quiet(fx: &[Effect]) -> bool {
    fx.iter().all(|e| matches!(e, Effect::ArmDeadline(_)))
}

// ---- 待ち時間（1.1・1.2・1.4・1.6） ----

#[test]
fn comes_once_at_800ms_not_at_799ms() {
    let mut r = Rig::new();
    assert_eq!(r.at(0, 50, 50), vec![Effect::ArmDeadline(r.t(800))]);
    assert_eq!(r.at(799, 50, 50), vec![Effect::ArmDeadline(r.t(800))]);

    let fx = r.at(800, 50, 50);
    assert_eq!(
        fx,
        vec![
            Effect::TurnStarted {
                token: TooltipTurnToken(1),
                window: r.window,
                range: r.a,
                pos_logical: PointF::new(50.0, 50.0),
                stored: true,
            },
            Effect::ArmDeadline(r.t(900)),
        ]
    );
    // 続いている間は重ねて来ない（1.7）。
    for ms in [900, 1_000, 5_000] {
        assert!(quiet(&r.at(ms, 50, 50)), "{ms} ミリ秒で知らせが出た");
    }
    // 設定を読むのは待ちに入るときだけ。
    assert_eq!(r.hover_reads, 1);
}

#[test]
fn came_carries_whether_text_is_deposited() {
    let mut r = Rig::new();
    r.has_text = false;
    r.at(0, 50, 50);
    assert_eq!(started(&r.at(800, 50, 50)).map(|s| s.2), Some(false));
}

#[test]
fn moving_inside_the_range_does_not_move_the_deadline() {
    let mut r = Rig::new();
    r.at(0, 10, 10);
    assert_eq!(r.at(300, 90, 90), vec![Effect::ArmDeadline(r.t(800))]);
    assert_eq!(r.at(799, 30, 70), vec![Effect::ArmDeadline(r.t(800))]);
    assert!(started(&r.at(800, 60, 20)).is_some());
    assert_eq!(r.hover_reads, 1);
}

/// A の出す番を `end_ms` に範囲の外で終わらせ、B に `enter_ms` に入る。B の待ち時間を返す。
fn wait_after_end(tip_shown: bool, end_ms: u64, enter_ms: u64) -> Duration {
    let mut r = Rig::new();
    r.active_on_a();
    if tip_shown {
        r.m.set_tip(Some(TIP), r.t(800));
    }
    r.at(end_ms, 150, 50);
    let fx = r.at(enter_ms, 250, 50);
    assert_eq!(r.m.tracked_range(), Some((r.window, r.b)));
    match fx[..] {
        [Effect::ArmDeadline(d)] => d - r.t(enter_ms),
        _ => panic!("待ちの期限が 1 つだけ出るはず: {fx:?}"),
    }
}

#[test]
fn reshow_within_200ms_uses_one_times_the_setting() {
    // 消えてから 200 ミリ秒以内は 1 倍、201 ミリ秒後は 2 倍。
    assert_eq!(wait_after_end(true, 1_000, 1_000), SETTING);
    assert_eq!(wait_after_end(true, 1_000, 1_200), SETTING);
    assert_eq!(wait_after_end(true, 1_000, 1_201), SETTING * 2);
}

#[test]
fn turn_without_a_shown_tip_is_not_a_reshow_origin() {
    // 文字を出さなかった出す番は起点にならない。
    assert_eq!(wait_after_end(false, 1_000, 1_100), SETTING * 2);
}

#[test]
fn fallback_setting_comes_at_800ms() {
    let mut r = Rig::new();
    r.setting = FALLBACK_HOVER_TIME;
    r.at(0, 50, 50);
    assert!(started(&r.at(799, 50, 50)).is_none());
    assert!(started(&r.at(800, 50, 50)).is_some());
}

// ---- 待ちの途中の取り消し（2.1・2.2・2.4・2.7・2.9） ----

/// 待ちの途中で cancel を起こし、何も出ずに待ちから外れることを見る。
fn cancel_while_waiting(cancel: impl FnOnce(&mut Rig) -> Vec<Effect>) -> Rig {
    let mut r = Rig::new();
    r.at(0, 50, 50);
    let fx = cancel(&mut r);
    assert!(fx.is_empty(), "待ちから外れるだけなら何も出さない: {fx:?}");
    assert_eq!(r.m.tracked_range(), None);
    r
}

#[test]
fn leaving_while_waiting_emits_nothing_and_reentry_counts_from_zero() {
    let mut r = cancel_while_waiting(|r| r.at(500, 150, 50));
    assert!(r.at(900, 150, 50).is_empty());
    // 入り直すと最初から数える。
    assert_eq!(r.at(1_000, 50, 50), vec![Effect::ArmDeadline(r.t(1_800))]);
    assert!(started(&r.at(1_799, 50, 50)).is_none());
    assert!(started(&r.at(1_800, 50, 50)).is_some());
}

#[test]
fn hidden_destroyed_unregistered_while_waiting_emit_nothing() {
    for tracked in [
        Tracked::WindowHidden,
        Tracked::WindowDestroyed,
        Tracked::RangeUnregistered,
    ] {
        let mut r = cancel_while_waiting(|r| r.gone(500, 50, 50, tracked));
        assert!(r.step(900, Some((50, 50)), false, None, None).is_empty());
    }
}

#[test]
fn not_receiving_without_moving_while_waiting_emits_nothing() {
    let mut r = cancel_while_waiting(|r| r.not_receiving(500, 50, 50));
    assert!(r.step(900, Some((50, 50)), false, None, None).is_empty());
}

#[test]
fn press_while_waiting_emits_nothing_until_reentry() {
    let mut r = cancel_while_waiting(|r| r.press(500, 50, 50));
    // 離して範囲の中に居続けても来ない（2.8）。
    for ms in [600, 900, 5_000] {
        assert!(r.at(ms, 60, 60).is_empty(), "{ms} ミリ秒");
    }
    // 出て入り直すと数え始める。
    r.at(5_100, 150, 50);
    assert_eq!(r.at(5_200, 50, 50), vec![Effect::ArmDeadline(r.t(6_000))]);
}

#[test]
fn unknown_cursor_does_not_release_the_press_rest() {
    let mut r = cancel_while_waiting(|r| r.press(100, 50, 50));
    // 位置が読めない回は「出た」とみなさない（殻は足元も渡せない）。
    assert!(r.step(200, None, false, None, None).is_empty());
    for ms in [300, 1_000, 5_000] {
        assert!(r.at(ms, 50, 50).is_empty(), "{ms} ミリ秒");
    }
    // 出て入り直すと数え始める。
    r.at(5_100, 150, 50);
    assert_eq!(r.at(5_200, 50, 50), vec![Effect::ArmDeadline(r.t(6_000))]);
}

#[test]
fn entering_with_the_button_down_does_not_count() {
    let mut r = Rig::new();
    assert!(r.press(0, 50, 50).is_empty());
    assert!(r.at(100, 50, 50).is_empty());
    assert!(r.at(2_000, 50, 50).is_empty());
}

// ---- 出す番の間（2.5・2.6・2.9・3.13） ----

#[test]
fn moving_in_the_safe_zone_does_not_end() {
    let mut r = Rig::new();
    r.active_on_a();
    r.m.set_tip(Some(TIP), r.t(800));
    // 範囲の中・通り道（範囲とツールチップの間）・ツールチップの上・範囲へ戻る。
    for (ms, x, y) in [
        (900, 90, 90),
        (1_000, 50, -10),
        (1_100, 50, -45),
        (1_200, 10, 10),
    ] {
        let fx = r.at(ms, x, y);
        assert_eq!(fx, vec![Effect::ArmDeadline(r.t(ms + 100))], "({x}, {y})");
    }
}

#[test]
fn inside_the_tip_continues_even_if_the_window_does_not_receive() {
    // ツールチップが範囲の矩形に重なって出た所へ動かす（範囲として見れば「安全地帯から出た」）。
    let mut r = Rig::new();
    r.active_on_a();
    let over_range = RectPx {
        left: 20,
        top: 60,
        right: 80,
        bottom: 90,
    };
    r.m.set_tip(Some(over_range), r.t(800));
    r.at(900, 50, 50);
    let tracked = r.present(false);
    let fx = r.step(1_000, Some((50, 75)), false, None, tracked);
    assert_eq!(fx, vec![Effect::ArmDeadline(r.t(1_100))]);
}

/// 出す番の途中で end を起こす。ツールチップを出したことにするかを選べる。
fn end_while_active(
    tip_shown: bool,
    end: impl FnOnce(&mut Rig) -> Vec<Effect>,
) -> (Rig, TooltipTurnToken, Vec<Effect>) {
    let mut r = Rig::new();
    let token = r.active_on_a();
    if tip_shown {
        r.m.set_tip(Some(TIP), r.t(800));
    }
    r.at(900, 50, 50);
    let fx = end(&mut r);
    (r, token, fx)
}

#[test]
fn each_end_reason_ends_once_with_hide() {
    use TooltipEndReason::*;
    type End = fn(&mut Rig) -> Vec<Effect>;
    let cases: [(End, TooltipEndReason); 6] = [
        (|r| r.at(1_000, 150, 50), LeftSafeZone),
        (|r| r.press(1_000, 50, 50), ButtonPressed),
        (
            |r| r.gone(1_000, 50, 50, Tracked::WindowHidden),
            WindowHidden,
        ),
        (
            |r| r.gone(1_000, 50, 50, Tracked::WindowDestroyed),
            WindowDestroyed,
        ),
        (
            |r| r.gone(1_000, 50, 50, Tracked::RangeUnregistered),
            RangeUnregistered,
        ),
        // 動かないまま受けなくなった（絵や当たり判定が消えた）（2.9）。
        (|r| r.not_receiving(1_000, 50, 50), WindowHidden),
    ];
    for (end, reason) in cases {
        let (mut r, token, fx) = end_while_active(true, end);
        assert_eq!(hides(&fx), vec![HideReason::End(reason)], "{reason:?}");
        assert_eq!(ended(&fx), vec![(token, reason)], "{reason:?}");
        assert_eq!(r.m.tracked_range(), None, "{reason:?}");
        // 終わりは 1 回だけ。
        let fx = r.step(1_100, Some((150, 50)), false, None, None);
        assert!(fx.is_empty(), "{reason:?}: {fx:?}");
    }
}

#[test]
fn ending_without_a_shown_tip_does_not_hide() {
    let (_, token, fx) = end_while_active(false, |r| r.at(1_000, 150, 50));
    assert!(hides(&fx).is_empty());
    assert_eq!(ended(&fx), vec![(token, TooltipEndReason::LeftSafeZone)]);
}

#[test]
fn moving_onto_a_spot_the_window_does_not_receive_is_leaving_the_safe_zone() {
    // 範囲の矩形の中でも、動いて透過の穴や上に重なった窓へ移ったなら「安全地帯から出た」。
    let (_, token, fx) = end_while_active(true, |r| {
        let tracked = r.present(false);
        r.step(1_000, Some((60, 60)), false, None, tracked)
    });
    assert_eq!(ended(&fx), vec![(token, TooltipEndReason::LeftSafeZone)]);
}

#[test]
fn after_press_no_turn_until_reentry_then_a_different_token() {
    let (mut r, first, fx) = end_while_active(true, |r| r.press(1_000, 50, 50));
    assert_eq!(ended(&fx), vec![(first, TooltipEndReason::ButtonPressed)]);
    // 離して範囲の中に居続けても来ない。
    for ms in [1_100, 2_000, 9_000] {
        assert!(r.at(ms, 40, 40).is_empty(), "{ms} ミリ秒");
    }
    r.at(9_100, 150, 50);
    r.at(9_200, 50, 50);
    let (second, range, _) = started(&r.at(10_000, 50, 50)).expect("入り直して来る");
    assert_eq!(range, r.a);
    assert_eq!(second, TooltipTurnToken(first.0 + 1));
}

#[test]
fn reentry_after_leaving_comes_with_a_different_token() {
    let mut r = Rig::new();
    let first = r.active_on_a();
    r.at(1_000, 150, 50);
    r.at(1_100, 50, 50);
    let (second, _, _) = started(&r.at(1_900, 50, 50)).expect("入り直して来る");
    assert_ne!(second, first);
    assert_eq!(second, TooltipTurnToken(first.0 + 1));
}

#[test]
fn leaving_straight_into_another_range_starts_waiting_in_the_same_step() {
    let mut r = Rig::new();
    let token = r.active_on_a();
    let fx = r.at(1_000, 250, 50);
    assert_eq!(
        fx,
        vec![
            Effect::TurnEnded {
                token,
                window: r.window,
                range: r.a,
                reason: TooltipEndReason::LeftSafeZone,
            },
            Effect::ArmDeadline(r.t(1_800)),
        ]
    );
    assert_eq!(r.m.tracked_range(), Some((r.window, r.b)));
    let (_, range, _) = started(&r.at(1_800, 250, 50)).expect("B で来る");
    assert_eq!(range, r.b);
}

// ---- 出す番の間に別の範囲へ（2.5・2.6・2.10・4.5・1.4） ----

#[test]
fn moving_into_the_inner_range_switches_with_the_reshow_wait() {
    let (mut r, token, fx) = end_while_active(true, |r| r.at(1_000, 80, 80));
    let reason = TooltipEndReason::EnteredOtherRange;
    assert_eq!(
        fx,
        vec![
            Effect::Hide {
                token,
                reason: HideReason::End(reason),
            },
            Effect::TurnEnded {
                token,
                window: r.window,
                range: r.a,
                reason,
            },
            // 直前のツールチップが同じ回に消えたので、待ちは設定の 1 倍（1.4）。
            Effect::ArmDeadline(r.t(1_000) + SETTING),
        ]
    );
    assert_eq!(r.m.tracked_range(), Some((r.window, r.c)));
    let (second, range, _) = started(&r.at(1_400, 85, 85)).expect("C で来る");
    assert_eq!(range, r.c);
    assert_eq!(second, TooltipTurnToken(token.0 + 1));
}

#[test]
fn moving_from_the_inner_range_back_to_the_outer_switches() {
    // C の説明が TIP に出ている。(75, 50) は A の中・C の外で、C と TIP の通り道の中。
    // (10, 50) は A の中で、安全地帯の外。どちらでも A へ切り替わる。
    use TooltipEndReason::*;
    for (x, reason) in [(75, EnteredOtherRange), (10, LeftSafeZone)] {
        let mut r = Rig::new();
        r.at(0, 80, 80);
        let token = started(&r.at(800, 80, 80)).expect("C で来る").0;
        r.m.set_tip(Some(TIP), r.t(800));
        let corridor = in_safe_zone(PointPx { x, y: 50 }, RECT_C, Some(TIP));
        assert_eq!(corridor, reason == EnteredOtherRange, "較正: ({x}, 50)");
        let fx = r.at(1_000, x, 50);
        assert_eq!(ended(&fx), vec![(token, reason)], "({x}, 50)");
        assert_eq!(fx.last(), Some(&Effect::ArmDeadline(r.t(1_000) + SETTING)));
        assert_eq!(r.m.tracked_range(), Some((r.window, r.a)), "({x}, 50)");
    }
}

#[test]
fn on_the_tip_a_range_under_it_does_not_switch() {
    let mut r = Rig::new();
    r.active_on_a();
    r.m.set_tip(Some(TIP), r.t(800));
    // 殻は、ツールチップの下にある範囲（ここでは B とする）も足元として渡す。
    let (over, tracked) = (Some(r.over_of(r.b, 50, -45)), r.present(true));
    let fx = r.step(1_000, Some((50, -45)), false, over, tracked);
    assert_eq!(fx, vec![Effect::ArmDeadline(r.t(1_100))]);
    assert_eq!(r.m.tracked_range(), Some((r.window, r.a)));
}

#[test]
fn entering_a_range_under_the_corridor_switches() {
    // 通り道のうち D の無い所で続くことは moving_in_the_safe_zone_does_not_end が見る。
    let (r, token, fx) = end_while_active(true, |r| r.at(1_000, 30, -20));
    let reason = TooltipEndReason::EnteredOtherRange;
    assert_eq!(hides(&fx), vec![HideReason::End(reason)]);
    assert_eq!(ended(&fx), vec![(token, reason)]);
    assert_eq!(r.m.tracked_range(), Some((r.window, r.d)));
}

#[test]
fn switching_before_any_tip_was_shown_waits_twice_the_setting() {
    // 文字がまだ渡されていない（動的）: 消えたツールチップが無いので出し直しではない（1.4）。
    let (r, token, fx) = end_while_active(false, |r| r.at(1_000, 80, 80));
    assert!(hides(&fx).is_empty());
    assert_eq!(
        ended(&fx),
        vec![(token, TooltipEndReason::EnteredOtherRange)]
    );
    assert_eq!(
        fx.last(),
        Some(&Effect::ArmDeadline(r.t(1_000) + SETTING * 2))
    );
}

#[test]
fn not_receiving_without_moving_is_window_hidden_even_over_another_range() {
    // 動かないまま受けなくなり、下の別の範囲（ここでは B とする）が足元になった（2.9 が先）。
    let (r, token, fx) = end_while_active(true, |r| {
        let (over, tracked) = (Some(r.over_of(r.b, 50, 50)), r.present(false));
        r.step(1_000, Some((50, 50)), false, over, tracked)
    });
    assert_eq!(ended(&fx), vec![(token, TooltipEndReason::WindowHidden)]);
    assert_eq!(r.m.tracked_range(), Some((r.window, r.b)));
}

// ---- 期限の預け直しと、位置が読めない回 ----

#[test]
fn deadline_is_rearmed_every_step_while_waiting_and_active() {
    let mut r = Rig::new();
    assert!(r.at(0, 150, 50).is_empty(), "何もない間は預けない");
    for ms in [0, 100, 400, 700] {
        assert_eq!(r.at(ms, 50, 50), vec![Effect::ArmDeadline(r.t(800))]);
    }
    r.at(800, 50, 50);
    for ms in [850, 1_234, 3_000] {
        assert_eq!(r.at(ms, 50, 50), vec![Effect::ArmDeadline(r.t(ms + 100))]);
    }
}

#[test]
fn unknown_cursor_neither_advances_waiting_nor_ends_the_turn() {
    let mut r = Rig::new();
    r.at(0, 50, 50);
    let tracked = r.present(true);
    let fx = r.step(900, None, false, None, tracked);
    assert_eq!(fx, vec![Effect::ArmDeadline(r.t(800))]);
    assert!(started(&r.at(950, 50, 50)).is_some());
    let tracked = r.present(true);
    let fx = r.step(1_000, None, false, None, tracked);
    assert_eq!(fx, vec![Effect::ArmDeadline(r.t(1_100))]);
}

#[test]
fn active_turn_keeps_the_area_it_started_with() {
    // 途中の差し替えは次の出す番から効く（殻は出す番の始まりの矩形で様子を作る）。
    let mut r = Rig::new();
    r.active_on_a();
    let State::Active(turn) = &r.m.state else {
        panic!("出す番のはず");
    };
    assert_eq!(turn.area, area(RECT_A));
}

// ---- 印の照合（3.12・4.3・4.4・4.7・5.1〜5.5・5.7・5.8・6.7） ----

#[test]
fn supply_on_a_live_token_shows_and_empty_hides() {
    let mut r = Rig::new();
    let token = r.active_on_a();
    assert_eq!(r.m.supply(token, false), SupplyDecision::Show);
    // 空（""）は出さない・出ていれば消す（3.12）。出す番は続く。
    r.m.set_tip(Some(TIP), r.t(850));
    assert_eq!(r.m.supply(token, true), SupplyDecision::HideEmpty);
    r.m.set_tip(None, r.t(860));
    assert_eq!(r.m.tracked_range(), Some((r.window, r.a)));
    assert_eq!(r.at(900, 50, 50), vec![Effect::ArmDeadline(r.t(1_000))]);
}

#[test]
fn ended_token_is_stale_and_cannot_dismiss() {
    let (mut r, token, _) = end_while_active(true, |r| r.at(1_000, 150, 50));
    assert_eq!(r.m.supply(token, false), SupplyDecision::Stale);
    // 終わった印なら、空でも「終わっていた」。
    assert_eq!(r.m.supply(token, true), SupplyDecision::Stale);
    assert!(!r.m.dismiss(token, r.t(1_100)));
}

#[test]
fn earlier_token_supplied_during_a_later_turn_is_stale() {
    let mut r = Rig::new();
    let first = r.active_on_a();
    r.at(1_000, 150, 50);
    // 次の範囲の待ちの間も、前の印は受けない。
    r.at(1_100, 250, 50);
    assert_eq!(r.m.supply(first, false), SupplyDecision::Stale);
    let (second, _, _) = started(&r.at(1_900, 250, 50)).expect("B で来る");
    assert_eq!(r.m.supply(first, false), SupplyDecision::Stale);
    assert!(!r.m.dismiss(first, r.t(1_950)));
    assert_eq!(r.m.supply(second, false), SupplyDecision::Show);
}

#[test]
fn supply_has_no_time_limit_while_the_turn_continues() {
    let mut r = Rig::new();
    let token = r.active_on_a();
    for ms in [60_000, 3_600_000] {
        assert!(quiet(&r.at(ms, 50, 50)), "{ms} ミリ秒");
    }
    assert_eq!(r.m.supply(token, false), SupplyDecision::Show);
}

#[test]
fn turn_without_deposited_text_shows_nothing_until_supplied() {
    let mut r = Rig::new();
    r.has_text = false;
    r.at(0, 50, 50);
    let (token, _, stored) = started(&r.at(800, 50, 50)).expect("来る");
    assert!(!stored, "預けた文字が無ければ殻は出さない（4.4・5.7）");
    // 何も渡さなければ、続いても消すも終わりも出ない。
    for ms in [900, 5_000] {
        assert!(quiet(&r.at(ms, 50, 50)), "{ms} ミリ秒");
    }
    assert_eq!(r.m.supply(token, false), SupplyDecision::Show);
}

#[test]
fn supplying_while_deposited_text_is_shown_replaces_it() {
    let mut r = Rig::new();
    let token = r.active_on_a();
    r.m.set_tip(Some(TIP), r.t(800));
    assert_eq!(r.m.supply(token, false), SupplyDecision::Show);
    // 置き換えた矩形が安全地帯になる（前の矩形の上は、範囲との通り道の外）。
    let replaced = RectPx {
        left: 120,
        top: -60,
        right: 180,
        bottom: -30,
    };
    r.m.set_tip(Some(replaced), r.t(900));
    let fx = r.at(1_000, 150, -45);
    assert_eq!(fx, vec![Effect::ArmDeadline(r.t(1_100))]);
    let fx = r.at(1_100, 50, -45);
    assert_eq!(
        hides(&fx),
        vec![HideReason::End(TooltipEndReason::LeftSafeZone)]
    );
    assert_eq!(ended(&fx), vec![(token, TooltipEndReason::LeftSafeZone)]);
}

#[test]
fn after_dismiss_the_turn_continues_and_supplying_shows_again() {
    let mut r = Rig::new();
    let token = r.active_on_a();
    r.m.set_tip(Some(TIP), r.t(800));
    assert!(r.m.dismiss(token, r.t(850)));
    assert_eq!(r.at(900, 50, 50), vec![Effect::ArmDeadline(r.t(1_000))]);
    assert_eq!(r.m.supply(token, false), SupplyDecision::Show);
    // 消した時刻が出し直しの起点になる（消して 200 ミリ秒以内に別の範囲へ入れば 1 倍）。
    let mut r2 = Rig::new();
    let t2 = r2.active_on_a();
    r2.m.set_tip(Some(TIP), r2.t(800));
    assert!(r2.m.dismiss(t2, r2.t(1_000)));
    let fx = r2.at(1_100, 250, 50);
    assert_eq!(ended(&fx), vec![(t2, TooltipEndReason::LeftSafeZone)]);
    assert!(hides(&fx).is_empty(), "消した後は終わりで消し直さない");
    assert_eq!(fx.last(), Some(&Effect::ArmDeadline(r2.t(1_100) + SETTING)));
}

#[test]
fn after_a_show_failure_ending_detection_continues() {
    // 表示の失敗は set_tip(None) で渡るだけ（6.7）。
    let mut r = Rig::new();
    let token = r.active_on_a();
    assert_eq!(r.at(900, 50, 50), vec![Effect::ArmDeadline(r.t(1_000))]);
    r.m.set_tip(None, r.t(950));
    assert_eq!(r.at(960, 50, 50), vec![Effect::ArmDeadline(r.t(1_060))]);
    let fx = r.at(1_000, 150, 50);
    assert!(hides(&fx).is_empty());
    assert_eq!(ended(&fx), vec![(token, TooltipEndReason::LeftSafeZone)]);
    // 出なかった出す番は出し直しの起点にならない（失敗から 100 ミリ秒でも 2 倍）。
    assert_eq!(
        r.at(1_050, 250, 50),
        vec![Effect::ArmDeadline(r.t(1_050) + SETTING * 2)]
    );
}

#[test]
fn unregistering_the_tracked_range_ends_synchronously() {
    let mut r = Rig::new();
    let token = r.active_on_a();
    r.m.set_tip(Some(TIP), r.t(800));
    // 追っていない範囲の取り消しは何もしない。
    assert!(r.m.on_unregistered(r.b, r.t(850)).is_empty());
    assert_eq!(r.m.tracked_range(), Some((r.window, r.a)));

    let fx = r.m.on_unregistered(r.a, r.t(900));
    assert_eq!(
        fx,
        vec![
            Effect::Hide {
                token,
                reason: HideReason::End(TooltipEndReason::RangeUnregistered),
            },
            Effect::TurnEnded {
                token,
                window: r.window,
                range: r.a,
                reason: TooltipEndReason::RangeUnregistered,
            },
        ]
    );
    assert_eq!(r.m.tracked_range(), None);
    assert_eq!(r.m.supply(token, false), SupplyDecision::Stale);
    // 次の判定で終わりを重ねない（殻はもう足元に A を渡さない）。
    assert!(r.step(950, Some((50, 50)), false, None, None).is_empty());
    // 消えた時刻が出し直しの起点になる。
    assert_eq!(
        r.at(1_000, 250, 50),
        vec![Effect::ArmDeadline(r.t(1_000) + SETTING)]
    );
}

#[test]
fn unregistering_the_waiting_range_drops_out_without_notice() {
    let mut r = Rig::new();
    r.at(0, 50, 50);
    assert!(r.m.on_unregistered(r.a, r.t(500)).is_empty());
    assert_eq!(r.m.tracked_range(), None);
    assert!(r.step(900, Some((50, 50)), false, None, None).is_empty());
}
