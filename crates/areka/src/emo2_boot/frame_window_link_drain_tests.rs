//! 窓の組・吸着・バルーン位置初期化の指令が、台本から位置の線を通って取り出しの相で当たる檻
//! （areka-P0-window-link-directives task 6・要件 1.4・1.7・3.1・3.5・7.1・設計 議題 8）。
//!
//! 受け口（`MoveCueSink`）へ台本と同じ cue を配り、同じ線を `run_move_drain_phase` で取り出す。
//! 1 本の線なので、同じ台本の `\![set,sticky-window,0,1]` の後の `\![move]` は届いた順に当たる。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};

use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use bevy_ecs::world::World;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::{DPI, Point, WindowHandle, WindowPos};

use crate::emo2_boot::move_cue::{MoveCueSink, PositionCue};
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::emo2_boot::zorder_cue::ZOrderDirective;
use crate::placement::follow::{BalloonFollow, MonitorSnapshot, OffsetBase};
use crate::placement::prepare_ghost_windows_with_work_area;
use crate::placement::resolver::PointPx;
use crate::placement::shared_test_support::{WA, balloon_root, emo2_root, with_com_initialized};
use crate::placement::spawn::{BalloonKeywordBase, spawn_ghost_windows};
use crate::placement::sticky_group_ledger::{StickyGroupLedger, seed_sticky_descript};

use super::test_support::{pos_of, resnap_world, synth_assets, zero_clock};
use super::{Emo2Wiring, run_move_drain_phase};

/// 位置の線の送り口を持つ受け口と、それを受ける headless な結線。
fn sink_and_wiring() -> (MoveCueSink, Emo2Wiring) {
    let (tx, rx): (Sender<PositionCue>, _) = mpsc::channel();
    let wiring = Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        rx,
        mpsc::channel::<TalkLifecycleSignal>().1,
        mpsc::channel::<ZOrderDirective>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        zero_clock(),
        synth_assets(&[(0, 0)]),
    );
    (MoveCueSink::new(tx), wiring)
}

