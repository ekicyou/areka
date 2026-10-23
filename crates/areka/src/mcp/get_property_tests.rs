//! `get_property` の決定論テスト（2 本）。
//!
//! - 実行系が無い（空の World）: `NG:Property system is not available`（isError: true）で答え、
//!   `event = "mcp_get_property_unavailable"` の `warn!` をちょうど 1 件残す（要件 3.1・3.3）。
//! - 本物の実行系を 1 回だけ起こし、ゴースト自身の問い手・全体・別の問い手・空の値を載せてから
//!   8 つの名前を聞く（値・空の値・無い名前・問い手の選び方・名前を手直ししない・要件 1.1〜1.4・2.1〜2.3）。
//!
//! どちらも `handle` を直に呼び、答えは返った直後に `try_answer` で取り出す（その場で答える・
//! 要件 4.2）。`ghost_name` はすべて省略（要件 4.8）。英字の大小違いの名前は
//! `areka-P0-property-name-case-fold` が足した `BASEWARE.NAME` の 1 つだけ（大小を畳んで `areka`・
//! 同 spec の要件 2.1）。前に空白のある ` baseware.name` は空白を削らず「無い名前」のまま。

use std::path::PathBuf;

use areka_ghost::sylphya_wiring::ghost_asker_id;
use areka_mcp::ToolOutcome;
use areka_mcp::tools::{ToolCall, ToolRequest};
use areka_sylphya::AskerId;
use log_capture_kit::capture;

use super::*;
use crate::ghost_session::GhostSlot;

/// `handle` を 1 回呼び、返った直後の答えを取り出す（まだ答えていなければ None）。
fn ask(world: &mut World, ghost: &ActiveGhost, property_name: &str) -> Option<ToolOutcome> {
    let args = Args {
        property_name: property_name.to_string(),
        ghost_name: None,
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetProperty(args.clone()));
    handle(world, ghost, args, req.reply);
    pending.try_answer().ok().flatten().map(|a| a.outcome)
}

#[test]
fn answers_unavailable_and_warns_once_without_a_runtime() {
    let ghost = ActiveGhost {
        name: Some("emily4".to_string()),
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let mut world = World::new();

    let (answer, events) = capture(|| ask(&mut world, &ghost, "baseware.name"));

    let warned: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("mcp_get_property_unavailable"))
        .map(|e| {
            (
                e.level,
                e.field_str("ghost").map(str::to_owned),
                e.field_str("property_name").map(str::to_owned),
            )
        })
        .collect();
    assert_eq!(
        (answer, warned),
        (
            Some(outcome::ng("Property system is not available")),
            vec![(
                tracing::Level::WARN,
                Some("emily4".to_owned()),
                Some("baseware.name".to_owned())
            )]
        ),
        "（その場の答え・記録の段と欄 ghost・property_name）"
    );
}

#[test]
fn reads_values_through_the_ghost_own_asker_on_a_real_runtime() {
    use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script(r"\0A\e"))),
    )]);
    rig.boot("A");
    let (publisher, own) = {
        let runtime = rig
            .world
            .get_non_send::<GhostSlot>()
            .and_then(|slot| slot.0.as_ref())
            .and_then(|session| session.runtime())
            .expect("置き場に実行系がある");
        (
            runtime.sylphya_publisher().clone(),
            ghost_asker_id(&runtime.mount().shiori.dir),
        )
    };
    let someone_else = AskerId::new("someone-else");
    publisher.set(own.clone(), "test.key".into(), "ghost".into());
    publisher.publish_static(
        own.clone(),
        vec![],
        vec![("test.key".into(), "global".into())],
    );
    publisher.set(someone_else.clone(), "test.key".into(), "other".into());
    publisher.set(someone_else, "test.other_only".into(), "other".into());
    publisher.set(own, "test.empty".into(), String::new());
    publisher.barrier().expect("載せた値が反映される");

    let ghost = crate::mcp::resolve::active(&rig.world).expect("起動中のゴーストがある");
    let names = [
        "baseware.name",
        "test.key",
        "test.other_only",
        "test.empty",
        "currentghost.name",
        "",
        " baseware.name",
        "BASEWARE.NAME",
    ];
    let answers: Vec<_> = names
        .iter()
        .map(|name| ask(&mut rig.world, &ghost, name))
        .collect();
    let down = rig.shutdown();

    let not_found = || Some(outcome::ng("Cannot find such property name."));
    assert_eq!(
        (answers, down),
        (
            vec![
                Some(outcome::value("areka")),
                Some(outcome::value("ghost")),
                not_found(),
                Some(outcome::value("")),
                not_found(),
                not_found(),
                not_found(),
                Some(outcome::value("areka")),
            ],
            true
        ),
        "（{names:?} への答え・降ろせた）"
    );
}

