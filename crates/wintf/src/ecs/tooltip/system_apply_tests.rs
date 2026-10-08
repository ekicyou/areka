//! 窓なしの World のテスト（適用と、文字を渡す・消す・取り消す口）。
//!
//! 表示の出入口には、呼ばれた順を記録するだけの閉包を渡す。知らせの関数も同じ記録へ書くので、
//! 表示と知らせの前後を 1 本の並びで確かめられる。時刻は基準の時刻からのミリ秒で渡す。

use super::geometry::{PointPx, RectPx};
use super::os::OsSample;
use super::system::{
    Tip, TooltipSession, dismiss_with, frame_with, supply_text_with, unregister_with, wake_at,
};
use super::*;
use crate::ecs::{DPI, GlobalArrangement, Point, PointerState, Rect, SizeI, WindowPos};
use std::cell::{Cell, RefCell};
use std::time::Duration;
use windows_numerics::Matrix3x2;

/// 窓の左上（画面）。
const ORIGIN: (i32, i32) = (100, 200);

/// 表示の出入口と知らせの関数が呼ばれた記録。
#[derive(Debug, Clone, PartialEq)]
enum Call {
    Show { text: String, at: PointPx },
    Hide,
    Notice(TooltipNotice),
}

thread_local! {
    static CALLS: RefCell<Vec<Call>> = const { RefCell::new(Vec::new()) };
    /// 真なら表示を失敗させる。
    static FAIL: Cell<bool> = const { Cell::new(false) };
    /// 来た知らせの中で渡す文字（取り出したら空にする）。
    static REPLY: RefCell<Option<String>> = const { RefCell::new(None) };
    /// 知らせの中で渡した結果（Debug の形）。
    static REPLIED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn calls() -> Vec<Call> {
    CALLS.take()
}

/// 記録するだけの表示の出入口を作って f に渡す。出た矩形はマウスの 20〜40 ピクセル上。
fn with_fake<R>(f: impl FnOnce(&mut Tip<'_>) -> R) -> R {
    let mut show = |text: &str, at: PointPx| {
        CALLS.with_borrow_mut(|c| {
            c.push(Call::Show {
                text: text.to_owned(),
                at,
            })
        });
        if FAIL.get() {
            Err(TooltipOsError::Show { stage: "test" })
        } else {
            Ok(RectPx {
                left: at.x - 20,
                top: at.y - 40,
                right: at.x + 20,
                bottom: at.y - 20,
            })
        }
    };
    let mut hide = || CALLS.with_borrow_mut(|c| c.push(Call::Hide));
    f(&mut Tip {
        show: &mut show,
        hide: &mut hide,
    })
}

/// 知らせの関数: 記録し、来た知らせなら REPLY の文字をその場で渡す。
fn on(world: &mut World, notice: &TooltipNotice) {
    CALLS.with_borrow_mut(|c| c.push(Call::Notice(*notice)));
    if let TooltipNotice::TurnStarted(turn) = notice
        && let Some(text) = REPLY.take()
    {
        let got = with_fake(|tip| supply_text_with(world, turn.token, &text, Instant::now(), tip));
        REPLIED.with_borrow_mut(|r| r.push(format!("{got:?}")));
    }
}

struct Rig {
    world: World,
    t0: Instant,
    window: Entity,
    id: TooltipRangeId,
}

impl Rig {
    /// 窓（DPI 96・左上 ORIGIN・300×150）と全体の当たり判定の子（ポインタの状態つき）と、
    /// 範囲 (10,10)-(60,40) を置く。知らせの関数を付ける。
    fn new(text: Option<&str>) -> Self {
        CALLS.take();
        REPLIED.take();
        REPLY.take();
        FAIL.set(false);
        let mut world = World::new();
        world.insert_non_send(TooltipSession::default());
        let (x, y) = (ORIGIN.0 as f32, ORIGIN.1 as f32);
        let window = world
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
                DPI::from_dpi(96, 96),
                OnTooltip(on),
            ))
            .id();
        let child = world
            .spawn((
                GlobalArrangement {
                    transform: Matrix3x2::translation(x, y),
                    bounds: Rect {
                        left: x,
                        top: y,
                        right: x + 300.0,
                        bottom: y + 150.0,
                    },
                },
                PointerState::default(),
            ))
            .id();
        world.entity_mut(window).add_children(&[child]);
        let id = register(
            &mut world,
            window,
            TooltipRange {
                area: TooltipArea::Rect(Rect {
                    left: 10.0,
                    top: 10.0,
                    right: 60.0,
                    bottom: 40.0,
                }),
                text: text.map(str::to_owned),
            },
        )
        .unwrap();
        Self {
            world,
            t0: Instant::now(),
            window,
            id,
        }
    }

    fn now(&self, ms: u64) -> Instant {
        self.t0 + Duration::from_millis(ms)
    }

    /// 1 回の画面更新（判定と適用）。x・y は窓の左上からの物理ピクセル。
    fn frame(&mut self, ms: u64, x: i32, y: i32) {
        let now = self.now(ms);
        let cursor = Some(screen(x, y));
        let world = &mut self.world;
        with_fake(|tip| {
            frame_with(
                world,
                now,
                &mut || OsSample {
                    cursor,
                    button_down: false,
                },
                &|_, _| true,
                &mut || Duration::from_millis(400),
                tip,
            )
        });
    }

    /// (30,20) で入り、800 ミリ秒で出す番にする。印を返す。
    fn start(&mut self) -> TooltipTurnToken {
        self.frame(0, 30, 20);
        self.frame(800, 30, 20);
        let token = CALLS.with_borrow(|c| {
            c.iter().find_map(|call| match call {
                Call::Notice(TooltipNotice::TurnStarted(t)) => Some(t.token),
                _ => None,
            })
        });
        token.expect("800 ミリ秒で来るはず")
    }

    fn supply(&mut self, ms: u64, token: TooltipTurnToken, text: &str) -> TooltipSupply {
        let now = self.now(ms);
        with_fake(|tip| supply_text_with(&mut self.world, token, text, now, tip))
    }
}

