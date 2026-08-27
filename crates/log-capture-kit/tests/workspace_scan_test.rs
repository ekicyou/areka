//! ワークスペース走査の部品（`workspace_scan`）の**自己較正**（要件 8.3・8.4・10.3）。
//!
//! ここで縛るのは「走査の部品が正しく動くこと」だけで、実際の違反検知（要件 8 の迂回検知・
//! 要件 10 の 1,000 行番人）はこの部品を消費する別の見張りテストが行う。
//!
//! # なぜ較正が要るのか
//!
//! 見張りテストはいずれも「違反が **0 件**なら緑」という形をとる。この形は**道具が壊れていても
//! 緑になる**——列挙が 1 ファイルも拾えていなくても、語の一致規則が何にも当たらなくても、
//! 結果は同じ空集合である。よって次の 3 つを別立てで固定する。
//!
//! 1. 既知の陽性（自前で捕捉先を差す文字列）で `scan_tokens` が確かに 1 件返す。
//! 2. コメントだけの見本では 0 件になる（コメント除去が効いている）。
//! 3. 例外表から 1 件外すと `over_limit` が当該ファイルを返す（例外表が効いている）。
//!
//! あわせて列挙が実行例・統合テスト・兄弟テストファイルの**実在するファイル**を拾うことを
//! 確かめる（被覆が黙って縮まないこと＝要件 8.3）。
//!
//! # 走査語を逐語で書かない理由
//!
//! 本ファイルは `crates/` の下にあるので、走査の対象そのものでもある。走査語を逐語で置くと
//! ⑴ 迂回検知の見張り（要件 8）が本ファイルを違反として拾い、例外表が較正の見本で膨らむ
//! ⑵ 着手前インベントリの捕捉サイト計数の母数が動く。
//! そこで見本の中の走査語は `concat!` で 2 片から組み立て、ファイルの字面には現れないようにする。

mod workspace_scan;

use workspace_scan::{
    FileLines, LINE_LIMIT, line_count, over_limit, scan_tokens, strip_comments,
    walk_workspace_sources,
};

/// 走査語（逐語で置かないため 2 片に割る。上の module doc を参照）。
const TOKEN_WITH_DEFAULT: &str = concat!("with_", "default(");
const TOKEN_SET_DEFAULT: &str = concat!("set_", "default(");
const TOKEN_SET_GLOBAL_DEFAULT: &str = concat!("set_global_", "default(");

fn tokens() -> Vec<&'static str> {
    vec![
        TOKEN_WITH_DEFAULT,
        TOKEN_SET_GLOBAL_DEFAULT,
        TOKEN_SET_DEFAULT,
    ]
}

/// 既知の陽性見本: 自前で捕捉先を差す素の呼出が 4 行目にある。
fn positive_fixture() -> String {
    [
        "//! 見本".to_string(),
        "fn body() {".to_string(),
        "    let sub = make_subscriber();".to_string(),
        format!("    tracing::subscriber::{TOKEN_WITH_DEFAULT}sub, || {{}});"),
        "}".to_string(),
    ]
    .join("\n")
}

#[test]
fn scan_tokens_reports_the_known_positive_direct_call_once() {
    let hits = scan_tokens(&positive_fixture(), &tokens());
    assert_eq!(
        hits,
        vec![(4usize, TOKEN_WITH_DEFAULT.to_string())],
        "既知の陽性見本は 4 行目に 1 件だけ出るはず"
    );
}

#[test]
fn scan_tokens_reports_nothing_when_the_same_call_is_only_in_comments() {
    let call = format!("tracing::subscriber::{TOKEN_WITH_DEFAULT}sub, || {{}});");
    let fixture = [
        format!("//! 説明: かつては {call} と書いていた"),
        format!("/// 行 doc: {call}"),
        format!("// 素の {call}"),
        "/* 塊コメント".to_string(),
        format!("   {call}"),
        "*/".to_string(),
        format!("fn body() {{}} // 行末に {call}"),
    ]
    .join("\n");

    assert_eq!(
        scan_tokens(&fixture, &tokens()),
        Vec::new(),
        "コメントの中の走査語は 1 件も出てはいけない"
    );
}

