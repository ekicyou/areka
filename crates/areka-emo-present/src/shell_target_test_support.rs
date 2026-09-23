//! `shell_target` の檻が共有する受け口（検体・COM 初期化・ログの捕捉窓）。
//!
//! 本ファイルを使う檻は 5 本（`shell_target_load_tests.rs`・`shell_target_base_image_tests.rs`・
//! `shell_target_template_tests.rs`・`shell_target_emo2_tests.rs`・
//! `presenter_keycolor_clickthrough_tests.rs`）に分かれるが、検体の複製と
//! COM の初期化は**テストバイナリに 1 つ**で足りる。各ファイルが自前で持つと、その数だけ
//! 検体の木が複製される（`areka-seriko` の `sample_test_support.rs` と同じ理由）。
//! 最後の 1 本だけは `presenter` の配下に在るので、本モジュールと、それが使う受け口 2 口
//! （`r_post_and_komainu_shell_dir`・`konnoyayame_shell_dir`）をクレート内公開にしている。
//!
//! `balloon_test_support.rs` を流用できないのは、あちらの受け口が `pub(super)` でバルーンの
//! モジュール境界の内側に閉じており、外から引けないためである。
//!
//! # 一時フォルダをここに置かない理由
//!
//! テスト用の一時フォルダは共有窓口 [`temp_path_kit::TempPath`] をその場で呼ぶ。本ファイルへ
//! 写し取った独自の型は置かない——窓口を迂回する新設は
//! `crates/log-capture-kit/tests/temp_path_guard_test.rs` が例外表への明示的な編集を求めて
//! 拒むためであり、迂回しない限り包み直す理由が無い。
//!
//! # ログの捕捉窓を書き写さない
//!
//! [`capture_events`] は硬化機構の唯一の定義元 [`log_capture_kit::capture`] へ委譲するだけで、
//! 捕捉窓の中身（常駐 probe・interest の再計算・番兵による空振り検出）は 1 文字も写さない。
//! 写し取った側だけが静かに嘘をつく形になるためである（機序は `log_capture_kit` の crate doc）。

use std::path::PathBuf;
use std::sync::LazyLock;

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use sample_ghost_kit::SampleRoot;

pub(super) use log_capture_kit::CapturedEvent;

/// COM を初期化して `f` を走らせる（`WicDecoderArm` の前提・`display_gpu_tests.rs` と同型）。
pub(super) fn with_com_initialized<F: FnOnce()>(f: F) {
    unsafe {
        // 既に初期化済みでも `RPC_E_CHANGED_MODE` を許容して続行する。
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    f();
    unsafe {
        CoUninitialize();
    }
}

/// `f` の実行中に出たイベントを捕捉して `(戻り値, イベント列)` を返す。
///
/// 捕捉と硬化は硬化機構の唯一の定義元 [`log_capture_kit::capture`] が行う。捕捉が働いて
/// いなければ空の結果を静かに返さず panic する。
pub(super) fn capture_events<T>(f: impl FnOnce() -> T) -> (T, Vec<CapturedEvent>) {
    log_capture_kit::capture(f)
}

// ── 検体（`sample_ghost_kit::SampleRoot::acquire` 経由のみ・要件 7.11）────────────────
//
// 段 ③ で `Drop` が展開した複製を消すため、一時値にせずプロセス寿命で保持する。
// `vendors/sample_ghost/<検体名>/` の直パスは 1 か所も綴らない。

/// `emo2` 検体（`areka` の実ゴースト・既存の期待値の出どころ）。
static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// 里々の標準テンプレート `R_POST_and_KOMAINU` 検体。
static R_POST_AND_KOMAINU: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire("R_POST_and_KOMAINU").expect("R_POST_and_KOMAINU は登記済みの検体")
});

/// YAYA の標準テンプレート `konnoyayame` 検体。
static KONNOYAYAME: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("konnoyayame").expect("konnoyayame は登記済みの検体"));

/// `emo2` のシェル（`shell/master/`）のフォルダ。
pub(super) fn emo2_shell_dir() -> PathBuf {
    EMO2.folder().join("shell").join("master")
}

/// `R_POST_and_KOMAINU` のシェル（`shell/master/`）のフォルダ。
pub(crate) fn r_post_and_komainu_shell_dir() -> PathBuf {
    R_POST_AND_KOMAINU.folder().join("shell").join("master")
}

/// `konnoyayame` のシェル（`shell/master/`）のフォルダ。
pub(crate) fn konnoyayame_shell_dir() -> PathBuf {
    KONNOYAYAME.folder().join("shell").join("master")
}

/// 3 つの受け口が実在するシェルのフォルダを指す（`surfaces.txt` が在る）。
///
/// 受け口そのものの較正である——検体の登記名や木の形が変わると、これを使う檻は「読めない」の
/// 一言で赤くなって原因が見えなくなる。ここが先に赤くなれば、原因が受け口側だと分かる。
#[test]
fn every_sample_receptor_points_at_a_real_shell_folder() {
    for (name, dir) in [
        ("emo2", emo2_shell_dir()),
        ("R_POST_and_KOMAINU", r_post_and_komainu_shell_dir()),
        ("konnoyayame", konnoyayame_shell_dir()),
    ] {
        assert!(
            dir.join("surfaces.txt").is_file(),
            "{name} の受け口が指す先に surfaces.txt が無い: {}",
            dir.display()
        );
    }
}