/// 窓の左上からの物理ピクセルを画面の位置にする。
fn screen(x: i32, y: i32) -> PointPx {
    PointPx {
        x: ORIGIN.0 + x,
        y: ORIGIN.1 + y,
    }
}

fn show(text: &str, x: i32, y: i32) -> Call {
    Call::Show {
        text: text.to_owned(),
        at: screen(x, y),
    }
}

fn is_started(c: &Call) -> bool {
    matches!(c, Call::Notice(TooltipNotice::TurnStarted(_)))
}

fn ended(c: &Call) -> Option<TooltipEndReason> {
    match c {
        Call::Notice(TooltipNotice::TurnEnded { reason, .. }) => Some(*reason),
        _ => None,
    }
}

#[test]
fn stored_text_is_shown_once_before_the_started_notice() {
    let mut rig = Rig::new(Some("説明"));
    rig.start();
    let got = calls();
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got[0], show("説明", 30, 20));
    assert!(is_started(&got[1]), "{got:?}");

    // 出ている間の見回りでは出し直さない。
    rig.frame(900, 35, 25);
    assert!(calls().is_empty());
}

#[test]
fn supplying_inside_the_notice_shows_again_in_the_same_pass() {
    // 預けた文字のある範囲: 預けた文字 → 知らせ → 知らせの中で渡した文字。
    let mut rig = Rig::new(Some("説明"));
    REPLY.set(Some("差し替え".into()));
    rig.start();
    let got = calls();
    assert_eq!(got.len(), 3, "{got:?}");
    assert_eq!(got[0], show("説明", 30, 20));
    assert!(is_started(&got[1]));
    assert_eq!(got[2], show("差し替え", 30, 20));
    assert_eq!(REPLIED.take(), vec!["Shown".to_owned()]);

    // 預けていない範囲: 知らせより前には出さず、知らせの中で渡した文字だけが出る。
    let mut rig = Rig::new(None);
    REPLY.set(Some("動的".into()));
    rig.start();
    let got = calls();
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(is_started(&got[0]));
    assert_eq!(got[1], show("動的", 30, 20));
    assert_eq!(REPLIED.take(), vec!["Shown".to_owned()]);
}

#[test]
fn empty_text_hides_and_is_cleared() {
    let mut rig = Rig::new(Some("説明"));
    let token = rig.start();
    calls();

    assert!(matches!(rig.supply(900, token, ""), TooltipSupply::Cleared));
    assert_eq!(calls(), vec![Call::Hide]);

    // 出す番は続いている（渡せばまた出る）。
    assert!(matches!(rig.supply(1000, token, "x"), TooltipSupply::Shown));
    assert_eq!(calls(), vec![show("x", 30, 20)]);
}

