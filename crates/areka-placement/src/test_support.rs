//! 配置の外のテストも使う支え（`test-support` の機能か、配置自身のテストのときだけ立つ）。
//!
//! 頼るのは配置の型・`windows`・`wintf`・`areka_sylphya`・`areka_parsers`・`areka_emo_atlas`・std だけ。
//! `#[cfg(test)]` だけの項目と共有のテスト用の crate
//! （`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`）には頼らない——本体のテストを組むときの
//! 配置は `test` が立たず機能だけが立つので、頼ると組めなくなる（spec: placement-crate-split 要件 3.3・3.4）。
//! 見本のゴーストの入口は crate ごとの `sample_test_support`、配置のテストだけが使う一時フォルダと
//! 合成の検体は `shared_test_support` にある（本体のテストも使う合成の検体はここに置く）。ログの判定の補助は `log_capture_kit` を直に使う（要件 3.1）。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use wintf::ecs::WindowHandle;

use crate::resolver::{PointPx, RectPx, SizePx};

/// COM 初期化下でクロージャを実行する（measure が `WicDecoderArm` を要求する）。
pub fn with_com_initialized<F: FnOnce()>(f: F) {
    // 初期化に成功（S_OK・S_FALSE）したときだけ後始末する（RPC_E_CHANGED_MODE で
    // 他の初期化を壊さない）。
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    f();
    if initialized {
        unsafe {
            CoUninitialize();
        }
    }
}

#[cfg(test)]
#[path = "test_support_com_tests.rs"]
mod com_tests;

/// 共有の偽の保存先（アクターへ渡す分と、`load_scope` で観測する分が同じストアを指す）。
/// clone は同じストアと同じ commit の数を共有する。
#[derive(Clone)]
pub struct SharedFakeIo {
    store: Arc<FakePersistIo>,
    commits: Arc<AtomicUsize>,
}
impl SharedFakeIo {
    pub fn new(store: Arc<FakePersistIo>) -> Self {
        Self {
            store,
            commits: Arc::default(),
        }
    }
    /// この口（と clone）を通った commit の回数。
    pub fn commit_count(&self) -> usize {
        self.commits.load(Ordering::SeqCst)
    }
}
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.store.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.commits.fetch_add(1, Ordering::SeqCst);
        self.store.commit(path, content)
    }
}

/// `(鍵, 値)` の並びから、descript を `areka_parsers::kv::parse_kv` で読んだのと同じ形の表を組む。
pub fn kv(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム）。
pub fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
    }
}

pub fn point(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

/// 矩形 (pos, size) が area へ 4 辺内包されているか（right/bottom は排他側）。
///
/// 右下辺の算出は `saturating_add`——極値入力（`i32::MAX` 位置）を与える檻でも、
/// **判定道具の側が先に溢れて**本体の欠陥を隠すことがあってはならない。
pub fn contained(pos: PointPx, size: SizePx, area: RectPx) -> bool {
    pos.x >= area.left
        && pos.y >= area.top
        && pos.x.saturating_add(size.w) <= area.right
        && pos.y.saturating_add(size.h) <= area.bottom
}

/// 決定論テスト用の合成 work area（物理 px・resolver T-R 群と同流儀）。
pub const WA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 1920,
    bottom: 1040,
};

/// キーワードのバルーン（`windowposition.x,center`・`windowposition.y,-20`）の descript の本文。
/// [`synth_declared_dpi_ghost`] が組んだバルーンの `descript.txt` へ書いて使う。
pub const KEYWORD_BALLOON_DESCRIPT: &str =
    "charset,UTF-8\ndpi,96\nwindowposition.x,center\nwindowposition.y,-20\n";

