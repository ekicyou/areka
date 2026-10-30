//! 実機の `--install` の試験（spec: startup-install-option task 6.1・要件 2.1・2.2・3.1・5.1・9.7・9.8）。
//!
//! 検体 emo2 を起こし、起動の行に `--install <写し> --option callghost,deletesource` を足す。
//! 写しは `target\test-roots` の下に、利用条件の無いゴーストの書庫（`konnoyayame.nar`）を写したもの
//! （窓口の既定 `AREKA_NO_ALERT=1` では利用条件つきの書庫は拒否になるため）。
//! `get_active_ghost_list` で入れたゴーストが起動中になったこと・置き場（窓口の根）に
//! `ghost\konnoyayame` が在ること・写しが消えたことを確かめ、利用者と同じ閉じ方で閉じる。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use real_run_kit::{EndedBy, Launch, RealRun};
use temp_path_kit::TempPath;

/// 入れる書庫（`vendors/sample_ghost/` の利用条件の無いゴースト）。
const NAR: &str = "konnoyayame.nar";
/// 書庫の `install.txt` の `directory`。
const FOLDER: &str = "konnoyayame";
/// 書庫のゴーストの descript の `name`（`get_active_ghost_list` の綴り）。
const GHOST_NAME: &str = "はろーYAYAわーるど";

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

#[test]
fn install_option_installs_calls_ghost_and_deletes_source() {
    let copies = TempPath::under_target("startup-install");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendors/sample_ghost")
        .join(NAR);
    let archive = copies.child(NAR);
    std::fs::copy(&source, &archive)
        .unwrap_or_else(|e| panic!("{} を写せません: {e}", source.display()));

    let root: Arc<Mutex<Option<PathBuf>>> = Arc::default();
    let seen = Arc::clone(&root);
    let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        // 根に emo2 を置き、同梱バルーンを根の balloon\ へも写す（書庫のゴーストは
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
            copy_dir(
                &root.join(r"ghost\emo2\balloon\emo2-kakukaku"),
                &root.join(r"balloon\emo2-kakukaku"),
            );
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
    for event in [
        "startup_install_done",
        "startup_source_deleted",
        "startup_callghost_requested",
    ] {
        assert!(out.contains(event), "記録に {event} が無い。\n{ctx}");
    }
}