#[test]
fn stale_token_does_not_show() {
    let mut rig = Rig::new(Some("説明"));
    let token = rig.start();
    rig.frame(900, 200, 100);
    let got = calls();
    assert_eq!(
        got.iter().filter_map(ended).collect::<Vec<_>>(),
        vec![TooltipEndReason::LeftSafeZone]
    );

    let (got, events) = log_capture_kit::capture(|| rig.supply(1000, token, "遅れた"));
    assert!(matches!(got, TooltipSupply::StaleTurn), "{got:?}");
    assert!(calls().is_empty(), "表示を呼ばない");
    assert!(
        events
            .iter()
            .any(|e| e.level == tracing::Level::DEBUG
                && e.message().contains("[tooltip_supply_stale]")),
        "{events:?}"
    );
}

#[test]
fn show_failure_is_failed_and_the_turn_continues_until_it_leaves_the_range() {
    let mut rig = Rig::new(None);
    let token = rig.start();
    calls();
    FAIL.set(true);

    let (got, events) = log_capture_kit::capture(|| rig.supply(900, token, "x"));
    assert!(matches!(got, TooltipSupply::Failed(_)), "{got:?}");
    assert!(
        events
            .iter()
            .any(|e| e.level == tracing::Level::WARN
                && e.message().contains("[tooltip_show_failed]")),
        "{events:?}"
    );
    calls();

    // 範囲の中では続く。
    rig.frame(1000, 50, 30);
    assert!(calls().iter().all(|c| ended(c).is_none()));
    // 範囲のすぐ上（出ていればツールチップとの通り道）へ出ると終わる（出ていないので安全地帯は範囲だけ）。
    rig.frame(1100, 30, 5);
    assert_eq!(
        calls().iter().filter_map(ended).collect::<Vec<_>>(),
        vec![TooltipEndReason::LeftSafeZone]
    );

    // 預けた文字の表示の失敗でも、来た知らせは出る。
    let mut rig = Rig::new(Some("説明"));
    FAIL.set(true);
    rig.start();
    let got = calls();
    assert_eq!(got[0], show("説明", 30, 20));
    assert!(is_started(&got[1]), "{got:?}");
}

#[test]
fn later_shows_keep_the_anchor_of_the_first_show() {
    // 預けた文字: 初めて出した位置 (30,20) に、動いた後に渡しても出す。
    let mut rig = Rig::new(Some("説明"));
    let token = rig.start();
    rig.frame(900, 50, 30);
    calls();
    rig.supply(1000, token, "y");
    assert_eq!(calls(), vec![show("y", 30, 20)]);

    // 預けていない: 初めて出すまではマウスに付いて動き、出した後は動かない。
    let mut rig = Rig::new(None);
    let token = rig.start();
    rig.frame(900, 50, 30);
    calls();
    rig.supply(1000, token, "1");
    rig.frame(1100, 20, 15);
    rig.supply(1200, token, "2");
    assert_eq!(calls(), vec![show("1", 50, 30), show("2", 50, 30)]);
}

#[test]
fn unregister_hides_and_ends_before_returning() {
    let mut rig = Rig::new(Some("説明"));
    let token = rig.start();
    calls();

    let (window, id, now) = (rig.window, rig.id, rig.now(900));
    assert!(with_fake(|tip| unregister_with(
        &mut rig.world,
        id,
        now,
        tip
    )));
    assert_eq!(
        calls(),
        vec![
            Call::Hide,
            Call::Notice(TooltipNotice::TurnEnded {
                window,
                range: id,
                token,
                reason: TooltipEndReason::RangeUnregistered,
            }),
        ]
    );
    // 2 回目は無いので偽。
    assert!(!with_fake(|tip| unregister_with(
        &mut rig.world,
        id,
        now,
        tip
    )));
    assert!(calls().is_empty());
    // 表は残し、取り消した持ち手を新しい登録に使い回さない。
    let again = register(
        &mut rig.world,
        window,
        TooltipRange {
            area: TooltipArea::WholeWindow,
            text: None,
        },
    )
    .unwrap();
    assert_ne!(again, id);
}

