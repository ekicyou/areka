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

/// 割った `measure_template_tests` の 2 つの片割れ（配置の crate と本体）が共に使う走査器
/// （spec: placement-crate-split 要件 3.6・5.3）。写しを作らず、ここに 1 つだけ置く。
pub mod measure_template {
    /// 見張る綴り。
    ///
    /// 末尾の `(` を落としてあるのは、`use areka_parsers::shell::parse;` で名前だけ持ち込んで
    /// `parse(` と呼ぶ形も同じ 1 本で捕まえるため（`use` 行にもこの綴りが現れる）。
    pub const SHELL_PARSE: &str = "shell::parse";

    /// 行コメント・ブロックコメント・文字列リテラルを落とした本文。
    ///
    /// 説明文（`//!`・`///`・`//`）の中の綴りを拾って偽陽性にせず、文字列リテラルへ逃がした綴りも
    /// 数えないための下拵え。本関数が扱えるのはコメントと通常の文字列リテラルだけで、
    /// 二重引用符の文字リテラルと生文字列（`r"…"`・`r#"…"#`）は扱わない。これらが持ち込まれると
    /// その `"` を文字列の始まりと読み違え、以降の本文を丸ごと文字列の中身として飲み込む——
    /// つまり失敗の向きは「赤になる」ではなく**静かに見逃す**である。だから散文で戒めるのではなく、
    /// 走査の前に [`assert_no_blind_spot_literals`] でこの 2 形式が 0 件であることを機械で主張する。
    /// 綴りを拾えること・説明文の中の綴りを拾わないことは、各片割れの較正 2 本が自分の crate の
    /// 実物で確かめる。
    pub fn code_only(source: &str) -> String {
        let chars: Vec<char> = source.chars().collect();
        let mut out = String::with_capacity(source.len());
        let mut i = 0;
        // ブロックコメントの入れ子の深さ（Rust のブロックコメントは入れ子になる）。
        let mut block_depth = 0usize;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            if block_depth > 0 {
                if c == '/' && next == Some('*') {
                    block_depth += 1;
                    i += 2;
                } else if c == '*' && next == Some('/') {
                    block_depth -= 1;
                    i += 2;
                } else {
                    if c == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                continue;
            }
            if c == '/' && next == Some('/') {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if c == '/' && next == Some('*') {
                block_depth = 1;
                i += 2;
                continue;
            }
            if c == '"' {
                i += 1;
                while i < chars.len() {
                    match chars[i] {
                        '\\' => i += 2,
                        '"' => {
                            i += 1;
                            break;
                        }
                        '\n' => {
                            out.push('\n');
                            i += 1;
                        }
                        _ => i += 1,
                    }
                }
                continue;
            }
            out.push(c);
            i += 1;
        }
        out
    }

    /// 二重引用符の文字リテラルの綴り（この綴り自体を走査対象へ書かないよう、文字列で持つ）。
    const CHAR_QUOTE_LITERAL: &str = "'\"'";

    /// [`code_only`] の前提条件を生のソースに対して主張する。
    ///
    /// 走査器はコメントと通常の文字列リテラルしか落とせない。二重引用符の文字リテラルや生文字列が
    /// 混ざると本物の呼び出しを静かに見逃すので、見逃しが起こり得る形式が 0 件であることを
    /// 本文の判定の**前**に確かめる。走査対象のファイルは今日すべて、どちらの形式も 0 件である。
    pub fn assert_no_blind_spot_literals(raw: &str, file: &str) {
        assert!(
            !raw.contains(CHAR_QUOTE_LITERAL),
            "{file}: 二重引用符の文字リテラルが持ち込まれた。走査器はこの形式を扱えない\
             （本物の呼び出しを静かに見逃す）。こういうリテラルを持ち込む前に走査器を強くすること"
        );
        let bytes = raw.as_bytes();
        for i in 0..bytes.len() {
            // 生文字列の始まり＝識別子の途中でない `r` に `"` か `#` が続く形。
            if bytes[i] != b'r' {
                continue;
            }
            if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
                continue;
            }
            if matches!(bytes.get(i + 1), Some(b'"') | Some(b'#')) {
                panic!(
                    "{file}: 生文字列が持ち込まれた。走査器はこの形式を扱えない\
                     （本物の呼び出しを静かに見逃す）。こういうリテラルを持ち込む前に走査器を強くすること"
                );
            }
        }
    }
}

