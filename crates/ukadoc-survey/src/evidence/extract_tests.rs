//! `extract.rs` の在中テスト。
//!
//! ファイルは 1 つも作らず、文字列だけを相手にする（設計 File Structure Plan）。
//!
//! 見本の本文には `/// ukadoc: <URL>` の形をした行が並ぶが、走査は
//! `crates/ukadoc-survey/` を除くので、この文字列が本物の証拠として読まれることは
//! ない（設計 D-3・`io/sources.rs` が実測で守っている）。
//!
//! **否定の主張には必ず肯定の主張を対で置く**。「拾われない」だけのテストは、
//! 何も返さない [`extract`] でも緑になる（タスク 1.6 の教訓）。だから
//! 「拾われないもの」と「拾われるもの」を同じ見本・同じテストに入れる。

use super::*;

/// 実在する形の正典 URL（アンカー付き）。
const URL_TAG_S: &str = "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#tag_s";
/// 実在する形の正典 URL（別の項目）。
const URL_TAG_B: &str = "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#tag_b";
/// 実在する形の正典 URL（アンカー無し＝ページ URL）。
const URL_PAGE: &str = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html";

/// 見本のファイルパス。証拠はこの綴りをそのまま載せる。
const PATH: &str = "crates/areka-parsers/src/sakura/lexer.rs";

/// 行の並びを 1 つの本文に綴じる。
fn body(lines: &[String]) -> String {
    lines.join("\n")
}

/// 拾われた URL だけを順に並べる（並びは崩さない）。
fn urls(hits: &[UrlHit]) -> Vec<&str> {
    hits.iter().map(|hit| hit.url.as_str()).collect()
}

// ---- タスクの完了条件（要件 5.6 と 5.1 を対で）----

/// 「ukadoc」の語だけで URL を伴わない行は 1 件も拾われず、URL 付きの行は拾われる。
///
/// 否定側だけでは何も返さない実装が素通りするので、同じ見本に肯定側を入れてある。
/// 否定側の 5 行は現に作業ツリーにある形を写したもの——`ukadoc:` が文中に埋まった形
/// （`placement/source_tests.rs:221`・`emo-compose/fold.rs:171`）、`ukadoc:` の後に
/// 説明文が続く形（`sakura/decode_tests.rs:438`）、語だけの形である。
#[test]
fn a_ukadoc_word_without_a_url_is_never_evidence_but_a_url_line_is() {
    let text = body(&[
        "/// 無宣言（キー不在）は既定 96（ukadoc: 何も指定しなければ 96 固定）。".to_owned(),
        "/// 依らず展開結果全体へ効く（ukadoc: `!` は集合から除去）。".to_owned(),
        "/// ukadoc: `\\0`/`\\h`=本体側、`\\1`/`\\u`=相方側）。数字形 と同一へ写像する。"
            .to_owned(),
        "// ukadoc の一覧を見ながら実装した。".to_owned(),
        "//! ukadoc に載っている挙動に合わせる。".to_owned(),
        format!("/// ukadoc: {URL_TAG_S}"),
    ]);

    let hits = extract(PATH, &text);

    // 肯定側——URL 付きの 1 行だけが拾われる。
    assert_eq!(urls(&hits), vec![URL_TAG_S], "拾われた URL が違う");
    // 否定側——語だけの行は 1 件も混ざらない。
    assert_eq!(hits.len(), 1, "URL を伴わない行が混ざった: {hits:?}");
}

// ---- 3 種のコメント記号と、その優先順位の罠 ----

