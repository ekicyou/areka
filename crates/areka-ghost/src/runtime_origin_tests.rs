//! 由来つき起動（[`boot_with_origin`]）の結線のテスト（要件 4.1・8.6）。
//!
//! 起動の由来とシェルのフォルダ名が kanade の設定へ届いたことを、偽の SHIORI が受けた
//! `OnGhostChanged`／`OnBoot` の Reference で確かめる（設定は kanade のスレッドの中にしか
//! 無いので、外から見えるのは SHIORI への要求だけ）。
use super::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_kanade::ChangedFrom;
use areka_sakura::contract::{CueSink, TalkCue};
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistScope, ScopeRoots, save_scope};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use temp_path_kit::TempPath;

type Requests = Arc<Mutex<Vec<(String, Vec<String>)>>>;

/// GET を `(id, references)` で記録し、すべて 204（`None`）で返す偽の SHIORI。
struct RecordingShiori(Requests);

impl ShioriBackend for RecordingShiori {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.0
            .lock()
            .expect("requests mutex poisoned")
            .push((id.to_string(), references.to_vec()));
        Ok(None)
    }

    fn notify(
        &mut self,
        _id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        Ok(())
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        Ok(ExitKind::Clean)
    }

    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }
}

#[derive(Clone)]
struct NoopSink;

impl CueSink for NoopSink {
    fn emit(&mut self, _cue: TalkCue) {}
}

/// シェル名（descript の `name`＝`TestShell`）とシェルのフォルダ名（`master`）が別の値に
/// なるゴーストを作り、起動記録を前もって保存しておく（初回起動の根を外すため）。
fn write_returning_ghost(root: &std::path::Path) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
    )
    .expect("write ghost descript.txt");
    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,TestShell\n",
    )
    .expect("write shell descript.txt");

    let profile = crate::sylphya_wiring::profile_areka_root(&ghost_master);
    std::fs::create_dir_all(&profile).expect("create ghost profile root");
    let roots = ScopeRoots {
        ghost: Some(profile),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &FsPersistIo,
        vec![(PersistKey::BootCount, "1".to_string())],
    );
}

/// 由来つきで起動し、`OnBoot` が届くまで待ってから SHIORI が受けた GET の列を返す。
fn boot_and_collect(tag: &str, origin: BootOrigin) -> Vec<(String, Vec<String>)> {
    let temp = TempPath::new(&format!("ghost-runtime-origin-{tag}"));
    let root = temp.path().to_path_buf();
    write_returning_ghost(&root);

    let requests: Requests = Arc::new(Mutex::new(Vec::new()));
    let backend_requests = Arc::clone(&requests);
    let options = GhostBootOptions {
        ghost_root: root,
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(RecordingShiori(backend_requests)) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(Box::new(SystemVarSnapshot::default)),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot_with_origin(options, None, origin).expect("boot should succeed");

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

    // 正規の終了を期限つきで待つ（join の宙吊りでスイート全体を止めない）。一時フォルダは
    // 終了が済むまで `temp` が生かしておく。
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
    seen
}

fn refs_of<'a>(requests: &'a [(String, Vec<String>)], id: &str) -> Vec<&'a Vec<String>> {
    requests
        .iter()
        .filter(|(seen, _)| seen == id)
        .map(|(_, refs)| refs)
        .collect()
}

/// 切替で来た起動: `OnGhostChanged` の Ref0〜3 に直前のゴーストの情報、Ref7 にシェルの
/// フォルダ名（シェル名 `TestShell` ではなくフォルダの末尾 `master`）が載る。
#[test]
fn changed_from_origin_and_shell_folder_reach_on_ghost_changed() {
    let requests = boot_and_collect(
        "changed-from",
        BootOrigin::ChangedFrom(ChangedFrom {
            sakura_name: "前の本体".to_string(),
            script: "\\0またね\\e".to_string(),
            name: "PrevGhost".to_string(),
            dir: "C:\\ghost\\prev".to_string(),
        }),
    );

    let changed = refs_of(&requests, "OnGhostChanged");
    assert_eq!(changed.len(), 1, "requests: {requests:?}");
    assert_eq!(
        changed[0],
        &vec![
            "前の本体".to_string(),
            "\\0またね\\e".to_string(),
            "PrevGhost".to_string(),
            "C:\\ghost\\prev".to_string(),
            String::new(),
            String::new(),
            String::new(),
            "master".to_string(),
        ]
    );
    assert!(refs_of(&requests, "OnFirstBoot").is_empty());
}

/// 前回落ちた起動: `OnBoot` の Ref6＝`halt`・Ref7＝落ちたゴースト名。
#[test]
fn halted_origin_reaches_on_boot_references() {
    let requests = boot_and_collect(
        "halted",
        BootOrigin::Halted {
            ghost_name: "FallenGhost".to_string(),
        },
    );

    let boots = refs_of(&requests, "OnBoot");
    assert_eq!(boots.len(), 1, "requests: {requests:?}");
    assert_eq!(
        boots[0],
        &vec![
            "TestShell".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "halt".to_string(),
            "FallenGhost".to_string(),
        ]
    );
    assert!(refs_of(&requests, "OnGhostChanged").is_empty());
    assert!(refs_of(&requests, "OnFirstBoot").is_empty());
}

/// ふつうの由来の起動: `OnBoot` は Ref0 だけ。
#[test]
fn plain_origin_keeps_on_boot_ref0_only() {
    let requests = boot_and_collect("plain", BootOrigin::Plain);
    let boots = refs_of(&requests, "OnBoot");
    assert_eq!(boots, vec![&vec!["TestShell".to_string()]]);
    assert!(refs_of(&requests, "OnGhostChanged").is_empty());
    assert!(refs_of(&requests, "OnFirstBoot").is_empty());
}
