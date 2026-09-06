//! 自前の道具を実データで較正する（タスク 8.4・要件 4.4・5.4・6.8・6.13）。
//!
//! ここにあるのは 3 つの較正である。どれも「小さな見本で緑」ではなく、**repo に実在
//! する本物のデータ**を相手にして、道具が壊れたら赤くなることだけを目的にしている。
//!
//! 1. **テーマ定義の見出し**（要件 4.4・6.8）— `doc/ukadoc-coverage/values.md` の
//!    `##` の見出し 8 つが [`THEMES`] と**順序まで**一致すること。
//! 2. **自前の TOML 書き出し**（要件 1.5・付録 A.3）— カタログの見出し 1,749 件を
//!    [`tomlout`] で組み上げ、`toml` で読み戻すと元の文字列に戻ること。
//! 3. **語彙表経路**（要件 5.4）— 既存の語彙台帳 `SHIORI_RESOURCE_IDS` の 159 要素を
//!    設計 D-5 の規則で拾い、`list_shiori_resource` ページの見出しへ 159 件すべてが
//!    1 件に定まって対応付くこと。突き合わせは件数ではなく**文字列そのもの**で行う。
//!
//! # なぜ ⑴ が鎖の要なのか（8.1 からの申し送り）
//!
//! 要件 6.8（台帳のテーマ名が定義に実在すること）は 3 段の鎖で成り立つ。
//!
//! - 台帳のテーマ名 ∈ `CheckInput::themes`（判定本体・`check/content.rs:263`）
//! - `CheckInput::themes` == [`THEMES`]（タスク 8.1 が `checks.rs` で釘付け済み）
//! - [`THEMES`] == `values.md` の見出し（**このファイル**）
//!
//! 3 段目に見張りが要るのは、`values.md` を書き換えても**どの報告も変わらない**から
//! である——`report/tally.rs:134` は集計表を [`THEMES`] から直接埋め、
//! `report/domain.rs:139-149` はそれと引数を足し合わせる（和）だけなので、テーマの
//! 行が減ることがない。だから `values.md` の書き換えは新しさの検査にも所見にも出ない。
//! ここが唯一の見張りである。
//!
//! # ⑶ の後半（実データでの語彙表経路）は今日成り立つ
//!
//! タスク 8.4 の完了条件には「実データの解決で語彙表経路により解決した件数が 0 でない」
//! も含まれる。道具を建てた当初、これは**構造的に成り立たなかった**——走査対象のどこにも
//! 正典 URL の 1 行コメントが 1 つも置かれておらず、語彙表経路の入口はページ URL の
//! 目印なので、入口に届く証拠が 1 件も無かった。だから空振りの緑を置く代わりに
//! **0 件であること自体を主張**にし、誰かが目印を置いた瞬間に赤くして書き換えを促す
//! 仕掛けにしてあった。
//!
//! **その仕掛けは設計どおり発火した。** 調査 spec（`areka-P0-ukadoc-survey-shiori`）が
//! 要件 5.4 どおり語彙表へ `/// ukadoc: <ページ URL>` を置いたためである。指示どおり
//! 主張を裏返し、[`the_vocabulary_route_binds_items_on_todays_real_data`] が
//! **入口（目印がある）と出口（項目 URL だけでは説明の付かない証拠がある）の 2 段**で
//! 非空を主張する。`checks.rs` の国勢調査も同じ回に 6.5（ソースの正典 URL）と 6.11
//! （証拠の付いた項目）を非空へ移してある。
//!
//! 較正の本体（⑶ の前半）は repo の状態に依らない——設計 Testing Strategy 17a の
//! 但し書き「ページ URL のコメントが置かれる前は、取り出し関数を直接呼んで確かめる」が
//! この経路である。実ソースに目印が置かれた今も、写しへ 1 行足して**足した分だけ
//! 取り出しが増える**ことを見る形で残してある（実ソースの目印を数え込まずに済ませる
//! ためではなく、較正が repo の状態に依らないことを保つためである）。
//!
//! # この較正が捕まえないもの（実データが構造として持っていない 2 つ）
//!
//! ⑶ の較正は実物の語彙表 1 本を相手にするので、その 1 本に現れない書き方は動かして
//! も結果が変わらない。次の 2 つは**この repo の実データでは書き換えても振る舞いが
//! 同じ**であり、**この較正では捕まらない**。どちらも捕まえていないのはこの較正だけ
//! であって、crate の中のテストが合成データで赤にしている（各項に file:line を挙げる）。
//!
//! - **要素の走査が行コメントを読み飛ばすこと**（`resolve::slice_element_names`）。
//!   語彙表のスライスの中の注記は 7 行ある。そのうち
//!   `crates/areka-sylphya/src/vocab/shiori_resource.rs:114` の 1 行だけが ASCII の
//!   角括弧を含む（`[変更不可]`）。それでも差が出ないのは、この 1 組が**釣り合って
//!   いて `[` が先に来る**からで、`[` で深さが 1 に上がり `]` で 0 へ戻り、走査が
//!   表の終わりだと読む `b']' if depth == 0` の腕（`evidence/resolve.rs:267`）に
//!   落ちないためである。残り 6 行には ASCII の引用符・コンマ・角括弧が 1 つも無い。
//!   したがって**差が出る条件**は次のどれかが注記に現れたときである。前の 4 つは
//!   表がそこで終わったことになり以降の要素が黙って消え、最後の 1 つは表を読み切れず
//!   要素が丸ごと落ちる。
//!
//!   1. 釣り合わない `]`
//!   2. `]` が `[` より先に来る組
//!   3. ASCII のコンマ
//!   4. ASCII の二重引用符
//!   5. 釣り合わない `(` または `{` ——深さが 0 へ戻らないので本物の `]` まで
//!      呑み込み、走査が末尾まで走って何も返さない。実測: 行コメントの読み飛ばしを
//!      外した状態で `shiori_resource.rs:209` の注記に `(` を 1 つ足すと、解決できる
//!      件数が 159 から 0 になる。
//!
//!   合成データでは `src/evidence/resolve_tests.rs:440` がこれを赤にしている。
//! - **全角形から ASCII への写し**（`resolve::to_ascii_form`）。実データで効くのは
//!   全角空白 1 文字だけで、それは [`char::is_whitespace`] でもあるため空白の
//!   畳み込みが同じ結果を出す（`resolve::normalized` の注記がこのことを先に書いて
//!   いる）。**正規化そのもの**を外せば
//!   [`the_vocabulary_route_maps_all_159_resource_ids_to_titles`] は赤くなる。全角形の
//!   帯（`FF01..=FF5E`）の写しだけを外した場合はこの較正では捕まらず、合成データの
//!   `src/evidence/resolve_tests.rs:508`（半角 `~` と全角 `～` の突き合わせ）が赤にする。

