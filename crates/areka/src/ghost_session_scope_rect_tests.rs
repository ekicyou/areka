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

/// スコープ 0 にキャラクターとバルーンの印を持つ表と、常に `LEFT_SCREEN` を返す偽の OS。
fn source(target: RectTarget) -> ScopeRectSource {
    let mut map = BTreeMap::new();
    map.insert(
        0,
        ScopeWindowKeys {
            character: Some(WindowKey(1)),
            balloon: Some(WindowKey(2)),
        },
    );
    ScopeRectSource::new(
        target,
        Arc::new(RwLock::new(ScopeWindowTable(map))),
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

/// 壊れた窓の印（0）は値なしで、呼んだスレッドの DPI の扱いは呼ぶ前と同じに戻る（要件 4.4・1.3）。
#[test]
fn os_window_rect_invalid_handle_is_none_and_restores_dpi_context() {
    use windows::Win32::UI::HiDpi::{AreDpiAwarenessContextsEqual, GetThreadDpiAwarenessContext};
    let before = unsafe { GetThreadDpiAwarenessContext() };
    assert_eq!(os_window_rect(WindowKey(0)), None);
    let after = unsafe { GetThreadDpiAwarenessContext() };
    assert!(unsafe { AreDpiAwarenessContextsEqual(before, after) }.as_bool());
}

// ---- 表の出し方（結線と系）----

mod wiring {
    use super::super::*;
    use super::LEFT_SCREEN;
    use crate::placement::follow::OffsetBase;
    use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
    use crate::placement::source::GhostTitles;
    use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};
    use areka_sylphya::{AskerContext, AskerId, DottedResolution, MirrorImage, SharedMirror};
    use bevy_ecs::schedule::Schedule;
    use windows::Win32::Foundation::{HINSTANCE, HWND};

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

    /// スコープ 0 の窓を組み、キャラクターの窓にだけ印（0x10）を付けた World。
    fn world_with_character_handle() -> (World, GhostWindows) {
        let mut world = World::new();
        let gw = spawn_ghost_windows(
            &mut world,
            &[placement()],
            &GhostTitles::from_scope_titles([(0, "s".to_string())]),
        );
        let ch = gw.char_window(0).unwrap();
        world.entity_mut(ch).insert(fake_handle(0x10));
        (world, gw)
    }

    fn table(world: &World) -> ScopeWindowTable {
        world
            .resource::<ScopeRectWiring>()
            .table
            .read()
            .unwrap()
            .clone()
    }

    fn keys(character: Option<isize>, balloon: Option<isize>) -> ScopeWindowKeys {
        ScopeWindowKeys {
            character: character.map(WindowKey),
            balloon: balloon.map(WindowKey),
        }
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
}
