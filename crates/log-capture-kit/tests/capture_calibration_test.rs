//! 較正（要件 3.4-b）: **硬化していない素の捕捉では取りこぼす**ことを、別プロセスで示す。
//!
//! # なぜ別プロセスなのか
//!
//! 焼き付き（interest キャッシュへ `never` が焼かれる経路）が成立する条件は
//! 「`Dispatchers::has_just_one` が真」＝プロセス内に生きている dispatcher が 1 個以下、
//! である（機序は `src/probe.rs` の doc）。同じプロセスで他のテストが並列に
//! `with_default` を走らせていると dispatcher が 2 個以上になり、**素の捕捉でも
//! たまたま取りこぼさない**。実際、in-crate の自己テスト
//! `captures_event_whose_callsite_another_thread_registers_inside_the_window` は
//! `--test-threads=1` か `--exact` のときしか硬化の有無を区別できず、既定の並列実行では
//! 硬化を外しても緑のままだった（タスク 2.2 の実測）。
//!
//! そこで本較正は、**自身のテストバイナリを子プロセスとして 2 通り起動**する。
//! 子は 1 件しかテストを走らせないので、dispatcher は「窓の 1 個」だけ＝
//! `has_just_one` は確定的に真になり、焼き付きが**毎回**起きる。
//!
//! # 場面（両モード共通・要件 3.4）
//!
//! 「捕捉窓の**内側**で、別スレッドが先に同じ発行点を登録する」——`join()` で順序を
//! 確定させるので、色は時間に依存しない。
//!
//! - `bare`（硬化なし＝移行対象の crate が今書いている形）: 捕捉 **0 件**。
//! - `hardened`（kit の [`log_capture_kit::capture`]）: 捕捉 **1 件**。
//!   別スレッドの発火自体はスレッド局所ゆえ混入しないので、期待は 2 件ではなく 1 件。
//!
//! 各子が**自分で**自分の期待件数を assert する。親は子の終了コードだけでなく標準出力に
//! `1 passed` が現れることを要求する——0 件実行でも終了コードは 0 になるため、
//! 名前の綴り違いやフィルタ漏れで「何も走らせずに緑」が成立してしまう（要件 8.4／9.6 の流儀）。

use std::process::Command;
use std::sync::{Arc, Mutex};

/// モードの受け渡しに使う環境変数。本ワークスペースの実行時環境変数は `AREKA_` 名前空間。
const MODE_ENV: &str = "AREKA_LOG_CAPTURE_CALIBRATION";
const MODE_BARE: &str = "bare";
const MODE_HARDENED: &str = "hardened";

/// 子テストの名前。親が `--exact` で 1 件だけ走らせるために逐語で持つ。
const CHILD_BARE: &str = "child_bare_capture_drops_the_event";
const CHILD_HARDENED: &str = "child_hardened_capture_keeps_the_event";

/// 較正専用の発行点（宛先）。この文字列を踏むのは本ファイルだけ。
const TARGET_CALIBRATION: &str = "log_capture_kit::calibration::inside_window";

/// 両モードが踏む**同一の**発行点。
fn emit_calibration_point() {
    tracing::info!(target: TARGET_CALIBRATION, mark = "calibration", "窓の内側で先に登録される発行点");
}

/// 窓の内側で別スレッドが先に発行点を登録し、そのあと自スレッドが発火する場面。
///
/// `join()` で「別スレッドの登録 → 自スレッドの発火」の順序を確定させる。
fn scene_inside_window() {
    std::thread::spawn(emit_calibration_point)
        .join()
        .expect("先着スレッドは panic しない");
    emit_calibration_point();
}

// ---- 親: 子プロセスを 2 モードで起動する --------------------------------

fn run_child(test_name: &str, mode: &str) {
    let exe = std::env::current_exe().expect("自身のテストバイナリの位置は取れる");
    let out = Command::new(&exe)
        // `--exact` と `--ignored` で「その 1 件だけ」を走らせる。子プロセス内に
        // 他の窓（＝他の dispatcher）を作らせないことが較正の前提。
        .args([test_name, "--exact", "--ignored"])
        .env(MODE_ENV, mode)
        .output()
        .unwrap_or_else(|e| panic!("子（{mode}）を起動できなかった: {e}"));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "子（{mode}・{test_name}）が失敗した\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    assert!(
        stdout.contains("test result: ok. 1 passed"),
        "子（{mode}・{test_name}）が 1 件も実行していない。0 件実行でも終了コードは 0 に\
         なるので、これを許すと「何も走らせずに緑」が通る\
         \n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
}

