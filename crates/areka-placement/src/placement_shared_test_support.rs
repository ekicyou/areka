//! 配置のテストだけが使う一時フォルダと合成の検体（`temp-path-kit` に頼る・`#[cfg(test)]`）。
//!
//! 外のテストも使う支えは `test_support`、見本のゴーストの入口は `sample_test_support`
//! （spec: placement-crate-split 設計 D8）。

use std::fs;
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;
use wintf::ecs::DPI;

use super::sample_test_support::balloon_root;

/// えも？？ の絵を写した一時フォルダへ、本物の えも？？ と同じ透過の宣言 `1` だけを書いた
/// `descript.txt` を置く。写しが宣言なしの決まりに当たり、絵の中身しだいで結果が変わることを
/// 防ぐ（下の合成の補助も同じ宣言を書く・spec: areka-P0-self-alpha-declaration 設計
/// 「入口を一時フォルダで通す既存のテストの棚卸し」）。`key` はシェルなら
/// `seriko.use_self_alpha`、バルーンなら `use_self_alpha`。
pub(super) fn write_self_alpha_one_descript(dir: &Path, key: &str) {
    fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\n{key},1\n"),
    )
    .expect("descript.txt に透過の宣言 1 を書く");
}

/// 決定論テストが [`super::resolver::resolve_placement`] へ渡す**採寸 DPI**（正典既定 96）。
///
/// 配置式 P1〜P5 は採寸 DPI を 1 度も読まない——値は基準対
/// （`ScopePlacement::balloon_offset_base`）へ刻まれるだけである。ゆえに幾何の檻は
/// この既定を渡せばよく、採寸 DPI の決め方そのものの檻は `placement_prepare_tests.rs`
/// が持つ（areka-P0-balloon-offset-dpi task 2.1）。
pub(super) const MEASURE_DPI: DPI = DPI {
    dpi_x: 96,
    dpi_y: 96,
};

// ── テスト用一時ディレクトリ（共通窓口 `temp-path-kit` の薄い包み）──

/// Drop 時に自身を再帰削除する一時ディレクトリ。
///
/// 名前の組み立ては共通窓口 [`TempPath`] へ一本化してある。この crate の中に
/// 「窓口」と「窓口の移植元になった同型の実装」の 2 系統を残さないための包みで、
/// 出来上がる名前は移植前と同じ `areka-placement-prepare-{プロセス識別子}-{連番}`。
pub(super) struct TempDir(TempPath);

impl TempDir {
    pub(super) fn new() -> Self {
        TempDir(TempPath::new("placement-prepare"))
    }

    pub(super) fn path(&self) -> &Path {
        self.0.path()
    }
}

/// バルーン定義だけの最小合成ディレクトリを組む（テスト補助・task 2.2）。
///
/// 中身は既定設定 `descript.txt` ＋ 面 0 の画像 1 枚のみ。`resolve_balloon_faces` は
/// **ファイル名の列挙**しか行わない（画像を復号しない）ため、これだけで面 0 の連鎖が
/// 解決でき、採寸も COM 初期化も不要な軽量検体になる。
///
/// `extra_lines` は既定設定へそのまま追記される行（末尾改行込みで渡すこと）。
/// `windowposition.limit,2` のような**数値化を経ない生値の語彙検体**を、転記層まで
/// 素通しで届けるための口である（同じ値を `synth_declared_dpi_ghost` の
/// `Option<(i32, i32)>` 引数では表現できない）。
pub(super) fn synth_balloon_dir(root: &TempDir, name: &str, extra_lines: &str) -> PathBuf {
    let balloon_dir = root.path().join(name);
    fs::create_dir_all(&balloon_dir).expect("create balloon dir");
    fs::write(
        balloon_dir.join("descript.txt"),
        format!("charset,UTF-8\ndpi,96\nuse_self_alpha,1\n{extra_lines}"),
    )
    .expect("balloon descript");
    fs::copy(
        balloon_root().join("balloons0.png"),
        balloon_dir.join("balloons0.png"),
    )
    .expect("balloons0.png 複写");
    balloon_dir
}
