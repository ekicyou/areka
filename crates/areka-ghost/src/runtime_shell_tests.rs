//! シェル名つきの起動（[`boot_with_origin`] の `shell`）と今のシェルの書き換え
//! （[`GhostRuntime::set_shell_dir`]）のテスト（要件 6.4・6.5・6.6）。
//!
//! 偽の SHIORI と検体のゴーストは由来つき起動のテストのものを使い、2 つ目のシェル
//! `shell/B/`（`name,ShellB`）を足して名前ありで起動する。
use super::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use temp_path_kit::TempPath;

use super::origin_tests::{NoopSink, RecordingShiori, Requests, refs_of, write_returning_ghost};

/// 名前ありで起動すると、マウントのシェルは `shell/B`、`OnBoot` の Ref0 は B の名前になり、
/// 書き換えの口の後はマウントのシェルが新しいフォルダを指す。
#[test]
fn named_shell_boot_mounts_it_and_set_shell_dir_rewrites_it() {
    let temp = TempPath::new("ghost-runtime-shell-named");
    let root = temp.path().to_path_buf();
    write_returning_ghost(&root);
    let shell_b = root.join("shell").join("B");
    std::fs::create_dir_all(&shell_b).expect("create shell/B");
    std::fs::write(
        shell_b.join("descript.txt"),
        b"charset,UTF-8\nname,ShellB\nsakura.bindgroup3.default,1\n",
    )
    .expect("write shell/B descript.txt");

    let requests: Requests = Arc::new(Mutex::new(Vec::new()));
    let backend_requests = Arc::clone(&requests);
    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(RecordingShiori(backend_requests)) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(Box::new(SystemVarSnapshot::default)),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let mut runtime =
        boot_with_origin(options, None, BootOrigin::Plain, Some("B")).expect("boot should succeed");
    assert_eq!(runtime.mount().shell.dir, shell_b);

    let deadline = Instant::now() + Duration::from_secs(10);
    let seen = loop {
        let seen = requests.lock().expect("requests mutex poisoned").clone();
        if seen.iter().any(|(id, _)| id == "OnBoot") {
            break seen;
        }
        assert!(
            Instant::now() < deadline,
            "OnBoot never reached SHIORI; requests so far: {seen:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(
        refs_of(&seen, "OnBoot"),
        vec![&vec!["ShellB".to_string()]],
        "requests: {seen:?}"
    );

    // 書き換えの口: マウントのシェルだけが新しいフォルダを指す（bindgroup は起動時のまま）。
    assert_eq!(runtime.mount().bindgroups.sakura_default_on, vec![3]);
    let shell_master = root.join("shell").join("master");
    runtime.set_shell_dir(shell_master.clone());
    assert_eq!(runtime.mount().shell.dir, shell_master);
    assert_eq!(runtime.mount().bindgroups.sakura_default_on, vec![3]);

    // 正規の終了を期限つきで待つ（join の宙吊りでスイート全体を止めない）。
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = runtime.shutdown(areka_kanade::CloseReason::System);
        let _ = done_tx.send(result.map_err(|err| format!("{err:?}")));
    });
    let outcome = done_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("shutdown did not complete within 10s (possible hang)");
    assert_eq!(outcome, Ok(()), "shutdown should succeed");
    drop(temp);
}
