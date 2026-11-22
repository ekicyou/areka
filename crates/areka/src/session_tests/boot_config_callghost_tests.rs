//! callghost の相手を受けた起動の解決。
//!
//! 起動の解決の中身 `resolve_boot_with` に callghost の相手（入れたゴーストのフォルダ名）を渡し、
//! 経路・場所・起動中の印の値を見る。根は `TempPath::under_target`（ワークツリーの `target\test-roots\` の下）。

use crate::session_tests::{no_pick, put_ghost, save_scope};
use areka_session::alert::AlertScene;
use areka_session::boot_config::{BootResolved, resolve_boot_with};
use areka_session::boot_resolve::{
    DEFAULT_BALLOON_FOLDER, DEFAULT_GHOST_FOLDER, GhostRoute, write_session_mark,
};
use std::path::PathBuf;
use temp_path_kit::TempPath;

/// 根にバルーン（既定）と `ghosts` のゴーストを置き、最後のゴーストの記憶を `g1` にする。
fn fixture(label: &str, ghosts: &[&str]) -> TempPath {
    let tmp = TempPath::under_target(label);
    std::fs::create_dir_all(tmp.child("ghost")).expect("ゴーストの置き場");
    for g in ghosts {
        put_ghost(&tmp.child("ghost").join(g));
    }
    let balloon = tmp.child("balloon").join(DEFAULT_BALLOON_FOLDER);
    std::fs::create_dir_all(&balloon).expect("フォルダを組む");
    std::fs::write(balloon.join("descript.txt"), "charset,UTF-8\n").expect("descript");
    save_scope(&tmp.child("app-profile"), "g1");
    tmp
}

/// 入れたゴースト `g2` を記憶の `g1`・既定のゴーストと並べた根。
fn installed(label: &str) -> TempPath {
    fixture(label, &["g1", "g2", DEFAULT_GHOST_FOLDER])
}

fn boot(
    tmp: &TempPath,
    rest: &[&str],
    call_ghost: Option<&str>,
) -> Result<BootResolved, AlertScene> {
    let args: Vec<String> = std::iter::once("areka.exe")
        .chain(rest.iter().copied())
        .map(str::to_owned)
        .collect();
    resolve_boot_with(
        areka_session::roots::single_env(tmp.path().to_path_buf()),
        &crate::cli::parse::parse(&args).options.boot,
        call_ghost,
        &tmp.child("app-profile"),
        no_pick,
        |_, dir| dir.to_path_buf(),
    )
}

/// 経路・ゴーストのフォルダ・印の値。
fn answer(got: Result<BootResolved, AlertScene>) -> (GhostRoute, PathBuf, Option<String>) {
    let got = got.unwrap_or_else(|_| panic!("決まる"));
    (got.1.route, got.1.dir, got.3)
}

/// 相手は記憶の前回のゴーストより先に選ばれる。
#[test]
fn call_ghost_comes_before_the_remembered_ghost() {
    let tmp = installed("cfg-callghost-memory");
    assert_eq!(
        answer(boot(&tmp, &[], Some("g2"))),
        (GhostRoute::CallGhost, tmp.child("ghost").join("g2"), None)
    );
    assert_eq!(
        answer(boot(&tmp, &[], None)).0,
        GhostRoute::Memory,
        "相手が無ければ記憶"
    );
}

/// 前回がきれいに終わらなかった（起動中の印がある）起動でも相手が選ばれ、印の値が返る。
#[test]
fn call_ghost_is_chosen_even_with_the_session_mark() {
    let tmp = installed("cfg-callghost-mark");
    write_session_mark(&tmp.child("app-profile"), "落ちた子").expect("印を書く");
    assert_eq!(
        answer(boot(&tmp, &[], Some("g2"))),
        (
            GhostRoute::CallGhost,
            tmp.child("ghost").join("g2"),
            Some("落ちた子".to_owned())
        )
    );
}

/// 指定（位置引数・`--ghost`）があれば指定が勝つ。
#[test]
fn explicit_ghost_wins_over_call_ghost() {
    let tmp = installed("cfg-callghost-explicit");
    let argv = tmp.child("ghost").join("g1");
    assert_eq!(
        answer(boot(&tmp, &[&argv.display().to_string()], Some("g2"))),
        (GhostRoute::Argv, argv, None),
        "位置引数"
    );
    let got = boot(&tmp, &["--ghost", "g1"], Some("g2")).unwrap_or_else(|_| panic!("決まる"));
    assert_eq!(
        (got.1.route, got.1.folder.as_deref()),
        (GhostRoute::Pointed, Some("g1")),
        "--ghost"
    );
}

/// 入れた後もゴーストが 0 体なら、「ゴーストなし」の告知の場面で止まる。
#[test]
fn no_ghost_after_install_stops_with_ghost_missing() {
    let tmp = fixture("cfg-callghost-none", &[]);
    match boot(&tmp, &[], Some("g2")) {
        Err(AlertScene::GhostMissing { argv: None, .. }) => {}
        Err(_) => panic!("ゴーストなしの場面で止まる"),
        Ok(_) => panic!("決まってはならない"),
    }
}