#[test]
fn scan_tokens_anchors_the_token_so_an_identifier_suffix_is_not_a_hit() {
    // `fn test_offset_default` の開き括弧までの形は走査語 `set_default` ＋開き括弧を含む
    //（実在: `crates/wintf/src/ecs/types.rs`）。アンカーが無いと偽陽性になる。
    let decoy = format!("fn test_off{TOKEN_SET_DEFAULT}) {{}}");
    assert_eq!(
        scan_tokens(&decoy, &tokens()),
        Vec::new(),
        "識別子の途中に現れた走査語は一致してはいけない"
    );

    // 較正の裏側: 語頭がきちんと立っていれば同じ語で 1 件出る（規則が「常に空」ではない）。
    let real = format!("    let g = tracing::subscriber::{TOKEN_SET_DEFAULT}sub);");
    assert_eq!(
        scan_tokens(&real, &tokens()),
        vec![(1usize, TOKEN_SET_DEFAULT.to_string())],
        "語頭が立った呼出は拾えねばならない"
    );
}

#[test]
fn scan_tokens_finds_the_process_wide_variant_as_one_hit() {
    // 本番の 3 語のうち一番長いものが、途中で切れたり分割されたりせず 1 件で出ること。
    // （これは「重なりの規則」ではない——本番の 3 語はどの 2 語も重ならない。下の 2 本を参照。）
    let src = format!("    tracing::subscriber::{TOKEN_SET_GLOBAL_DEFAULT}sub).unwrap();");
    assert_eq!(
        scan_tokens(&src, &tokens()),
        vec![(1usize, TOKEN_SET_GLOBAL_DEFAULT.to_string())],
        "プロセス全体へ差す語も 1 件として出るはず"
    );
}

#[test]
fn scan_tokens_returns_the_longer_of_two_tokens_that_start_at_the_same_place() {
    // 2 語が**同じ位置**で当たるのは、一方が他方の接頭辞のときだけ。
    // 本番の 3 語にはその組が無いので、規則を縛るには接頭辞の組を自分で作るしかない。
    // 語の並びは**わざと短い方を先に**渡す——`scan_tokens` 側の長さ降順の並べ替えが
    // 消えると、呼出側の並び順がそのまま結果を決めてしまうことを露出させるため。
    let short = TOKEN_SET_DEFAULT;
    let long = concat!("set_", "default(sub");
    let src = format!("    tracing::subscriber::{TOKEN_SET_DEFAULT}sub, || {{}});");

    assert!(
        long.starts_with(short),
        "この較正は接頭辞の組でなければ意味がない"
    );
    assert_eq!(
        scan_tokens(&src, &[short, long]),
        vec![(1usize, long.to_string())],
        "同じ位置に 2 語が当たるときは長い方を 1 件だけ返すはず（短い方が返ったら並べ替えが死んでいる）"
    );
}

#[test]
fn scan_tokens_does_not_count_one_call_twice_when_a_longer_token_contains_a_shorter_one() {
    // 修飾つきの語と裸の語を両方持つと、1 つの呼出の**内側**で裸の語がもう一度当たる
    //（`::` の直後なのでアンカーは通ってしまう）。当たった語の内側を飛ばさないと 2 件になる。
    let qualified = concat!("subscriber::set_", "default(");
    let bare = TOKEN_SET_DEFAULT;
    let src = format!("    tracing::subscriber::{TOKEN_SET_DEFAULT}sub);");

    assert!(
        qualified.contains(bare) && !qualified.starts_with(bare),
        "この較正は「長い語の内側に短い語が現れる」組でなければ意味がない"
    );
    assert_eq!(
        scan_tokens(&src, &[qualified, bare]),
        vec![(1usize, qualified.to_string())],
        "1 つの呼出が 2 件に数えられてはいけない"
    );
}