#[test]
fn dismiss_hides_and_the_turn_continues() {
    let mut rig = Rig::new(Some("説明"));
    let token = rig.start();
    calls();
    let now = rig.now(900);

    assert!(with_fake(|tip| dismiss_with(
        &mut rig.world,
        token,
        now,
        tip
    )));
    assert_eq!(calls(), vec![Call::Hide]);
    // 出す番は続く（渡せばまた出る・見回りで終わらない）。
    rig.frame(1000, 30, 20);
    assert!(calls().is_empty());
    assert!(matches!(rig.supply(1100, token, "x"), TooltipSupply::Shown));
    assert_eq!(calls(), vec![show("x", 30, 20)]);

    // 終わった印は偽で、何も呼ばない。
    rig.frame(1200, 200, 100);
    calls();
    let now = rig.now(1300);
    assert!(!with_fake(|tip| dismiss_with(
        &mut rig.world,
        token,
        now,
        tip
    )));
    assert!(calls().is_empty());
}

#[test]
fn shown_and_hidden_are_debug_with_fields_and_no_body() {
    const BODY: &str = "ひみつの本文";
    let mut rig = Rig::new(Some(BODY));
    let debug = |e: &&log_capture_kit::CapturedEvent| e.level == tracing::Level::DEBUG;

    let (token, events) = log_capture_kit::capture(|| rig.start());
    let shown = events
        .iter()
        .filter(debug)
        .find(|e| e.message().contains("[tooltip_shown]"))
        .unwrap_or_else(|| panic!("{events:?}"));
    assert_eq!(
        shown.field("window"),
        Some(format!("{:?}", rig.window).as_str())
    );
    assert_eq!(shown.field("token"), Some(format!("{token:?}").as_str()));
    assert_eq!(shown.field_str("source"), Some("stored"));
    assert_eq!(shown.field("chars"), Some("6"));
    let mut all = events;

    let ((), events) = log_capture_kit::capture(|| {
        rig.supply(900, token, BODY);
    });
    let supplied = events
        .iter()
        .filter(debug)
        .find(|e| e.message().contains("[tooltip_shown]"))
        .unwrap_or_else(|| panic!("{events:?}"));
    assert_eq!(supplied.field_str("source"), Some("supplied"));
    all.extend(events);

    let ((), events) = log_capture_kit::capture(|| rig.frame(1000, 200, 100));
    let hidden = events
        .iter()
        .filter(debug)
        .find(|e| e.message().contains("[tooltip_hidden]"))
        .unwrap_or_else(|| panic!("{events:?}"));
    assert_eq!(
        hidden.field("window"),
        Some(format!("{:?}", rig.window).as_str())
    );
    assert_eq!(hidden.field("token"), Some(format!("{token:?}").as_str()));
    assert_eq!(hidden.field("reason"), Some("End(LeftSafeZone)"));
    all.extend(events);

    // 表示の失敗も本文を載せない。
    let mut rig = Rig::new(None);
    let token = rig.start();
    FAIL.set(true);
    let (_, events) = log_capture_kit::capture(|| rig.supply(900, token, BODY));
    all.extend(events);

    for e in &all {
        for (name, value) in e.fields_map() {
            assert!(!value.contains(BODY), "{name} に本文が載った: {e:?}");
        }
    }
}

#[test]
fn past_deadlines_are_not_rearmed_in_the_past() {
    let t = Instant::now();
    let later = t + Duration::from_millis(50);
    assert_eq!(wake_at(later, t), later, "先の期限はそのまま");
    for past in [t, t - Duration::from_millis(5)] {
        assert_eq!(wake_at(past, t), t + Duration::from_millis(100));
    }
}

/// 記録のうち `[tooltip_hidden]`（debug）の理由の並び。
fn hidden_reasons(events: &[log_capture_kit::CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::DEBUG && e.message().contains("[tooltip_hidden]"))
        .filter_map(|e| e.field("reason").map(str::to_owned))
        .collect()
}

/// 出ていたツールチップの矩形の中（窓の左上からの物理ピクセル）。(30,20) で出した矩形は
/// (10,-20)-(50,0) で、範囲 (10,10)-(60,40) の外。出ていれば安全地帯の中、消えていれば外。
const IN_FORMER_TIP: (i32, i32) = (30, -10);

