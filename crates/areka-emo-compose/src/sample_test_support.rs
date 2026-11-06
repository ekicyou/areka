//! 検体を本 crate のテストへ配る共有の受け口（spec: areka-P0-nar-install 要件 1.6）。
//!
//! 検体は窓口 `sample-ghost-kit` から取得し、**プロセス寿命で保持**する。段 ③ で `SampleRoot` の
//! `Drop` が展開した複製を消すため、関数内の一時値にすると借用の元がその場で消える。保持を
//! 1 か所に束ねてあるので、段 ③ で複製を作るのもテストバイナリあたり 1 回で済む
//! （各ファイルが自前で保持すると、その数だけ木が複製される）。

use std::path::PathBuf;
use std::sync::LazyLock;

use sample_ghost_kit::SampleRoot;

static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// emo2 検体のゴーストフォルダ。
pub(crate) fn emo2_root() -> PathBuf {
    EMO2.folder().to_path_buf()
}

/// emo2 検体の `shell/master/surfaces.txt` を読んで解析する（`read_to_string`＋`areka_parsers::shell::parse`。
/// COM／WIC／atlas bake は経由しない純粋な parse ゆえ headless で走る）。
pub(crate) fn emo2_shell() -> areka_parsers::shell::Shell {
    let path = emo2_root().join("shell/master/surfaces.txt");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
    areka_parsers::shell::parse(&content)
}

/// YAYA の標準テンプレート `konnoyayame` 検体（spec: areka-P0-shell-implicit-surface 要件 7.11）。
static KONNOYAYAME: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("konnoyayame").expect("konnoyayame は登記済みの検体"));

/// `konnoyayame` 検体のシェルのフォルダ（`shell/master/`）。
pub(crate) fn konnoyayame_shell_root() -> PathBuf {
    KONNOYAYAME.folder().join("shell/master")
}

/// 受け口そのものの較正（spec: areka-P0-keycolor-clickthrough-coverage 要件 3.1）——検体の登記名や
/// 木の形が変わると、これを使うテストは「読めない」の一言で赤くなって原因が見えなくなる。
/// ここが先に赤くなれば、原因が受け口側だと分かる。
#[test]
fn every_sample_receptor_points_at_a_real_folder() {
    let shell = konnoyayame_shell_root();
    assert!(
        shell.join("surfaces.txt").is_file(),
        "konnoyayame の受け口が指す先に surfaces.txt が無い: {}",
        shell.display()
    );
    let ghost = emo2_root();
    assert!(
        ghost.join("shell").join("master").is_dir(),
        "emo2 の受け口が指す先に shell/master が無い: {}",
        ghost.display()
    );
}
