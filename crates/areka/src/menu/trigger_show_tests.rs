//! 計画ができてから動作が終わるまで（[`display`]・[`finish`]・[`run_show`]）の決定論テスト
//! （要件 1.1・1.4・1.7・1.10・6.5・7.2・7.4・8.1〜8.3・8.5）。
//!
//! OS のメニューは出さない: 表示の関数は引数で差し替えられるので、呼ばれた値を書き留めて
//! 決めた結果を返すだけの閉包を渡す。外側の World は本番と同じ `Rc<RefCell<EcsWorld>>` を
//! 実際に作って使う（窓は作らない）。表示のタスクを実行器へ載せる 1 行だけはここでは通らない。
//!
//! 期待値（記録の綴り・識別子・座標・項目数）はすべて書き下した値で、実装の表から導かない。

use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use areka_kanade::KanadeMsg;
use areka_kanade::resources::ResourceOutcome;
use log_capture_kit::{LineFormat, capture_lines};
use windows::Win32::Foundation::{E_ACCESSDENIED, HINSTANCE};

use super::*;
use crate::emo2_boot::hit_region::HitRegion;
use crate::input_events::RegionSource;
use crate::menu::ItemBody;
use crate::menu::captions::CaptionMap;

/// 動作が呼ばれた記録: どの項目が・どのスコープで・表示中の印が立ったまま呼ばれたか。
type Calls = Rc<RefCell<Vec<(&'static str, u32, bool)>>>;

/// 当たり判定の代役（このテストでは当たり判定を引かないので中身は使われない）。
fn no_region(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: None,
        surface_point: (x, y),
    }
}

/// テスト用の一式。外側の World は本番と同じ型で、窓 entity は相方側（スコープ 1）。
struct Fixture {
    outer: Rc<RefCell<EcsWorld>>,
    /// 運行（kanade）宛ての受信端。預かりが送られてしまえばここへ届く。
    kanade: Receiver<KanadeMsg>,
    window: Entity,
    /// 表示 1 枚の旗。
    flag: Rc<Cell<bool>>,
    calls: Calls,
}

fn fixture() -> Fixture {
    let (tx, kanade) = channel::<KanadeMsg>();
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    world.insert_non_send(MouseWiring::new(tx, RegionSource::Mock(no_region)));
    let window = world
        .spawn(WindowHandle {
            hwnd: fake_hwnd(),
            instance: HINSTANCE::default(),
        })
        .id();
    Fixture {
        outer: Rc::new(RefCell::new(ecs)),
        kanade,
        window,
        flag: Rc::new(Cell::new(false)),
        calls: Rc::new(RefCell::new(Vec::new())),
    }
}

/// 偽の窓ハンドル（OS へは渡らない）。
fn fake_hwnd() -> HWND {
    HWND(0x1234 as *mut _)
}

impl Fixture {
    fn weak(&self) -> Weak<RefCell<EcsWorld>> {
        Rc::downgrade(&self.outer)
    }

    fn request(&self) -> MenuRequest {
        MenuRequest {
            scope: 1,
            entity: self.window,
            hwnd: fake_hwnd(),
            screen_pos: (1030, 2040),
        }
    }

    /// 呼ばれたことを書き留めるだけの動作を持つ項目。
    fn item(&self, name: &'static str) -> MenuItem {
        let calls = Rc::clone(&self.calls);
        let flag = Rc::clone(&self.flag);
        MenuItem {
            label: name.to_string(),
            caption_resource: None,
            enabled: true,
            checked: None,
            body: ItemBody::Action(Rc::new(move |_, ctx| {
                calls.borrow_mut().push((name, ctx.scope, flag.get()));
            })),
        }
    }

    /// 説明書（識別子 1）と終了（識別子 2）の 2 項目の計画。
    fn plan(&self) -> MenuPlan {
        plan::build(
            vec![
                (Frame::Readme, self.item("readme")),
                (Frame::Close, self.item("close")),
            ],
            &CaptionMap::default(),
        )
    }

    /// 旗を立てた、表示の準備が整ったメニュー。
    fn ready(&self) -> ReadyMenu {
        ReadyMenu {
            request: self.request(),
            plan: self.plan(),
            guard: InFlightGuard::engage(&self.flag),
        }
    }

    /// 表示中に届いた右ダブルクリックを 1 件預ける（押下ハンドラが置くのと同じ形）。
    fn defer_double_click(&self) {
        self.outer
            .borrow_mut()
            .world_mut()
            .get_non_send_mut::<MouseWiring>()
            .expect("MouseWiring は挿入済み")
            .defer_right_double_click(PendingDoubleClick {
                scope: 1,
                surface_pos: (7, 4),
                region: None,
            });
    }