// ---- 窓の矩形（spec mcp-scope-rect-property 要件 3.4・6.3・6.6・7.4・8.3）----

mod scope_rect {
    use std::sync::Arc;

    use areka_ghost::sylphya_wiring::ghost_asker_id;
    use shiori_abi::error::ShioriError;
    use shiori_abi::interface::IShioriHost;
    use windows::Win32::Foundation::{HINSTANCE, HWND};
    use windows::core::HSTRING;
    use wintf::ecs::WindowHandle;

    use super::*;
    use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
    use crate::ghost_session::ghost_session_scope_rect::{
        ScopeRectReadSeam, ScreenRect, WindowKey,
    };
    use crate::ghost_session::{RuntimeRebuild, rebuild_runtime_in_place};
    use crate::placement::follow::OffsetBase;
    use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
    use crate::placement::source::GhostTitles;
    use crate::placement::spawn::spawn_ghost_windows;
    use crate::shiori_host::ShioriHostSink;

    /// 窓の印ごとに違う矩形（左＝印の値）を返す偽の OS で、答えの数からどの窓を引いたか分かる。
    fn rect_of(raw: isize) -> ScreenRect {
        let x = raw as i32;
        ScreenRect {
            left: x,
            top: 0,
            right: x + 1,
            bottom: 1,
        }
    }

    fn fake_handle(raw: usize) -> WindowHandle {
        WindowHandle {
            hwnd: HWND(raw as *mut _),
            instance: HINSTANCE::default(),
        }
    }