use std::collections::BTreeSet;

use ukadoc_survey::evidence::extract::extract;
use ukadoc_survey::evidence::resolve::resolve;
use ukadoc_survey::model::THEMES;
use ukadoc_survey::tomlout::{basic_string, keyed_table_header};

use super::RepoData;

// ---------------------------------------------------------------------------
// 較正 ⑴ テーマ定義の見出し（要件 4.4・6.8）
// ---------------------------------------------------------------------------

/// `values.md` の見出しの段の形。要件 4.4 が凍結するのは 8 つのテーマである。
const EXPECTED_LEVEL1: usize = 1;
const EXPECTED_LEVEL2: usize = 8;

/// テーマ定義の `##` の見出しが [`THEMES`] と順序まで一致する（要件 4.4・6.8）。
///
/// 突き合わせは**綴りそのもの**である。「気配」と「気配り」は片方が他方の接頭辞
/// なので、部分一致や前後の空白落としを混ぜると取り違える（`model::parse_theme` の
/// 注記と同じ理由）。
#[test]
fn the_theme_headings_of_values_md_match_the_constant_in_order() {
    let data = RepoData::load();
    let headings = headings_of_level(&data.values, 2);

    assert_eq!(
        headings,
        THEMES.to_vec(),
        "values.md の `##` の見出しと model::THEMES が食い違う。\
         要件 6.8 の鎖はこの一致だけで吊られている——\
         どちらかを直したら必ずもう一方も直すこと"
    );
}