    /// 預かりが残っているか（取り出すので、呼んだ後は必ず空になる）。
    fn deferred_double_click_remains(&self) -> bool {
        take_deferred_double_click(self.outer.borrow_mut().world_mut()).is_some()
    }

    fn remove_window_handle(&self) {
        self.outer
            .borrow_mut()
            .world_mut()
            .entity_mut(self.window)
            .remove::<WindowHandle>();
    }

    fn calls(&self) -> Vec<(&'static str, u32, bool)> {
        self.calls.borrow().clone()
    }
}

fn access_denied() -> ShowResult {
    Err(windows::core::Error::from_hresult(E_ACCESSDENIED))
}

/// World を借りられる終わり方 1 つ分: 名前・表示の結果・表示中に窓が消えていたか。
type Exit = (&'static str, fn() -> ShowResult, bool);

/// World を借りられる終わり方の全部（選択・未選択・表示失敗・窓が消えていた）。
fn exits() -> [Exit; 4] {
    [
        ("selected", || Ok(Some(2)), false),
        ("dismissed", || Ok(None), false),
        ("failed", access_denied, false),
        ("window gone", || Ok(Some(2)), true),
    ]
}

/// `[menu]` の記録のうち、指定のレベルの行だけを拾う。
fn menu_lines<'a>(lines: &'a [String], level: &str) -> Vec<&'a String> {
    let needle = format!("level={level}");
    lines
        .iter()
        .filter(|l| l.contains(&needle) && l.contains("[menu]"))
        .collect()
}

/// 指定のレベルの `[menu]` の行がちょうど 1 行で、挙げた綴りを全部含むことを確かめる。
fn assert_one_line(lines: &[String], level: &str, needles: &[&str]) {
    let found = menu_lines(lines, level);
    assert_eq!(found.len(), 1, "{level}: {lines:?}");
    for needle in needles {
        assert!(
            found[0].contains(needle),
            "{needle:?} が無い: {:?}",
            found[0]
        );
    }
}

/// `finish` を記録を捕まえながら呼ぶ。
fn finish_captured(f: &Fixture, outcome: ShowResult) -> Vec<String> {
    let (request, plan, weak) = (f.request(), f.plan(), f.weak());
    capture_lines(LineFormat::LevelFields, || {
        finish(&weak, &request, &plan, outcome)
    })
    .1
}

/// 表示は「出した」を 1 行記録してから表示の関数を 1 回呼び、その結果をそのまま返す。
#[test]
fn display_logs_shown_once_and_calls_the_os_function_with_the_request() {
    let f = fixture();
    let (request, plan) = (f.request(), f.plan());
    let mut seen = Vec::new();

    let (outcome, lines) = capture_lines(LineFormat::LevelFields, || {
        display(&request, &plan, |hwnd, pos, plan| {
            seen.push((hwnd, pos, plan.item_count()));
            Ok(Some(2))
        })
    });

    assert_eq!(outcome.expect("表示の関数の結果がそのまま返る"), Some(2));
    assert_eq!(seen, [(fake_hwnd(), (1030, 2040), 2)]);
    assert_one_line(&lines, "INFO", &["[menu] shown", "scope=1", "items=2"]);
    assert_eq!(menu_lines(&lines, "ERROR").len(), 0, "{lines:?}");
}

/// 選ばれた識別子の動作だけが、ちょうど 1 回、操作した窓のスコープで呼ばれる。
#[test]
fn a_selected_item_runs_its_own_action_exactly_once() {
    let f = fixture();
    let lines = finish_captured(&f, Ok(Some(2)));
    assert_one_line(
        &lines,
        "INFO",
        &["[menu] selected", "scope=1", "frame=Close", "id=2"],
    );
    assert!(menu_lines(&lines, "DEBUG").is_empty(), "{lines:?}");
    assert_eq!(f.calls(), [("close", 1, false)]);

    let g = fixture();
    let lines = finish_captured(&g, Ok(Some(1)));
    assert_one_line(
        &lines,
        "INFO",
        &["[menu] selected", "scope=1", "frame=Readme", "id=1"],
    );
    assert_eq!(g.calls(), [("readme", 1, false)]);
}

/// 何も選ばずに閉じたら `debug!` を 1 行出し、動作は呼ばない。
#[test]
fn a_dismissed_menu_logs_once_and_runs_nothing() {
    let f = fixture();
    let lines = finish_captured(&f, Ok(None));
    assert_one_line(&lines, "DEBUG", &["[menu] dismissed", "scope=1"]);
    assert!(menu_lines(&lines, "INFO").is_empty(), "{lines:?}");
    assert!(f.calls().is_empty());
}

