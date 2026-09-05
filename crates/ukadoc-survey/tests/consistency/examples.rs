//! 記入例が実データに実在し、道具でそのまま読めることの主張（タスク 12.1・
//! 要件 2.1・2.6・6.3・6.7・付録 A.1・A.3・付録 B.5）。
//!
//! # なぜ記入例に見張りが要るのか
//!
//! 要件 2.1 は調査 spec 4 本に「本 spec の実装を待たず**付録 A の形で**台帳を書き始め
//! てよい」と言っている。つまり付録 A.1 の記入例は読み物ではなく、4 本が最初に写す
//! 手本である。手本の id が実在しない綴りだと、写した台帳は要件 6.3
//! （`LedgerIdNotInCatalog`）で必ず赤くなり、書き手は「道具が壊れている」と読む。
//!
//! 実際に 2026-09-05 まで、付録 A.1 の記入例 4 件のうち 3 件は実データに無い綴り
//! （`balloon.scope(ID).width` など）だった。実在する id へ訂正したのが親の差分で、
//! **訂正が二度と剥がれない**ようにするのがこのファイルである。
//!
//! # 何を相手にするか
//!
//! ⑴ 要件文書と ⑵ `doc/ukadoc-coverage/README.md` の、` ```toml ` の囲みの中に
//! 現れる `"ukadoc:…"` の綴りすべて。囲みの外（説明の地の文や表）は相手にしない
//! ——地の文には「実在しない綴りの例」を反例として挙げることがあり、それは正しい
//! 書き方だからである。囲みの中は「そのまま写せる手本」だけを置く場所として扱う。
//!
//! # 空振りの緑を作らない
//!
//! 取り出しが 0 件を返せば「全部実在する」は無条件に成り立つ。だから件数の下限を
//! 釘付けし（要件 4 件・README 3 件）、さらに取り出し自体の較正
//! （[`the_id_scan_reports_an_id_that_the_catalog_does_not_have`]）を 1 本置く。
//! 較正は実在する id と実在しない id を 1 つずつ混ぜた小さな本文を食わせ、後者が
//! 「カタログに無い」と報告されることを確かめる。
//!
//! # 読むだけ
//!
//! ファイルも一時ディレクトリも作らない（設計 File Structure Plan）。読むのは要件
//! 文書と README とカタログと台帳 4 本だけである。

use std::collections::{BTreeMap, BTreeSet};

use ukadoc_survey::assignment::PageAssignment;
use ukadoc_survey::io::{files, paths};
use ukadoc_survey::ledger::read::read as read_ledger;
use ukadoc_survey::model::{Domain, EntryId, PageName, Status};

use super::RepoData;

/// 要件文書の置き場（ワークスペース根からの相対）。
///
/// **`/kiro-complete` の書き換え対象**である。2026-09-05 の完了処理で spec 一式は
/// `.kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/` へ移り、その手順 5-2 が
/// `crates/` 全域から feature 名を grep してこの綴りを書き換えた。綴りをこの定数 1 つに
/// 集めてあるのはそのためで——本文中に散らすと、移動後にどれか 1 つが取り残されて
/// [`read_workspace_file`] が「探した絶対パス」を添えて止まる。
const REQUIREMENTS_MD: &str =
    ".kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/requirements.md";

/// 台帳の README（形式の説明の正本・要件 2.5）。
const README_MD: &str = "doc/ukadoc-coverage/README.md";

/// 要件文書の囲みの中にある id の件数の下限。
///
/// 2026-09-05 の実測はちょうど 4 件（別名の項目・その `alias_of` の指す先・
/// `system.year`・`queries` の指す先）。**下限**にしてあるのは、記入例が増えることは
/// 起こり得ても減ることは要件 2.6 の凍結に反するからで、増えた分も同じ検査を通る。
const REQUIREMENTS_ID_FLOOR: usize = 4;

/// README の囲みの中にある id の件数の下限。
///
/// 2026-09-05 の実測はちょうど 3 件（初期状態の見本 1 件と、書き終えた 1 項目の見本に
/// 現れる 2 件）。下限である理由は [`REQUIREMENTS_ID_FLOOR`] と同じ。
const README_ID_FLOOR: usize = 3;