/// 個数がちょうど 8 つであること（要件 4.4 の「8 つだけ」）。
///
/// 上のテストの `assert_eq!` でも件数の食い違いは捕まるが、そのときの失敗は
/// 「一覧が違う」という読みにくい形になる。破った約束を名指しで落とすために分ける。
#[test]
fn values_md_freezes_exactly_eight_themes() {
    let data = RepoData::load();

    assert_eq!(THEMES.len(), EXPECTED_LEVEL2, "model::THEMES が 8 つでない");
    assert_eq!(
        headings_of_level(&data.values, 2).len(),
        EXPECTED_LEVEL2,
        "values.md のテーマの見出しが 8 つでない（要件 4.4）"
    );
}

/// 見出しの段の形が凍結されていること（7.1 からの申し送り）。
///
/// 抽出は `##` をテーマ名専用とみなす前提に立っている。`#` が 2 本になったり
/// `###` が現れたりすると、抽出の前提が黙って崩れる。前提そのものを主張にする。
#[test]
fn values_md_uses_one_top_heading_and_no_deeper_levels() {
    let data = RepoData::load();

    assert_eq!(
        headings_of_level(&data.values, 1).len(),
        EXPECTED_LEVEL1,
        "values.md の `# ` は 1 本でなければならない（テーマ抽出の前提）"
    );
    let deeper: Vec<&str> = data
        .values
        .lines()
        .filter(|line| heading(line).is_some_and(|(level, _)| level >= 3))
        .collect();
    assert!(
        deeper.is_empty(),
        "values.md に `###` 以上の見出しがある: {deeper:?}。\
         テーマ抽出は `##` をテーマ名専用として読んでいる"
    );
}

/// `values.md` の見出しのうち、ちょうど `level` 段のものを本文の順に返す。
fn headings_of_level(values: &str, level: usize) -> Vec<&str> {
    values
        .lines()
        .filter_map(heading)
        .filter(|(found, _)| *found == level)
        .map(|(_, title)| title)
        .collect()
}

/// 1 行を見出しとして読む。`#` の連なりの後に空白が要る。
///
/// `strip_prefix("## ")` で済ませてはいけない——`### x` はその綴りでも剥がれて
/// `# x` が残り、3 段の見出しが 2 段として数えられる。
fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if level == 0 {
        return None;
    }
    let title = line[level..].strip_prefix(' ')?;
    Some((level, title.trim_end()))
}

// ---------------------------------------------------------------------------
// 較正 ⑵ 自前の TOML 書き出しの読み戻し（要件 1.5・付録 A.3）
// ---------------------------------------------------------------------------

/// カタログの見出しの実測件数（設計 D-1・タスク 7.2 の実測）。
const CATALOG_ENTRIES: usize = 1_749;

/// 自前の書き出しで組んだ本文を `toml` で読み戻すと、見出し 1,749 件が元に戻る。
///
/// 組み立てには [`keyed_table_header`] と [`basic_string`] を通す。ここを通さずに
/// 文字列をそのまま並べると、読み戻しの一致は**自分自身との比較**になって何も
/// 較正しない。逆斜線を含む見出しが実データに多数あるので（下の非空虚の主張）、
/// 逃がしが 1 つでも欠けると `toml` の解析か読み戻しの一致のどちらかが落ちる。
#[test]
fn the_own_toml_writer_round_trips_every_catalog_title() {
    let data = RepoData::load();
    assert_eq!(
        data.catalog.entries.len(),
        CATALOG_ENTRIES,
        "カタログの件数が実測と違う（この較正は 1,749 件すべてを回す）"
    );

    let document = catalog_titles_document(&data);
    let root: toml::Table = document
        .parse()
        .unwrap_or_else(|err| panic!("自前の書き出しが TOML として読めない: {err}"));
    let entries = root
        .get("entry")
        .and_then(toml::Value::as_table)
        .expect("読み戻した本文に entry の表が無い");

    assert_eq!(
        entries.len(),
        CATALOG_ENTRIES,
        "読み戻した項目の数が書き出した数と違う（キーが衝突して畳まれていないか）"
    );
    for (id, entry) in &data.catalog.entries {
        let read_back = entries
            .get(id.as_str())
            .and_then(toml::Value::as_table)
            .and_then(|table| table.get("title"))
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("読み戻しに id が無い: {}", id.as_str()));
        assert_eq!(
            read_back,
            entry.title,
            "読み戻した見出しが元と違う: id={}",
            id.as_str()
        );
    }
}

