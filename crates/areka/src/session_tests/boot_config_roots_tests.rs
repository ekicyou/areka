//! 起動の入口（`resolve_boot_from`）を 2 つの根の和の上で踏む。
//!
//! 確かめること: 既定ゴーストと既定バルーンの採用・0 体の失敗が各根の格納フォルダを持つこと。
//! 1 ゴースト分のバルーンの口と起動中の印の名前は `areka-session` の側が見る。根は `TempPath::under_target`。

use crate::session_tests::resolve_boot_from;
use areka_ghost::{BasewareRoot, BasewareRoots};
use areka_session::alert::AlertScene;
use areka_session::boot_resolve::{BalloonRoute, DEFAULT_BALLOON_FOLDER, GhostRoute};
use areka_session::roots::{ResolvedRoots, RootStatus, RootsOrigin};
use log_capture_kit::capture;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

fn no_pick(n: usize) -> usize {
    panic!("無作為の段へ届いてはならない（候補 {n}）")
}

/// 上の根 `top` と下の根 `low`（どちらも `ghost\`・`balloon\` を持つ）。
fn two_roots(tmp: &TempPath) -> (PathBuf, PathBuf, BasewareRoots) {
    let (top, low) = (tmp.child("top"), tmp.child("low"));
    for root in [&top, &low] {
        std::fs::create_dir_all(root.join("ghost")).expect("フォルダを組む");
        std::fs::create_dir_all(root.join("balloon")).expect("フォルダを組む");
    }
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(top.clone()),
        BasewareRoot::new(low.clone()),
    ]);
    (top, low, roots)
}

fn resolved(roots: &BasewareRoots) -> ResolvedRoots {
    ResolvedRoots {
        roots: roots.clone(),
        origin: RootsOrigin::EnvVar,
        status: roots
            .roots()
            .iter()
            .map(|r| RootStatus {
                dir: r.dir().to_path_buf(),
                found: true,
            })
            .collect(),
        skipped: Vec::new(),
        memory_fell_back: false,
    }
}

fn put_ghost(root: &Path, folder: &str, descript: &str) -> PathBuf {
    let dir = root.join("ghost").join(folder);
    let master = dir.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\n{descript}"),
    )
    .expect("descript");
    dir
}

fn put_balloon(root: &Path, folder: &str) -> PathBuf {
    let dir = root.join("balloon").join(folder);
    std::fs::create_dir_all(&dir).expect("フォルダを組む");
    std::fs::write(dir.join("descript.txt"), "charset,UTF-8\n").expect("descript");
    dir
}

/// 既定ゴーストが下の根にだけ在る → 和の上で既定の段が当たり、場所は下の根の項目。
/// 既定バルーンも下の根にだけ在る → 既定の段が当たる。
#[test]
fn default_ghost_and_balloon_are_adopted_from_the_lower_root() {
    let tmp = TempPath::under_target("cfg-roots-default");
    let (top, low, roots) = two_roots(&tmp);
    put_ghost(&top, "a", "");
    put_ghost(&low, "b", "");
    let emo2 = put_ghost(&low, "emo2", "");
    put_balloon(&top, "x");
    let default_balloon = put_balloon(&low, DEFAULT_BALLOON_FOLDER);
    let app_dir = tmp.child("app-profile");
    let (got, _) = capture(|| {
        resolve_boot_from(
            Ok(resolved(&roots)),
            &["areka.exe".to_owned()],
            &app_dir,
            no_pick,
            |_, dir| dir.to_path_buf(),
        )
    });
    let (cfg, ghost, balloon, _, _) = got.expect("決まる");
    assert_eq!((ghost.route, ghost.dir), (GhostRoute::Default, emo2));
    assert_eq!(
        (balloon.route, balloon.dir),
        (BalloonRoute::Default, default_balloon)
    );
    assert_eq!(cfg.ghost_root, tmp.child("low").join("ghost").join("emo2"));
}

/// 和にゴーストが 0 体 → 告知の場面は各根の `ghost\` の列を持つ。
#[test]
fn no_ghost_in_any_root_carries_every_ghost_store() {
    let tmp = TempPath::under_target("cfg-roots-no-ghost");
    let (top, low, roots) = two_roots(&tmp);
    let app_dir = tmp.child("app-profile");
    let got = resolve_boot_from(
        Ok(resolved(&roots)),
        &["areka.exe".to_owned()],
        &app_dir,
        no_pick,
        |_, dir| dir.to_path_buf(),
    );
    assert_eq!(
        got.map(|_| ()),
        Err(AlertScene::GhostMissing {
            ghost_stores: vec![top.join("ghost"), low.join("ghost")],
            argv: None,
        })
    );
}