/// `///`・`//!`・`//` の 3 種すべてが記号として認められる。
///
/// `///` は `//` でも始まるので、短い方から見ると `/ ukadoc: ...` と読み違える。
/// 3 種を同じ見本に置いて、どれも落ちないことを逐語で確かめる。
#[test]
fn all_three_comment_markers_are_accepted() {
    let text = body(&[
        format!("/// ukadoc: {URL_TAG_S}"),
        format!("//! ukadoc: {URL_TAG_B}"),
        format!("// ukadoc: {URL_PAGE}"),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(
        urls(&hits),
        vec![URL_TAG_S, URL_TAG_B, URL_PAGE],
        "3 種の記号のいずれかが落ちている"
    );
}

/// 記号は 3 種ちょうどで、`////` や `//!!` は記号ではない。
///
/// 記号の照合を「長い方から」ではなく「`//` を剥がして残りを見る」に変えると、
/// `////` の残りが `/ ukadoc: ...` ではなく `ukadoc: ...` に見えてしまう形がある。
/// 肯定側（3 種は通る）を同じテストに置いて、全部落とす実装を塞ぐ。
#[test]
fn four_slashes_and_double_bang_are_not_markers() {
    let text = body(&[
        format!("//// ukadoc: {URL_TAG_S}"),
        format!("//!! ukadoc: {URL_TAG_B}"),
        format!("/// ukadoc: {URL_PAGE}"),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(
        urls(&hits),
        vec![URL_PAGE],
        "3 種の記号ちょうどでなくなっている"
    );
}

// ---- 説明文が続く行は取らない（要件 5.3）----

/// URL の後ろに語が続く行は証拠にしない。
///
/// これが「1 項目 1 行・説明文なし」（要件 5.3）を機械側で守る唯一の仕掛けである。
/// 肯定側を対で置いて、全部落とす実装を塞ぐ。
#[test]
fn a_line_with_prose_after_the_url_is_not_evidence() {
    let text = body(&[
        format!("/// ukadoc: {URL_TAG_S} 表示位置を指定する"),
        format!("//! ukadoc: {URL_TAG_B} — 別名"),
        format!("// ukadoc: {URL_PAGE} (未確認)"),
        format!("/// ukadoc: {URL_TAG_S}"),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(urls(&hits), vec![URL_TAG_S], "説明文が続く行が拾われている");
}

/// 語が 1 つも続かない行は証拠にしない（空の URL を作らない）。
#[test]
fn a_line_without_a_word_after_the_token_is_not_evidence() {
    let text = body(&[
        "/// ukadoc:".to_owned(),
        "//! ukadoc:   ".to_owned(),
        "// ukadoc:".to_owned(),
        format!("/// ukadoc: {URL_TAG_S}"),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(urls(&hits), vec![URL_TAG_S], "空の URL が作られている");
}

// ---- `ukadoc:` の目印そのもの ----

/// 記号はあるが `ukadoc:` が無い行は証拠にしない。
#[test]
fn a_comment_without_the_ukadoc_token_is_not_evidence() {
    let text = body(&[
        format!("/// {URL_TAG_S}"),
        format!("// see: {URL_TAG_B}"),
        "/// ukadoc は SSP の公式仕様書である。".to_owned(),
        format!("/// ukadoc: {URL_PAGE}"),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(urls(&hits), vec![URL_PAGE], "目印の判定が緩んでいる");
}

/// `ukadoc:` と URL の間には空白が要る（設計「取り出しの行の形」の逐語）。
///
/// 空白を求めないと `// ukadoc:上記を参照` のような散文の 1 かたまりが URL として
/// 拾われ、解決の段（設計 D-4 の 3 段目）で赤になる。散文で検査を赤にしないため、
/// 区切りの空白を必須にした。肯定側を対で置く。
#[test]
fn the_token_needs_a_whitespace_separator_before_the_url() {
    let text = body(&[
        format!("/// ukadoc:{URL_TAG_S}"),
        "// ukadoc:上記を参照".to_owned(),
        format!("/// ukadoc: {URL_TAG_B}"),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(urls(&hits), vec![URL_TAG_B], "区切りの空白の扱いが違う");
}

/// URL の後ろの空白だけなら証拠のまま（見えない 1 文字で根拠を落とさない）。
///
/// 行末の空白は編集画面で見えないので、これを落とすと「書いたのに拾われない」が
/// 黙って起きる。前後の空白は畳んで受け入れる、が採った側である。
#[test]
fn trailing_whitespace_after_the_url_is_tolerated() {
    let text = body(&[
        format!("/// ukadoc: {URL_TAG_S}  "),
        format!("//! ukadoc:  {URL_TAG_B}\t"),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(
        urls(&hits),
        vec![URL_TAG_S, URL_TAG_B],
        "行末の空白で証拠が落ちている"
    );
}

// ---- 行の中での記号の位置 ----

/// 行頭の空白（字下げ）は許すが、記号の前にコードや引用符があってはいけない。
///
/// 採った側とその理由: 証拠の行は**それ 1 行で完結する**（要件 5.3）。さらに要件
/// 5.4 の語彙表は「ページ URL の単独行の直後にスライス定数が始まる」形を要求する
/// （設計 D-5）ので、コードの尻尾に付いた注釈を拾うと表の起点が定まらなくなる。
/// 見本のソース文に現れる `"// ukadoc: ..."` のような文字列リテラルも同じ規則で落ちる。
#[test]
fn the_marker_must_start_the_trimmed_line() {
    let text = body(&[
        format!("        /// ukadoc: {URL_TAG_S}"),
        format!("\t//! ukadoc: {URL_TAG_B}"),
        format!("let x = 1; // ukadoc: {URL_PAGE}"),
        format!("    \"// ukadoc: {URL_PAGE}\","),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(
        urls(&hits),
        vec![URL_TAG_S, URL_TAG_B],
        "記号の位置の規則が違う"
    );
}

// ---- 並びとパス ----

/// 複数の証拠は本文に現れた順で返り、どれも渡されたパスを載せる。
///
/// 見本の URL は**わざと名前順に並べていない**。整列済みの見本では「並べ替える」
/// 実装が素通りするため（タスク 2.5 の教訓）。
#[test]
fn hits_keep_file_order_and_carry_the_given_path() {
    let text = body(&[
        format!("/// ukadoc: {URL_PAGE}"),
        "pub struct A;".to_owned(),
        format!("//! ukadoc: {URL_TAG_S}"),
        "pub struct B;".to_owned(),
        format!("// ukadoc: {URL_TAG_B}"),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(
        urls(&hits),
        vec![URL_PAGE, URL_TAG_S, URL_TAG_B],
        "本文の順が保たれていない"
    );
    for hit in &hits {
        assert_eq!(hit.path, PATH, "パスが載っていない");
    }
}

/// 別のパスを渡せばそのパスが載る（`PATH` の直書きを塞ぐ）。
#[test]
fn the_path_comes_from_the_argument() {
    let other = "crates/areka-sylphya/src/vocab/dotted.rs";
    let text = format!("/// ukadoc: {URL_TAG_S}");

    let hits = extract(other, &text);

    assert_eq!(hits.len(), 1, "1 件拾えていない");
    assert_eq!(hits[0].path, other, "渡したパスが載っていない");
}

// ---- 型の形（要件 5.1・6.11）----

/// [`UrlHit`] の欄はパスと URL のちょうど 2 つで、行番号を持たない。
///
/// 網羅的な分解なので、3 つ目の欄（行番号など）を足すとこのテストは**コンパイルが
/// 通らなくなる**。整理で行が動いても証拠が壊れないこと（要件 6.11）は、この型の
/// 形が背負っている。
#[test]
fn url_hit_has_exactly_two_fields_and_no_line_number() {
    let text = format!("/// ukadoc: {URL_TAG_S}");
    let hits = extract(PATH, &text);
    assert_eq!(hits.len(), 1, "1 件拾えていない");

    let UrlHit { path, url } = hits[0].clone();

    assert_eq!(path, PATH);
    assert_eq!(url, URL_TAG_S);
}

// ---- 現実に近い本文 ----

/// 本物の証拠・ukadoc に触れた散文・ただのコードが混ざった本文から、証拠だけが出る。
#[test]
fn a_realistic_body_yields_only_the_genuine_annotations() {
    let text = body(&[
        "//! さくらスクリプトの字句解析。".to_owned(),
        "//!".to_owned(),
        "//! ukadoc の一覧を見ながら書いた（ここは散文なので証拠ではない）。".to_owned(),
        String::new(),
        "use crate::model::Token;".to_owned(),
        String::new(),
        "/// サーフェス切り替え。".to_owned(),
        format!("/// ukadoc: {URL_TAG_S}"),
        "pub fn surface() -> Token {".to_owned(),
        format!("    // ukadoc: {URL_TAG_B} ← 説明を書いてしまった行"),
        "    let raw = \"\\\\s[0]\"; // ukadoc に合わせる".to_owned(),
        "    Token::Surface(raw)".to_owned(),
        "}".to_owned(),
        String::new(),
        "/// 語彙表の目印。".to_owned(),
        format!("/// ukadoc: {URL_PAGE}"),
        "pub const NAMES: &[&str] = &[\"sakura.recommendsites\"];".to_owned(),
    ]);

    let hits = extract(PATH, &text);

    assert_eq!(
        urls(&hits),
        vec![URL_TAG_S, URL_PAGE],
        "本物の証拠だけが出ていない"
    );
}