/// 読み戻しの一致だけでは足りない分を補う（設計 `tomlout` 節の 2 本立て）。
///
/// 非 ASCII をすべて `\uXXXX` に潰す書き出しでも読み戻しは通ってしまうが、それでは
/// カタログが人に読めなくなり要件 9.5 と設計 D-1（1 行最大 579 文字）を破る。だから
/// **逃がしが現れてよいのは逆斜線と二重引用符だけ**であることも主張する。
///
/// 併せて、この較正が空回りしていないことを示す——逆斜線を含む見出しと非 ASCII を
/// 含む見出しがどちらも実データに存在する（逃がす道と逃がさない道の両方を通る）。
#[test]
fn the_own_toml_writer_escapes_only_what_it_must() {
    let data = RepoData::load();

    let mut with_backslash = 0usize;
    let mut with_non_ascii = 0usize;
    for entry in data.catalog.entries.values() {
        if entry.title.contains('\\') {
            with_backslash += 1;
        }
        if !entry.title.is_ascii() {
            with_non_ascii += 1;
        }
        let written = basic_string(&entry.title);
        let expected = format!(
            "\"{}\"",
            entry.title.replace('\\', r"\\").replace('"', "\\\"")
        );
        assert_eq!(
            written, expected,
            "見出しの書き方が付録 A.3 の形から外れた: {}",
            entry.title
        );
    }

    assert!(
        with_backslash > 0,
        "逆斜線を含む見出しが 1 件も無い。逃がしの道を 1 度も通らないので較正が空回りする"
    );
    assert!(
        with_non_ascii > 0,
        "非 ASCII の見出しが 1 件も無い。逃がさない道を 1 度も通らないので較正が空回りする"
    );
}

