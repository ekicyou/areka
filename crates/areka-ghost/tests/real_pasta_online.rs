//! env ゲート実 pasta 追験: emo2（ぱすた）は `Status` に `online` が載っている間は雑談を
//! 始めない（spec `areka-P0-status-execution-states` 要件 8.4・design「E2E / 実機」）。
//!
//! # なぜ別の実行ファイルか
//! 実ゴーストの結線は `KanadeConfig::new` の既定を使い、殻はプロセスに 1 つの
//! `areka_kanade::online::PROCESS` を読む。このテストはそれを立てるので、`Status` を完全一致で
//! 照合する `tests/ghost.rs` の束ねと同じ実行ファイルに入れると、並列に走る他のテストへ
//! `online` が漏れる（design「online の数と殻の同期 > テストの決定論 ⑶」）。
//!
//! # env ゲート（`tests/ghost/real_pasta_test.rs` と同じ語法）
//! `HOST32_PASTA_DLL`（pasta DLL のフルパス）が未設定なら silent skip。設定済みだが DLL 不在なら
//! 明示 fail。helper exe は `HOST32_HELPER_EXE` 優先→`target/i686-pc-windows-msvc/{debug,release}`。
//!
//! # フィクスチャ
//! pasta は辞書（`dic/`）・`scripts/`・`pasta.toml` を `ghost/master` から読むので、DLL 単体の
//! フィクスチャでは応答できない。検体 `emo2` の使い捨ての複製（`sample-ghost-kit`・置き場は
//! `target\` の下で短いパス）を根にし、その `ghost/master/pasta.dll` を `HOST32_PASTA_DLL` で
//! 上書きする。
//!
//! # 雑談の間隔の出どころ
//! pasta の雑談（`OnTalk`）は OnSecondChange の中で `virtual_dispatcher.lua` の `check_talk` が
//! 壁時計で判定し、間隔は emo2 の `ghost/master/pasta.toml` の `[ghost]`
//! `talk_interval_min = 15`・`talk_interval_max = 30`（秒）。`Status` に `online` 等が含まれると
//! `M.is_blocked` で dispatch の入口から抜ける（`kick_force` が立っていない限り。`kick_force` は
//! キックの保留でしか立たないので、起動の挨拶の後に他の入力を与えなければ立たない）。
//! 最大 30 秒に余裕を足して [`ONLINE_SECOND_CHANGES`] 回の OnSecondChange GET を見る。
//!
//! # 較正（0 件が空振りでないこと）
//! 0 件の主張が「pasta が何も返せない環境」で偽って緑にならないよう、⑴ 起動の挨拶が `Value` で
//! 返ったこと ⑵ 見張りの間の OnSecondChange の `Status` に `online` が載っていたこと ⑶ 守り手を
//! 落とした後は間隔内に OnSecondChange へ `Value`（雑談か時報）が返ること、を併せて確かめる。
//!
//! # 共有した補助・写した補助
//! 交信の記録（`ghost/recorder.rs`）と既定のシステム変数（`ghost/common.rs`）はパス指定で取り込む。
//! `ghost/real_pasta_test.rs` は `crate::spine_e2e_test` に依存し自身の `#[test]` も持つので取り込めず、
//! helper の解決（`resolve_helper_exe`）と `BoxedBackend` だけを写した
//! （`ghost/snapshot_capture_test.rs` の同名の項目と同一）。