// ---------------------------------------------------------------------------
// 取り出し
// ---------------------------------------------------------------------------

/// ` ```toml ` で始まり ` ``` ` で閉じる囲みの中身だけを、現れた順に返す。
///
/// 入れ子は扱わない（Markdown の囲みは入れ子にならない）。閉じ忘れた囲みは
/// 本文の終わりまでを 1 つの囲みとして返す——取りこぼして黙って緑になるより、
/// 中身を検査に掛けたほうが安全側である。
fn toml_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in markdown.lines() {
        match current.as_mut() {
            Some(lines) => {
                if line.trim_end() == "```" {
                    blocks.push(lines.join("\n"));
                    current = None;
                } else {
                    lines.push(line);
                }
            }
            None => {
                if line.trim_end() == "```toml" {
                    current = Some(Vec::new());
                }
            }
        }
    }
    if let Some(lines) = current {
        blocks.push(lines.join("\n"));
    }
    blocks
}

/// 本文に現れる `"ukadoc:…"` の綴りを、現れた順に返す（重複は落とさない）。
///
/// 二重引用符で囲まれた `ukadoc:` 始まりの文字列だけを拾う。台帳では id は必ず
/// 引用符の中に書かれる（`[entry."<id>"]` と `to = "<id>"`）ので、これで表の鍵も
/// 関連の相手も同じ規則で拾える。備考の複数行文字列（`"""`）の中に id 形の綴りが
/// 現れることは現データでは無いが、現れたとしても引用符で囲まれていなければ
/// 拾わない。
fn quoted_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("\"ukadoc:") {
        let after_quote = &rest[start + 1..];
        match after_quote.find('"') {
            Some(end) => {
                ids.push(after_quote[..end].to_owned());
                rest = &after_quote[end + 1..];
            }
            // 閉じ引用符が無い＝壊れた本文。ここで打ち切る。
            None => break,
        }
    }
    ids
}

/// 囲みの中の id を重複を落として文字順に返す。
fn ids_in_toml_blocks(markdown: &str) -> BTreeSet<String> {
    toml_blocks(markdown)
        .iter()
        .flat_map(|block| quoted_ids(block))
        .collect()
}

/// ワークスペース根からの相対パスで 1 本読む（復帰文字は入出力層が落とす）。
fn read_workspace_file(relative: &str) -> String {
    let path = paths::workspace_root().join(relative);
    files::read_normalized(&path).unwrap_or_else(|err| panic!("{err}"))
}

/// 付録 A.1 の記入例（台帳 1 ファイル分の形をした唯一の囲み）を返す。
fn requirements_ledger_example(markdown: &str) -> String {
    let mut found: Vec<String> = toml_blocks(markdown)
        .into_iter()
        .filter(|block| block.contains("[ledger]") && block.contains("[entry."))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "要件文書に台帳 1 ファイル分の記入例が 1 つだけあるはずが {} 件だった（{REQUIREMENTS_MD}）",
        found.len()
    );
    found.remove(0)
}

// ---------------------------------------------------------------------------
// 主張
// ---------------------------------------------------------------------------

/// 要件文書の囲みの中の id が全部カタログに実在する（要件 2.1・6.3・付録 B.5）。
#[test]
fn every_id_in_the_requirements_toml_blocks_exists_in_the_catalog() {
    let catalog = RepoData::load().catalog;
    let ids = ids_in_toml_blocks(&read_workspace_file(REQUIREMENTS_MD));

    assert!(
        ids.len() >= REQUIREMENTS_ID_FLOOR,
        "要件文書の囲みから取り出せた id が {} 件しかない（下限 {REQUIREMENTS_ID_FLOOR} 件）。\
         取り出しが空振りしていないか、記入例が消えていないかを見よ（{REQUIREMENTS_MD}）",
        ids.len()
    );

    let missing = missing_from_catalog(&ids, &catalog.entries);
    assert!(
        missing.is_empty(),
        "要件文書の記入例にカタログへ無い id がある（{REQUIREMENTS_MD} 付録 A.1）。\
         見た目で書き下さず catalog.toml から写すこと（付録 B.5）:\n{}",
        missing.join("\n")
    );
}

