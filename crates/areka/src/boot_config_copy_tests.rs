//! 最初の起動のゴーストを写しから起こす口（`resolve_boot_from` の `prepare_ghost`・
//! areka-P0-user-data-root タスク 4.1・要件 4.3・4.5・7.3・7.4・10.3・10.4）。
//!
//! 写す判断と写す中身は `roots/user_root_tests.rs` が固定する。ここで見るのは「起動の引数では口を
//! 呼ばない」「引数でなければ口の答えでバルーンと構成の入力が決まる」「前回きれいに終わらなかった
//! ときの既定ゴーストも口を通る」「口を通るのはゴーストの 1 回だけ（表示するだけのバルーンは写さない）」。
//! 根は `TempPath::under_target`（ワークツリーの `target\test-roots\` の下）。

use crate::boot_config::resolve_boot_from;
use crate::boot_resolve::{BalloonRoute, GhostRoute, write_session_mark};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

fn no_pick(n: usize) -> usize {
    panic!("無作為の段へ届いてはならない（候補 {n}）")
}

thread_local! {
    /// 口が受け取ったフォルダ（呼ばれた順）。
    static CALLS: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    /// 口の答え（`None` なら受け取ったフォルダのまま）。
    static ANSWER: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// 偽の写しの口: 受け取ったフォルダを数え、決めておいた答えを返す。
fn fake_copy(_roots: &areka_ghost::BasewareRoots, dir: &Path) -> PathBuf {
    CALLS.with(|c| c.borrow_mut().push(dir.to_path_buf()));
    ANSWER
        .with(|a| a.borrow().clone())
        .unwrap_or_else(|| dir.to_path_buf())
}

fn never_copy(_roots: &areka_ghost::BasewareRoots, dir: &Path) -> PathBuf {
    panic!(
        "起動の引数で渡したゴーストを写しの口へ通した: {}",
        dir.display()
    )
}

fn put_ghost(dir: &Path, balloon: &str) {
    let master = dir.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(
        master.join("descript.txt"),
        format!("charset,UTF-8\ndefault.balloon.path,{balloon}\n"),
    )
    .expect("descript");
}

fn put_balloon(dir: &Path) {
    std::fs::create_dir_all(dir).expect("フォルダを組む");
    std::fs::write(dir.join("descript.txt"), "charset,UTF-8\nname,B\n").expect("descript");
}

/// 根にゴースト `folder`（バルーン `b1` を指す）とバルーン `b1`・`b2`・既定を置き、
/// 根の外に写し（バルーン `b2` を指す）を置く。写しのフォルダを返す。
fn fixture(tmp: &TempPath, folder: &str) -> PathBuf {
    put_ghost(&tmp.child("ghost").join(folder), "b1");
    for b in ["b1", "b2", "StayseeBalloon"] {
        put_balloon(&tmp.child("balloon").join(b));
    }
    let copy = tmp.child("copied").join(folder);
    put_ghost(&copy, "b2");
    copy
}

fn boot(
    tmp: &TempPath,
    args: &[String],
    prepare: fn(&areka_ghost::BasewareRoots, &Path) -> PathBuf,
) -> crate::boot_config::BootResolved {
    resolve_boot_from(
        crate::roots::single_env(tmp.path().to_path_buf()),
        args,
        &tmp.child("app-profile"),
        no_pick,
        prepare,
    )
    .expect("決まる")
}

/// 起動の引数でゴーストを渡したときは写しの口を呼ばない（要件 4.3・10.3）。
#[test]
fn argv_ghost_is_not_passed_to_the_copy() {
    let tmp = TempPath::under_target("cfg-copy-argv");
    fixture(&tmp, "g1");
    let argv = tmp.child("ghost").join("g1");
    let (cfg, ghost, _, _, _) = boot(
        &tmp,
        &["areka.exe".to_owned(), argv.display().to_string()],
        never_copy,
    );
    assert_eq!((ghost.route, &ghost.dir), (GhostRoute::Argv, &argv));
    assert_eq!(cfg.ghost_root, argv);
}

/// 引数なしなら口の答えがゴーストのフォルダになり、バルーンも写しの descript で決まる
/// （要件 4.3・7.4）。口を通るのはゴーストの 1 回だけ（表示するだけのバルーンは写さない・要件 4.5）。
#[test]
fn without_argv_balloon_and_config_come_from_the_copy() {
    let tmp = TempPath::under_target("cfg-copy-noargv");
    let copy = fixture(&tmp, "g1");
    ANSWER.with(|a| *a.borrow_mut() = Some(copy.clone()));
    let (cfg, ghost, balloon, _, _) = boot(&tmp, &["areka.exe".to_owned()], fake_copy);
    assert_eq!((ghost.route, &ghost.dir), (GhostRoute::Only, &copy));
    assert_eq!(ghost.folder.as_deref(), Some("g1"));
    assert_eq!(
        (balloon.route, &balloon.dir),
        (BalloonRoute::Descript, &tmp.child("balloon").join("b2")),
        "バルーンは写しの descript で決まる"
    );
    assert_eq!(cfg.ghost_root, copy);
    assert_eq!(cfg.balloon_root, balloon.dir);
    assert_eq!(
        CALLS.with(|c| c.borrow().clone()),
        vec![tmp.child("ghost").join("g1")],
        "口を通るのは元のゴーストの 1 回だけ"
    );
}

/// 前回きれいに終わらなかったときの既定ゴーストも同じ口を通る（要件 4.3）。
#[test]
fn default_ghost_after_unclean_exit_goes_through_the_copy() {
    let tmp = TempPath::under_target("cfg-copy-mark");
    let copy = fixture(&tmp, crate::boot_resolve::DEFAULT_GHOST_FOLDER);
    put_ghost(&tmp.child("ghost").join("other"), "b1");
    write_session_mark(&tmp.child("app-profile"), "other");
    ANSWER.with(|a| *a.borrow_mut() = Some(copy.clone()));
    let (cfg, ghost, _, mark, _) = boot(&tmp, &["areka.exe".to_owned()], fake_copy);
    assert_eq!(mark.as_deref(), Some("other"));
    assert_eq!((ghost.route, &ghost.dir), (GhostRoute::Default, &copy));
    assert_eq!(cfg.ghost_root, copy);
    assert_eq!(
        CALLS.with(|c| c.borrow().clone()),
        vec![
            tmp.child("ghost")
                .join(crate::boot_resolve::DEFAULT_GHOST_FOLDER)
        ]
    );
}