#[test]
fn strip_comments_keeps_line_numbers_and_does_not_cut_string_literals() {
    let src = [
        "// 先頭のコメント行".to_string(),
        "fn body() {".to_string(),
        r#"    let url = "http://example.invalid//path"; // 行末コメント"#.to_string(),
        format!("    tracing::subscriber::{TOKEN_WITH_DEFAULT}sub, || {{}});"),
        "}".to_string(),
    ]
    .join("\n");

    let stripped = strip_comments(&src);
    assert_eq!(
        stripped.lines().count(),
        src.lines().count(),
        "コメント除去で行数が変わってはいけない（行番号が狂う）"
    );
    assert!(
        stripped.contains("http://example.invalid//path"),
        "文字列リテラルの中の 2 連斜線をコメントと誤認してはいけない: {stripped}"
    );
    assert!(
        !stripped.contains("行末コメント"),
        "行末コメントは落ちているはず: {stripped}"
    );
    assert_eq!(
        scan_tokens(&src, &tokens()),
        vec![(4usize, TOKEN_WITH_DEFAULT.to_string())],
        "文字列リテラルを含む見本でも行番号は保たれる"
    );
}

#[test]
fn strip_comments_handles_raw_strings_and_nested_block_comments() {
    let src = [
        r##"    let re = r#"// これはコメントではない"#;"##.to_string(),
        "    /* 外 /* 内 */ まだ塊の中 */ let live = 1;".to_string(),
    ]
    .join("\n");

    let stripped = strip_comments(&src);
    assert!(
        stripped.contains("// これはコメントではない"),
        "raw 文字列の中身は残るはず: {stripped}"
    );
    assert!(
        stripped.contains("let live = 1;"),
        "入れ子の塊コメントを正しく閉じられていない: {stripped}"
    );
    assert!(
        !stripped.contains("まだ塊の中"),
        "入れ子の内側で塊コメントを閉じてしまっている: {stripped}"
    );
}

#[test]
fn strip_comments_does_not_confuse_a_lifetime_with_a_char_literal() {
    let src = [
        "fn f<'a>(s: &'a str) -> &'a str { s } // 落ちる".to_string(),
        r#"    let quote = '"'; // これも落ちる"#.to_string(),
        "    let live = 2;".to_string(),
    ]
    .join("\n");

    let stripped = strip_comments(&src);
    assert!(
        !stripped.contains("落ちる"),
        "ライフタイム注記を文字リテラルと誤読してコメント除去が止まっている: {stripped}"
    );
    assert!(
        stripped.contains("let live = 2;"),
        "文字リテラルの中の引用符で文字列状態に入ってしまっている: {stripped}"
    );
}

#[test]
fn line_count_matches_the_newline_definition_used_by_the_inventory() {
    // 着手前インベントリ（verification/remeasure.md §6）は改行の個数で数えている。
    assert_eq!(line_count(""), 0);
    assert_eq!(line_count("a\n"), 1);
    assert_eq!(line_count("a\nb\n"), 2);
    assert_eq!(
        line_count("a\nb"),
        1,
        "末尾に改行が無い行は着手前インベントリと同じく数えない"
    );
    assert_eq!(line_count("a\r\nb\r\n"), 2, "CRLF でも改行の個数は 2");
}

#[test]
fn the_limit_is_the_one_thousand_lines_the_rule_names() {
    // 上限そのものを縛る。下の見本を `LINE_LIMIT` からの**相対**で書くと、閾値を
    // 動かしても assert が一緒に動いて 1 本も赤にならない（見張りが恒真になる穴）。
    // よって見本は逐語の行数で書き、閾値はここで 1 度だけ直に主張する。
    assert_eq!(
        LINE_LIMIT, 1000,
        "1 ファイル 1,000 行は要件 10.1 と structure.md:176 が名指しした値"
    );
}

/// 見本の行数は**逐語**。1618／1006 は着手前インベントリ（`verification/remeasure.md` §6）の
/// 最大と最小の実測値、1001 はちょうど 1 行超過、1000 は境界、12 は普通のファイル。
fn sample_files() -> Vec<FileLines> {
    vec![
        FileLines::new("crates/demo/src/huge_tests.rs", 1618),
        FileLines::new("crates/demo/src/big.rs", 1006),
        FileLines::new("crates/demo/src/just_over.rs", 1001),
        FileLines::new("crates/demo/src/exactly_at_limit.rs", 1000),
        FileLines::new("crates/demo/src/small.rs", 12),
    ]
}