/// 宣言 DPI 付きの最小合成ゴーストを一時ディレクトリへ組む（テスト補助）。
///
/// emo2 の実 PNG を複写した最小 shell に、`seriko.dpi`／balloon `dpi` の宣言を足したもの。
/// 準備のテストが共に使い、descript だけを差し替えて別の形のゴーストにもする。
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
/// `balloon_windowposition`: `Some((x, y))` ならバルーン既定設定へ
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
pub(crate) const EMILY_SHAPED_GHOST_DESCRIPT: &str =
    "charset,UTF-8\nname,かたち\nsakura.name,ひとり\nkero.name,ふたり\nchar2.name,さんにん\n";

/// えみりと同じ形のシェルの `descript.txt`（同上）。`char2.balloon.offsetx`・
/// `char3.seriko.alignmenttodesktop`・`char4.seriko.alignmenttodesktop` を持つ。
pub(crate) const EMILY_SHAPED_SHELL_DESCRIPT: &str = "charset,UTF-8\nseriko.use_self_alpha,1\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,0\nkero.defaultx,0\nchar2.balloon.offsetx,0\nchar3.seriko.alignmenttodesktop,free\nchar4.seriko.alignmenttodesktop,free\n";

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

/// 採寸の読み手のテスト（配置の `measure_first_frame_tests` と本体の `measure_bake_log_tests`）が
/// `MemoryDecoder` へ入れる 2×2 の絵と、2 コマの動く絵。
pub mod animated_image {
    use std::num::NonZeroU32;

    use areka_emo_atlas::{AnimatedImage, AnimationFrame, AnimationInfo, DecodedImage, LoopCount};

    /// 2×2 の α つきの絵（画素は全部 `px`）。
    pub fn image(px: [u8; 4]) -> DecodedImage {
        DecodedImage {
            width: 2,
            height: 2,
            stride: 8,
            bgra: [px; 4].concat(),
            has_alpha: true,
        }
    }

    /// 2×2・2 コマの動く絵の見出し。
    pub fn info() -> AnimationInfo {
        AnimationInfo {
            width: 2,
            height: 2,
            frame_count: 2,
        }
    }

    /// 2 コマの動く絵の全コマ（コマの絵は `px0`・`px1`）。
    pub fn frames(px0: [u8; 4], px1: [u8; 4]) -> AnimatedImage {
        AnimatedImage {
            frames: [px0, px1]
                .iter()
                .map(|&px| AnimationFrame {
                    image: image(px),
                    delay_ms: 100,
                })
                .collect(),
            loop_count: LoopCount::Finite(NonZeroU32::new(1).unwrap()),
        }
    }
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
    /// 二重引用符の文字リテラル（`'"'`・`'\"'`）と生文字列（`r"…"`・`r#"…"#`・`br"…"`・`cr"…"`）は扱わない。これらが持ち込まれると
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

    /// 二重引用符の文字リテラルの綴り 2 通り（素の `"` と、逆斜線で逃がした `\"`。この綴り自体を
    /// 走査対象へ書かないよう、文字列で持つ）。
    const CHAR_QUOTE_LITERALS: [&str; 2] = ["'\"'", "'\\\"'"];