/// 表示に失敗したら `error!` を 1 行出し、動作は呼ばない。
#[test]
fn a_failed_display_logs_one_error_and_runs_nothing() {
    let f = fixture();
    let lines = finish_captured(&f, access_denied());
    assert_one_line(
        &lines,
        "ERROR",
        &[
            "[menu] TrackPopupMenuEx failed",
            "error=",
            "hresult=0x80070005",
        ],
    );
    assert!(menu_lines(&lines, "INFO").is_empty(), "{lines:?}");
    assert!(f.calls().is_empty());
}

/// 計画に無い識別子が返ってきたら `warn!` を 1 行出し、動作は呼ばない。
#[test]
fn an_unknown_id_logs_one_warning_and_runs_nothing() {
    let f = fixture();
    let lines = finish_captured(&f, Ok(Some(99)));
    assert_one_line(&lines, "WARN", &["id=99"]);
    assert!(menu_lines(&lines, "INFO").is_empty(), "{lines:?}");
    assert!(f.calls().is_empty());
}

/// 表示中に窓が消えていたら、選ばれたことは記録するが動作は呼ばない（窓ハンドルが外れた場合）。
#[test]
fn a_selection_on_a_window_without_a_handle_runs_nothing() {
    let f = fixture();
    f.remove_window_handle();
    let lines = finish_captured(&f, Ok(Some(2)));
    assert_one_line(&lines, "INFO", &["[menu] selected", "id=2"]);
    assert_one_line(
        &lines,
        "DEBUG",
        &["[menu] window gone after menu", "scope=1", "id=2"],
    );
    assert!(f.calls().is_empty());
}

/// 同じく、窓の entity そのものが消えていた場合。
#[test]
fn a_selection_on_a_despawned_window_runs_nothing() {
    let f = fixture();
    f.outer.borrow_mut().world_mut().despawn(f.window);
    let lines = finish_captured(&f, Ok(Some(2)));
    assert_one_line(
        &lines,
        "DEBUG",
        &["[menu] window gone after menu", "scope=1", "id=2"],
    );
    assert!(f.calls().is_empty());
}

/// 外側の World が既に捨てられていても落ちず、動作は呼ばない。
#[test]
fn a_dropped_outer_world_is_logged_and_runs_nothing() {
    let f = fixture();
    let (request, plan, weak) = (f.request(), f.plan(), f.weak());
    let Fixture { outer, calls, .. } = f;
    drop(outer);

    let (_, lines) = capture_lines(LineFormat::LevelFields, || {
        finish(&weak, &request, &plan, Ok(Some(2)))
    });

    assert_one_line(&lines, "INFO", &["[menu] selected", "id=2"]);
    assert_one_line(
        &lines,
        "DEBUG",
        &[
            "[menu] world unavailable after menu",
            "reason=\"world dropped\"",
        ],
    );
    assert!(calls.borrow().is_empty());
}

/// 外側の World が借りられている最中でも落ちず、待たず、動作は呼ばない。
#[test]
fn a_busy_outer_world_is_logged_and_runs_nothing() {
    let f = fixture();
    let (request, plan, weak) = (f.request(), f.plan(), f.weak());
    let held = f.outer.borrow_mut();

    let (_, lines) = capture_lines(LineFormat::LevelFields, || {
        finish(&weak, &request, &plan, Ok(Some(2)))
    });
    drop(held);

    assert_one_line(
        &lines,
        "DEBUG",
        &[
            "[menu] world unavailable after menu",
            "reason=\"world busy\"",
        ],
    );
    assert!(f.calls().is_empty());
}

/// 表示中に届いた預かりは、World を借りられたどの終わり方でも捨てられ、送られない（要件 1.10）。
#[test]
fn a_double_click_deferred_while_the_menu_was_open_is_dropped_on_every_exit() {
    for (name, outcome, window_gone) in exits() {
        let f = fixture();
        if window_gone {
            f.remove_window_handle();
        }
        f.defer_double_click();

        finish_captured(&f, outcome());

        assert!(!f.deferred_double_click_remains(), "{name}: 預かりが残った");
        assert!(f.kanade.try_recv().is_err(), "{name}: 預かりが送られた");
    }
}