/// 要件 3.4-b の較正本体。素の捕捉が取りこぼし、硬化した捕捉が取りこぼさないことを、
/// 同じ場面・同じ発行点・別プロセスで対比する。
#[test]
fn bare_capture_drops_what_hardened_capture_keeps() {
    run_child(CHILD_BARE, MODE_BARE);
    run_child(CHILD_HARDENED, MODE_HARDENED);
}

// ---- 子: 通常実行から除外され、環境変数が無ければ即座に終わる ------------

/// 環境変数が無ければ `None`。親プロセス内で `--include-ignored` により走っても、
/// ここで即 return するので**親の実行順にも probe の有無にも依存しない**。
fn mode_from_env() -> Option<String> {
    std::env::var(MODE_ENV).ok()
}

/// 硬化なし（素の `with_default`）の捕捉先。kit の `CaptureSubscriber` から
/// **硬化だけを取り除いた**同型で、移行対象の crate が今書いている形に対応する。
///
/// `register_callsite` は**あえて既定実装のまま**にしてある（＝上書きしない）。ここを
/// `sometimes` に固定してしまうと、それ自体が硬化の一種になって較正が成立しない。
#[derive(Clone)]
struct BareSubscriber(Arc<Mutex<Vec<String>>>);

impl tracing::Subscriber for BareSubscriber {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        self.0
            .lock()
            .expect("捕捉バッファは毒化していない")
            .push(event.metadata().target().to_string());
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// 素の捕捉。`ensure_interest_probes` も窓内の `rebuild_interest_cache` も番兵も**無い**。
/// kit の関数は 1 つも呼ばない（呼ぶと内部で probe が常駐して較正が意味を失う）。
fn bare_capture_count() -> usize {
    let sink: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let subscriber = BareSubscriber(Arc::clone(&sink));

    tracing::subscriber::with_default(subscriber, scene_inside_window);

    let targets = sink.lock().expect("捕捉バッファは毒化していない");
    targets.iter().filter(|t| *t == TARGET_CALIBRATION).count()
}

/// 硬化ありの捕捉（kit の既定 API）。
fn hardened_capture_count() -> usize {
    let ((), events) = log_capture_kit::capture(scene_inside_window);
    events
        .iter()
        .filter(|e| e.target == TARGET_CALIBRATION)
        .count()
}

/// 子（bare）: 素の捕捉は同じ場面で**取りこぼす**（0 件）。
#[test]
#[ignore = "親テスト bare_capture_drops_what_hardened_capture_keeps が子プロセスとして起動する"]
fn child_bare_capture_drops_the_event() {
    let Some(mode) = mode_from_env() else {
        return; // 環境変数が無い＝親プロセス内での巻き添え実行。何もしない。
    };
    assert_eq!(mode, MODE_BARE, "{MODE_ENV} の値が子の想定と違う");

    assert_eq!(
        bare_capture_count(),
        0,
        "硬化なしの捕捉が取りこぼさなかった。較正が成立していない\
         （子プロセス内に別の dispatcher が生きていて has_just_one が偽になっている等）"
    );
}

/// 子（hardened）: kit の捕捉は同じ場面で**取りこぼさない**（1 件）。
#[test]
#[ignore = "親テスト bare_capture_drops_what_hardened_capture_keeps が子プロセスとして起動する"]
fn child_hardened_capture_keeps_the_event() {
    let Some(mode) = mode_from_env() else {
        return; // 環境変数が無い＝親プロセス内での巻き添え実行。何もしない。
    };
    assert_eq!(mode, MODE_HARDENED, "{MODE_ENV} の値が子の想定と違う");

    assert_eq!(
        hardened_capture_count(),
        1,
        "硬化ありの捕捉が窓内先着の発行点を取りこぼした（要件 3.2／3.4-a）"
    );
}