/// 全項目の id と見出しだけを載せた TOML 本文を、自前の書き出しで組む。
fn catalog_titles_document(data: &RepoData) -> String {
    let mut out = String::new();
    for (id, entry) in &data.catalog.entries {
        out.push_str(&keyed_table_header("entry", id.as_str()));
        out.push('\n');
        out.push_str("title = ");
        out.push_str(&basic_string(&entry.title));
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// 較正 ⑶ 語彙表経路（要件 5.4・設計 D-5・Testing Strategy 17a）
// ---------------------------------------------------------------------------

/// 既存の語彙台帳（要件 9.2 の「置き換えない」相手）。
const VOCABULARY_PATH: &str = "crates/areka-sylphya/src/vocab/shiori_resource.rs";

/// その語彙表のスライス定数の書き出し。走査の起点であり、この綴りは本文で 1 度だけ
/// 現れる（テストがそれを確かめる）。
const VOCABULARY_CONST: &str = "pub const SHIORI_RESOURCE_IDS: &[&str] = &[";

/// 語彙表が写しているページ。
const RESOURCE_PAGE: &str = "list_shiori_resource";

/// 語彙表の要素数（`areka-sylphya` 側の件数檻と同じ 159・R1.4）。
const RESOURCE_IDS: usize = 159;

/// 素の綴りではカタログの見出しと食い違う唯一の要素（語彙表の側・半角空白）。
const HALFWIDTH_ELEMENT: &str = "(入力ボックス種類).defaultleft (入力ボックス種類).defaulttop";

/// その相手のカタログの見出し（正典の側・全角空白 U+3000）。
///
/// この 1 件があるので、突き合わせ前の正規化（`resolve::normalized`）は実データで
/// 現に効いている。159 件のうち素の綴りで一致するのは 158 件である。
const FULLWIDTH_TITLE: &str = "(入力ボックス種類).defaultleft\u{3000}(入力ボックス種類).defaulttop";

/// 語彙表 159 要素が、ページの見出しへ 1 件ずつ**逐語で**対応付く（要件 5.4）。
///
/// 手順は実際の書き方どおりである——表の先頭にページ URL を 1 行置き（要件 5.4）、
/// 取り出し（[`extract`]）と解決（[`resolve`]）を素通しで通す。目印を置いた本文は
/// テストの中だけの写しで、ディスクには 1 バイトも書かない。
///
/// 期待値は語彙表の本文から**独立に**読む（1 行 1 要素という実物の綴りに寄りかかる
/// 素朴な読み方で、`resolve` の走査とは別物）。だから取り出し規則が要素を落としたり
/// 別の見出しに結んだりすれば、件数ではなく**文字列**で食い違って落ちる。
#[test]
fn the_vocabulary_route_maps_all_159_resource_ids_to_titles() {
    let data = RepoData::load();
    let source = source_text(&data, VOCABULARY_PATH);

    let expected = elements_by_line(source);
    assert_eq!(
        expected.len(),
        RESOURCE_IDS,
        "語彙表の要素数が {RESOURCE_IDS} でない（areka-sylphya 側の件数檻と食い違う）"
    );

    let page_url = page_url_of(&data, RESOURCE_PAGE);

    // **足す前**——実ソースの語彙表には、調査 spec が要件 5.4 どおりに置いた目印が
    // すでに 1 行ある。だから「目印はちょうど 1 件」では見られない。代わりに、この
    // ファイルから取り出せる URL が**どれも目印のページ URL である**ことを見る
    // （項目 URL が混ざっていれば、下の解決は語彙表経路を通らずに `by_id` を埋める）。
    let bare = extract(VOCABULARY_PATH, source);
    assert!(
        bare.iter().all(|hit| hit.url == page_url),
        "語彙表から目印以外の URL が取り出せる: {:?}。\
         この較正は語彙表経路だけを通す前提で組んである。\
         直し方——項目 URL はその項目の定義箇所へ置くこと（語彙表には置かない）。\
         語彙表に置いてよいのはページ URL の目印 1 行だけである",
        bare.iter().map(|hit| &hit.url).collect::<Vec<_>>()
    );

    // **足した後**——書き足した 1 行の分だけ取り出しが増え、増えた分も目印である。
    let marked = with_marker(source, &page_url);
    let hits = extract(VOCABULARY_PATH, &marked);
    assert_eq!(
        hits.len(),
        bare.len() + 1,
        "目印を 1 行足したのに取り出しが {} 件から {} 件になった",
        bare.len(),
        hits.len()
    );
    assert!(
        hits.iter().all(|hit| hit.url == page_url),
        "取り出した URL に目印以外が混ざった: {:?}",
        hits.iter().map(|hit| &hit.url).collect::<Vec<_>>()
    );

    let sources = vec![(VOCABULARY_PATH.to_owned(), marked)];
    let index = resolve(&hits, &sources, &data.catalog);

    assert!(
        index.unmatched_names.is_empty(),
        "対応の付かなかった要素がある: {:?}",
        index.unmatched_names
    );
    assert!(
        index.unresolved.is_empty(),
        "解決できなかった URL がある: {:?}",
        index.unresolved
    );
    assert_eq!(
        index.by_id.len(),
        RESOURCE_IDS,
        "1 件に定まった要素が {} 件しかない（{RESOURCE_IDS} 件すべてが定まること）",
        index.by_id.len()
    );

    // 逐語の突き合わせ。結んだ先の見出しを取り出し、語彙表の要素と文字列で比べる。
    let titles = data.catalog.titles_of_page(&page_of(&data, RESOURCE_PAGE));
    let mut resolved: Vec<&str> = Vec::with_capacity(index.by_id.len());
    for (id, paths) in &index.by_id {
        assert_eq!(
            paths.as_slice(),
            [VOCABULARY_PATH.to_owned()].as_slice(),
            "証拠のファイルが語彙表以外を指している: id={}",
            id.as_str()
        );
        let title = titles
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, title)| *title)
            .unwrap_or_else(|| panic!("結んだ先が {RESOURCE_PAGE} の項目でない: {}", id.as_str()));
        resolved.push(title);
    }

    let mut want: Vec<&str> = expected
        .iter()
        .map(|name| {
            if name == HALFWIDTH_ELEMENT {
                FULLWIDTH_TITLE
            } else {
                name.as_str()
            }
        })
        .collect();
    want.sort_unstable();
    resolved.sort_unstable();
    assert_eq!(
        resolved, want,
        "結んだ見出しの一覧が語彙表の要素と逐語で一致しない"
    );
}

