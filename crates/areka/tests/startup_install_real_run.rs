//! 実機の `--install` の試験（spec: install-before-boot task 7.1・要件 9.10・9.11）。
//!
//! `--install` は起動の前（根を解いた後・ゴーストを決める前）に入れる。
//! - emo2 の居る根に `--install <写し> --option callghost,deletesource`: 入れたゴーストが起動中・
//!   置き場に在る・写しが消えた・開始と終わり方と消去と callghost を選んだ記録・経路 `CallGhost`・
//!   既定のバルーン（要件 9.11）。
//! - 空の根（既定のバルーンだけ）に `--install <写し>`: 入れたゴーストが起動中・置き場に在る・
//!   経路 `Only`・写しが残る（要件 9.10）。
//!
//! 写しは `target\test-roots` の下に、利用条件の無いゴーストの書庫（`konnoyayame.nar`）を写したもの
//! （窓口の既定 `AREKA_NO_ALERT=1` では利用条件つきの書庫は拒否になるため）。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use real_run_kit::{EndedBy, Launch, RealRun};
use sample_ghost_kit::SampleRoot;
use temp_path_kit::TempPath;

/// 入れる書庫（`vendors/sample_ghost/` の利用条件の無いゴースト）。
const NAR: &str = "konnoyayame.nar";
/// 書庫の `install.txt` の `directory`。
const FOLDER: &str = "konnoyayame";
/// 書庫のゴーストの descript の `name`（`get_active_ghost_list` の綴り）。
const GHOST_NAME: &str = "はろーYAYAわーるど";
/// 根に置く既定のバルーン（配布物の `balloon\` と同じ名前の検体）。
const BALLOON: &str = "areka-balloon";

/// `from` の中身を `to` へ再帰で写す（写せなければ場所と理由で落とす）。
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap_or_else(|e| panic!("{} を作れません: {e}", to.display()));
    for entry in std::fs::read_dir(from).unwrap_or_else(|e| panic!("{}: {e}", from.display())) {
        let path = entry.unwrap().path();
        let dest = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest);
        } else {
            std::fs::copy(&path, &dest)
                .unwrap_or_else(|e| panic!("{} を写せません: {e}", path.display()));
        }
    }
}

/// `target\` の下に書庫の写しを作る（写しの置き場の取っ手と写しの道）。
fn archive_copy(dir: &str) -> (TempPath, PathBuf) {
    let copies = TempPath::under_target(dir);
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendors/sample_ghost")
        .join(NAR);
    let archive = copies.child(NAR);
    std::fs::copy(&source, &archive)
        .unwrap_or_else(|e| panic!("{} を写せません: {e}", source.display()));
    (copies, archive)
}

/// `event` の行のうち、欄の綴り `field` をすべて含む行があるか。
fn has_line(out: &str, event: &str, fields: &[&str]) -> bool {
    let event = format!("event=\"{event}\"");
    out.lines()
        .any(|l| l.contains(&event) && fields.iter().all(|f| l.contains(f)))
}