/// `\N![name,tokens...]`（`N` は実行者）の cue。
fn carrier(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

/// スコープ 1 を基準スコープ 0 から動かす `\1\![move,-353,40,,0,base,base]`。
fn move_scope1() -> TalkCue {
    carrier("1", "move", &["-353", "40", "", "0", "base", "base"])
}

fn diff(after: Point, before: Point) -> Point {
    Point {
        x: after.x - before.x,
        y: after.y - before.y,
    }
}

/// 同じ台本の `\![set,sticky-window,0,1]` → `\![move]` は届いた順に当たり、組の相手が同じ量だけ
/// 連れ動く。順を逆にすると相手は動かない（届いた順で当たっていることの対照）。
#[test]
fn set_sticky_then_move_applies_in_arrival_order_and_partner_follows() {
    for sticky_first in [true, false] {
        let (mut world, gw) = resnap_world();
        seed_sticky_descript(&mut world, None);
        let target = gw.char_window(1).unwrap();
        let partner = gw.char_window(0).unwrap();
        let (t0, p0) = (
            pos_of(&world, target).unwrap(),
            pos_of(&world, partner).unwrap(),
        );
        let (mut sink, wiring) = sink_and_wiring();
        let sticky = carrier("0", "set", &["sticky-window", "0", "1"]);
        if sticky_first {
            sink.emit(sticky);
            sink.emit(move_scope1());
        } else {
            sink.emit(move_scope1());
            sink.emit(sticky);
        }

        run_move_drain_phase(&wiring, &mut world);

        let moved = diff(pos_of(&world, target).unwrap(), t0);
        let followed = diff(pos_of(&world, partner).unwrap(), p0);
        assert_ne!(moved, Point { x: 0, y: 0 }, "動かす窓が動いていない");
        if sticky_first {
            assert_eq!(followed, moved, "組の相手が同じ量だけ連れ動いていない");
        } else {
            assert_eq!(
                followed,
                Point { x: 0, y: 0 },
                "組を作る前の移動で相手が動いた"
            );
        }
        assert_eq!(
            world.resource::<StickyGroupLedger>().partners_of(1),
            vec![0],
            "組が帳簿に入っていない"
        );
        drop(world);
        let _residue = drain_window_pos_commands();
    }
}

/// `\![reset,sticky-window]` は台本の組を解き、`\![set,alignmenttodesktop,…]` は吸着の手続きへ届く
/// （実行者のスコープの窓・記録は手続きの中）。
#[test]
fn reset_sticky_and_alignment_reach_their_procedures() {
    let (mut world, _gw) = resnap_world();
    seed_sticky_descript(&mut world, None);
    let (mut sink, wiring) = sink_and_wiring();
    sink.emit(carrier("0", "set", &["sticky-window", "0", "1"]));
    sink.emit(carrier("0", "reset", &["sticky-window"]));
    sink.emit(carrier("1", "set", &["alignmenttodesktop", "top"]));

    let ((), events) =
        crate::placement::test_support::capture_logs(|| run_move_drain_phase(&wiring, &mut world));

    assert!(
        world.resource::<StickyGroupLedger>().groups().is_empty(),
        "台本の組が解けていない"
    );
    let alignment: Vec<_> = events
        .iter()
        .filter(|e| {
            e.field_str("event")
                .is_some_and(|ev| ev.starts_with("alignment_"))
        })
        .collect();
    assert!(
        !alignment.is_empty(),
        "吸着の手続きへ届いていない: {events:?}"
    );
    assert!(
        alignment.iter().all(|e| e.field("scope") == Some("1")),
        "実行者のスコープではない: {alignment:?}"
    );
    drop(world);
    let _residue = drain_window_pos_commands();
}

/// 台本の `\![execute,resetballoonpos]` で、ずらしたバルーンが標準の位置（起動の準備が解いた位置）へ戻る。
#[test]
fn execute_resetballoonpos_returns_the_balloon() {
    with_com_initialized(|| {
        let prepared =
            prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
                .expect("起動の配置の準備は成功する");
        let mut world = World::new();
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![WA],
        });
        let gw = spawn_ghost_windows(&mut world, &prepared.placements, &prepared.titles);
        let mut raw = 0x100;
        for scope in gw.scopes() {
            for e in [gw.char_window(scope), gw.balloon_window(scope)] {
                raw += 1;
                world.entity_mut(e.unwrap()).insert(WindowHandle {
                    hwnd: HWND(raw as *mut _),
                    instance: HINSTANCE::default(),
                });
            }
        }
        world.flush();
        for scope in gw.scopes() {
            for e in [gw.char_window(scope), gw.balloon_window(scope)] {
                world.entity_mut(e.unwrap()).insert(DPI::from_dpi(96, 96));
            }
        }
        world.insert_resource(prepared.default_basis.clone());
        // ドラッグの後と同じ形に、スコープ 0 のバルーンをずらす。
        let (c, b) = (gw.char_window(0).unwrap(), gw.balloon_window(0).unwrap());
        let dragged = PointPx { x: 777, y: -333 };
        let char_pos = pos_of(&world, c).unwrap();
        world
            .entity_mut(c)
            .insert(BalloonFollow::new(b, OffsetBase::unpinned(dragged)));
        world.entity_mut(c).remove::<BalloonKeywordBase>();
        world.get_mut::<WindowPos>(b).unwrap().position =
            Some(Point::new(char_pos.x + dragged.x, char_pos.y + dragged.y));
        let standard = prepared
            .placements
            .iter()
            .find(|p| p.scope == 0)
            .unwrap()
            .balloon_pos;

        let (mut sink, wiring) = sink_and_wiring();
        sink.emit(carrier("0", "execute", &["resetballoonpos"]));
        run_move_drain_phase(&wiring, &mut world);

        assert_eq!(
            pos_of(&world, b),
            Some(Point::new(standard.x, standard.y)),
            "バルーンが標準の位置へ戻っていない"
        );
        drop(world);
        let _residue = drain_window_pos_commands();
    });
}
