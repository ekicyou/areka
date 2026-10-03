//! 入口の受理の判定・待ちの開始・取り出しの系・終了の片付けの決定論テスト（spec:
//! areka-P0-shell-balloon-switch 要件 1.1・1.5・1.12・1.13・1.14・1.16・2.1・5.7）。
//!
//! 置き場のゴーストは kanade の送出端だけを持つテスト用の組み立てに、テスト用の seriko の送り手を
//! 持たせたもの。実行系が無いので今のシェルは無く、`OnShellChanging` の Ref1 は空文字列になる
//! （Ref1 の実物は `shell_balloon_switch_session_tests.rs` が偽の SHIORI で突き合わせる）。
//! 目録は一時フォルダに実物の `descript.txt` を置いて読む。判定は集めてから 1 回。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use areka_actor::reply_channel;
use areka_emo_compose::BindSet;
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::BasewareRoot;
use areka_kanade::{GapRaise, KanadeMsg, ShioriMethod};
use areka_seriko::{
    AnimationTable, BindResolver, MockSurfaceOutput, SerikoLoopConfig, SerikoSink, SurfaceResolver,
    spawn_seriko,
};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;
use wintf::ecs::Input;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::assets::{BootAssets, LoopTables};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::ghost_switch::{PrevGhost, SwitchInFlight, SwitchStage, SwitchTarget};
use crate::emo2_boot::talk_clock::TalkClock;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::update::desk::{UpdateDesk, insert_running_desk_for_test};

// ---------------------------------------------------------------- 土台

/// 一時の根と、文脈・置き場・結線をそろえた World（今のゴーストは `A`・今のバルーンは `kaku`）。
struct Rig {
    world: World,
    /// kanade の代わり（置き場のゴーストの送出端の受信端）。
    kanade: Receiver<KanadeMsg>,
    root: BasewareRoot,
    _temp: TempPath,
}

fn write_descript(dir: &Path, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\n{body}\r\n"),
    )
    .unwrap();
}

/// テスト用の seriko の送り手（空の定義・ループ無し・出力は観測用の器）。
fn test_seriko_sink() -> SerikoSink {
    spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::default(),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        MockSurfaceOutput::new(),
    )
    .0
}

/// 窓も GPU も持たない結線（入口は有無だけを見る）。
fn headless_wiring() -> Emo2Wiring {
    Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel().1,
        mpsc::channel().1,
        mpsc::channel().1,
        mpsc::channel().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        TalkClock::new(Arc::new(|| 0.0)),
        BootAssets {
            shells: Vec::new(),
            balloons: Vec::new(),
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: BindSet::default(),
            bind_resolver: BindResolver::empty(),
            loop_tables: LoopTables {
                shell: AnimationTable::empty(),
                balloon: BTreeMap::new(),
            },
            shell_author_dpi: 96,
            balloon_author_dpi: 96,
            boxes: crate::emo2_boot::shell_box_assets::ShellBoxAssets::default(),
        },
    )
}

/// 置き場のゴースト（kanade の送出端と seriko の送り手を持つかを選ぶ）。
fn session(
    root: &BasewareRoot,
    kanade: Option<mpsc::Sender<KanadeMsg>>,
    seriko: bool,
) -> GhostSlot {
    let s = GhostSession::for_test(kanade, root.ghost_dir("A"));
    GhostSlot(Some(if seriko {
        s.with_seriko_sink(test_seriko_sink())
    } else {
        s
    }))
}

fn rig(label: &str) -> Rig {
    let temp = TempPath::new(label);
    let root = BasewareRoot::new(temp.path().to_path_buf());
    let shell = root.ghost_dir("A").join("shell");
    write_descript(&shell.join("master"), "name,通常");
    write_descript(&shell.join("summer"), "name,夏服");
    write_descript(&root.balloon_dir("kaku"), "name,かくかく");
    write_descript(&root.balloon_dir("fluffy"), "name,ふわふわ");

    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_resource(BootContext {
        root: root.clone(),
        app_profile_dir: temp.path().join("profile"),
        helper_exe: temp.path().join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("A"),
                balloon_root: root.balloon_dir("kaku"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir("A"),
                folder: Some("A".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("kaku"),
                folder: Some("kaku".to_owned()),
            },
        },
    });
    let (tx, kanade) = mpsc::channel();
    world.insert_non_send(session(&root, Some(tx), true));
    world.insert_non_send(headless_wiring());
    Rig {
        world,
        kanade,
        root,
        _temp: temp,
    }
}

fn req(kind: SkinKind, target: SkinSpec, origin: SkinOrigin) -> SkinRequest {
    SkinRequest {
        kind,
        target,
        origin,
    }
}

fn name(s: &str) -> SkinSpec {
    SkinSpec::Name(s.to_owned())
}

