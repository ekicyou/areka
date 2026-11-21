//! 実行ファイルの入口が「どの流れへ何を出し、どの終了コードで終わるか」を守る。
//!
//! 在中テストからは `cli::run` の戻り値までしか見えない。**標準エラーへの写し**と
//! **戻り値を終了コードへ写す対応表**は `main.rs` にあり（標準出力へ書くのは `cli` の
//! 側）、実行ファイルを起こして初めて観測できる。どちらも壊しても在中テストは全部緑のまま
//! 通る——部品を釘付けしても入口の配線は別に守る必要がある。
//!
//! # ここへ足してよい事例の決まり
//!
//! 「副手続きの中身に入らないこと」ではなく、**外の世界の当たり外れに寄りかからない
//! こと**が守るべき線である。次の 3 つに 1 つでも寄りかかる事例は置かない。
//!
//! - 本物のスナップショットが在ること
//! - repo の中身（カタログ・台帳・報告＝`doc/ukadoc-coverage/`）が特定の状態であること
//! - 副手続きの出力の**中身**が特定の値であること（版面はそれぞれの在中テストの持ち物）
//!
//! 2 つ目が禁じるのは「中身がこうである」と言うことであって、木を**見る**ことでは
//! ない。走行の前と後を見比べて「同じである」とだけ言う事例は、木がどんな状態でも
//! 成り立つので、この線を踏まない（失敗の事例の木の見比べを見よ）。
//!
//! 失敗の腕（[`catalog_with_a_missing_snapshot_fails_quietly_and_leaves_the_tree_untouched`]）は
//! 指す先が実在しない固定の綴りなので本物のスナップショットに寄りかからず、**失敗の
//! 本文の言い回しは主張しない**（読めない理由の書き方が変わっても書き換えない）。
//! 成功の腕（[`candidates_writes_its_result_to_stdout_and_exits_zero`]）は `candidates`
//! が `crates/**/*.rs` だけを読んで成功するので、上の 3 つの線を 1 つも踏まずに閉じられる。
//!
//! 事例はファイルを 1 つも作らず、一時ディレクトリも使わない。

use std::process::{Command, Output};

/// 使い方の本文の 1 行目（`cli::usage` の実装を引かず、独立した文字列で書く）。
const USAGE_FIRST_LINE: &str = "使い方: cargo run -p ukadoc-survey -- <副手続き>";

/// スナップショットの場所を指す環境変数（`AREKA_` 冠）。
const SNAPSHOT_ENV: &str = "AREKA_UKADOC_SNAPSHOT";

/// 実在しない場所の固定の綴り（読むだけなので一時ディレクトリは作らない）。
const MISSING_SNAPSHOT: &str = "/nonexistent/ukadoc-survey-review/index.json";

/// 実行ファイルを 1 度起こして、両方の流れと終了コードを受け取る。
fn run_binary(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ukadoc-survey"))
        .args(args)
        .output()
        .expect("実行ファイルを起こせなかった")
}

/// 引数の誤りの経路が満たすべき 3 つを一度に確かめる。
fn assert_usage_error(args: &[&str], expected_in_stderr: &[&str]) {
    let output = run_binary(args);
    let stdout = String::from_utf8(output.stdout).expect("標準出力が UTF-8 でない");
    let stderr = String::from_utf8(output.stderr).expect("標準エラーが UTF-8 でない");

    assert_eq!(
        output.status.code(),
        Some(2),
        "引数の誤りなのに終了コードが 2 でない（引数 {args:?}）\n標準出力: {stdout}\n標準エラー: {stderr}"
    );
    assert_eq!(
        stdout, "",
        "標準出力に結果以外が出ている（引数 {args:?}）: {stdout}"
    );
    for needle in expected_in_stderr {
        assert!(
            stderr.contains(needle),
            "標準エラーに {needle} が無い（引数 {args:?}）: {stderr}"
        );
    }
}

#[test]
fn no_argument_writes_the_usage_text_to_stderr_and_exits_two() {
    assert_usage_error(&[], &[USAGE_FIRST_LINE]);
}

#[test]
fn an_unknown_subcommand_echoes_the_typed_name_to_stderr_and_exits_two() {
    assert_usage_error(&["nosuchthing"], &["nosuchthing", USAGE_FIRST_LINE]);
}

#[test]
fn priority_apply_takes_no_arguments_and_exits_two_when_given_one() {
    // `priority-apply` も引数を取らない。ここは**振り分けの手前で断る**経路だけを見るので、
    // 書き戻しそのものは走らない——走れば repo の台帳を書き換える。
    assert_usage_error(
        &["priority-apply", "shiori"],
        &["priority-apply", "shiori", USAGE_FIRST_LINE],
    );
}

