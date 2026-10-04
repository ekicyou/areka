use super::*;
use crate::sylphya_wiring::{
    from_sylphya_provider, spawn_ghost_sylphya, translate_snapshot_source,
};
use crate::test_log_capture::{assert_logged, assert_logged_event, capture};
use areka_sakura::contract::SystemVarSnapshot;
use areka_sylphya::{AskerId, ScopeRoots};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::Level;

/// `username` に「太郎」を持つ写しを返す源（呼ばれた回数を `calls` に数える）。
fn taro_source(calls: Arc<AtomicUsize>) -> SystemVarSource {
    Box::new(move || {
        calls.fetch_add(1, Ordering::SeqCst);
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", "太郎");
        snapshot
    })
}

/// 展開の関数は、呼ばれるたびに写しを 1 回読み、その値で台詞の環境変数を展開する（要件 2.3）。
#[test]
fn script_expander_expands_with_one_snapshot_per_call() {
    let calls = Arc::new(AtomicUsize::new(0));
    let expand = make_script_expander(taro_source(Arc::clone(&calls)));

    assert_eq!(expand("%usernameさん"), "太郎さん");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "1 回の展開で写しを 1 回読む"
    );
    assert_eq!(expand(r"\0%usernameさん\e"), r"\0太郎さん\e");
    assert_eq!(calls.load(Ordering::SeqCst), 2, "呼ばれるたびに読み直す");
}

/// 分けた 2 つの源は、同じ源を呼んで同じ値を返す（要件 2.3）。
#[test]
fn split_halves_return_the_same_snapshot() {
    let calls = Arc::new(AtomicUsize::new(0));
    let (translate, playback) = split_source(taro_source(Arc::clone(&calls)));

    let a = translate();
    let b = playback();
    assert_eq!(a.get("username"), Some("太郎"));
    assert_eq!(a, b, "翻訳用と再生用は同じ値を返す");
    assert_eq!(calls.load(Ordering::SeqCst), 2, "どちらも元の源を呼ぶ");
}

/// 源が途中で panic して排他が壊れたら、記録 `translate_snapshot_poisoned` を残して空の写しで
/// 進む（源がその後は値を返せる状態でも、壊れた排他の向こうは読まない・要件 8.4）。
#[test]
fn poisoned_source_logs_and_yields_empty_snapshot() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let source: SystemVarSource = Box::new(move || {
        if counted.fetch_add(1, Ordering::SeqCst) == 0 {
            panic!("源の 1 回目の読みで panic（排他を壊すため）");
        }
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", "太郎");
        snapshot
    });
    let (translate, playback) = split_source(source);
    assert!(
        catch_unwind(AssertUnwindSafe(translate)).is_err(),
        "1 回目の読みは panic する"
    );

    let mut snapshot = None;
    let events = capture(|| snapshot = Some(playback()));

    assert_logged_event(
        &events,
        Level::ERROR,
        "areka_ghost",
        "translate_snapshot_poisoned",
    );
    assert_eq!(
        snapshot,
        Some(SystemVarSnapshot::default()),
        "空の写しで進む"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "壊れた排他の向こうの源は呼ばない"
    );
}

/// 翻訳用の sylphya の読み口は再生用と同じ値を返し、再生用の固定の記録（実機の確かめで数を
/// 数える記録）を出さず、翻訳用の `debug!` を出す（要件 2.3・8.4）。
#[test]
fn translate_reader_matches_playback_without_its_fixed_log() {
    let parts = spawn_ghost_sylphya(ScopeRoots::default());
    let asker = AskerId::new("ghost/translate-reader-cage");
    parts.publisher.publish_static(
        asker.clone(),
        vec![
            ("selfname".into(), "さくら".into()),
            ("keroname".into(), "うにゅう".into()),
        ],
        vec![],
    );
    parts
        .publisher
        .barrier()
        .expect("barrier while actor alive");
    let translate = translate_snapshot_source(parts.reader.clone(), asker.clone());
    let playback = from_sylphya_provider(parts.reader.clone(), asker);

    let mut snapshot = None;
    let events = capture(|| snapshot = Some(translate()));

    let snapshot = snapshot.expect("translate reader produced a snapshot");
    assert_eq!(snapshot.get("selfname"), Some("さくら"));
    assert_eq!(snapshot, playback(), "翻訳用と再生用は同じ読み口の同じ値");
    assert_logged(
        &events,
        Level::DEBUG,
        "areka_ghost",
        "translate snapshot from sylphya reader",
    );
    assert!(
        !events
            .iter()
            .any(|e| e.message.contains("talk snapshot from sylphya reader")),
        "翻訳用の読み口は再生用の固定の記録を出さない: {:?}",
        events.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
    );

    parts.publisher.close();
    parts
        .handle
        .join()
        .expect("clean close joins without panic");
}

// ---- ゴーストの起動の経路（task 4.2） ----

use crate::runtime::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::ShioriBackend;
use areka_parsers::charset::DefaultEncoding;
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use std::sync::mpsc::{Sender, channel};

/// 挨拶に `%usernameさん` を返し、`OnTranslate` の Reference0 を `seen` へ送る偽の SHIORI
/// （他の照会は 204・`OnTranslate` も 204）。
struct TranslateProbeBackend {
    seen: Sender<String>,
}

impl ShioriBackend for TranslateProbeBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        match id {
            "OnBoot" => Ok(Some(r"\0%usernameさん\e".to_string())),
            "OnTranslate" => {
                let _ = self.seen.send(references[0].clone());
                Ok(None)
            }
            _ => Ok(None),
        }
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

/// ゴーストの起動の経路は口つきの起動を通り、挨拶の `OnTranslate` の Reference0 は
/// 注入した写しの源で展開済みの台詞になる（要件 1.1・2.3。素通しの口なら `%username` のまま）。
#[test]
fn boot_sends_on_translate_with_the_expanded_script() {
    let temp = temp_path_kit::TempPath::new("ghost-translate-wiring-boot");
    let root = temp.path();
    let ghost_master = root.join("ghost").join("master");
    let shell_master = root.join("shell").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::create_dir_all(&shell_master).expect("create shell/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,TestGhost\nshiori,dummy.dll\n",
    )
    .expect("write ghost descript.txt");
    std::fs::write(
        shell_master.join("descript.txt"),
        "charset,UTF-8\nname,TestShell\n",
    )
    .expect("write shell descript.txt");

    let (seen_tx, seen_rx) = channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let options = GhostBootOptions {
        ghost_root: root.to_path_buf(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(TranslateProbeBackend { seen: seen_tx }) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![],
        system_vars: SystemVarWiring::Custom(taro_source(calls)),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");
    let script = seen_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("OnTranslate never reached the SHIORI during boot");
    assert_eq!(script, r"\0太郎さん\e", "挨拶は展開してから翻訳へ送る");

    runtime
        .shutdown(areka_kanade::CloseReason::System)
        .expect("shutdown after boot");
}
