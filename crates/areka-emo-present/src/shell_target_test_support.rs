//! `shell_target` の檻が共有する受け口（検体・COM 初期化・復号器と記録の手伝い）。
//!
//! 檻は `shell_target_*_tests.rs` と `presenter_keycolor_clickthrough_tests.rs` に分かれるが、
//! 検体の複製と COM の初期化は**テストバイナリに 1 つ**で足りる。各ファイルが自前で持つと、その数だけ
//! 検体の木が複製される（`areka-seriko` の `sample_test_support.rs` と同じ理由）。
//! `presenter` の配下の檻と `balloon` の檻からも引くので、本モジュールと、そこから引く受け口は
//! クレート内公開にしている。
//!
//! # 一時フォルダをここに置かない理由
//!
//! テスト用の一時フォルダは共有窓口 [`temp_path_kit::TempPath`] をその場で呼ぶ。本ファイルへ
//! 写し取った独自の型は置かない——窓口を迂回する新設は
//! `crates/log-capture-kit/tests/temp_path_guard_test.rs` が例外表への明示的な編集を求めて
//! 拒むためであり、迂回しない限り包み直す理由が無い。ログの捕捉は
//! [`log_capture_kit::capture`] を各檻がそのまま呼ぶ。

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use areka_emo_atlas::MemoryDecoder;
use areka_emo_compose::ComposedSurface;
use sample_ghost_kit::SampleRoot;

use super::EmoWorld;

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

/// 一時フォルダのシェルに `seriko.use_self_alpha,1` の descript.txt を置く（今までの `On` 固定と
/// 同じ条件・descript.txt が読めない `warn!` も出ない・spec: areka-P0-self-alpha-declaration）。
pub(super) fn write_descript(dir: &std::path::Path) {
    std::fs::write(
        dir.join("descript.txt"),
        "charset,UTF-8\nseriko.use_self_alpha,1\n",
    )
    .expect("descript.txt 作成");
}

/// 本モジュールが出す記録の宛先（既定の target＝モジュールパス）。
pub(super) const SHELL_TARGET: &str = "areka_emo_present::shell_target";

/// 不透明 1×1 PBGRA の絵を `names` の名前で入れた復号器（bake が placement を必ず産む＝非退化）。
pub(super) fn decoder_with(dir: &Path, names: &[&str]) -> MemoryDecoder {
    let mut dec = MemoryDecoder::new();
    for name in names {
        dec.insert(dir.join(name), 1, 1, 4, vec![10u8, 20, 30, 255], true);
    }
    dec
}

/// 全画素が同じ色の不透明な絵を復号器へ登録する。
pub(super) fn insert_solid(
    dec: &mut MemoryDecoder,
    dir: &Path,
    name: &str,
    (w, h): (u32, u32),
    color: [u8; 4],
) {
    let bgra: Vec<u8> = color
        .iter()
        .copied()
        .cycle()
        .take((w * h * 4) as usize)
        .collect();
    dec.insert(dir.join(name), w, h, w * 4, bgra, true);
}

/// 合成結果の 1 画素（premultiplied BGRA）を引く。
pub(super) fn pixel(surface: &ComposedSurface, x: u32, y: u32) -> [u8; 4] {
    let at = (y * surface.stride() + x * 4) as usize;
    surface.bytes()[at..at + 4]
        .try_into()
        .expect("4 バイトの画素")
}

/// 面の表の層 0 の画像パスを引く（層 0 が無ければ `None`）。
pub(super) fn layer0_path(world: &EmoWorld, id: u32) -> Option<String> {
    world
        .surface(id)?
        .elements
        .iter()
        .find(|e| e.layer == 0)
        .map(|e| e.path.as_str().to_string())
}

/// 記録 1 行を「本文|欄=値…」の 1 本の文字列にする（`message` 以外の欄は名前の昇順）。
pub(super) fn describe(e: &CapturedEvent) -> String {
    let mut line = e.message().to_string();
    for (name, value) in e.fields_map() {
        if name != "message" {
            line.push_str(&format!("|{name}={value}"));
        }
    }
    line
}

// ── 他クレートの検体（書き換えずに読む）────────────────────────────────────────

/// 入れ子の検体のフォルダ（`areka-emo-compose/tests/fixtures/surface-nesting/`）。
pub(super) fn nesting_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("areka-emo-compose")
        .join("tests")
        .join("fixtures")
        .join("surface-nesting")
}

/// 入れ子の検体のフォルダに在る画像（`surfaces.txt` 以外の全部）。
pub(super) const NESTING_FIXTURE_IMAGES: [&str; 8] = [
    "body.png",
    "eye.png",
    "eye_closed.png",
    "mouth.png",
    "mouth_open.png",
    "part.png",
    "surface11.png",
    "surface2.png",
];

/// 箱の検体のフォルダ（`areka-emo-text/tests/fixtures/shell-balloon/`）。
pub(super) fn boxes_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("areka-emo-text")
        .join("tests")
        .join("fixtures")
        .join("shell-balloon")
}

/// 箱の検体の element定義の overlay が指す画像（フォルダには無いので復号器へ入れる）。
pub(super) const BOXES_FIXTURE_IMAGES: [&str; 4] =
    ["body0.png", "body1.png", "body2.png", "body3.png"];

// ── 検体（`sample_ghost_kit::SampleRoot::acquire` 経由のみ・要件 7.11）────────────────
//
// 段 ③ で `Drop` が展開した複製を消すため、一時値にせずプロセス寿命で保持する。
// `vendors/sample_ghost/<検体名>/` の直パスは 1 か所も綴らない。

/// `emo2` 検体（`areka` の実ゴースト・既存の期待値の出どころ）。
pub(crate) static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// 里々の標準テンプレート `R_POST_and_KOMAINU` 検体。
static R_POST_AND_KOMAINU: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire("R_POST_and_KOMAINU").expect("R_POST_and_KOMAINU は登記済みの検体")
});

/// YAYA の標準テンプレート `konnoyayame` 検体。
static KONNOYAYAME: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("konnoyayame").expect("konnoyayame は登記済みの検体"));

/// クローディア `claudia` 検体（element定義の `base` を使う無改変の実ゴースト）。
static CLAUDIA: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("claudia").expect("claudia は登記済みの検体"));

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

/// `claudia` のシェル（`shell/master/`）のフォルダ。
pub(super) fn claudia_shell_dir() -> PathBuf {
    CLAUDIA.folder().join("shell").join("master")
}

/// 4 つの受け口が実在するシェルのフォルダを指す（`surfaces.txt` が在る）。
///
/// 受け口そのものの較正である——検体の登記名や木の形が変わると、これを使う檻は「読めない」の
/// 一言で赤くなって原因が見えなくなる。ここが先に赤くなれば、原因が受け口側だと分かる。
#[test]
fn every_sample_receptor_points_at_a_real_shell_folder() {
    for (name, dir) in [
        ("emo2", emo2_shell_dir()),
        ("R_POST_and_KOMAINU", r_post_and_komainu_shell_dir()),
        ("konnoyayame", konnoyayame_shell_dir()),
        ("claudia", claudia_shell_dir()),
    ] {
        assert!(
            dir.join("surfaces.txt").is_file(),
            "{name} の受け口が指す先に surfaces.txt が無い: {}",
            dir.display()
        );
    }
}