#[test]
fn candidates_writes_its_result_to_stdout_and_exits_zero() {
    // 成功の腕（`Outcome::Done` → 終了コード 0）を守る唯一の事例。
    //
    // 選んだのは `candidates` である。冒頭の 3 つの線を 1 つも踏まないのはこれだけで、
    // 理由は「読むのが `crates/**/*.rs` だけ」だから——スナップショットも
    // `doc/ukadoc-coverage/`（カタログ・台帳・報告）も 1 バイトも読まない。走査の先は
    // この実行ファイル自身が組み上がっている以上どの作業ツリーにも在る。
    //
    // 主張は「終了コード 0」「標準出力に結果がある」「標準エラーは静か」の 3 つで、
    // **本文の中身には触れない**。候補が何件見つかるかは repo の中身しだいなので、
    // そこへ寄りかかると事例が repo の状態を見ることになる。件数の見出しだけは
    // 0 件でも必ず出る（`cli::inspect::render_candidates`）ので、標準出力が空でない
    // ことは repo に依らずに主張できる。
    let output = run_binary(&["candidates"]);
    let stdout = String::from_utf8(output.stdout).expect("標準出力が UTF-8 でない");
    let stderr = String::from_utf8(output.stderr).expect("標準エラーが UTF-8 でない");

    assert_eq!(
        output.status.code(),
        Some(0),
        "成功なのに終了コードが 0 でない\n標準出力: {stdout}\n標準エラー: {stderr}"
    );
    assert!(
        !stdout.is_empty(),
        "成功したのに標準出力へ結果が出ていない（標準エラー: {stderr}）"
    );
    assert_eq!(stderr, "", "成功したのに標準エラーへ何か出ている: {stderr}");
}

// ---------------------------------------------------------------------------
// 失敗の腕: 標準出力は空・標準エラーに本文・終了コード 1・木は走行の前後で同じ
// ---------------------------------------------------------------------------

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// 生成物を置くディレクトリ（`io::paths` を引かず、独立した綴りで書く）。
const COVERAGE_DIR: &str = "doc/ukadoc-coverage";

/// ワークスペース根。この crate の manifest の 2 段上（`io::paths` と同じ求め方）。
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<クレート>/ の 2 段上が取れない")
        .to_path_buf()
}

/// 中身の指紋。バイト列が 1 つでも違えば別の値になる。
fn digest(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// `doc/ukadoc-coverage/` の今の姿を、並び順の決まった一覧にする。
///
/// ファイルは「置き場からの相対の綴りと中身の指紋」、ディレクトリは「綴りの末尾に
/// `/` を付けたものと 0」で表す。ディレクトリも数えるのは、失敗の途中で**空の置き場を
/// 掘る**壊し方を見るためである（中身が 1 つも増えないので、ファイルだけ見ると素通りする）。
fn coverage_tree() -> Vec<(String, u64)> {
    let root = workspace_root().join(COVERAGE_DIR);
    let mut out: Vec<(String, u64)> = Vec::new();
    collect_tree(&root, &root, &mut out);
    out.sort();
    out
}

/// `coverage_tree` の下働き。置き場が無ければ何も足さない（＝空の一覧）。
fn collect_tree(root: &Path, dir: &Path, out: &mut Vec<(String, u64)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
        if path.is_dir() {
            out.push((format!("{name}/"), 0));
            collect_tree(root, &path, out);
        } else {
            let bytes = std::fs::read(&path).expect("読めるはずのファイルが読めない");
            out.push((name, digest(&bytes)));
        }
    }
}

#[test]
fn catalog_with_a_missing_snapshot_fails_quietly_and_leaves_the_tree_untouched() {
    // 失敗の腕（終了コード 1）を守る唯一の事例。失敗の本文を標準出力へ出す壊し方と、
    // 読み終える前に書き始める壊し方（終了コード・標準出力・標準エラーだけでは素通りする）を、1 回の走行で
    // 両方捕まえる。
    //
    // 環境変数は子プロセスにだけ渡す（`Command::env`）。同一プロセスの環境を書き換える
    // わけではないので、テストで `set_var` を使わない決まりには当たらない。
    // `catalog` はこの指定を読んで、指す先が無いことで失敗する。指定を外すと本物の
    // スナップショットを読みに行くので、外してはならない。
    //
    // 木の見比べが冒頭の決まりに触れないのは、主張が「前と後が同じ」だけで、中身が
    // どうであるかを 1 つも言わないからである。木が無い作業ツリーでも成り立つ（前も後も
    // 空。失敗の途中で空の置き場を掘れば後だけが増えるので、どちらの作業ツリーでも赤く
    // なる）。ファイルは作らず**読むだけ**である。
    let before = coverage_tree();

    let output = Command::new(env!("CARGO_BIN_EXE_ukadoc-survey"))
        .arg("catalog")
        .env(SNAPSHOT_ENV, MISSING_SNAPSHOT)
        .output()
        .expect("実行ファイルを起こせなかった");
    let stdout = String::from_utf8(output.stdout).expect("標準出力が UTF-8 でない");
    let stderr = String::from_utf8(output.stderr).expect("標準エラーが UTF-8 でない");

    assert_eq!(
        output.status.code(),
        Some(1),
        "失敗なのに終了コードが 1 でない\n標準出力: {stdout}\n標準エラー: {stderr}"
    );
    assert_eq!(stdout, "", "失敗の本文が標準出力へ出ている: {stdout}");
    assert!(
        !stderr.is_empty(),
        "黙って失敗している（標準エラーに本文が無い）"
    );
    assert_eq!(
        coverage_tree(),
        before,
        "失敗した走行が {COVERAGE_DIR} を書き換えた（要件 1.8）"
    );
}
