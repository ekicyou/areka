//! 先送りした正典語彙が、本フィーチャーの wintf 側の本番コードに 1 つも入り込んで
//! いないことを機械的に確かめるテスト（要件 8.3／8.4／8.5、および重なり順のグループ機構の
//! 要件 10.4／11.4）。
//!
//! # なぜ文章ではなく走査で確かめるのか
//!
//! 要件 8.3〜8.5 は「実装しない」「発生させない」「変更しない」という**不在**の主張である。
//! 不在は書き足された瞬間に静かに崩れるので、レビュー時の目視や設計文書の記述では守れない。
//! そこで本テストは、本フィーチャーが持つ本番ファイルの中身を読み、先送り語彙に対応する
//! 語が 1 つも現れないことを毎回の `cargo test` で確かめる。
//!
//! # 走査の対象（なぜこの 8 ファイルなのか）
//!
//! 要件 8.3〜8.5（ペア機構）と要件 11.4（グループ機構）が言うのは「**本フィーチャーに
//! おいて**足さない」ことである。リポジトリ全体を
//! 走査すると、別フィーチャーが正当に持っている窓の一般機構（`wintf` の窓スタイル層など）まで
//! 拾ってしまい、主張が「areka 全体が最小化を実装していない」という別物へすり替わる。
//! よって対象は**2 つの機構が全部を書いたファイル**に限る——`zorder_pair` 系の本番
//! ファイル 5 本と、`zorder_group` 系の本番ファイル 3 本である。areka 側（ペア機構の 2 本と
//! グループ機構の 4 本）は兄弟の `spawn_zorder_pair_deferred_tests.rs` が同じ形で受け持つ。
//!
//! 新設ファイルが走査から漏れないことは、
//! `the_scanned_roster_covers_every_zorder_production_source_in_this_crate` が
//! 実在するファイルと名簿の両方向で見張る（要件 10.4）。
//!
//! 本フィーチャーが共有ファイルへ足した継ぎ目は、`window_proc` の非活性化の枝、`api.rs` の
//! 走査ラッパ（`get_window_above`／`get_window_below`／`is_window_visible`）と帯の所属を
//! 読む `is_window_always_on_top`、areka の `main.rs` の結線 1 行である。それらのファイルは
//! 他フィーチャーの持ち物でもあるため全文走査の対象には入れない——他所の追記で赤くなると、
//! 本フィーチャーへの誤った告発になる。
//!
//! # `api.rs` に常時最前面の語が 1 つある（要件 8.3 の違反ではない）
//!
//! 上の継ぎ目のうち `is_window_always_on_top` だけは、下の走査の第 1 語である `topmost` を
//! **コード行に持つ**——`crates/wintf/src/api.rs:126` の
//! `Ok((ex_style as u32) & WS_EX_TOPMOST.0 != 0)` である。本フィーチャー以前の `api.rs` に
//! この語は 1 つも無く（`git show HEAD:crates/wintf/src/api.rs` を数えて 0 件）、
//! 持ち込んだのは本フィーチャーである。事実として明記しておく。
//!
//! それでも要件 8.3 の違反ではない。下の「`\v` を語として走査しない理由」が定めるとおり、
//! ここでいう「実装する」とは**窓を常時最前面の帯へ入れる**ことであり、`WS_EX_TOPMOST` を
//! 書く側の話である。`is_window_always_on_top` はその 1 ビットを**読むだけ**であり、しかも
//! 読む目的は「帯の中の窓を挿入位置に指さない」——すなわち**帯へ決して書かないため**で
//! ある（要件 8.1・design.md「挿入位置が常時最前面の窓だった場合」）。語による走査は
//! 読みと書きを区別できないので、この向きの取りこぼしは走査の既知の性質である。
//!
//! よって `api.rs` は走査の対象に**入れない**。入れれば、帯へ書かないための読み取りが
//! 「帯へ書いた」と告発されることになり、主張が裏返る。`api.rs` を走査対象一覧
//! （`PRODUCTION_FILES`）へ入れない判断と、走査語（`DEFERRED_NEEDLES`）の 9 語は、
//! グループ機構のファイルを足した後も変えない。帯へ**書く**側が対象のファイルへ
//! 入り込めば、下の走査がそのまま捕まえる。
//!
//! # `\v` を語として走査しない理由
//!
//! `\v`（常時最前面へ上げる指定）を「実装する」とは、窓を常時最前面の帯へ入れる何かを
//! 書くということである。それは必ず下の `topmost` の語として現れるので、常時最前面の語を
//! 走査すれば `\v` の実装も同時に捕まる。ソース中の 2 文字並び `\v` を探すのは
//! Rust のエスケープ表記と区別が付かず、意味を持たない。
//!
//! # 行コメントを除いてから探す
//!
//! 本番ファイルの doc コメントには「常時最前面にはしない」という**否定の説明**が書いてある
//! （それ自体が要件 8.1 の設計意図の記録である）。素の全文を探すとその説明で赤くなるため、
//! 行頭が `//` の行を落としてから探す。落とし過ぎ・落とし漏れが起きていないことは、
//! 下の 2 つの対照が示す——素の全文には語があり、コードだけの本文には無い、という
//! 食い違いを名指しで主張してある。