/// 割った `spawn_zorder_pair_deferred_tests` の 2 つの片割れ（配置の crate と本体）が共に使う
/// 語の表と走査器（spec: placement-crate-split 要件 3.6・5.3）。写しを作らず、ここに 1 つだけ置く。
/// [`measure_template`] の `code_only` とは中身が違う（こちらは `//` の行を落とすだけ）のでまとめない。
pub mod zorder_pair_deferred {
    /// 先送り語彙（小文字）と、それが何の入口かの説明。
    ///
    /// 最小化・タスクバーの扱いは別の spec（`tasktray-minimize` ほか）の持ち物で、まだ先送りの
    /// ままである。兄弟の wintf 側テスト（`crates/wintf/src/ecs/window/zorder_pair_deferred_vocabulary_tests.rs`）
    /// と同じ 2 つの表（この表と [`BAND_NEEDLES`]）を持つ。片方だけを増やすと守りが片肺になるので、
    /// どちらの表へ足すときも両方のファイル（wintf の兄弟とこのファイル）へ足すこと。
    pub const DEFERRED_NEEDLES: [(&str, &str); 6] = [
        (
            "minimize",
            "最小化（`\\![set,windowstate,minimize]`／`OnWindowStateMinimize`・要件 8.3／8.4／8.5）",
        ),
        ("iconic", "最小化状態の読み取り（要件 8.5）"),
        (
            "onwindowstate",
            "`OnWindowStateMinimize`／`OnWindowStateRestore`（要件 8.4）",
        ),
        (
            "onfullscreenapp",
            "`OnFullScreenAppMinimize`／`OnFullScreenAppRestore`（要件 8.4）",
        ),
        ("appwindow", "タスクバー露出の切り替え（要件 8.5）"),
        (
            "taskbar",
            "タスクバー・アプリ切り替え一覧の扱い（要件 8.5）",
        ),
    ];

    /// 帯の語（小文字）。常に手前を読むのは本体の受け口と取り出しの相のファイルだけであり
    /// （`stay-on-top-directives`）、重なり順の機構の本番ファイルには 1 つも無い。
    pub const BAND_NEEDLES: [(&str, &str); 3] = [
        (
            "topmost",
            "常に手前の帯（`\\v`／`\\![set,windowstate,stayontop]` の働き）",
        ),
        ("stayontop", "`\\![set,windowstate,stayontop]`"),
        ("windowstate", "`\\![set,windowstate,...]`"),
    ];

    /// 行頭が `//` の行（通常のコメント・doc コメント）を落とした本文。
    pub fn code_only(source: &str) -> String {
        source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// `manifest/src` 配下の `.rs` を再帰で集め、`manifest` からの相対パス（`/` 区切り）で返す。
    /// 各片割れは自分の crate の `CARGO_MANIFEST_DIR` を渡す（自分の木だけを歩く）。
    pub fn rust_sources_under(manifest: &std::path::Path) -> Vec<String> {
        let mut stack = vec![manifest.join("src")];
        let mut found = Vec::new();
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("ソースの木を辿れなかった（{}）: {e}", dir.display()));
            for entry in entries {
                let path = entry.expect("ディレクトリ項目を読めなかった").path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    let relative = path
                        .strip_prefix(manifest)
                        .expect("マニフェスト配下のはず")
                        .to_string_lossy()
                        .replace(std::path::MAIN_SEPARATOR, "/");
                    found.push(relative);
                }
            }
        }
        found.sort();
        found
    }

    /// 本文に現れる先送りの語と帯の語（小文字にして探す）。
    pub fn deferred_hits(code: &str) -> Vec<(&'static str, &'static str)> {
        let lowered = code.to_lowercase();
        DEFERRED_NEEDLES
            .iter()
            .chain(BAND_NEEDLES.iter())
            .filter(|(needle, _)| lowered.contains(*needle))
            .copied()
            .collect()
    }
}