/// 正規化がこの較正で現に働いていること（設計 D-5 の実測 158/159 → 159/159）。
///
/// 素の綴りで食い違う 1 件がちょうど 1 つあることを固定する。この 1 件が消えると、
/// 上の較正は正規化を 1 度も通らずに緑になり、見張りとして空回りする。
#[test]
fn exactly_one_element_needs_normalization_to_match_its_title() {
    let data = RepoData::load();
    let source = source_text(&data, VOCABULARY_PATH);
    let expected = elements_by_line(source);

    let odd: Vec<&String> = expected
        .iter()
        .filter(|name| name.as_str() == HALFWIDTH_ELEMENT)
        .collect();
    assert_eq!(
        odd.len(),
        1,
        "語彙表に半角空白の要素が {} 件ある（実測は 1 件）",
        odd.len()
    );

    let page = page_of(&data, RESOURCE_PAGE);
    let titles: BTreeSet<&str> = data
        .catalog
        .titles_of_page(&page)
        .into_iter()
        .map(|(_, title)| title)
        .collect();
    assert!(
        titles.contains(FULLWIDTH_TITLE),
        "全角空白の見出しがカタログから消えた。正規化の要る 1 件はこれだけだった"
    );
    assert!(
        !titles.contains(HALFWIDTH_ELEMENT),
        "半角空白の見出しがカタログに現れた。素の綴りで一致するなら正規化を通らない"
    );

    let raw_matches = expected
        .iter()
        .filter(|name| titles.contains(name.as_str()))
        .count();
    assert_eq!(
        raw_matches,
        RESOURCE_IDS - 1,
        "素の綴りで一致する要素が 158 件でない（設計 D-5 の実測）"
    );
}

/// 実データの語彙表経路が現に項目を結んでいる（タスク 8.4 の完了条件の後半）。
///
/// **この主張はかつて「0 件である」の側に立っていた。** 道具を建てた当初はソースに
/// 正典 URL が 1 つも置かれておらず、語彙表経路の入口——ページ URL の目印——に届く
/// 証拠が 1 件も無かったので、空振りの緑を置く代わりに 0 件そのものを主張にし、誰かが
/// 目印を置いた瞬間に赤くして非空の主張への書き換えを促す仕掛けにしてあった。
///
/// **その仕掛けは設計どおり発火した。** 調査 spec（`areka-P0-ukadoc-survey-shiori`）が
/// 要件 5.4 どおり語彙表 `crates/areka-sylphya/src/vocab/shiori_resource.rs` へ
/// `/// ukadoc: <ページ URL>` を置いたためである。そこで書き換えの指示どおり、非空の
/// 主張を 2 段で置く。
///
/// ⑴ **入口**——目印がソースに 1 件以上ある。
/// ⑵ **出口**——実データの証拠索引に、**項目 URL の 1 行コメントだけでは説明の付かない**
///    項目がある。項目 URL は `evidence/resolve.rs` の `by_url` の腕が目印を 1 度も
///    通さずに `by_id` を埋めるので、単に `by_id` が非空であることを主張しても
///    「語彙表経路」と名乗りながらそれより広いものを見張ることになる。差を取って
///    語彙表経路の取り分だけを見る。
///
/// **赤になったら**——⑴ が落ちたなら語彙表の目印の行が剥がれている。⑵ だけが落ちたなら
/// 目印は残っているのに要素と見出しの突き合わせが 1 件も結べていない、つまり
/// `resolve::match_vocabulary` の側が壊れている。
#[test]
fn the_vocabulary_route_binds_items_on_todays_real_data() {
    let data = RepoData::load();

    let by_url = data.catalog.by_url();
    let page_urls = data.catalog.page_urls();
    let hits: Vec<_> = data
        .sources
        .iter()
        .flat_map(|(path, text)| extract(path, text))
        .collect();

    let markers: Vec<String> = hits
        .iter()
        .filter(|hit| !by_url.contains_key(hit.url.as_str()) && page_urls.contains_key(&hit.url))
        .map(|hit| format!("{}: {}", hit.path, hit.url))
        .collect();
    assert!(
        !markers.is_empty(),
        "語彙表経路の入口——ページ URL の目印——がソースに 1 件も無い。\
         要件 5.4 の目印の行が剥がれていないか確かめること"
    );

    // 項目 URL の 1 行コメントだけで説明の付く項目。ここに入らない証拠が語彙表経路の
    // 取り分である。
    let direct: BTreeSet<&str> = hits
        .iter()
        .filter_map(|hit| by_url.get(hit.url.as_str()).map(|id| id.as_str()))
        .collect();
    let by_route: Vec<&str> = data
        .evidence
        .by_id
        .keys()
        .map(|id| id.as_str())
        .filter(|id| !direct.contains(id))
        .collect();

    assert!(
        !by_route.is_empty(),
        "実データの証拠 {} 件がすべて項目 URL で説明が付き、\
         語彙表経路で結ばれた項目が 1 件も無い。\
         目印は {} 件あるので、要素と見出しの突き合わせの側が結べていない",
        data.evidence.by_id.len(),
        markers.len()
    );
}