#[test]
fn clearing_with_empty_text_removes_the_tip_from_the_safe_zone() {
    let mut rig = Rig::new(Some("説明"));
    let token = rig.start();
    calls();
    let (got, events) = log_capture_kit::capture(|| rig.supply(900, token, ""));
    assert!(matches!(got, TooltipSupply::Cleared), "{got:?}");
    assert_eq!(hidden_reasons(&events), vec!["EmptyText".to_owned()]);
    assert_eq!(calls(), vec![Call::Hide]);

    // 消えたツールチップの所は安全地帯ではない。
    rig.frame(1000, IN_FORMER_TIP.0, IN_FORMER_TIP.1);
    assert_eq!(
        calls().iter().filter_map(ended).collect::<Vec<_>>(),
        vec![TooltipEndReason::LeftSafeZone]
    );
    // 消えてから 200 ミリ秒以内に入り直すと、設定の 1 倍（400 ミリ秒）で来る。
    rig.frame(1050, 30, 20);
    rig.frame(1449, 30, 20);
    assert!(!calls().iter().any(is_started));
    rig.frame(1450, 30, 20);
    assert!(calls().iter().any(is_started));
}

#[test]
fn failed_replacement_removes_the_tip_from_the_safe_zone() {
    let mut rig = Rig::new(Some("説明"));
    let token = rig.start();
    calls();
    FAIL.set(true);
    let (got, events) = log_capture_kit::capture(|| rig.supply(900, token, "置き換え"));
    let TooltipSupply::Failed(err) = got else {
        panic!("{got:?}")
    };
    let failed = events
        .iter()
        .find(|e| e.level == tracing::Level::WARN && e.message().contains("[tooltip_show_failed]"))
        .unwrap_or_else(|| panic!("{events:?}"));
    assert_eq!(failed.field("token"), Some(format!("{token:?}").as_str()));
    assert_eq!(failed.field("error"), Some(err.to_string().as_str()));
    calls();

    rig.frame(1000, IN_FORMER_TIP.0, IN_FORMER_TIP.1);
    assert_eq!(
        calls().iter().filter_map(ended).collect::<Vec<_>>(),
        vec![TooltipEndReason::LeftSafeZone]
    );
}

#[test]
fn hidden_reasons_for_dismiss_and_unregister_are_logged() {
    let mut rig = Rig::new(Some("説明"));
    let token = rig.start();
    let now = rig.now(900);
    let (_, events) =
        log_capture_kit::capture(|| with_fake(|tip| dismiss_with(&mut rig.world, token, now, tip)));
    assert_eq!(hidden_reasons(&events), vec!["Dismissed".to_owned()]);

    rig.supply(1000, token, "x");
    let (id, now) = (rig.id, rig.now(1100));
    let (_, events) =
        log_capture_kit::capture(|| with_fake(|tip| unregister_with(&mut rig.world, id, now, tip)));
    assert_eq!(
        hidden_reasons(&events),
        vec!["End(RangeUnregistered)".to_owned()]
    );
}

#[test]
fn emptied_table_returns_early_without_reading_the_os() {
    let mut rig = Rig::new(Some("説明"));
    let (id, now) = (rig.id, rig.now(0));
    assert!(with_fake(|tip| unregister_with(
        &mut rig.world,
        id,
        now,
        tip
    )));
    let samples = Cell::new(0);
    let frame = |rig: &mut Rig, ms: u64| {
        let now = rig.now(ms);
        with_fake(|tip| {
            frame_with(
                &mut rig.world,
                now,
                &mut || {
                    samples.set(samples.get() + 1);
                    OsSample {
                        cursor: Some(screen(30, 20)),
                        button_down: false,
                    }
                },
                &|_, _| true,
                &mut || Duration::from_millis(400),
                tip,
            )
        });
    };
    frame(&mut rig, 100);
    assert_eq!(samples.get(), 0, "空の表だけなら OS の見本を読まない");

    // 較正: 登録し直せば読む。
    let window = rig.window;
    register(
        &mut rig.world,
        window,
        TooltipRange {
            area: TooltipArea::WholeWindow,
            text: None,
        },
    )
    .unwrap();
    frame(&mut rig, 200);
    assert_eq!(samples.get(), 1);
}