/// README の囲みの中の id が全部カタログに実在する（要件 2.5・6.3）。
#[test]
fn every_id_in_the_readme_toml_blocks_exists_in_the_catalog() {
    let catalog = RepoData::load().catalog;
    let ids = ids_in_toml_blocks(&read_workspace_file(README_MD));

    assert!(
        ids.len() >= README_ID_FLOOR,
        "README の囲みから取り出せた id が {} 件しかない（下限 {README_ID_FLOOR} 件）（{README_MD}）",
        ids.len()
    );

    let missing = missing_from_catalog(&ids, &catalog.entries);
    assert!(
        missing.is_empty(),
        "README の記入例にカタログへ無い id がある（{README_MD}）:\n{}",
        missing.join("\n")
    );
}

/// 付録 A.1 の記入例が、道具の台帳読み手でそのまま property の台帳として読める
/// （要件 2.1・付録 A.1）。
///
/// 突き合わせに使うのは道具自身の [`read_ledger`] であって、この場での TOML の
/// 読み直しではない。手本が「読める形」であることの証明は、実際に読み手を通す以外に
/// 無いからである。並び順も同じ理由で [`ukadoc_survey::ledger::Ledger::file_order`]
/// （本文に現れた順・設計 D-12）から取る。
#[test]
fn the_requirements_example_reads_as_a_property_ledger() {
    let example = requirements_ledger_example(&read_workspace_file(REQUIREMENTS_MD));
    let ledger = read_ledger(&example, Domain::Property)
        .unwrap_or_else(|err| panic!("付録 A.1 の記入例を台帳として読めない: {err}"));

    assert_eq!(
        ledger.entries.len(),
        2,
        "付録 A.1 の記入例は 2 項目（別名の見本と実装済みの見本）のはず"
    );

    let declared: BTreeSet<&PageName> = ledger.pages.iter().collect();
    let canonical = PageAssignment::canonical().pages_of(Domain::Property);
    let canonical: BTreeSet<&PageName> = canonical.iter().collect();
    assert_eq!(
        declared, canonical,
        "付録 A.1 の前置きの pages が、割り当て表の property の担当ページと食い違う（要件 3.1）"
    );

    for id in ledger.entries.keys() {
        assert!(
            declared.contains(&id.page()),
            "付録 A.1 の記入例に、前置きの pages に無いページの id がある: {}",
            id.as_str()
        );
    }

    assert!(
        ledger.file_order.windows(2).all(|pair| pair[0] < pair[1]),
        "付録 A.1 の記入例が id の文字順（byte 昇順）に並んでいない（付録 A.2）: {:?}",
        ledger
            .file_order
            .iter()
            .map(EntryId::as_str)
            .collect::<Vec<_>>()
    );
}

/// 記入例の `alias_of` の指す先が実在し、実台帳でそれが `alias` ではない（要件 6.7）。
///
/// 「別名の連鎖の禁止」は手本にこそ効く。手本が連鎖を作っていれば、4 本の台帳が
/// それを写して一斉に赤くなる。実在の確認（肯定）と、指す先が `alias` でないこと
/// （否定）を同じ 1 本に置いてあるのは、指す先が見つからないまま否定だけが成り立つ
/// 空振りを避けるためである。
#[test]
fn the_alias_example_points_at_an_entry_that_is_not_an_alias() {
    let repo = RepoData::load();
    let example = requirements_ledger_example(&read_workspace_file(REQUIREMENTS_MD));
    let ledger = read_ledger(&example, Domain::Property)
        .unwrap_or_else(|err| panic!("付録 A.1 の記入例を台帳として読めない: {err}"));

    let targets: Vec<&EntryId> = ledger
        .entries
        .values()
        .filter(|entry| entry.status == Status::Alias)
        .filter_map(|entry| entry.alias_of.as_ref())
        .collect();
    assert_eq!(
        targets.len(),
        1,
        "付録 A.1 の記入例には `status = \"alias\"` の見本が 1 件あるはず"
    );

    for target in targets {
        assert!(
            repo.catalog.entries.contains_key(target),
            "記入例の alias_of の指す先がカタログに無い: {}",
            target.as_str()
        );
        let found = repo
            .ledgers
            .iter()
            .find_map(|ledger| ledger.entries.get(target));
        let found = found.unwrap_or_else(|| {
            panic!(
                "記入例の alias_of の指す先が repo の台帳 4 本のどこにも無い: {}",
                target.as_str()
            )
        });
        assert_ne!(
            found.status,
            Status::Alias,
            "記入例の alias_of が別名を指している（別名の連鎖・要件 6.7）: {}",
            target.as_str()
        );
    }
}