#[path = "ghost/common.rs"]
mod common;
#[path = "ghost/recorder.rs"]
mod recorder;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_ghost::shiori_wiring::real_connect;
use areka_ghost::sink::DiscardSink;
use areka_ghost::ticker::TickerConfig;
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::{CloseReason, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package;
use sample_ghost_kit::SampleRoot;
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

use crate::recorder::{ExchangeKind, ExchangeOutcome, ExchangeRecord, Recorder, RecorderHandle};

/// emo2 の雑談の間隔の上限（`pasta.toml` の `talk_interval_max`・秒）。
const TALK_INTERVAL_MAX_SECS: usize = 30;

/// `online` の間に見る OnSecondChange GET の回数（間隔の上限 30 秒＋余裕 15 回）。
const ONLINE_SECOND_CHANGES: usize = TALK_INTERVAL_MAX_SECS + 15;

/// 起動の挨拶が終わる（最初の OnSecondChange GET が出る）までの上限。
const GREETING_TIMEOUT: Duration = Duration::from_secs(120);

/// 見張りと較正のそれぞれの上限（1 秒周期の Tick に十分な余裕）。
const WATCH_TIMEOUT: Duration = Duration::from_secs(120);

/// 実時計のポーリング間隔（`TickerMode::Real` の env ゲート追験に限り実 sleep を許す）。
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// `shutdown()` の有界観測。
const SHUTDOWN_BOUND: Duration = Duration::from_secs(30);

/// `Box<dyn ShioriBackend>` を `Recorder` に包むための委譲（`snapshot_capture_test.rs` と同一）。
struct BoxedBackend(Box<dyn ShioriBackend>);

impl ShioriBackend for BoxedBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.0.get(id, references, status)
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.0.notify(id, references, status)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.0.unload()
    }

    fn status(&mut self) -> HelperStatus {
        self.0.status()
    }
}

/// i686 helper の解決（`real_pasta_test.rs::resolve_helper_exe` と同一）。
fn resolve_helper_exe() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("HOST32_HELPER_EXE") {
        let p = PathBuf::from(&explicit);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "HOST32_HELPER_EXE={explicit:?} が指すファイルが存在しません（i686 helper を先にビルド）"
        ));
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.clone());
    let target_base = workspace_root.join("target").join("i686-pc-windows-msvc");
    for profile in ["debug", "release"] {
        let candidate = target_base.join(profile).join("shiori-host32-helper.exe");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "i686 helper exe が見つかりません（探索先: {}\\{{debug,release}}\\shiori-host32-helper.exe）。\
         PowerShell で先に i686 helper をビルドしてください: \
         cargo build -p shiori-host32-helper --target i686-pc-windows-msvc",
        target_base.display()
    ))
}

fn is_second_change_get(r: &ExchangeRecord) -> bool {
    r.kind == ExchangeKind::Get && r.id.as_deref() == Some("OnSecondChange")
}

fn has_online(r: &ExchangeRecord) -> bool {
    r.status
        .as_deref()
        .is_some_and(|s| s.split(',').any(|w| w == "online"))
}

