//! 定義を替えた合図 `Rebased` を置き換えの命令へ写す橋渡しの兄弟テスト
//! （spec: areka-P0-shell-balloon-switch task 9.1・要件 4.1・4.2・design「Adapter」）。

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use areka_actor::{ReplyReceiver, reply_channel};
use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_emo_present::{PresentCommand, PresentOutcome};
use areka_sakura::ActorKey;
use areka_seriko::{DisplayCommand, RebaseKind, RebasedShow, SurfaceOutput};
use log_capture_kit::{LineFormat, capture_lines};

use super::PresentBridge;
use crate::emo2_boot::switch_assets::{SwapPayload, SwapSlot};
use crate::emo2_boot::target_map::{balloon_target, shell_target};

/// scope ごとに作者の DPI を変えた荷物（`96 + scope`＝写した先の取り違えを見分ける印）と、
/// scope ごとの返信の受け手を作る。
fn payload(epoch: u64, scopes: &[u32]) -> (SwapPayload, Vec<(u32, ReplyReceiver<PresentOutcome>)>) {
    let mut targets = Vec::new();
    let mut replies = Vec::new();
    let mut receivers = Vec::new();
    for &scope in scopes {
        targets.push((
            scope,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96 + scope as u16,
        ));
        let (tx, rx) = reply_channel::<PresentOutcome>();
        replies.push((scope, tx));
        receivers.push((scope, rx));
    }
    (
        SwapPayload {
            epoch,
            targets,
            replies,
        },
        receivers,
    )
}

fn slot_with(p: SwapPayload) -> SwapSlot {
    Arc::new(Mutex::new(Some(p)))
}

fn show(scope: &str, surface_id: Option<u32>, binds: BindSet) -> RebasedShow {
    RebasedShow {
        scope: ActorKey::from(scope),
        surface_id,
        binds,
    }
}

/// 置き換えの命令 1 件の中身（`PresentCommand` は比較できないので分解して写す）。
struct Replaced {
    target: areka_emo_present::TargetId,
    author_dpi: u16,
    show: Option<(u32, BindSet)>,
    reply: Option<areka_actor::ReplySender<PresentOutcome>>,
}

fn drain(rx: &Receiver<PresentCommand>) -> Vec<PresentCommand> {
    rx.try_iter().collect()
}

fn as_replaced(cmd: PresentCommand) -> Replaced {
    match cmd {
        PresentCommand::ReplaceTarget {
            target,
            author_dpi,
            show,
            reply,
            ..
        } => Replaced {
            target,
            author_dpi,
            show,
            reply,
        },
        _ => panic!("置き換えの命令であるべき"),
    }
}

fn count(lines: &[String], level: &str, event: &str) -> usize {
    let lv = format!("level={level}");
    let ev = format!("event=\"{event}\"");
    lines
        .iter()
        .filter(|l| l.contains(&lv) && l.contains(&ev))
        .count()
}

/// シェルの合図: scope ごとに `2*scope` の置き換えの命令が 1 件ずつ出る。最初の表示は合図の
/// 同じ scope の面と着せ替え、合図に無い scope は登録だけ（`show: None`）。返信の送り手は
/// 同じ scope の受け手と対になり、置き場は空になる。
#[test]
fn shell_rebased_maps_to_replace_target_per_scope_on_shell_targets() {
    let (p, receivers) = payload(7, &[0, 1]);
    let slot = slot_with(p);
    let (tx, rx) = mpsc::channel();
    let mut bridge = PresentBridge::new(tx).with_swap_slot(slot.clone());

    let binds = BindSet::from_ids([1100, 1500]);
    bridge.send(DisplayCommand::Rebased {
        epoch: 7,
        kind: RebaseKind::Shell,
        shows: vec![show("0", Some(5), binds.clone())],
    });

    let cmds: Vec<Replaced> = drain(&rx).into_iter().map(as_replaced).collect();
    assert_eq!(cmds.len(), 2, "scope ごとに 1 件");
    assert_eq!(cmds[0].target, shell_target(0));
    assert_eq!(cmds[0].author_dpi, 96, "scope 0 の荷物が scope 0 へ");
    assert_eq!(
        cmds[0].show,
        Some((5, binds)),
        "合図の同じ scope の面と着せ替え"
    );
    assert_eq!(cmds[1].target, shell_target(1));
    assert_eq!(cmds[1].author_dpi, 97, "scope 1 の荷物が scope 1 へ");
    assert_eq!(cmds[1].show, None, "合図に無い scope は登録だけ");
    assert!(
        slot.lock().unwrap().is_none(),
        "荷物は取り出されて置き場は空"
    );

    // 返信の対: scope 1 の命令の返信は scope 1 の受け手にだけ届く。
    let mut cmds = cmds;
    let reply1 = cmds[1].reply.take().expect("返信の送り手が載る");
    assert!(reply1.send(Ok(())).is_ok());
    let rx0 = &receivers[0].1;
    let rx1 = &receivers[1].1;
    assert!(
        matches!(rx1.try_recv(), Ok(Some(Ok(())))),
        "scope 1 の受け手に届く"
    );
    assert!(
        matches!(rx0.try_recv(), Ok(None)),
        "scope 0 の受け手はまだ空"
    );
    assert!(
        cmds[0].reply.is_some(),
        "scope 0 の命令にも返信の送り手が載る"
    );
}