fn folder(s: &str) -> SkinSpec {
    SkinSpec::Folder(s.to_owned())
}

const PLAIN: SkinOrigin = SkinOrigin::Script { raise_event: false };
const RAISE: SkinOrigin = SkinOrigin::Script { raise_event: true };

/// kanade に届いた便り（待ちの依頼は印、それ以外は `None`）。
fn sent(rx: &Receiver<KanadeMsg>) -> Vec<Option<Option<GapRaise>>> {
    rx.try_iter()
        .map(|m| match m {
            KanadeMsg::AwaitTalkGap { raise, .. } => Some(raise),
            _ => None,
        })
        .collect()
}

fn count(events: &[CapturedEvent], event: &str, level: Level) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event) && e.level == level)
        .count()
}

fn level_count(events: &[CapturedEvent], level: Level) -> usize {
    events.iter().filter(|e| e.level == level).count()
}

fn absolute(path: &Path) -> String {
    std::path::absolute(path)
        .expect("絶対パスにする")
        .display()
        .to_string()
}

/// 印のイベント（`OnShellChanging`・Ref0＝切替先の名前・Ref1＝今のシェルの名前・Ref2＝絶対パス）。
fn changing(to_name: &str, current_name: &str, to_dir: &Path) -> GapRaise {
    GapRaise {
        id: "OnShellChanging".to_owned(),
        references: vec![
            to_name.to_owned(),
            current_name.to_owned(),
            absolute(to_dir),
        ],
        method: ShioriMethod::Get,
    }
}

/// 進行中の印を直に据える（受信端は空の線）。
fn plant_marker(world: &mut World) {
    world.insert_non_send(SkinSwitchInFlight {
        kind: SkinKind::Shell,
        target: SkinCandidate {
            dir: PathBuf::from("planted"),
            folder: "planted".to_owned(),
            name: None,
            hidden: false,
        },
        stage: SkinSwitchStage::Waiting {
            gap: Some(reply_channel().1),
            gap_result: None,
            build: mpsc::channel().1,
            built: None,
        },
    });
}

fn plant_ghost_switch(world: &mut World) {
    world.insert_non_send(SwitchInFlight {
        target: SwitchTarget {
            dir: PathBuf::from("B"),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: PathBuf::from("A"),
            name: None,
            sakura_name: None,
        },
        stage: SwitchStage::SendOff,
        boot_event: None,
    });
}

fn marker_folder(world: &World) -> Option<String> {
    world
        .get_non_send::<SkinSwitchInFlight>()
        .map(|m| m.target.folder.clone())
}

// ---------------------------------------------------------------- 断り

/// 1 件の要求を記録つきで出し、(判定, 名指しの `warn!` の件数, `warn!` の総数, kanade への便りの数)。
fn refuse(rig: &mut Rig, request: SkinRequest, event: &str) -> (SkinVerdict, usize, usize, usize) {
    let (verdict, events) = capture(|| request_skin_switch(&mut rig.world, request));
    (
        verdict,
        count(&events, event, Level::WARN),
        level_count(&events, Level::WARN),
        sent(&rig.kanade).len(),
    )
}

/// 判定は 進行中 → ゴースト切替中 → 更新中 → 文脈なし → 該当なし の順。断りの全部を立ててから
/// 1 つずつ外し、各段で名指しの `warn!` が 1 件・kanade への便りが 0 件、最後に受理へ至る
/// （要件 1.7・1.12・1.13・1.16）。
#[test]
fn refusals_in_order_leave_one_warning_each_and_send_nothing() {
    let mut rig = rig("areka-skin-entry-refusals");
    plant_marker(&mut rig.world);
    plant_ghost_switch(&mut rig.world);
    insert_running_desk_for_test(&mut rig.world);
    rig.world.remove_non_send::<Emo2Wiring>();
    let unknown = || req(SkinKind::Shell, name("無い"), PLAIN);

    let mut got = Vec::new();
    got.push(refuse(&mut rig, unknown(), "skin_switch_busy"));
    rig.world.remove_non_send::<SkinSwitchInFlight>();
    got.push(refuse(&mut rig, unknown(), "skin_switch_ghost_switching"));
    rig.world.remove_non_send::<SwitchInFlight>();
    got.push(refuse(&mut rig, unknown(), "skin_switch_updating"));
    rig.world.remove_non_send::<UpdateDesk>();
    got.push(refuse(&mut rig, unknown(), "skin_switch_no_context"));
    rig.world.insert_non_send(headless_wiring());
    got.push(refuse(&mut rig, unknown(), "skin_switch_unknown"));
    let marker_before_accept = marker_folder(&rig.world);
    let accepted = request_skin_switch(&mut rig.world, req(SkinKind::Shell, name("夏服"), PLAIN));

    assert_eq!(
        (got, marker_before_accept, accepted, sent(&rig.kanade).len()),
        (
            vec![
                (SkinVerdict::Busy, 1, 1, 0),
                (SkinVerdict::GhostSwitching, 1, 1, 0),
                (SkinVerdict::Updating, 1, 1, 0),
                (SkinVerdict::NoContext, 1, 1, 0),
                (SkinVerdict::NotFound, 1, 1, 0),
            ],
            None,
            SkinVerdict::Accepted,
            1,
        ),
        "(断りの各段の (判定, 名指しの warn!, warn! の総数, 便り), 受理の前の印, 最後の判定, 受理の便り)"
    );
}

