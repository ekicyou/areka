//! 実行ファイルの入口が「どの流れへ何を出し、どの終了コードで終わるか」を守る
//! （設計「入口 / cli」の 2 つ目の箇条書き・「Error Handling / 見張り」・要件 6.12）。
//!
//! 在中テストからは `cli::run` の戻り値までしか見えない。**標準出力と標準エラーの
//! 選び分け**と、**戻り値を終了コードへ写す対応表**は `main.rs` の 3 行にあり、
//! 実行ファイルを起こして初めて観測できる。どちらも壊しても在中テストは全部緑のまま
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
//! この線を引く限り、事例はタスク 6.2・6.3 が中身を入れた後も書き換えずに生き残る。
//! 失敗の腕（[`catalog_with_a_missing_snapshot_writes_nothing_to_stdout_and_exits_one`]）が
//! その例で、今は「まだ中身が繋がっていない」で、6.2 の後は「スナップショットが
//! 読めない」で、同じ主張を満たす。だから**失敗の本文の言い回しは主張しない**。
//!
//! タスク 6.1 の時代はここに「副手続きが**成功する**こと」も禁じられていた。8 つが
//! すべて `Err` を返す間は成功の腕へ到達する術が無く、書けば必ずスナップショットか
//! repo の木に寄りかかったからである。4 つが繋がった今は `candidates` が
//! `crates/**/*.rs` だけを読んで成功するので、上の 3 つを 1 つも踏まずに成功の腕を
//! 閉じられる（[`candidates_writes_its_result_to_stdout_and_exits_zero`]）。
//!
//! 事例はファイルを 1 つも作らず、一時ディレクトリも使わない（設計 File Structure Plan）。

use std::process::{Command, Output};

/// 使い方の本文の 1 行目（`cli::usage` の実装を引かず、独立した文字列で書く）。
const USAGE_FIRST_LINE: &str = "使い方: cargo run -p ukadoc-survey -- <副手続き>";

/// スナップショットの場所を指す環境変数（要件 9.7・`AREKA_` 冠）。
const SNAPSHOT_ENV: &str = "AREKA_UKADOC_SNAPSHOT";

/// 実在しない場所の固定の綴り。一時ディレクトリを引かないので、
/// ワークスペースの一時パス検査（`log-capture-kit`）の数え上げを揺らさない。
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
fn candidates_writes_its_result_to_stdout_and_exits_zero() {
    // 成功の腕（`Outcome::Done` → 終了コード 0）を守る唯一の事例。タスク 6.1 の時代は
    // 8 つの副手続きがすべて `Err` を返したので、この腕へはどんな入力からも到達でき
    // なかった（先送りではなく不可能）。4 つが繋がった今なら閉じられる。
    //
    // 選んだのは `candidates` である。上の 3 つの線を 1 つも踏まないのはこれだけで、
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
        "成功なのに終了コードが 0 でない
標準出力: {stdout}
標準エラー: {stderr}"
    );
    assert!(
        !stdout.is_empty(),
        "成功したのに標準出力へ結果が出ていない（標準エラー: {stderr}）"
    );
    assert_eq!(stderr, "", "成功したのに標準エラーへ何か出ている: {stderr}");
}

#[test]
fn catalog_with_a_missing_snapshot_writes_nothing_to_stdout_and_exits_one() {
    // 失敗の腕（終了コード 1）を守る唯一の事例。ここを空けておくと、失敗の本文を
    // 標準出力へ出す壊し方が誰にも気づかれずに通る。
    //
    // 環境変数は子プロセスにだけ渡す（`Command::env`）。同一プロセスの環境を書き換える
    // わけではないので、設計 Testing Strategy 19 の禁止（`set_var`）には当たらない。
    // 指す先は実在しない固定の綴りで、一時ディレクトリは使わない。
    //
    // 今は環境変数まで読まずに「まだ中身が繋がっていない」で失敗する（飾りの指定）。
    // タスク 6.2 が中身を入れると、この指定が効いて「スナップショットが読めない」で
    // 失敗する。どちらの時代でも下の 3 つは同じなので、この事例は書き換わらない。
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
        "失敗なのに終了コードが 1 でない
標準出力: {stdout}
標準エラー: {stderr}"
    );
    assert_eq!(stdout, "", "失敗の本文が標準出力へ出ている: {stdout}");
    assert!(
        !stderr.is_empty(),
        "黙って失敗している（標準エラーに本文が無い）"
    );
}