/// 記入例の `links` の相手 id がカタログに実在する（要件 6.7 の「関連の両端」）。
#[test]
fn every_link_target_in_the_requirements_example_exists_in_the_catalog() {
    let repo = RepoData::load();
    let example = requirements_ledger_example(&read_workspace_file(REQUIREMENTS_MD));
    let ledger = read_ledger(&example, Domain::Property)
        .unwrap_or_else(|err| panic!("付録 A.1 の記入例を台帳として読めない: {err}"));

    let links: Vec<&EntryId> = ledger
        .entries
        .values()
        .flat_map(|entry| entry.links.iter().map(|link| &link.to))
        .collect();
    assert!(
        !links.is_empty(),
        "付録 A.1 の記入例には `links` の見本が 1 件以上あるはず（空だと この主張が空振りする）"
    );

    for to in links {
        assert!(
            repo.catalog.entries.contains_key(to),
            "記入例の links の相手 id がカタログに無い: {}",
            to.as_str()
        );
    }
}

/// 取り出しの較正——実在しない id を混ぜたら、それが名指しで挙がる。
///
/// これが無いと、[`toml_blocks`] や [`quoted_ids`] が壊れて 0 件を返すだけで、
/// 上の 4 本が「1 件も違反が無い」と言って緑になる。件数の下限だけでは、下限を
/// 満たしたうえで別の id を取りこぼす壊れ方を捕まえられない。
#[test]
fn the_id_scan_reports_an_id_that_the_catalog_does_not_have() {
    let catalog = RepoData::load().catalog;
    let real = "ukadoc:list_propertysystem:system.year:1";
    let fake = "ukadoc:list_propertysystem:balloon.scope(ID).width:1";

    // 囲みの外にも id 形の綴りを置いてある。囲みの中だけを見ていることの確認になる。
    let markdown = format!(
        "地の文に \"{fake}\" と書いても拾わない。\n\n```toml\n\
         [entry.\"{real}\"]\nstatus = \"unclassified\"\nlinks = [\n  \
         {{ kind = \"queries\", to = \"{fake}\" }},\n]\n```\n\n\
         ```text\n\"{fake}\"\n```\n"
    );

    let ids = ids_in_toml_blocks(&markdown);
    assert_eq!(
        ids.iter().map(String::as_str).collect::<Vec<_>>(),
        vec![fake, real],
        "toml の囲みの中の id 2 件だけを、文字順で拾うはず"
    );

    let missing = missing_from_catalog(&ids, &catalog.entries);
    assert_eq!(
        missing,
        vec![fake.to_owned()],
        "カタログに無いほうの id だけが挙がるはず"
    );
}

// ---------------------------------------------------------------------------
// 共通
// ---------------------------------------------------------------------------

/// カタログに無い id を文字順で返す。
///
/// id の形として読めない綴り（`EntryId::parse` が拒む形）も「無い」側に数える。
/// 手本としてはどちらも同じ結果——写した人の台帳が読み込みか検査で赤くなる——を
/// 招くからである。
fn missing_from_catalog<V>(ids: &BTreeSet<String>, catalog: &BTreeMap<EntryId, V>) -> Vec<String> {
    ids.iter()
        .filter(|raw| match EntryId::parse(raw) {
            Ok(id) => !catalog.contains_key(&id),
            Err(_) => true,
        })
        .cloned()
        .collect()
}