/// バルーンの合図: `2*scope+1` へ写り、最初の表示は合図の面（バルーンは空の着せ替え）。
#[test]
fn balloon_rebased_maps_to_replace_target_on_balloon_targets() {
    let (p, _receivers) = payload(3, &[0, 1]);
    let (tx, rx) = mpsc::channel();
    let mut bridge = PresentBridge::new(tx).with_swap_slot(slot_with(p));

    bridge.send(DisplayCommand::Rebased {
        epoch: 3,
        kind: RebaseKind::Balloon,
        shows: vec![
            show("0", Some(0), BindSet::default()),
            show("1", Some(2), BindSet::default()),
        ],
    });

    let cmds: Vec<Replaced> = drain(&rx).into_iter().map(as_replaced).collect();
    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0].target, balloon_target(0));
    assert_eq!(cmds[0].show, Some((0, BindSet::default())));
    assert_eq!(cmds[1].target, balloon_target(1));
    assert_eq!(cmds[1].author_dpi, 97);
    assert_eq!(cmds[1].show, Some((2, BindSet::default())));
}

/// 荷物が無い（置き場が空・置き場そのものが無い）と `error!` 1 件で命令 0。
#[test]
fn rebased_without_payload_logs_error_once_and_sends_nothing() {
    for bridge_slot in [Some(Arc::new(Mutex::new(None))), None] {
        let (tx, rx) = mpsc::channel();
        let mut bridge = PresentBridge::new(tx);
        if let Some(slot) = bridge_slot {
            bridge = bridge.with_swap_slot(slot);
        }
        let ((), lines) = capture_lines(LineFormat::LevelTargetFields, || {
            bridge.send(DisplayCommand::Rebased {
                epoch: 1,
                kind: RebaseKind::Shell,
                shows: vec![show("0", Some(0), BindSet::default())],
            });
        });
        assert_eq!(
            count(&lines, "ERROR", "rebased_payload_missing"),
            1,
            "{lines:?}"
        );
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)), "命令 0");
    }
}

/// 世代の違う荷物は取り出さない: `error!` 1 件で命令 0、荷物は置き場に残る。
#[test]
fn rebased_with_other_epoch_logs_error_once_and_leaves_payload() {
    let (p, _receivers) = payload(2, &[0]);
    let slot = slot_with(p);
    let (tx, rx) = mpsc::channel();
    let mut bridge = PresentBridge::new(tx).with_swap_slot(slot.clone());

    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, || {
        bridge.send(DisplayCommand::Rebased {
            epoch: 1,
            kind: RebaseKind::Shell,
            shows: Vec::new(),
        });
    });
    assert_eq!(
        count(&lines, "ERROR", "rebased_payload_missing"),
        1,
        "{lines:?}"
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)), "命令 0");
    assert_eq!(
        slot.lock().unwrap().as_ref().map(|p| p.epoch),
        Some(2),
        "世代の違う荷物は置き場に残る"
    );
}

/// 合図より前の指令は置き換えの命令より前に、後の指令は後に並ぶ（それ以外の指令は今日どおり）。
#[test]
fn commands_before_rebased_stay_before_replace_targets() {
    let (p, _receivers) = payload(9, &[0]);
    let (tx, rx) = mpsc::channel();
    let mut bridge = PresentBridge::new(tx).with_swap_slot(slot_with(p));

    bridge.send(DisplayCommand::Show {
        scope: ActorKey::from("0"),
        surface_id: 10,
        binds: BindSet::default(),
        pattern: PatternState::default(),
    });
    bridge.send(DisplayCommand::Rebased {
        epoch: 9,
        kind: RebaseKind::Shell,
        shows: vec![show("0", Some(10), BindSet::default())],
    });
    bridge.send(DisplayCommand::HideBalloon {
        scope: ActorKey::from("0"),
    });

    let cmds = drain(&rx);
    assert_eq!(cmds.len(), 3);
    assert!(
        matches!(&cmds[0], PresentCommand::ShowSurface { target, surface_id: 10, .. } if *target == shell_target(0)),
        "合図より前の Show が先"
    );
    assert!(
        matches!(&cmds[1], PresentCommand::ReplaceTarget { target, .. } if *target == shell_target(0)),
        "合図の置き換えが次"
    );
    assert!(
        matches!(&cmds[2], PresentCommand::Hide { target, .. } if *target == balloon_target(0)),
        "合図より後の指令が後"
    );
}