/// 文脈なし: 置き場のゴースト・起動の文脈（根）・kanade の送出端・seriko の送り手のどれが欠けても
/// `warn!` 1 件で断り、kanade へは何も送らず印も置かない（結線の欠けは上の順の判定が見る）。
#[test]
fn missing_context_refuses_with_one_warning() {
    let cases: [(&str, fn(&mut Rig)); 4] = [
        ("ghost_slot", |r| {
            r.world.insert_non_send(GhostSlot(None));
        }),
        ("boot_context", |r| {
            r.world.remove_resource::<BootContext>();
        }),
        ("kanade", |r| {
            let slot = session(&r.root, None, true);
            r.world.insert_non_send(slot);
        }),
        ("seriko_sink", |r| {
            let (tx, rx) = mpsc::channel();
            r.kanade = rx;
            let slot = session(&r.root, Some(tx), false);
            r.world.insert_non_send(slot);
        }),
    ];
    let mut got = Vec::new();
    for (reason, break_it) in cases {
        let mut rig = rig("areka-skin-entry-no-context");
        break_it(&mut rig);
        let (verdict, warned, warns, sent) = refuse(
            &mut rig,
            req(SkinKind::Shell, name("夏服"), RAISE),
            "skin_switch_no_context",
        );
        got.push((
            reason,
            verdict,
            warned,
            warns,
            sent,
            marker_folder(&rig.world),
        ));
    }
    let want: Vec<_> = ["ghost_slot", "boot_context", "kanade", "seriko_sink"]
        .into_iter()
        .map(|r| (r, SkinVerdict::NoContext, 1, 1, 0, None))
        .collect();
    assert_eq!(
        got, want,
        "(欠けたもの, 判定, 名指しの warn!, warn! の総数, 便り, 印)"
    );
}

// ---------------------------------------------------------------- 受理

/// 受理は `info!(skin_switch_requested)` 1 件と待ちの依頼 1 件で、印つき（`OnShellChanging`）は
/// メニューのシェルと `raise-event` 付きの台本のシェルだけ。印の Ref0＝切替先の `name`・Ref1＝今の
/// シェルの名前（実行系の無い置き場では空）・Ref2＝切替先の絶対パス。進行中の印は切替先を指す
/// （要件 1.1・1.5・1.16・2.1）。
#[test]
fn acceptance_starts_one_wait_marked_only_for_menu_and_raise_event_shells() {
    // (要求, 印つきか, 切替先のフォルダ)
    let cases = [
        (req(SkinKind::Shell, name("夏服"), RAISE), true, "summer"),
        (
            req(SkinKind::Shell, folder("summer"), SkinOrigin::Menu),
            true,
            "summer",
        ),
        (req(SkinKind::Shell, name("summer"), PLAIN), false, "summer"),
        (
            req(SkinKind::Balloon, name("ふわふわ"), PLAIN),
            false,
            "fluffy",
        ),
        (
            req(SkinKind::Balloon, folder("fluffy"), SkinOrigin::Menu),
            false,
            "fluffy",
        ),
    ];
    let mut got = Vec::new();
    let mut want = Vec::new();
    for (request, marked, to) in cases {
        let mut rig = rig("areka-skin-entry-accept");
        let summer = rig.root.ghost_dir("A").join("shell").join("summer");
        let raise = marked.then(|| changing("夏服", "", &summer));
        let kind = request.kind;
        let (verdict, events) = capture(|| request_skin_switch(&mut rig.world, request));
        got.push((
            verdict,
            count(&events, "skin_switch_requested", Level::INFO),
            level_count(&events, Level::WARN) + level_count(&events, Level::ERROR),
            sent(&rig.kanade),
            rig.world
                .get_non_send::<SkinSwitchInFlight>()
                .map(|m| (m.kind, m.target.folder.clone())),
        ));
        want.push((
            SkinVerdict::Accepted,
            1,
            0,
            vec![Some(raise)],
            Some((kind, to.to_owned())),
        ));
    }
    assert_eq!(got, want, "(判定, 受理の info!, warn!＋error!, 便り, 印)");
}