#[test]
fn install_option_installs_calls_ghost_and_deletes_source() {
    let (_copies, archive) = archive_copy("startup-install");

    let balloon = SampleRoot::acquire(BALLOON).expect("areka-balloon は登記済みの検体");
    let balloon_dir = balloon.folder().to_path_buf();

    let root: Arc<Mutex<Option<PathBuf>>> = Arc::default();
    let seen = Arc::clone(&root);
    let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        // 根に emo2 を置き、配布物と同じく既定のバルーンを根の balloon\ へ写す（書庫のゴーストは
        // バルーンを持たず、切り替えは根とそのゴーストの中のバルーンしか見ないため）。
        .launch(Launch::SampleRoot {
            name: "emo2".into(),
        })
        .arg("--install")
        .arg(&archive)
        .arg("--option")
        .arg("callghost,deletesource")
        // 緑の走りで 40 秒近く掛かったので、負荷の下でも保険の自動終了に掛からない幅を取る。
        .safety_exit(Duration::from_secs(90))
        .deadline(Duration::from_secs(90))
        .before_spawn(move |s| {
            let root = s.root();
            copy_dir(&balloon_dir, &root.join("balloon").join(BALLOON));
            *seen.lock().unwrap() = Some(root.to_path_buf());
        })
        .spawn();
    let root = root
        .lock()
        .unwrap()
        .clone()
        .expect("before_spawn で根を控えたはず");

    let reply = run
        .call_until("get_active_ghost_list", "{}", GHOST_NAME)
        .unwrap_or_else(|e| panic!("入れたゴーストが起動中になりません: {e}\n{}", run.output()));
    let installed = root.join("ghost").join(FOLDER);
    let installed_present = installed.join("ghost").join("master").is_dir();
    let archive_present = archive.exists();

    let ended = run.close();
    let out = run.output();
    let ctx = format!("--- get_active_ghost_list ---\n{}\n{out}", reply.text);

    assert_eq!(
        ended.by,
        EndedBy::UserClose,
        "利用者と同じ閉じ方で終わるはず。\n{ctx}"
    );
    assert!(
        installed_present,
        "置き場に {} が無い。\n{ctx}",
        installed.display()
    );
    assert!(
        !archive_present,
        "写し {} が消えていない。\n{ctx}",
        archive.display()
    );
    for (event, fields) in [
        (
            "startup_install_started",
            &["before_boot", "StartupArgs"][..],
        ),
        ("startup_install_done", &["before_boot", "installed"][..]),
        ("startup_source_deleted", &[NAR][..]),
        ("startup_callghost_chosen", &[FOLDER][..]),
        ("ghost_resolved", &["route=CallGhost", FOLDER][..]),
        // 起こす先は emo2 の同梱バルーンでなく根の既定のバルーンを使う。
        ("balloon_resolved", &[BALLOON][..]),
    ] {
        assert!(
            has_line(&out, event, fields),
            "記録に {event}（{fields:?}）が無い。\n{ctx}"
        );
    }
}

#[test]
fn install_option_into_empty_root_boots_the_only_ghost() {
    let (_copies, archive) = archive_copy("startup-install-empty");

    let balloon = SampleRoot::acquire(BALLOON).expect("areka-balloon は登記済みの検体");
    let balloon_dir = balloon.folder().to_path_buf();

    let root: Arc<Mutex<Option<PathBuf>>> = Arc::default();
    let seen = Arc::clone(&root);
    let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        // 空の根に既定のバルーンだけを置く（ゴーストは起動の前に入れる 1 体だけ）。
        .launch(Launch::Empty)
        .arg("--install")
        .arg(&archive)
        .safety_exit(Duration::from_secs(90))
        .deadline(Duration::from_secs(90))
        .before_spawn(move |s| {
            let root = s.root();
            copy_dir(&balloon_dir, &root.join("balloon").join(BALLOON));
            *seen.lock().unwrap() = Some(root.to_path_buf());
        })
        .spawn();
    let root = root
        .lock()
        .unwrap()
        .clone()
        .expect("before_spawn で根を控えたはず");

    let reply = run
        .call_until("get_active_ghost_list", "{}", GHOST_NAME)
        .unwrap_or_else(|e| panic!("入れたゴーストが起動中になりません: {e}\n{}", run.output()));
    let installed = root.join("ghost").join(FOLDER);
    let installed_present = installed.join("ghost").join("master").is_dir();
    let archive_present = archive.exists();

    let ended = run.close();
    let out = run.output();
    let ctx = format!("--- get_active_ghost_list ---\n{}\n{out}", reply.text);

    assert_eq!(
        ended.by,
        EndedBy::UserClose,
        "利用者と同じ閉じ方で終わるはず。\n{ctx}"
    );
    assert!(
        installed_present,
        "置き場に {} が無い。\n{ctx}",
        installed.display()
    );
    // deletesource を付けないので写しは残る。
    assert!(
        archive_present,
        "写し {} が消えた。\n{ctx}",
        archive.display()
    );
    assert!(
        has_line(&out, "startup_install_done", &["before_boot", "installed"]),
        "記録に起動の前の startup_install_done が無い。\n{ctx}"
    );
    assert!(
        has_line(&out, "ghost_resolved", &["route=Only", FOLDER]),
        "起こすゴーストの経路が Only でない。\n{ctx}"
    );
}
