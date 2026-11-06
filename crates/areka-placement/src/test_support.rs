//! 配置の外のテストも使う支え（`test-support` の機能か、配置自身のテストのときだけ立つ）。
//!
//! 頼るのは配置の型・`windows`・std だけ。`#[cfg(test)]` だけの項目と共有のテスト用の crate
//! （`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`）には頼らない——本体のテストを組むときの
//! 配置は `test` が立たず機能だけが立つので、頼ると組めなくなる（spec: placement-crate-split 要件 3.3・3.4）。
//! 見本のゴーストの入口は crate ごとの `sample_test_support`、一時フォルダと合成の検体は
//! `shared_test_support` にある。ログの判定の補助は `log_capture_kit` を直に使う（要件 3.1）。

use std::fs;
use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use crate::resolver::RectPx;

/// COM 初期化下でクロージャを実行する（measure が `WicDecoderArm` を要求・
/// measure.rs テストと同一パターン）。
pub fn with_com_initialized<F: FnOnce()>(f: F) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    f();
    unsafe {
        CoUninitialize();
    }
}

/// 決定論テスト用の合成 work area（物理 px・resolver T-R 群と同流儀）。
pub const WA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 1920,
    bottom: 1040,
};

/// 宣言 DPI 付きの最小合成ゴーストを一時ディレクトリへ組む（テスト補助）。
///
/// `prepare_never_reads_or_writes_ghost_dat` の合成ゴーストと同型（emo2 の実 PNG を
/// 複写した最小 shell）に、`seriko.dpi`／balloon `dpi` の宣言を足したもの。
/// 返り値は `(ghost_root, balloon_root)`。`root` の下に組み、絵は `emo2_root`（見本のゴースト）と
/// `balloon_root`（そのバルーン）から写す（道は呼び手が自分の crate の `sample_test_support` から渡す）。
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
pub fn synth_declared_dpi_ghost(
    root: &Path,
    emo2_root: &Path,
    balloon_root: &Path,
    shell_dpi: &str,
    balloon_dpi: &str,
    balloon_windowposition: Option<(i32, i32)>,
) -> (PathBuf, PathBuf) {
    let ghost_master = root.join("ghost").join("master");
    let shell_master = root.join("shell").join("master");
    let balloon_dir = root.join("balloon-declared");
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
            "charset,UTF-8\nseriko.dpi,{shell_dpi}\nseriko.use_self_alpha,1\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,0\nkero.defaultx,0\nsakura.balloon.alignment,left\nkero.balloon.alignment,right\n"
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
            emo2_root.join("shell/master").join(png),
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
        format!("charset,UTF-8\ndpi,{balloon_dpi}\nuse_self_alpha,1\n{windowposition}"),
    )
    .expect("balloon descript");
    fs::copy(
        balloon_root.join("balloons0.png"),
        balloon_dir.join("balloons0.png"),
    )
    .expect("balloons0.png 複写");
    (root.to_path_buf(), balloon_dir)
}

/// えみり（`emily4`）と同じ形のゴーストの `descript.txt`（spec: areka-P0-emily-ghost-verification
/// 要件 6.7）。`kero.name` に加えて `char2.name` を持つ。名前は架空で、えみりの文面は写さない。
pub const EMILY_SHAPED_GHOST_DESCRIPT: &str =
    "charset,UTF-8\nname,かたち\nsakura.name,ひとり\nkero.name,ふたり\nchar2.name,さんにん\n";

/// えみりと同じ形のシェルの `descript.txt`（同上）。`char2.balloon.offsetx`・
/// `char3.seriko.alignmenttodesktop`・`char4.seriko.alignmenttodesktop` を持つ。
pub const EMILY_SHAPED_SHELL_DESCRIPT: &str = "charset,UTF-8\nseriko.use_self_alpha,1\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,0\nkero.defaultx,0\nchar2.balloon.offsetx,0\nchar3.seriko.alignmenttodesktop,free\nchar4.seriko.alignmenttodesktop,free\n";

/// えみりの形の固定入力から配置の設定が返す scope の全部（窓を手で書かずに作るための元）。
pub fn emily_shaped_scopes() -> Vec<usize> {
    crate::config::build_placement_config(
        &areka_parsers::kv::parse_kv(EMILY_SHAPED_GHOST_DESCRIPT),
        &areka_parsers::kv::parse_kv(EMILY_SHAPED_SHELL_DESCRIPT),
    )
    .scopes
    .into_keys()
    .collect()
}
