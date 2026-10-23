//! 窓の矩形の読み手の決定論テスト（数の形と選択子の読み）。

use super::*;
use areka_sylphya::parse_dotted;

/// 主の画面より左上の画面の矩形（SSP の段 1 の値）。
const LEFT_SCREEN: ScreenRect = ScreenRect {
    left: -782,
    top: 201,
    right: 49,
    bottom: 1227,
};

fn keys(character: Option<isize>, balloon: Option<isize>) -> ScopeWindowKeys {
    ScopeWindowKeys {
        character: character.map(WindowKey),
        balloon: balloon.map(WindowKey),
    }
}

/// 左上と幅・高さから矩形を作る（右＝左＋幅・下＝上＋高さ）。
fn rect_of(left: i32, top: i32, width: i32, height: i32) -> ScreenRect {
    ScreenRect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

fn source_with(
    target: RectTarget,
    map: BTreeMap<usize, ScopeWindowKeys>,
    read_rect: WindowRectFn,
) -> ScopeRectSource {
    ScopeRectSource::new(
        target,
        Arc::new(RwLock::new(ScopeWindowTable(map))),
        read_rect,
    )
}

/// スコープ 0 にキャラクターとバルーンの印を持つ表と、常に `LEFT_SCREEN` を返す偽の OS。
fn source(target: RectTarget) -> ScopeRectSource {
    source_with(
        target,
        BTreeMap::from([(0, keys(Some(1), Some(2)))]),
        Arc::new(|_| Some(LEFT_SCREEN)),
    )
}

fn read(src: &ScopeRectSource, name: &str) -> Option<String> {
    src.read(&parse_dotted(name).unwrap())
}

#[test]
fn rect_text_joins_four_numbers_without_spaces() {
    assert_eq!(rect_text(LEFT_SCREEN), "-782,201,49,1227");
}

#[test]
fn index_selector_answers_the_window_rect() {
    let ch = source(RectTarget::Character);
    let ba = source(RectTarget::Balloon);
    assert_eq!(
        read(&ch, "currentghost.scope(0).rect").as_deref(),
        Some("-782,201,49,1227")
    );
    assert_eq!(
        read(&ba, "currentghost.balloon.scope(0).rect").as_deref(),
        Some("-782,201,49,1227")
    );
}

#[test]
fn negative_selector_is_a_scope_without_window() {
    let ch = source(RectTarget::Character);
    let ba = source(RectTarget::Balloon);
    assert_eq!(
        read(&ch, "currentghost.scope(-1).rect").as_deref(),
        Some("0,0,0,0")
    );
    assert_eq!(read(&ba, "currentghost.balloon.scope(-1).rect"), None);
}

#[test]
fn malformed_selector_has_no_value() {
    let ch = source(RectTarget::Character);
    let ba = source(RectTarget::Balloon);
    for sel in ["x", "", "+1", "-"] {
        assert_eq!(
            read(&ch, &format!("currentghost.scope({sel}).rect")),
            None,
            "{sel}"
        );
        assert_eq!(
            read(&ba, &format!("currentghost.balloon.scope({sel}).rect")),
            None,
            "{sel}"
        );
    }
}

/// 主の画面より左上の画面（`-2560,195`）の `-782,201` から幅 831・高さ 1026 の窓（要件 1.3〜1.5・8.2）。
#[test]
fn left_upper_screen_window_answers_left_top_right_bottom() {
    let rect = rect_of(-782, 201, 831, 1026);
    for (target, name) in [
        (RectTarget::Character, "currentghost.scope(0).rect"),
        (RectTarget::Balloon, "currentghost.balloon.scope(0).rect"),
    ] {
        let src = source_with(
            target,
            BTreeMap::from([(0, keys(Some(1), Some(2)))]),
            Arc::new(move |_| Some(rect)),
        );
        assert_eq!(read(&src, name).as_deref(), Some("-782,201,49,1227"));
    }
}

/// 表に無い番号・印の無い窓・OS の失敗・負の番号は窓が無い答え（要件 3.1〜3.3・3.5）。
#[test]
fn scope_without_window_is_zero_for_character_and_none_for_balloon() {
    let map = BTreeMap::from([
        (0, keys(Some(1), Some(2))),
        (1, keys(None, None)),
        (3, keys(Some(7), Some(7))),
    ]);
    // 印 7 は OS が読めない（壊れた窓）。
    let read_rect: WindowRectFn = Arc::new(|k| (k != WindowKey(7)).then_some(LEFT_SCREEN));
    let ch = source_with(RectTarget::Character, map.clone(), read_rect.clone());
    let ba = source_with(RectTarget::Balloon, map, read_rect);
    for sel in ["5", "1", "3", "-1"] {
        assert_eq!(
            read(&ch, &format!("currentghost.scope({sel}).rect")).as_deref(),
            Some("0,0,0,0"),
            "{sel}"
        );
        assert_eq!(
            read(&ba, &format!("currentghost.balloon.scope({sel}).rect")),
            None,
            "{sel}"
        );
    }
    // 窓の在るスコープは答える（偽の読み取りが本当に答えを持つことの確かめ）。
    assert_eq!(
        read(&ba, "currentghost.balloon.scope(0).rect").as_deref(),
        Some("-782,201,49,1227")
    );
}

/// 1 回の読みは OS へ 1 回だけ聞き、答えはその 1 回の矩形そのもの（要件 4.7・8.4）。
#[test]
fn one_read_asks_the_os_once_and_answers_that_rect() {
    use std::sync::atomic::{AtomicI32, Ordering};
    for (target, name) in [
        (RectTarget::Character, "currentghost.scope(0).rect"),
        (RectTarget::Balloon, "currentghost.balloon.scope(0).rect"),
    ] {
        let calls = Arc::new(AtomicI32::new(0));
        let c = calls.clone();
        let src = source_with(
            target,
            BTreeMap::from([(0, keys(Some(1), Some(2)))]),
            Arc::new(move |_| {
                let n = c.fetch_add(1, Ordering::SeqCst) + 1;
                Some(rect_of(n, 10 * n, 100 * n, 1000 * n))
            }),
        );
        for n in 1..=3 {
            let expected = rect_text(rect_of(n, 10 * n, 100 * n, 1000 * n));
            assert_eq!(read(&src, name), Some(expected), "{name} {n}");
            assert_eq!(calls.load(Ordering::SeqCst), n, "{name} {n}");
        }
    }
}

/// 偽の OS の矩形を替えた後の読みは新しい矩形。重なった窓は同じ数・3 人目も同じ形（要件 1.7・2.3・4.1〜4.6）。
#[test]
fn reads_after_the_window_moved_answer_the_new_rect() {
    use std::sync::Mutex;
    let os = Arc::new(Mutex::new(BTreeMap::<isize, ScreenRect>::new()));
    let o = os.clone();
    let read_rect: WindowRectFn = Arc::new(move |k| o.lock().unwrap().get(&k.0).copied());
    let map = BTreeMap::from([
        (0, keys(Some(1), Some(2))),
        (1, keys(Some(3), Some(4))),
        (2, keys(Some(5), Some(6))),
    ]);
    let ch = source_with(RectTarget::Character, map.clone(), read_rect.clone());
    let ba = source_with(RectTarget::Balloon, map, read_rect);
    let set = |k: isize, r: ScreenRect| {
        os.lock().unwrap().insert(k, r);
    };
    let answer = |scope: usize| {
        (
            read(&ch, &format!("currentghost.scope({scope}).rect")),
            read(&ba, &format!("currentghost.balloon.scope({scope}).rect")),
        )
    };
    set(1, rect_of(1483, 733, 434, 687));
    set(2, rect_of(1071, 708, 223, 158));
    let steps: [(&str, ScreenRect, ScreenRect); 5] = [
        // ドラッグ: キャラクターだけが動き、バルーンは追って動く。
        (
            "drag",
            rect_of(1200, 500, 434, 687),
            rect_of(788, 475, 223, 158),
        ),
        // `\![move]`: 台本で別の位置へ。
        (
            "move",
            rect_of(100, 300, 434, 687),
            rect_of(-312, 275, 223, 158),
        ),
        // 拡大率の違う左上の画面: 物理の画素で大きさも変わる。
        (
            "dpi",
            rect_of(-782, 201, 831, 1026),
            rect_of(-1400, 160, 446, 316),
        ),
        // 面の替わり: 同じ位置で大きさだけ替わる。
        (
            "surface",
            rect_of(-782, 201, 600, 900),
            rect_of(-1400, 160, 446, 316),
        ),
        // 一緒に動いた窓とバルーン。
        (
            "together",
            rect_of(-682, 251, 600, 900),
            rect_of(-1300, 210, 446, 316),
        ),
    ];
    for (step, c, b) in steps {
        set(1, c);
        set(2, b);
        assert_eq!(
            answer(0),
            (Some(rect_text(c)), Some(rect_text(b))),
            "{step}"
        );
    }
    // 重なった 2 つの窓（スコープ 0 と 1 のキャラクター）は同じ数。
    set(3, rect_of(-682, 251, 600, 900));
    assert_eq!(answer(1).0, answer(0).0);
    // 窓の在る 3 人目（scope(2)）も同じ形で答える。
    set(5, rect_of(300, 400, 120, 240));
    set(6, rect_of(10, 380, 200, 100));
    assert_eq!(
        answer(2),
        (
            Some("300,400,420,640".into()),
            Some("10,380,210,480".into())
        )
    );
}

/// 壊れた窓の印（0）は値なしで、呼んだスレッドの DPI の扱いは呼ぶ前と同じに戻る（要件 4.4・1.3）。
#[test]
fn os_window_rect_invalid_handle_is_none_and_restores_dpi_context() {
    use windows::Win32::UI::HiDpi::{AreDpiAwarenessContextsEqual, GetThreadDpiAwarenessContext};
    let before = unsafe { GetThreadDpiAwarenessContext() };
    assert_eq!(os_window_rect(WindowKey(0)), None);
    let after = unsafe { GetThreadDpiAwarenessContext() };
    assert!(unsafe { AreDpiAwarenessContextsEqual(before, after) }.as_bool());
}

/// 実の窓 1 枚の矩形を、DPI 非対応のスレッドから呼んでも物理の画素で返し、
/// 呼んだスレッドの扱いを非対応へ戻す（要件 1.3・1.6・8.5）。拡大率 100% より大きい画面が要る。
#[test]
#[ignore = "real window: needs a monitor scaled above 100%"]
fn os_window_rect_real_window_answers_physical_pixels_from_unaware_thread() {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::HiDpi::{
        AreDpiAwarenessContextsEqual, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        DPI_AWARENESS_CONTEXT_UNAWARE, GetDpiForWindow, GetThreadDpiAwarenessContext,
        SetThreadDpiAwarenessContext,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos,
        WINDOW_EX_STYLE, WS_POPUP,
    };
    use windows::core::w;

    // 奇数の位置と大きさ（拡大率で割ると端数になり、論理の数なら一致しない）。
    let placed = rect_of(201, 151, 321, 187);
    // SAFETY: Win32 境界。テストのスレッドで自分の窓を作り、置き、読み、壊す。
    unsafe {
        // マニフェストの無いテストの実行ファイルは非対応で走るので、窓を作る前に v2 にする。
        let original = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            w!("scope-rect-real-window"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW");
        SetWindowPos(
            hwnd,
            None,
            placed.left,
            placed.top,
            placed.right - placed.left,
            placed.bottom - placed.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .expect("SetWindowPos");
        let dpi = GetDpiForWindow(hwnd);

        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_UNAWARE);
        let answer = os_window_rect(WindowKey(hwnd.0 as isize));
        let after = GetThreadDpiAwarenessContext();
        let still_unaware = AreDpiAwarenessContextsEqual(after, DPI_AWARENESS_CONTEXT_UNAWARE);

        let _ = DestroyWindow(hwnd);
        SetThreadDpiAwarenessContext(original);

        assert_ne!(
            dpi, 96,
            "拡大率 100% より大きい画面が要る（窓の DPI が 96）"
        );
        assert_eq!(answer, Some(placed));
        assert!(
            still_unaware.as_bool(),
            "読み取りがスレッドの扱いを戻さなかった"
        );
    }
}

// ---- 表の出し方（結線と系）----

mod wiring {
    use super::super::*;
    use super::{LEFT_SCREEN, keys, rect_of};
    use crate::placement::follow::OffsetBase;
    use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
    use crate::placement::source::GhostTitles;
    use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};
    use areka_sylphya::{AskerContext, AskerId, DottedResolution, MirrorImage, SharedMirror};
    use bevy_ecs::schedule::Schedule;
    use bevy_ecs::schedule::SingleThreadedExecutor;
    use log_capture_kit::{LineFormat, capture_lines};
    use std::sync::Mutex;
    use windows::Win32::Foundation::{HINSTANCE, HWND};
    use windows::Win32::UI::WindowsAndMessaging::WS_VISIBLE;
    use wintf::ecs::{Point, WindowPos, WindowStyle};

    fn fake_handle(raw: usize) -> WindowHandle {
        WindowHandle {
            hwnd: HWND(raw as *mut _),
            instance: HINSTANCE::default(),
        }
    }

    fn placement() -> ScopePlacement {
        ScopePlacement {
            scope: 0,
            char_pos: PointPx { x: 1483, y: 733 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 1071, y: 708 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: -412, y: -25 },
            balloon_offset_base: OffsetBase::unpinned(PointPx { x: -412, y: -25 }),
            balloon_limit: true,
            anchor: Anchor::Bottom,
            balloon_keyword_base: None,
        }
    }

    /// スコープ 0 の窓の一式だけ（印なし）を組んだ World。
    fn world_without_handles() -> (World, GhostWindows) {
        let mut world = World::new();
        let gw = spawn_ghost_windows(
            &mut world,
            &[placement()],
            &GhostTitles::from_scope_titles([(0, "s".to_string())]),
        );
        (world, gw)
    }

    /// スコープ 0 の窓を組み、キャラクターの窓にだけ印（0x10）を付けた World。
    fn world_with_character_handle() -> (World, GhostWindows) {
        let (mut world, gw) = world_without_handles();
        let ch = gw.char_window(0).unwrap();
        world.entity_mut(ch).insert(fake_handle(0x10));
        (world, gw)
    }

    /// 印 → 矩形の偽の OS を差し替え口に入れ、その中身を返す（載っていない印は読めない窓）。
    fn install_fake_os(world: &mut World) -> Arc<Mutex<BTreeMap<isize, ScreenRect>>> {
        let os = Arc::new(Mutex::new(BTreeMap::new()));
        let o = os.clone();
        world.insert_resource(ScopeRectReadSeam(Arc::new(move |k: WindowKey| {
            o.lock().unwrap().get(&k.0).copied()
        })));
        os
    }

    /// 表を出す系だけのフレーム（記録の捕捉はスレッドごとなので、テストのスレッドで回す）。
    fn frame() -> Schedule {
        let mut schedule = Schedule::default();
        schedule.set_executor(SingleThreadedExecutor::new());
        schedule.add_systems(sync_scope_window_table);
        schedule
    }

    /// 置き場の表を目印へ差し替える（系が組み直せば消える）。
    fn plant(world: &World, marker: &ScopeWindowTable) {
        *world.resource::<ScopeRectWiring>().table.write().unwrap() = marker.clone();
    }

    fn table(world: &World) -> ScopeWindowTable {
        world
            .resource::<ScopeRectWiring>()
            .table
            .read()
            .unwrap()
            .clone()
    }

    fn reader() -> SylphyaReader {
        SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())))
    }

    fn resolve(reader: &SylphyaReader, name: &str) -> DottedResolution {
        let asker = AskerContext {
            asker: AskerId::new("test"),
        };
        reader.resolve_dotted_str(&asker, name)
    }

    /// 表は窓の一式のスコープごとに窓の印を引き、印の無い窓は空にする（要件 3.3）。
    #[test]
    fn wire_builds_the_table_from_ghost_windows_and_handles() {
        let (mut world, _) = world_with_character_handle();
        wire_reader(&mut world, &reader());
        assert_eq!(table(&world).0.get(&0), Some(&keys(Some(0x10), None)));
    }

    /// 結線は差し替え口の読み取りで 2 つの形へ読み手を登録する（要件 3.5・7.6）。
    #[test]
    fn wire_registers_both_shapes_with_the_seam() {
        let (mut world, _) = world_with_character_handle();
        world.insert_resource(ScopeRectReadSeam(Arc::new(|_| Some(LEFT_SCREEN))));
        let r = reader();
        wire_reader(&mut world, &r);
        assert_eq!(
            resolve(&r, "currentghost.scope(0).rect"),
            DottedResolution::Value("-782,201,49,1227".into())
        );
        // バルーンの窓には印がまだ無い＝値なし。
        assert_eq!(
            resolve(&r, "currentghost.balloon.scope(0).rect"),
            DottedResolution::NotFound
        );
    }

    /// 系は窓の印が付いた時だけ表を組み直し、変わらないフレームでは触らない（要件 6.1・6.2・7.7）。
    #[test]
    fn sync_rebuilds_only_when_handles_or_windows_change() {
        let (mut world, gw) = world_with_character_handle();
        wire_reader(&mut world, &reader());
        let mut schedule = Schedule::default();
        schedule.add_systems(sync_scope_window_table);
        // 1 回目は結線の前に付いた印の Added で起きるが、中身は同じ。
        schedule.run(&mut world);
        assert_eq!(table(&world).0.get(&0), Some(&keys(Some(0x10), None)));

        // 変わらないフレームでは組み直さない（置き場へ置いた目印が残る）。
        let marker = ScopeWindowTable(BTreeMap::from([(9, keys(Some(9), Some(9)))]));
        *world.resource::<ScopeRectWiring>().table.write().unwrap() = marker.clone();
        schedule.run(&mut world);
        assert_eq!(table(&world), marker);

        // バルーンの窓に印が付いたら組み直す。
        let ba = gw.balloon_window(0).unwrap();
        world.entity_mut(ba).insert(fake_handle(0x20));
        schedule.run(&mut world);
        assert_eq!(
            table(&world).0,
            BTreeMap::from([(0, keys(Some(0x10), Some(0x20)))])
        );

        // 印が外れたら組み直す。
        world.entity_mut(ba).remove::<WindowHandle>();
        schedule.run(&mut world);
        assert_eq!(
            table(&world).0,
            BTreeMap::from([(0, keys(Some(0x10), None))])
        );
    }

    /// 置き場が無い（実行系が無い）なら系は何もしない。
    #[test]
    fn sync_without_wiring_does_nothing() {
        let (mut world, _) = world_with_character_handle();
        let mut schedule = Schedule::default();
        schedule.add_systems(sync_scope_window_table);
        schedule.run(&mut world);
        assert!(!world.contains_resource::<ScopeRectWiring>());
    }

    fn answers(r: &SylphyaReader) -> (DottedResolution, DottedResolution) {
        (
            resolve(r, "currentghost.scope(0).rect"),
            resolve(r, "currentghost.balloon.scope(0).rect"),
        )
    }

    fn value(s: &str) -> DottedResolution {
        DottedResolution::Value(s.into())
    }

    /// 印が付く前は窓が無い答え・付いた後のフレームで矩形・entity を消したフレームで窓が無い答え。
    /// 可視の変化では表が替わらず、隠れていても矩形を答える（要件 1.8・2.4・3.3・3.5）。
    #[test]
    fn table_follows_handles_and_despawn_but_not_visibility() {
        let (mut world, gw) = world_without_handles();
        let os = install_fake_os(&mut world);
        os.lock().unwrap().extend([
            (0x10, rect_of(1483, 733, 434, 687)),
            (0x20, rect_of(1071, 708, 223, 158)),
        ]);
        let r = reader();
        wire_reader(&mut world, &r);
        let mut schedule = frame();
        schedule.run(&mut world);
        let none = (value("0,0,0,0"), DottedResolution::NotFound);
        assert_eq!(answers(&r), none);

        let (ch, ba) = (gw.char_window(0).unwrap(), gw.balloon_window(0).unwrap());
        world.entity_mut(ch).insert(fake_handle(0x10));
        world.entity_mut(ba).insert(fake_handle(0x20));
        schedule.run(&mut world);
        let shown = (value("1483,733,1917,1420"), value("1071,708,1294,866"));
        assert_eq!(answers(&r), shown);

        // 可視の変化（面を隠す・バルーンを隠す相当）では組み直さない（目印が残る）。
        let current = table(&world);
        let marker = ScopeWindowTable(BTreeMap::from([(9, keys(Some(9), Some(9)))]));
        plant(&world, &marker);
        for e in [ch, ba] {
            world.get_mut::<WindowPos>(e).unwrap().hide_window = true;
            world.get_mut::<WindowStyle>(e).unwrap().style &= !WS_VISIBLE;
        }
        schedule.run(&mut world);
        assert_eq!(table(&world), marker);
        plant(&world, &current);
        assert_eq!(answers(&r), shown);

        // entity を消したフレームで窓が無い答え（窓の一式はそのスコープの記載ごと外す）。
        world.despawn(ch);
        schedule.run(&mut world);
        assert_eq!(answers(&r), none);
    }

    /// 窓の一式を入れ直したら（`Changed<GhostWindows>`・印の付け外しなし）表を組み直す。
    #[test]
    fn replacing_ghost_windows_rebuilds_the_table() {
        let (mut world, _) = world_with_character_handle();
        wire_reader(&mut world, &reader());
        let mut schedule = frame();
        schedule.run(&mut world);
        assert_eq!(table(&world).0.get(&0), Some(&keys(Some(0x10), None)));

        spawn_ghost_windows(
            &mut world,
            &[placement()],
            &GhostTitles::from_scope_titles([(0, "s".to_string())]),
        );
        schedule.run(&mut world);
        assert_eq!(table(&world).0.get(&0), Some(&keys(None, None)));
    }

    /// 窓の位置を毎フレーム書き換える間、表の仕事は 0 件。読みを繰り返しても WARN 以上は 0 件
    /// （要件 5.4・5.5）。
    #[test]
    fn moving_windows_does_no_table_work_and_reads_never_warn() {
        let (mut world, gw) = world_with_character_handle();
        let (ch, ba) = (gw.char_window(0).unwrap(), gw.balloon_window(0).unwrap());
        world.entity_mut(ba).insert(fake_handle(0x20));
        // バルーンの印 0x20 は偽の OS に載せない＝読めない窓（失敗の道でも WARN を出さない）。
        let os = install_fake_os(&mut world);
        let r = reader();
        wire_reader(&mut world, &r);
        let mut schedule = frame();
        schedule.run(&mut world);

        let marker = ScopeWindowTable(BTreeMap::from([
            (0, keys(Some(0x10), Some(0x20))),
            (9, keys(Some(9), None)),
        ]));
        plant(&world, &marker);

        let ((), lines) = capture_lines(LineFormat::LevelTargetFields, || {
            for n in 0..40 {
                for e in [ch, ba] {
                    world.get_mut::<WindowPos>(e).unwrap().position = Some(Point { x: n, y: -n });
                }
                let rect = rect_of(n, -n, 434, 687);
                os.lock().unwrap().insert(0x10, rect);
                schedule.run(&mut world);
                assert_eq!(
                    answers(&r),
                    (value(&rect_text(rect)), DottedResolution::NotFound)
                );
            }
        });
        let updated = |lines: &[String]| {
            lines
                .iter()
                .filter(|l| l.contains("scope_rect_table_updated"))
                .count()
        };
        assert_eq!(table(&world), marker);
        assert_eq!(updated(&lines), 0, "{lines:?}");
        assert!(
            !lines
                .iter()
                .any(|l| l.starts_with("level=WARN") || l.starts_with("level=ERROR")),
            "{lines:?}"
        );
        // 捕捉が効いていることの確かめ: 読めない窓の記録は取れていて、印が外れれば 1 件出る。
        assert!(lines.iter().any(|l| l.contains("scope_rect_window_gone")));
        let ((), lines) = capture_lines(LineFormat::LevelTargetFields, || {
            world.entity_mut(ba).remove::<WindowHandle>();
            schedule.run(&mut world);
        });
        assert_eq!(updated(&lines), 1, "{lines:?}");
    }
}