// ---------------------------------------------------------------------------
// 較正 ⑶ の下ごしらえ
// ---------------------------------------------------------------------------

/// 走査で読んだ本文から 1 本引く。無ければそのパスを告げて止まる（要件 6.12）。
fn source_text<'a>(data: &'a RepoData, path: &str) -> &'a str {
    data.sources
        .iter()
        .find(|(found, _)| found == path)
        .map(|(_, text)| text.as_str())
        .unwrap_or_else(|| panic!("走査に載っていない: {path}"))
}

/// ページ名からフラグメント無しのページ URL を引く。
fn page_url_of(data: &RepoData, page: &str) -> String {
    data.catalog
        .page_urls()
        .into_iter()
        .find(|(_, found)| found.as_str() == page)
        .map(|(url, _)| url)
        .unwrap_or_else(|| panic!("カタログにページが無い: {page}"))
}

/// ページ名の値を作る。
fn page_of(data: &RepoData, page: &str) -> ukadoc_survey::model::PageName {
    data.catalog
        .page_urls()
        .into_values()
        .find(|found| found.as_str() == page)
        .unwrap_or_else(|| panic!("カタログにページが無い: {page}"))
}

/// スライス定数の直前にページ URL の目印を差し込んだ写しを返す（要件 5.4 の書き方）。
///
/// 差し込む先は doc コメントの続きなので、この形は実際のソースにそのまま置ける。
/// ディスクには書かない——値として組み立てて [`resolve`] へ渡すだけである。
fn with_marker(source: &str, page_url: &str) -> String {
    assert_eq!(
        source.matches(VOCABULARY_CONST).count(),
        1,
        "語彙表のスライス定数の書き出しが 1 度だけ現れない（走査の起点が定まらない）"
    );
    source.replace(
        VOCABULARY_CONST,
        &format!("/// ukadoc: {page_url}\n{VOCABULARY_CONST}"),
    )
}

/// 語彙表の要素を**独立に**読む（1 行 1 要素という実物の綴りに寄りかかった素朴な読み）。
///
/// `resolve` の走査（設計 D-5 の規則）とは別物である。期待値を作る側が同じ規則を
/// 使ってしまうと、規則が壊れても両側が揃って壊れて緑のまま通る。
fn elements_by_line(source: &str) -> Vec<String> {
    let start = source
        .find(VOCABULARY_CONST)
        .expect("語彙表のスライス定数が見つからない")
        + VOCABULARY_CONST.len();
    let body = &source[start..];
    let end = body.find("\n];").expect("語彙表のスライス定数が閉じない");

    let mut out = Vec::new();
    for line in body[..end].lines() {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_suffix(',')
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.strip_suffix('"'))
        else {
            continue;
        };
        assert!(
            !inner.contains('\\'),
            "語彙表の要素に逃がし形が現れた: {inner}。\
             この素朴な読み方は逃がしを戻さないので、期待値の作り方を直すこと"
        );
        out.push(inner.to_owned());
    }
    out
}