use std::path::PathBuf;

/// 重なり順の 2 機構が全部を書いた wintf 側の本番ファイル（`CARGO_MANIFEST_DIR` からの相対）。
///
/// 前半 5 本はペア機構（`ghost-window-zorder`）、後半 3 本はグループ機構
/// （`areka-P0-scope-zorder-pinning`）が新設したもの。下の
/// `the_scanned_roster_covers_every_zorder_production_source_in_this_crate` が
/// 「実在する `zorder_` 系の本番ファイルが 1 本残らずここに載っている」ことを機械で見張る。
const PRODUCTION_FILES: [&str; 8] = [
    "src/ecs/window/zorder_pair.rs",
    "src/ecs/window/zorder_pair_diag.rs",
    "src/ecs/window/zorder_pair_establish.rs",
    "src/ecs/window/zorder_pair_maintain.rs",
    "src/ecs/window/zorder_pair_sink.rs",
    "src/ecs/window/zorder_group.rs",
    "src/ecs/window/zorder_group_diag.rs",
    "src/ecs/window/zorder_group_maintain.rs",
];

/// 先送り語彙（小文字で保持し、走査も小文字化して行う）と、それが何の入口かの説明。
///
/// 兄弟の areka 側テスト（`spawn_zorder_pair_deferred_tests.rs`）と同じ表を持つ。
/// 片方だけを増やすと守りが片肺になるので、足すときは両方へ足すこと。
const DEFERRED_NEEDLES: [(&str, &str); 9] = [
    (
        "topmost",
        "常時最前面（`\\v`／`\\![set,windowstate,stayontop]` の実装面・要件 8.1／8.3）",
    ),
    ("stayontop", "`\\![set,windowstate,stayontop]`（要件 8.3）"),
    ("windowstate", "`\\![set,windowstate,...]`（要件 8.3）"),
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

fn source_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_source(relative: &str) -> String {
    let path = source_path(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("本番ファイルを読めなかった（{}）: {e}", path.display()))
}

/// 行頭が `//` の行（通常のコメント・doc コメント）を落とした本文。
///
/// 行末に付いたコメントは落とさない——落とすほど主張は弱くなるので、迷う側は
/// 「残して赤くする」へ倒す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 本文に現れた先送り語彙（説明つき）。
fn deferred_hits(code: &str) -> Vec<(&'static str, &'static str)> {
    let lowered = code.to_lowercase();
    DEFERRED_NEEDLES
        .iter()
        .filter(|(needle, _)| lowered.contains(*needle))
        .copied()
        .collect()
}

/// 先送り語彙は、本フィーチャーの wintf 側本番ファイルのコードに 1 つも無い。
#[test]
fn deferred_window_state_vocabulary_is_absent_from_this_features_wintf_sources() {
    let mut scanned_files = 0usize;
    let mut scanned_code_bytes = 0usize;

    for relative in PRODUCTION_FILES {
        let raw = read_source(relative);
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
            "{relative}: 先送りした語彙が本番コードに現れている（要件 8.3〜8.5）: {hits:?}"
        );

        scanned_files += 1;
        scanned_code_bytes += code.len();
    }

    // 走査そのものが行われたことを数で固定する（対象一覧が空・パスが違うで恒真にならない）。
    assert_eq!(
        scanned_files,
        PRODUCTION_FILES.len(),
        "対象ファイルを全部読めていない"
    );
    assert!(
        scanned_code_bytes > 20_000,
        "読んだコードが少なすぎる（走査が実体に届いていない）: {scanned_code_bytes} バイト"
    );
}