/// `pred` が記録の上で真になるまで実時計で待つ。真になった時点の記録を返す。
fn wait_records(
    handle: &Arc<Mutex<Option<RecorderHandle>>>,
    timeout: Duration,
    what: &str,
    pred: impl Fn(&[ExchangeRecord]) -> bool,
) -> Vec<ExchangeRecord> {
    let deadline = Instant::now() + timeout;
    loop {
        let records = handle
            .lock()
            .expect("recorder handle slot poisoned")
            .as_ref()
            .map(RecorderHandle::records)
            .unwrap_or_default();
        if pred(&records) {
            return records;
        }
        assert!(
            Instant::now() < deadline,
            "{what} が {timeout:?} 以内に起きなかった。記録: {records:#?}"
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[test]
fn emo2_does_not_start_chatting_while_online() {
    let Ok(pasta_dll) = std::env::var("HOST32_PASTA_DLL") else {
        eprintln!(
            "HOST32_PASTA_DLL 未設定のため実 pasta の online 追験を skip（任意 gate・要件 8.4）。"
        );
        return;
    };
    let dll_path = PathBuf::from(&pasta_dll);
    assert!(
        dll_path.is_file(),
        "HOST32_PASTA_DLL={pasta_dll:?} が指す DLL が存在しません（指定 DLL 不在は明示 fail）"
    );
    let helper_exe = resolve_helper_exe().expect("i686 helper exe の解決に失敗");

    // 起動記録の無い新品の emo2（辞書込み）に、指定の DLL を持ち込む。
    let sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let ghost_root = sample.folder().to_path_buf();
    std::fs::copy(
        &dll_path,
        ghost_root.join("ghost").join("master").join("pasta.dll"),
    )
    .expect("HOST32_PASTA_DLL を emo2 の ghost/master へ写せませんでした");

    let mount = package::resolve(&ghost_root, DefaultEncoding::Ansi)
        .expect("emo2 の複製はマウント解決を通るはず");
    let slot: Arc<Mutex<Option<RecorderHandle>>> = Arc::new(Mutex::new(None));
    let slot2 = Arc::clone(&slot);
    let connect = real_connect(helper_exe, mount.shiori, DefaultEncoding::Ansi);
    let wiring = ShioriWiring::Custom(Box::new(move || {
        let (recorder, handle) = Recorder::new(BoxedBackend(connect()?));
        *slot2.lock().expect("recorder handle slot poisoned") = Some(handle);
        Ok(Box::new(recorder) as Box<dyn ShioriBackend>)
    }));

    // 起こす前から通信中にしておく（殻は最初のメッセージの前に数を読む）。
    let online = areka_kanade::online::PROCESS.begin("test");

    let runtime = boot(GhostBootOptions {
        ghost_root,
        default_encoding: DefaultEncoding::Ansi,
        shiori: wiring,
        sinks: vec![Box::new(DiscardSink)],
        system_vars: SystemVarWiring::Custom(common::test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Real(TickerConfig::default()),
    })
    .expect("real pasta 越しの boot は DLL/helper 実在確認済みなら成功するはず");

    // 挨拶の再生中の OnSecondChange は NOTIFY。最初の GET が出た時点で挨拶は終わっている。
    let records = wait_records(&slot, GREETING_TIMEOUT, "起動の挨拶の完了", |rs| {
        rs.iter().any(is_second_change_get)
    });
    let greeted_at = Instant::now();
    let mark = records
        .iter()
        .position(is_second_change_get)
        .expect("待ちの条件どおり");
    assert!(
        records[..mark]
            .iter()
            .any(|r| r.kind == ExchangeKind::Get && matches!(r.outcome, ExchangeOutcome::Value(_))),
        "起動の挨拶が Value で返っていない（pasta が応答できていない＝0 件の主張が空振りになる）: {records:#?}"
    );

    // 他の入力を与えず、間隔を超える回数の OnSecondChange を待つ。
    let records = wait_records(
        &slot,
        WATCH_TIMEOUT,
        "online の間の OnSecondChange",
        |rs| {
            rs[mark..]
                .iter()
                .filter(|r| is_second_change_get(r))
                .count()
                >= ONLINE_SECOND_CHANGES
        },
    );
    // 回数でなく壁時計で、見張りが雑談の間隔の上限をまたいだことを判定する。
    let watched_for = greeted_at.elapsed();
    assert!(
        watched_for >= Duration::from_secs(TALK_INTERVAL_MAX_SECS as u64),
        "見張りの時間 {watched_for:?} が雑談の間隔の上限 {TALK_INTERVAL_MAX_SECS} 秒に届いていない\
         （OnSecondChange が 1 秒より速く届いた＝0 件の主張の前提が崩れている）"
    );
    let watched = &records[mark..];
    let values: Vec<_> = watched
        .iter()
        .filter(|r| r.kind == ExchangeKind::Get && matches!(r.outcome, ExchangeOutcome::Value(_)))
        .collect();
    assert!(
        values.is_empty(),
        "online の間に emo2 が Value を返した（雑談を始めた）: {values:#?}"
    );

    let second_changes: Vec<_> = watched.iter().filter(|r| is_second_change_get(r)).collect();
    assert!(
        second_changes.iter().all(|r| has_online(r)),
        "online の間の OnSecondChange の Status に online が載っていない: {second_changes:#?}"
    );

    // 較正: 数を戻すと、間隔内に OnSecondChange へ Value が返る。
    drop(online);
    let after = records.len();
    wait_records(
        &slot,
        WATCH_TIMEOUT,
        "online を外した後の雑談",
        |rs| {
            rs[after..].iter().any(|r| {
                is_second_change_get(r)
                    && !has_online(r)
                    && matches!(r.outcome, ExchangeOutcome::Value(_))
            })
        },
    );

    let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<Result<(), String>>(0);
    std::thread::spawn(move || {
        let result = runtime
            .shutdown(CloseReason::System)
            .map_err(|err| format!("{err:?}"));
        let _ = done_tx.send(result);
    });
    match done_rx.recv_timeout(SHUTDOWN_BOUND) {
        Ok(result) => assert!(result.is_ok(), "shutdown は Ok(()) を返すはず: {result:?}"),
        Err(_) => panic!("shutdown が {SHUTDOWN_BOUND:?} 以内に完了しなかった（宙吊りの疑い）"),
    }
}