/// 表示中の印は表示の間も動作の間も立っていて、どの終わり方でも終わった後に降りている。
/// 表示の間、外側の World は借りられていない（要件 7.2）。
#[test]
fn the_in_flight_flag_covers_display_and_action_and_always_lowers() {
    for (name, outcome, window_gone) in exits() {
        let f = fixture();
        if window_gone {
            f.remove_window_handle();
        }
        let ready = f.ready();
        assert!(f.flag.get(), "{name}: 準備ができた時点で印は立っている");
        let mut shows = 0;

        run_show(&f.weak(), ready, |_, _, _| {
            shows += 1;
            assert!(f.flag.get(), "{name}: 表示の間も印は立っている");
            assert!(
                f.outer.try_borrow_mut().is_ok(),
                "{name}: 表示の間は外側の World を借りていない"
            );
            outcome()
        });

        assert_eq!(shows, 1, "{name}: 表示は 1 要求につき 1 回");
        assert!(!f.flag.get(), "{name}: 終わったら印は降りている");
        let expected: &[(&str, u32, bool)] = if name == "selected" {
            &[("close", 1, true)]
        } else {
            &[]
        };
        assert_eq!(f.calls(), expected, "{name}");
    }
}

/// 外側の World が捨てられていた終わり方でも印は降りる。
#[test]
fn the_in_flight_flag_lowers_when_the_outer_world_is_gone() {
    let f = fixture();
    let (ready, weak) = (f.ready(), f.weak());
    let Fixture {
        outer, flag, calls, ..
    } = f;
    drop(outer);

    run_show(&weak, ready, |_, _, _| Ok(Some(2)));

    assert!(!flag.get());
    assert!(calls.borrow().is_empty());
}

/// 表示が失敗したときの記録は「出した」→「失敗」の順に 1 行ずつ出る。
#[test]
fn a_failed_display_logs_shown_then_the_error() {
    let f = fixture();
    let (ready, weak) = (f.ready(), f.weak());

    let (_, lines) = capture_lines(LineFormat::LevelFields, || {
        run_show(&weak, ready, |_, _, _| access_denied())
    });

    let menu: Vec<&String> = lines.iter().filter(|l| l.contains("[menu]")).collect();
    assert_eq!(menu.len(), 2, "{lines:?}");
    assert!(menu[0].contains("[menu] shown"), "{:?}", menu[0]);
    assert!(
        menu[1].contains("[menu] TrackPopupMenuEx failed"),
        "{:?}",
        menu[1]
    );
}

/// 表示のタスクが 1 度も進まないまま捨てられても印は降りる（OS の表示は呼ばれない）。
#[test]
fn a_show_task_dropped_before_it_runs_lowers_the_flag() {
    let f = fixture();
    let ReadyMenu {
        request,
        plan,
        guard,
    } = f.ready();

    let task = show_task(f.weak(), request, plan, guard);
    assert!(f.flag.get());
    drop(task);

    assert!(!f.flag.get());
}

/// 計画ができた tick に外側の World への参照が無ければ、`warn!` を 1 行出して要求を捨てる
/// （表示のタスクは起こさない・印は降りる）。
#[test]
fn a_ready_plan_without_the_outer_world_reference_is_dropped_with_one_warning() {
    let (tx, _kanade) = channel::<KanadeMsg>();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::new(tx.clone(), RegionSource::Mock(no_region)));
    let window = world
        .spawn(WindowHandle {
            hwnd: fake_hwnd(),
            instance: HINSTANCE::default(),
        })
        .id();
    let (reply, rx) = areka_actor::reply_channel::<QueryReply>();
    reply
        .send(vec![(
            "kero.popupmenu.visible",
            ResourceOutcome::Value("1".to_string()),
        )])
        .expect("受け口は生きている");
    let mut wiring = MenuWiring::with_to_screen(tx, |_, x, y| Some((x, y)));
    let guard = InFlightGuard::engage(&wiring.in_flight);
    wiring.pending = Some(PendingQuery::new(
        MenuRequest {
            scope: 1,
            entity: window,
            hwnd: fake_hwnd(),
            screen_pos: (1030, 2040),
        },
        Vec::new(),
        Some(rx),
        Instant::now() + Duration::from_secs(60),
        guard,
    ));
    world.insert_non_send(wiring);

    let (_, lines) = capture_lines(LineFormat::LevelFields, || poll_menu_query(&mut world));

    assert_one_line(&lines, "WARN", &["[menu]", "scope=1"]);
    let wiring = world.get_non_send::<MenuWiring>().expect("挿入済み");
    assert!(wiring.pending.is_none());
    assert!(!wiring.in_flight.get(), "要求を捨てたら印は降りる");
}
