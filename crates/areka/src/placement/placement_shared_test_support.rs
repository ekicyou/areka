use std::fs;
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use wintf::ecs::DPI;

use super::resolver::RectPx;

/// COM 初期化下でクロージャを実行する（measure が `WicDecoderArm` を要求・
/// measure.rs テストと同一パターン）。
pub(super) fn with_com_initialized<F: FnOnce()>(f: F) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    f();
    unsafe {
        CoUninitialize();
    }
}

/// emo2 fixture ルート（source.rs／measure.rs テストと同一アンカー規約）。
pub(super) fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// emo2 fixture のバルーンルート（task 4.1 テストの規約を踏襲）。
pub(super) fn balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}

/// 決定論テスト用の合成 work area（物理 px・resolver T-R 群と同流儀）。
pub(super) const WA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 1920,
    bottom: 1040,
};

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
        format!("charset,UTF-8\ndpi,96\n{extra_lines}"),
    )
    .expect("balloon descript");
    fs::copy(
        balloon_root().join("balloons0.png"),
        balloon_dir.join("balloons0.png"),
    )
    .expect("balloons0.png 複写");
    balloon_dir
}

/// 宣言 DPI 付きの最小合成ゴーストを一時ディレクトリへ組む（テスト補助）。
///
/// `prepare_never_reads_or_writes_ghost_dat` の合成ゴーストと同型（emo2 の実 PNG を
/// 複写した最小 shell）に、`seriko.dpi`／balloon `dpi` の宣言を足したもの。
/// 返り値は `(ghost_root, balloon_root)`。
///
/// 注意: 合成 shell の `surfaces.txt` は単一 overlay ゆえ、scope0 は emo2 実測と同じ
/// 434×687 になるが scope1（surface10 単体）は emo2 実 shell の合成寸（336×400）とは
/// 異なる——本補助を使う檻は scope0 と balloon を期待値の錨に用いる。
///
/// 合成バルーンへ複写するのは本体側 `balloons0.png` の 1 枚のみ（相方側 `balloonk*` を
/// 置かない）。ゆえに全 scope の連鎖が本体側系列へ収束し、バルーン寸は scope に依らず
/// 同一になる——本補助を使う檻は同時に要件 3.7（`balloonk*` 不在時の後方互換）の錨でもある。
///
/// `balloon_windowposition`（task 4.3）: `Some((x, y))` ならバルーン既定設定へ
/// `windowposition.x`/`.y` を宣言する（面別上書き `balloons0s.txt` は置かないので、
/// 確定値はこの基層のみが供給する）。`None` なら**どの層にも `windowposition` が無い**
/// ＝要件 3.4 の「数値指定なし」検体になる。
pub(super) fn synth_declared_dpi_ghost(
    root: &TempDir,
    shell_dpi: &str,
    balloon_dpi: &str,
    balloon_windowposition: Option<(i32, i32)>,
) -> (PathBuf, PathBuf) {
    let ghost_master = root.path().join("ghost").join("master");
    let shell_master = root.path().join("shell").join("master");
    let balloon_dir = root.path().join("balloon-declared");
    fs::create_dir_all(&ghost_master).expect("create ghost/master");
    fs::create_dir_all(&shell_master).expect("create shell/master");
    fs::create_dir_all(&balloon_dir).expect("create balloon dir");
    fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,えも\nsakura.name,むらさき\nkero.name,エモ\n",
    )
    .expect("ghost descript");
    fs::write(
        shell_master.join("descript.txt"),
        format!(
            "charset,UTF-8\nseriko.dpi,{shell_dpi}\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,0\nkero.defaultx,0\nsakura.balloon.alignment,left\nkero.balloon.alignment,right\n"
        ),
    )
    .expect("shell descript");
    fs::write(
        shell_master.join("surfaces.txt"),
        "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface10\n{\nelement0,overlay,surface10.png,0,0\n}\n",
    )
    .expect("surfaces.txt");
    for png in ["surface0.png", "surface10.png"] {
        fs::copy(
            emo2_root().join("shell/master").join(png),
            shell_master.join(png),
        )
        .unwrap_or_else(|e| panic!("{png} 複写: {e}"));
    }
    let windowposition = match balloon_windowposition {
        Some((x, y)) => format!("windowposition.x,{x}\nwindowposition.y,{y}\n"),
        None => String::new(),
    };
    fs::write(
        balloon_dir.join("descript.txt"),
        format!("charset,UTF-8\ndpi,{balloon_dpi}\n{windowposition}"),
    )
    .expect("balloon descript");
    fs::copy(
        balloon_root().join("balloons0.png"),
        balloon_dir.join("balloons0.png"),
    )
    .expect("balloons0.png 複写");
    (root.path().to_path_buf(), balloon_dir)
}
