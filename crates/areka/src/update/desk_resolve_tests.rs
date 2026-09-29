//! 対象の解決の兄弟テスト（design「窓口と入口」の対象の解決・要件 1.4・1.7）。
//!
//! 一時の根に emo2 風のフォルダ（`ghost/<名>/ghost/master`・`ghost/<名>/shell/<名>`・
//! `balloon/<名>`）を置き、今の 3 つの `dir`・名前・倒れ先の `homeurl` と、`updateother` の
//! 名前引き（目録の `name` と完全一致）を判定する。

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use areka_ghost::BasewareRoot;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;

use super::{Here, resolve, resolve_targets};
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::update::{RawUpdateRequest, TargetKind, TargetSpec};

const GHOST_URL: &str = "https://ghost.example/emo/";
const SHELL_URL: &str = "https://shell.example/master/";
const BALLOON_URL: &str = "https://balloon.example/kaku/";

/// 一時の根（捨てると木が消える）。
struct Root {
    tmp: TempPath,
}

impl Root {
    /// `ghost/emo`（名前・homeurl あり）・`ghost/bare`（名前・homeurl なし）・
    /// `ghost/emo/shell/{master,Hidden}`・`balloon/{kaku,plain}` を置く。
    fn new(label: &str) -> Self {
        let tmp = TempPath::new(label);
        let r = tmp.path();
        write(
            &r.join("ghost/emo/ghost/master"),
            &format!("charset,UTF-8\r\nname,Emo\r\nhomeurl,{GHOST_URL}\r\n"),
        );
        write(&r.join("ghost/bare/ghost/master"), "charset,UTF-8\r\n");
        write(
            &r.join("ghost/emo/shell/master"),
            &format!("charset,UTF-8\r\nname,MasterShell\r\nhomeurl,{SHELL_URL}\r\n"),
        );
        write(
            &r.join("ghost/emo/shell/Hidden"),
            "charset,UTF-8\r\nname,HiddenShell\r\nmenu,hidden\r\nhomeurl,https://hidden.example/\r\n",
        );
        write(
            &r.join("balloon/kaku"),
            &format!("charset,UTF-8\r\nname,Kaku\r\nhomeurl,{BALLOON_URL}\r\n"),
        );
        write(&r.join("balloon/plain"), "charset,UTF-8\r\n");
        Root { tmp }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.tmp.path().join(rel)
    }

    fn baseware(&self) -> BasewareRoot {
        BasewareRoot::new(self.tmp.path().to_path_buf())
    }

    /// 今のゴースト `emo`・シェル `master`・バルーン `kaku`（SHIORI の名前は無し）。
    fn here(&self) -> Here {
        Here {
            root: self.baseware(),
            ghost_dir: self.path("ghost/emo"),
            ghost_name: None,
            shell_dir: Some(self.path("ghost/emo/shell/master")),
            balloon_dir: self.path("balloon/kaku"),
        }
    }
}

fn write(dir: &Path, descript: &str) {
    std::fs::create_dir_all(dir).expect("検体のフォルダを作る");
    std::fs::write(dir.join("descript.txt"), descript).expect("descript.txt を書く");
}

fn spec(kind: TargetKind, dir: PathBuf, name: &str, homeurl: Option<&str>) -> TargetSpec {
    TargetSpec {
        kind,
        dir,
        name: name.to_owned(),
        descript_homeurl: homeurl.map(str::to_owned),
    }
}

fn all_three() -> RawUpdateRequest {
    RawUpdateRequest::Current(vec![
        TargetKind::Ghost,
        TargetKind::Shell,
        TargetKind::Balloon,
    ])
}

fn skipped(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("update_target_skipped"))
        .collect()
}

#[test]
fn current_three_resolve_dirs_names_and_fallback_homeurls() {
    let root = Root::new("areka-update-resolve-current");
    let (specs, events) = capture(|| resolve(&root.here(), &all_three()));
    assert_eq!(
        specs,
        vec![
            spec(
                TargetKind::Ghost,
                root.path("ghost/emo"),
                "Emo",
                Some(GHOST_URL)
            ),
            spec(
                TargetKind::Shell,
                root.path("ghost/emo/shell/master"),
                "MasterShell",
                Some(SHELL_URL),
            ),
            spec(
                TargetKind::Balloon,
                root.path("balloon/kaku"),
                "Kaku",
                Some(BALLOON_URL)
            ),
        ]
    );
    assert!(skipped(&events).is_empty(), "{events:?}");
}

#[test]
fn ghost_name_prefers_the_shiori_name() {
    let root = Root::new("areka-update-resolve-shiori-name");
    let here = Here {
        ghost_name: Some("FromShiori".to_owned()),
        ..root.here()
    };
    let specs = resolve(&here, &RawUpdateRequest::Current(vec![TargetKind::Ghost]));
    assert_eq!(
        specs,
        vec![spec(
            TargetKind::Ghost,
            root.path("ghost/emo"),
            "FromShiori",
            Some(GHOST_URL)
        )]
    );
}