/// 走査の道具が実際に語を見つけられること、そしてコメントだけを落としていることを固定する。
///
/// 上のテストは「無い」を主張するので、道具が壊れていても緑になりうる。ここでは
/// ①合成した本文で 9 語すべてが検出されること、②本番ファイルの素の全文には
/// 常時最前面の語があり、コードだけの本文には無いこと、③コードだけの本文にコードが
/// 残っていること、の 3 点を名指しで確かめる。
#[test]
fn the_deferred_vocabulary_scan_can_actually_find_what_it_looks_for() {
    // ① 各語がそれぞれ独立に検出される（表のどれかが死んでいれば落ちる）。
    for (needle, why) in DEFERRED_NEEDLES {
        let sample = format!("let handle = SomeApi::{needle}(hwnd);");
        let hits = deferred_hits(&sample);
        assert!(
            hits.iter().any(|(n, _)| *n == needle),
            "語 `{needle}`（{why}）を検出できていない: {hits:?}"
        );
    }
    // 大文字表記でも見つかる（`HWND_TOPMOST`・`WS_EX_TOPMOST` の実際の綴り）。
    assert_eq!(
        deferred_hits("SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);")
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>(),
        vec!["topmost"],
        "大文字の綴りを取り逃がしている"
    );
    // 何も無い本文では 1 つも出ない（常に何かを返す道具ではない）。
    assert!(
        deferred_hits("let decision = decide_pair_fix(&observation);").is_empty(),
        "無関係な本文で語を検出している"
    );

    // ② 素の全文には常時最前面の語があり、コードだけの本文には無い。
    //    ——コメントを落とす処理が「何も落としていない」でも「全部落とした」でもないこと、
    //      かつ本番ファイルの語が説明文の中にしか無いことを、同時に示す対照である。
    let raw = read_source("src/ecs/window/zorder_pair.rs");
    assert!(
        raw.to_lowercase().contains("topmost"),
        "前提が崩れている: 素の全文には常時最前面の語（否定の説明）があるはず"
    );
    let code = code_only(&raw);
    assert!(
        !code.to_lowercase().contains("topmost"),
        "常時最前面の語が説明文以外に現れている（要件 8.1／8.3）"
    );

    // ③ コードだけの本文にコードが残っている（落とし過ぎていない）。
    let maintain = code_only(&read_source("src/ecs/window/zorder_pair_maintain.rs"));
    assert!(
        maintain.contains("SetWindowPosCommand::enqueue"),
        "コードだけの本文から実装が消えている（走査が中身を見ていない）"
    );
    assert!(
        maintain.contains("pub fn apply_zorder_pair_maintenance"),
        "コードだけの本文から維持系の定義が消えている"
    );
}

/// `src` 配下の `.rs` を再帰で集め、`CARGO_MANIFEST_DIR` からの相対パス（`/` 区切り）で返す。
fn all_source_files() -> Vec<String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
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
                    .strip_prefix(&manifest)
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

/// 重なり順の 2 機構（ペア・グループ）が全部を書いた wintf 側の本番ファイル
/// ＝ファイル名が `zorder_` で始まり、テストでもテスト専用の道具立てでもないもの。
fn zorder_production_sources() -> Vec<String> {
    all_source_files()
        .into_iter()
        .filter(|relative| {
            let name = relative.rsplit('/').next().unwrap_or(relative);
            name.starts_with("zorder_")
                && !name.ends_with("_tests.rs")
                && !name.ends_with("_test_support.rs")
        })
        .collect()
}

/// 走査対象の一覧が、実在する本番ファイルへ両方向で追随していることを固定する。
///
/// 上の 2 つのテストは `PRODUCTION_FILES` に**載っているものだけ**を読む。名簿が実物から
/// ずれても「無い」の主張はそのまま緑になるので、守りは静かに狭まる（新しい本番ファイルが
/// 生えても誰も赤くならない）。ここでは
/// ⑴ 実在する `zorder_` 系の本番ファイルが 1 本残らず名簿に載っていること、
/// ⑵ 名簿の各項目が実在すること、
/// の両方向を機械で確かめる。走査から外すのはテスト（`*_tests.rs`）と、`#[cfg(test)]` でしか
/// 結線されないテスト専用の道具立て（`*_test_support.rs`）だけである。
#[test]
fn the_scanned_roster_covers_every_zorder_production_source_in_this_crate() {
    let actual = zorder_production_sources();

    // ① 道具の較正: 既知の正例が挙がり、既知の偽例（テスト）は挙がらない。
    assert!(
        actual.contains(&"src/ecs/window/zorder_pair.rs".to_string()),
        "実物の走査が効いていない（既知の本番ファイルを見つけられない）: {actual:?}"
    );
    assert!(
        !actual.iter().any(|p| p.ends_with("_tests.rs")),
        "テストファイルを本番ファイルとして数えている: {actual:?}"
    );

    // ② 実物が名簿から漏れていない。
    let missing: Vec<&String> = actual
        .iter()
        .filter(|relative| !PRODUCTION_FILES.contains(&relative.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "本番ファイルが先送り語彙の走査対象に載っていない（名簿へ足すこと）: {missing:?}"
    );

    // ③ 名簿の項目が実在する（改名・移動で名簿だけが取り残されない）。
    for relative in PRODUCTION_FILES {
        assert!(
            source_path(relative).is_file(),
            "走査対象の一覧に実在しないファイルが載っている: {relative}"
        );
    }
}