    fn placement(scope: usize) -> ScopePlacement {
        ScopePlacement {
            scope,
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

    /// 起動の前に偽の OS と、スコープ 0・1 の窓の一式（キャラクター 0x10・0x11、バルーン 0x20・0x21）を
    /// 窓なしで入れた A の試験台を起こし、定常に着かせる。
    fn booted_rig() -> SwitchRig {
        let mut rig = SwitchRig::new(vec![(
            "A",
            FakeShiori::Scripted(Box::new(|| standard_script(r"\0A\e"))),
        )]);
        rig.plant_boot_record("A");
        rig.world
            .insert_resource(ScopeRectReadSeam(Arc::new(|k: WindowKey| {
                Some(rect_of(k.0))
            })));
        let gw = spawn_ghost_windows(
            &mut rig.world,
            &[placement(0), placement(1)],
            &GhostTitles::from_scope_titles([(0, "s".to_string()), (1, "k".to_string())]),
        );
        for (scope, ch, ba) in [(0, 0x10, 0x20), (1, 0x11, 0x21)] {
            rig.world
                .entity_mut(gw.char_window(scope).unwrap())
                .insert(fake_handle(ch));
            rig.world
                .entity_mut(gw.balloon_window(scope).unwrap())
                .insert(fake_handle(ba));
        }
        rig.boot("A");
        assert!(rig.wait_steady(), "A が定常に着く");
        rig
    }

    const NAMES: [&str; 4] = [
        "currentghost.scope(0).rect",
        "currentghost.scope(1).rect",
        "currentghost.balloon.scope(0).rect",
        "currentghost.balloon.scope(2).rect",
    ];

    /// 同じ時点に MCP の `handle` と、同じ実行系の読み口から作ったゴーストの道（`ShioriHostSink` の
    /// `GetProperty`）が同じ値・同じ値なしを返す（要件 3.4・6.6・7.4・8.3）。
    ///
    /// # 非空虚性
    /// 結線（`wire`）を外すと両方の道が全部「無い名前」で赤。ゴーストの道だけが読み手に届かないと
    /// ゴーストの答えが `PropertyNotFound` になって赤。
    #[test]
    fn mcp_and_ghost_paths_answer_the_same_window_rects() {
        let mut rig = booted_rig();
        let sink = {
            let runtime = rig
                .world
                .get_non_send::<GhostSlot>()
                .and_then(|slot| slot.0.as_ref())
                .and_then(|session| session.runtime())
                .expect("置き場に実行系がある");
            ShioriHostSink::with_sylphya(
                runtime.sylphya_reader().clone(),
                runtime.sylphya_publisher().clone(),
                ghost_asker_id(&runtime.mount().shiori.dir),
            )
        };
        let host: IShioriHost = sink.into();
        let ghost = crate::mcp::resolve::active(&rig.world).expect("起動中のゴーストがある");

        let mcp: Vec<_> = NAMES
            .iter()
            .map(|n| ask(&mut rig.world, &ghost, n))
            .collect();
        let shiori: Vec<_> = NAMES
            .iter()
            .map(|n| match host.get_property(&HSTRING::from(*n)) {
                Ok(v) => Ok(v.to_string()),
                Err(ShioriError::PropertyNotFound) => Err("PropertyNotFound".to_string()),
                Err(e) => Err(format!("{e:?}")),
            })
            .collect();
        let down = rig.shutdown();

        assert_eq!(
            (mcp, shiori, down),
            (
                vec![
                    Some(outcome::value("16,0,17,1")),
                    Some(outcome::value("17,0,18,1")),
                    Some(outcome::value("32,0,33,1")),
                    Some(outcome::ng("Cannot find such property name.")),
                ],
                vec![
                    Ok("16,0,17,1".to_string()),
                    Ok("17,0,18,1".to_string()),
                    Ok("32,0,33,1".to_string()),
                    Err("PropertyNotFound".to_string()),
                ],
                true
            ),
            "（{NAMES:?} への MCP の答え・ゴーストの道の答え・降ろせた）"
        );
    }

    /// `reload`（実行系をその場で作り直す道）の後、MCP の `handle` が新しい sylphya の読み口で同じ窓の
    /// 矩形を答える（要件 6.3）。前の実行系にだけ載せた `test.marker` が後で無い名前になることで、
    /// 答えが新しい読み口から来たと分かる。
    ///
    /// # 非空虚性
    /// `rewire_runtime_consumers` の `wire` を外すと、作り直した後の矩形が「無い名前」で赤。
    #[test]
    fn reload_answers_the_same_rect_through_the_new_reader() {
        let mut rig = booted_rig();
        {
            let runtime = rig
                .world
                .get_non_send::<GhostSlot>()
                .and_then(|slot| slot.0.as_ref())
                .and_then(|session| session.runtime())
                .expect("置き場に実行系がある");
            let publisher = runtime.sylphya_publisher();
            publisher.set(
                ghost_asker_id(&runtime.mount().shiori.dir),
                "test.marker".into(),
                "old".into(),
            );
            publisher.barrier().expect("載せた値が反映される");
        }
        let ghost = crate::mcp::resolve::active(&rig.world).expect("起動中のゴーストがある");
        let ask_both =
            |rig: &mut SwitchRig| [NAMES[0], "test.marker"].map(|n| ask(&mut rig.world, &ghost, n));
        let before = ask_both(&mut rig);
        let rebuilt = rebuild_runtime_in_place(&mut rig.world, "A");
        let after = ask_both(&mut rig);
        let down = rig.shutdown();

        assert!(matches!(rebuilt, RuntimeRebuild::Rebuilt), "{rebuilt:?}");
        let rect = || Some(outcome::value("16,0,17,1"));
        assert_eq!(
            (before, after, down),
            (
                [rect(), Some(outcome::value("old"))],
                [rect(), Some(outcome::ng("Cannot find such property name."))],
                true
            ),
            "（作り直す前の答え・作り直した後の答え・降ろせた）"
        );
    }
}