#[test]
fn names_fall_back_to_the_folder_and_homeurl_to_none() {
    let root = Root::new("areka-update-resolve-folder-name");
    let here = Here {
        ghost_dir: root.path("ghost/bare"),
        balloon_dir: root.path("balloon/plain"),
        ..root.here()
    };
    let specs = resolve(
        &here,
        &RawUpdateRequest::Current(vec![TargetKind::Ghost, TargetKind::Balloon]),
    );
    assert_eq!(
        specs,
        vec![
            spec(TargetKind::Ghost, root.path("ghost/bare"), "bare", None),
            spec(
                TargetKind::Balloon,
                root.path("balloon/plain"),
                "plain",
                None
            ),
        ]
    );
}

#[test]
fn a_missing_folder_or_unknown_shell_is_skipped_with_one_warning_each() {
    let root = Root::new("areka-update-resolve-missing");
    let here = Here {
        shell_dir: None,
        balloon_dir: root.path("balloon/gone"),
        ..root.here()
    };
    let (specs, events) = capture(|| resolve(&here, &all_three()));
    assert_eq!(
        specs,
        vec![spec(
            TargetKind::Ghost,
            root.path("ghost/emo"),
            "Emo",
            Some(GHOST_URL)
        )]
    );
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 2, "{events:?}");
    assert!(skipped.iter().all(|e| e.level == Level::WARN), "{events:?}");
}

#[test]
fn updateother_looks_names_up_exactly_in_the_order_given() {
    let root = Root::new("areka-update-resolve-other");
    let raw = RawUpdateRequest::Other(vec![
        (TargetKind::Balloon, "Kaku".to_owned()),
        (TargetKind::Shell, "MasterShell".to_owned()),
    ]);
    let (specs, events) = capture(|| resolve(&root.here(), &raw));
    assert_eq!(
        specs,
        vec![
            spec(
                TargetKind::Balloon,
                root.path("balloon/kaku"),
                "Kaku",
                Some(BALLOON_URL)
            ),
            spec(
                TargetKind::Shell,
                root.path("ghost/emo/shell/master"),
                "MasterShell",
                Some(SHELL_URL),
            ),
        ]
    );
    assert!(skipped(&events).is_empty(), "{events:?}");
}

#[test]
fn updateother_does_not_find_a_case_mismatched_name() {
    let root = Root::new("areka-update-resolve-case");
    let raw = RawUpdateRequest::Other(vec![
        (TargetKind::Shell, "mastershell".to_owned()),
        (TargetKind::Balloon, "KAKU".to_owned()),
    ]);
    let (specs, events) = capture(|| resolve(&root.here(), &raw));
    assert!(specs.is_empty(), "{specs:?}");
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 2, "{events:?}");
    assert!(skipped.iter().all(|e| e.level == Level::WARN), "{events:?}");
}

#[test]
fn updateother_does_not_find_a_hidden_shell() {
    let root = Root::new("areka-update-resolve-hidden");
    let raw = RawUpdateRequest::Other(vec![(TargetKind::Shell, "HiddenShell".to_owned())]);
    let (specs, events) = capture(|| resolve(&root.here(), &raw));
    assert!(specs.is_empty(), "{specs:?}");
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, Level::WARN);
}

/// World から写す口: 置き場のゴーストの根と起動の文脈のバルーンを読む。実行系の無い置き場
/// （テスト用の組み立て）はシェルを解けないので、シェルだけ `warn!` で飛ばす。
#[test]
fn resolve_targets_reads_the_slot_and_the_boot_context() {
    let root = Root::new("areka-update-resolve-world");
    let base = root.baseware();
    let mut world = World::new();
    world.insert_resource(BootContext {
        root: base.clone(),
        app_profile_dir: root.path("profile"),
        helper_exe: root.path("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: base.ghost_dir("emo"),
                balloon_root: base.balloon_dir("kaku"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: base.ghost_dir("emo"),
                folder: Some("emo".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: base.balloon_dir("kaku"),
                folder: Some("kaku".to_owned()),
            },
        },
    });
    let (tx, _rx) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        base.ghost_dir("emo"),
    ))));

    let (specs, events) = capture(|| resolve_targets(&world, &all_three()));
    assert_eq!(
        specs,
        vec![
            spec(
                TargetKind::Ghost,
                base.ghost_dir("emo"),
                "Emo",
                Some(GHOST_URL)
            ),
            spec(
                TargetKind::Balloon,
                base.balloon_dir("kaku"),
                "Kaku",
                Some(BALLOON_URL)
            ),
        ]
    );
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].field_str("kind"), Some("shell"));
}

/// 起動の文脈か置き場のゴーストが無ければ何も解かない。
#[test]
fn resolve_targets_without_a_context_resolves_nothing() {
    let world = World::new();
    assert!(resolve_targets(&world, &all_three()).is_empty());
}