/// kanade へ送れなければ `error!(skin_switch_send_failed)` 1 件で印を置かない。
#[test]
fn send_failure_leaves_one_error_and_no_marker() {
    let mut rig = rig("areka-skin-entry-send-failed");
    let (_, dead) = mpsc::channel();
    drop(std::mem::replace(&mut rig.kanade, dead));
    let (verdict, events) =
        capture(|| request_skin_switch(&mut rig.world, req(SkinKind::Shell, name("夏服"), RAISE)));
    assert_eq!(
        (
            verdict,
            count(&events, "skin_switch_send_failed", Level::ERROR),
            level_count(&events, Level::ERROR),
            marker_folder(&rig.world),
        ),
        (SkinVerdict::NoContext, 1, 1, None),
        "(判定, 送り失敗の error!, error! の総数, 印)"
    );
}

// ---------------------------------------------------------------- 取り出しの系と終了の片付け

fn input_systems_len(world: &World) -> usize {
    world
        .resource::<Schedules>()
        .get(Input)
        .map_or(0, |input| input.systems_len())
}

/// 登録は入力の段へ取り出しの系を 1 つだけ足し、受信端は結線が据える。溜まった台本の要求は
/// 全件が入口へ渡る（1 件目が受理され、2 件目は進行中で断られる＝入口に届いた）。
#[test]
fn drain_hands_every_script_request_to_the_entry() {
    let mut rig = rig("areka-skin-entry-drain");
    register_switch_drain(&mut rig.world);
    let registered = (
        input_systems_len(&rig.world),
        rig.world.get_non_send::<SwitchRx>().is_some(),
    );
    let (tx, rx) = mpsc::channel();
    wire_switch_rx(&mut rig.world, rx);
    for raw in [
        SkinRequestRaw {
            kind: SkinKind::Shell,
            name: "夏服".to_owned(),
            raise_event: true,
        },
        SkinRequestRaw {
            kind: SkinKind::Balloon,
            name: "fluffy".to_owned(),
            raise_event: false,
        },
    ] {
        tx.send(raw).unwrap();
    }
    let ((), events) = capture(|| rig.world.run_schedule(Input));
    let summer = rig.root.ghost_dir("A").join("shell").join("summer");
    assert_eq!(
        (
            registered,
            input_systems_len(&rig.world),
            count(&events, "skin_switch_requested", Level::INFO),
            count(&events, "skin_switch_busy", Level::WARN),
            sent(&rig.kanade),
        ),
        (
            (1, false),
            1,
            1,
            1,
            vec![Some(Some(changing("夏服", "", &summer)))]
        ),
        "((登録後の系の数, 登録だけで受信端が在るか), 結線後の系の数, 受理, 進行中の断り, 便り)"
    );
}

/// 終了が始まったら登記した片付けが `info!(skin_switch_dropped, reason=exit)` 1 件で印を消す。
/// 印が無ければ何も残さない（要件 5.7）。
#[test]
fn exit_cleanup_drops_the_marker_with_one_info() {
    let mut rig = rig("areka-skin-entry-exit");
    register_switch_drain(&mut rig.world);
    let accepted = request_skin_switch(
        &mut rig.world,
        req(SkinKind::Balloon, name("fluffy"), PLAIN),
    );
    let had = marker_folder(&rig.world);
    let ((), events) = capture(|| {
        crate::exit_wait::begin_close(&mut rig.world);
    });
    let dropped: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("skin_switch_dropped") && e.level == Level::INFO)
        .map(|e| e.field_str("reason").map(str::to_owned))
        .collect();
    let ((), idle) = capture(|| discard_for_exit(&mut rig.world));
    assert_eq!(
        (
            accepted,
            had,
            dropped,
            marker_folder(&rig.world),
            idle.len()
        ),
        (
            SkinVerdict::Accepted,
            Some("fluffy".to_owned()),
            vec![Some("exit".to_owned())],
            None,
            0
        ),
        "(受理, 片付けの前の印, 片付けの記録の reason, 片付けの後の印, 印が無いときの記録の数)"
    );
}

/// Reference の名前は `descript.txt` の `name`、無ければフォルダ名。パスは絶対パス（要件 2.1・2.4・3.2）。
#[test]
fn reference_name_falls_back_to_folder_and_path_is_absolute() {
    let named = SkinCandidate {
        dir: PathBuf::from("rel/summer"),
        folder: "summer".to_owned(),
        name: Some("夏服".to_owned()),
        hidden: false,
    };
    let bare = SkinCandidate {
        name: None,
        ..named.clone()
    };
    assert_eq!(
        (
            skin_ref_name(&named),
            skin_ref_name(&bare),
            skin_ref_path(&named.dir)
        ),
        (
            "夏服".to_owned(),
            "summer".to_owned(),
            absolute(Path::new("rel/summer"))
        )
    );
}