    /// [`code_only`] の前提条件を生のソースに対して主張する。
    ///
    /// 走査器はコメントと通常の文字列リテラルしか落とせない。二重引用符の文字リテラルや生文字列が
    /// 混ざると本物の呼び出しを静かに見逃すので、見逃しが起こり得る形式が 0 件であることを
    /// 本文の判定の**前**に確かめる。走査対象のファイルは今日すべて、どちらの形式も 0 件である。
    pub fn assert_no_blind_spot_literals(raw: &str, file: &str) {
        assert!(
            !CHAR_QUOTE_LITERALS.iter().any(|form| raw.contains(form)),
            "{file}: 二重引用符の文字リテラルが持ち込まれた。走査器はこの形式を扱えない\
             （本物の呼び出しを静かに見逃す）。こういうリテラルを持ち込む前に走査器を強くすること"
        );
        let bytes = raw.as_bytes();
        for i in 0..bytes.len() {
            // 生文字列の始まり＝識別子の途中でない `r`（`br`・`cr` の接頭辞も含む）に `"` か `#` が続く形。
            if bytes[i] != b'r' {
                continue;
            }
            let start = if i > 0 && matches!(bytes[i - 1], b'b' | b'c') {
                i - 1
            } else {
                i
            };
            if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
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
/// 名簿の走査（[`assert_vocabulary_absent`]・[`assert_roster_covers`]）と探針の自己検査
/// （[`assert_needles_are_detected`]）もここにあり、片割れは名簿と実物での較正だけを持つ。
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

    /// `manifest/src` 配下の重なり順の本番ファイル＝ファイル名が `zorder_` で始まり、テストでも
    /// テスト専用の道具立て（`*_test_support.rs`）でもないもの（`manifest` からの相対パス）。
    pub fn zorder_production_sources(manifest: &std::path::Path) -> Vec<String> {
        rust_sources_under(manifest)
            .into_iter()
            .filter(|relative| {
                let name = relative.rsplit('/').next().unwrap_or(relative);
                name.starts_with("zorder_")
                    && !name.ends_with("_tests.rs")
                    && !name.ends_with("_test_support.rs")
            })
            .collect()
    }

    /// `manifest`（`CARGO_MANIFEST_DIR`）からの相対で本番ファイルを読む。読めなければ panic する。
    pub fn read_source(manifest: &std::path::Path, relative: &str) -> String {
        let path = manifest.join(relative);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("本番ファイルを読めなかった（{}）: {e}", path.display()))
    }

    /// 名簿の各ファイルを実際に読み、コメントを落とした本文に先送りの語も帯の語も無いことを主張する。
    /// 帯の語を持つ持ち主は `band_owners`（失敗の文言にだけ使う）。
    pub fn assert_vocabulary_absent(
        manifest: &std::path::Path,
        roster: &[&str],
        band_owners: &str,
    ) {
        let mut scanned_code_bytes = 0usize;
        for relative in roster {
            let raw = read_source(manifest, relative);
            assert!(
                raw.len() > 1_000,
                "{relative}: 本番ファイルにしては短すぎる（読み違えの疑い）: {} バイト",
                raw.len()
            );
            let code = code_only(&raw);
            assert!(
                !code.trim().is_empty(),
                "{relative}: コメントを落としたら本文が空になった（走査が空振りしている）"
            );
            let hits = deferred_hits(&code);
            assert!(
                hits.is_empty(),
                "{relative}: 先送りの語か帯の語が本番コードに現れている（帯の語を持つのは {band_owners} だけ）: {hits:?}"
            );
            scanned_code_bytes += code.len();
        }
        assert!(
            scanned_code_bytes > 10_000,
            "読んだコードが少なすぎる（走査が実体に届いていない）: {scanned_code_bytes} バイト"
        );
    }

    /// 探針の自己検査: 表の語はどれも、その語を含む合成の 1 行から見つかる。
    pub fn assert_needles_are_detected() {
        for (needle, why) in DEFERRED_NEEDLES.into_iter().chain(BAND_NEEDLES) {
            let sample = format!("let handle = SomeApi::{needle}(hwnd);");
            let hits = deferred_hits(&sample);
            assert!(
                hits.iter().any(|(n, _)| *n == needle),
                "語 `{needle}`（{why}）を検出できていない: {hits:?}"
            );
        }
    }

    /// 名簿が実物へ両方向で追随していること: ⑴ 実在する本番ファイルが名簿から漏れていない、
    /// ⑵ 名簿の各項目が実在する。`actual` は [`zorder_production_sources`] の結果。
    pub fn assert_roster_covers(manifest: &std::path::Path, roster: &[&str], actual: &[String]) {
        let missing: Vec<&String> = actual
            .iter()
            .filter(|relative| !roster.contains(&relative.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "本番ファイルが先送り語彙の走査対象に載っていない（名簿へ足すこと）: {missing:?}"
        );
        for relative in roster {
            assert!(
                manifest.join(relative).is_file(),
                "走査対象の一覧に実在しないファイルが載っている: {relative}"
            );
        }
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