#[test]
fn over_limit_is_empty_while_every_known_offender_is_listed() {
    let allow = [
        "crates/demo/src/huge_tests.rs",
        "crates/demo/src/big.rs",
        "crates/demo/src/just_over.rs",
    ];
    assert_eq!(
        over_limit(&sample_files(), &allow),
        Vec::new(),
        "例外表に全件載っていれば違反は 0 件"
    );
}

#[test]
fn over_limit_returns_the_file_that_was_dropped_from_the_allow_list() {
    // 較正: 「0 件なら緑」の検査は道具が壊れていても緑になるので、
    // 例外を 1 件外したときに確かにその 1 件が返ることを固定する。
    let allow = [
        "crates/demo/src/huge_tests.rs",
        "crates/demo/src/just_over.rs",
    ];
    assert_eq!(
        over_limit(&sample_files(), &allow),
        vec![FileLines::new("crates/demo/src/big.rs", 1006)],
        "例外表から外した 1 件だけが返るはず"
    );
}

#[test]
fn over_limit_treats_the_limit_itself_as_within_bounds() {
    let files = vec![
        FileLines::new("crates/demo/src/exactly_at_limit.rs", 1000),
        FileLines::new("crates/demo/src/just_over.rs", 1001),
    ];
    assert_eq!(
        over_limit(&files, &[]),
        vec![FileLines::new("crates/demo/src/just_over.rs", 1001)],
        "1000 ちょうどは超過ではなく 1001 は超過（着手前インベントリの境界と同じ）"
    );
}

/// 列挙が実際に拾わねばならない既知ファイル（要件 8.3）。
/// 上から「統合テストの入れ子」「実行例」「本番の隣に置いた兄弟テストファイル」「共有 crate 自身」。
const KNOWN_SOURCES: &[&str] = &[
    "crates/areka-ghost/tests/ghost/spine_e2e_test.rs",
    "crates/pilot/examples/pilot-clickthrough-alpha-toggle/main.rs",
    "crates/areka/src/emo2_boot/spine_display_tests.rs",
    "crates/log-capture-kit/tests/capture_calibration_test.rs",
];

#[test]
fn walk_includes_examples_integration_tests_and_sibling_test_files() {
    let found = walk_workspace_sources();
    assert!(
        found.len() > 500,
        "列挙が極端に少ない＝走査が空振りしている疑い: {} 件",
        found.len()
    );
    for known in KNOWN_SOURCES {
        assert!(
            found.iter().any(|p| p == known),
            "列挙に既知ファイルが含まれていない: {known}"
        );
    }
}

#[test]
fn walk_does_not_vacuously_contain_everything() {
    // 較正: 上の検査が「何を渡しても真」で通っていないことを示す。
    let found = walk_workspace_sources();
    assert!(
        !found
            .iter()
            .any(|p| p == "crates/areka/src/this_file_does_not_exist.rs"),
        "存在しないパスが列挙に現れている＝比較が壊れている"
    );
}

#[test]
fn walk_excludes_build_artifacts_and_vendored_sources() {
    for path in walk_workspace_sources() {
        assert!(path.ends_with(".rs"), "拡張子が .rs でない: {path}");
        assert!(path.starts_with("crates/"), "走査範囲の外: {path}");
        assert!(!path.contains("/target/"), "生成物を拾っている: {path}");
        assert!(
            !path.contains("/vendors/"),
            "外部取り込みを拾っている: {path}"
        );
        assert!(
            !path.contains('\\'),
            "区切り文字が正規化されていない: {path}"
        );
    }
}

#[test]
fn walk_result_is_sorted_and_free_of_duplicates() {
    let found = walk_workspace_sources();
    let mut sorted = found.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        found, sorted,
        "列挙は昇順・重複無しで返るはず（失敗の再現性のため）"
    );
}
