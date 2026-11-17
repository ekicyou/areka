//! `priority-apply` の書き戻しが持つ「順番」と「1 バイトも書かない」を釘付けする
//! （要件 7.1・7.2・7.6・1.4・12.2）。`generate_tests.rs` の子のモジュールで、台帳の
//! 置き場の綴りと小道具をそちらと共有する。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;

use super::super::priority_apply_with;
use super::{LEDGER_FILES, under_coverage};
use crate::documents::fields::{self, document_file};
use crate::error::SurveyError;
use crate::io::files;
use crate::model::{Domain, EntryId};

// ---------------------------------------------------------------------------
// priority-apply（要件 7.1・7.2・7.6・1.4・12.2）
// ---------------------------------------------------------------------------
//
// 書き戻しは 2 文書と台帳 4 本を読んで 4 本を書き直す。ここで見るのはその**順番**
// （6 本を読み、導き、4 本の本文が全部決まってから書く）と、「1 バイトも書かない」
// という主張が本物であることである。読み手も書き手も引数で受けるので、ファイルを
// 1 つも作らずに走らせられる（[`priority_apply_with`]）。
//
// 実データは相手にしない。実データは束や順位の中身で結果が動くので、見本の本文だけを
// ここで組み立てる。

/// 見本の項目 id（[`Domain::ALL`] と同じ並び＝shiori・assets・sakura-script・property）。
const SAMPLE_IDS: [&str; 4] = [
    "ukadoc:list_shiori_event:OnBoot:1",
    "ukadoc:descript_shell:seriko.zorder:1",
    "ukadoc:list_sakura_script:_5c_65:1",
    "ukadoc:list_propertysystem:system.year:1",
];

/// 見本の束の名前。
const SAMPLE_BUNDLE: &str = "見本の束";

/// 見本の帰属と順位から決まる `priority`（段階 A の順位 1）。
const SAMPLE_PRIORITY: &str = "A1";

/// `linkage.md` の置き場（ワークスペース根からの相対）。
fn linkage_file() -> String {
    document_file(fields::LINKAGE_FILE)
}

/// `briefing.md` の置き場。
fn briefing_file() -> String {
    document_file(fields::BRIEFING_FILE)
}

/// 見本の台帳 1 本（項目 1 件）。備考は呼び手が決める。
fn sample_ledger(domain: Domain, id: &str, priority: &str, note: &str) -> String {
    let page = EntryId::parse(id)
        .expect("見本の id は要件 1.9 の 2 形のいずれかのはず")
        .page();
    format!(
        "# 見本の台帳（{domain_key}）\n\n\
         [ledger]\n\
         domain = \"{domain_key}\"\n\
         pages = [\"{page}\"]\n\n\
         [entry.\"{id}\"]\n\
         status = \"absent\"\n\
         introduced = \"\"\n\
         owner = \"\"\n\
         priority = \"{priority}\"\n\
         values = []\n\
         links = []\n\
         note = \"\"\"\n{note}\"\"\"\n",
        domain_key = domain.as_key(),
        page = page.as_str(),
    )
}

/// 見本の `linkage.md`。構成 id は呼び手が決める（外した id は帰属を失う）。
fn sample_linkage(members: &[&str]) -> String {
    let listed: String = members.iter().map(|id| format!("  \"{id}\",\n")).collect();
    format!(
        "# 束の名付けと帰属（見本）\n\n\
         ```toml\n\
         [bundle.\"{SAMPLE_BUNDLE}\"]\n\
         machine = []\n\
         members = [\n{listed}]\n\
         hand = []\n\
         domains = [\"shiori\", \"assets\", \"sakura-script\", \"property\"]\n\
         foundation = \"見本の基盤\"\n\
         breakage = \"黙って壊れる\"\n\
         themes = []\n\
         ```\n\n\
         ```toml\n\
         [tally]\n\
         target = 4\n\
         from_machine = 4\n\
         by_hand = 0\n\
         singles = 0\n\
         alias_excluded = 0\n\
         not_applicable_excluded = 0\n\n\
         [tally.singles_by_domain]\n\
         shiori = 0\n\
         assets = 0\n\
         sakura-script = 0\n\
         property = 0\n\
         ```\n"
    )
}

/// 見本の `briefing.md`（段階 A の順位 1 に見本の束を置くだけ）。
fn sample_briefing() -> String {
    let stages: String = ["A", "B", "C", "D", "E"]
        .iter()
        .map(|stage| format!("[stage.{stage}]\nbundles = 0\nsingles = 0\nitems = 0\n\n"))
        .collect();
    format!(
        "# 段階と順位（見本）\n\n\
         ```toml\n{stages}```\n\n\
         ```toml\n\
         [[rank]]\n\
         stage = \"A\"\n\
         rank = 1\n\
         bundle = \"{SAMPLE_BUNDLE}\"\n\
         ```\n\n\
         ```toml\n\
         [priority_blank]\n\
         alias = 0\n\
         not_applicable = 0\n\
         ```\n"
    )
}

/// 見本の 6 文書。鍵は `doc/ukadoc-coverage/` から先の綴り。
///
/// `members` は帰属に載せる id、`property_note` は 4 本目の台帳の備考である
/// （備考の**行頭**に `priority = ` を置くと、その台帳の本文だけが決まらなくなる）。
fn sample_files(members: &[&str], property_note: &str) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    files.insert(linkage_file(), sample_linkage(members));
    files.insert(briefing_file(), sample_briefing());
    for (index, domain) in Domain::ALL.into_iter().enumerate() {
        let note = if domain == Domain::Property {
            property_note
        } else {
            ""
        };
        files.insert(
            LEDGER_FILES[index].to_owned(),
            sample_ledger(domain, SAMPLE_IDS[index], "", note),
        );
    }
    files
}

/// 書き手が受け取った 1 本ぶん（場所・本文・変更件数）。
type Written = (String, String, usize);

/// 見本を読ませて書き戻しを 1 度走らせる。ファイルは 1 つも作らない。
///
/// 返るのは ⑴ 走行の結果、⑵ 読みに来た場所の並び、⑶ 書き手が受け取った並びである。
fn apply_over(
    files: &BTreeMap<String, String>,
) -> (Result<(), SurveyError>, Vec<String>, Vec<Written>) {
    let asked: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let read = |path: &Path| -> Result<String, SurveyError> {
        let spelled = under_coverage(path);
        asked.borrow_mut().push(spelled.clone());
        files
            .get(&spelled)
            .cloned()
            .ok_or_else(|| files::io_error(path, "見本に無い"))
    };

    let mut written: Vec<Written> = Vec::new();
    let outcome = {
        let mut record = |target: &Path, body: &str, changed: usize| -> Result<(), SurveyError> {
            written.push((under_coverage(target), body.to_owned(), changed));
            Ok(())
        };
        priority_apply_with(&read, &mut record)
    };
    (outcome, asked.into_inner(), written)
}

#[test]
fn the_derived_priority_lands_in_every_one_of_the_four_ledgers() {
    // 対の腕（母数 0 の緑を潰す）。冪等の主張だけを置くと、1 行も書き換えない実装でも
    // 緑になる。まず「確かに書き換わる」を見る。
    let (outcome, asked, written) = apply_over(&sample_files(&SAMPLE_IDS, ""));
    outcome.expect("見本の 2 文書と台帳 4 本なら書き戻せるはず");

    let mut expected_reads = vec![linkage_file(), briefing_file()];
    expected_reads.extend(LEDGER_FILES.iter().map(|file| (*file).to_owned()));
    assert_eq!(asked, expected_reads, "読みに来た文書が 6 本と違う");

    let places: Vec<&str> = written.iter().map(|(place, _, _)| place.as_str()).collect();
    assert_eq!(places, LEDGER_FILES, "書き出す先が台帳 4 本と違う");

    let filled = format!("priority = \"{SAMPLE_PRIORITY}\"");
    for (index, (place, body, changed)) in written.iter().enumerate() {
        assert_eq!(*changed, 1, "{place} の変更件数が 1 件でない");
        assert!(
            body.contains(&filled),
            "{place} に確定値が入っていない: {body}"
        );
        // 置き換えるのは priority の 1 行だけ（要件 12.2）。戻せば元の本文に一致する。
        let before = sample_ledger(Domain::ALL[index], SAMPLE_IDS[index], "", "");
        assert_eq!(
            body.replace(&filled, "priority = \"\""),
            before,
            "{place} の priority 以外の行が変わっている"
        );
    }
}

#[test]
fn re_running_the_write_back_changes_no_item_and_leaves_the_bodies_byte_for_byte() {
    // 冪等（設計「入口 / cli::generate::priority_apply」）。1 度目の本文をそのまま
    // 食わせて、変更件数が 0 で本文が 1 バイトも変わらないことを見る。
    let (outcome, _, first) = apply_over(&sample_files(&SAMPLE_IDS, ""));
    outcome.expect("1 度目が通らないと 2 度目の主張が立たない");

    let mut second_input = sample_files(&SAMPLE_IDS, "");
    for (place, body, _) in &first {
        second_input.insert(place.clone(), body.clone());
    }

    let (outcome, _, second) = apply_over(&second_input);
    outcome.expect("2 度目も通るはず");

    assert_eq!(second.len(), 4, "2 度目に書き出した本数が 4 本でない");
    for (index, (place, body, changed)) in second.iter().enumerate() {
        assert_eq!(*changed, 0, "2 度目なのに {place} の変更件数が 0 でない");
        assert_eq!(
            body, &first[index].1,
            "2 度目の {place} の本文が 1 バイト以上変わっている"
        );
    }
}

#[test]
fn an_id_that_belongs_to_no_bundle_stops_the_run_before_a_single_write() {
    // 導出に失敗する id があれば挙げて止まり、1 本も書かない（要件 7.1・1.4）。
    // 4 本目（property）の id だけを帰属から外す。
    let (outcome, asked, written) = apply_over(&sample_files(&SAMPLE_IDS[..3], ""));
    let err = outcome.expect_err("帰属の無い id があるのに書き戻せてしまった");

    assert!(written.is_empty(), "導出に失敗したのに書き出している");
    assert_eq!(asked.len(), 6, "6 本を読み終える前に落ちている");
    let body = err.to_string();
    assert!(
        body.contains(SAMPLE_IDS[3]),
        "どの id が導けないかが本文に無い: {body}"
    );
}

#[test]
fn a_body_that_cannot_be_decided_on_the_fourth_ledger_leaves_the_first_three_unwritten() {
    // 4 本の本文が全部決まってから書く（設計「入口 / cli::generate::priority_apply」）。
    // 4 本目の備考の**行頭**に `priority = ` を置くと、その台帳だけが「どの行を直す
    // べきか決められない」で落ちる。順番が逆なら、先に決まっている 3 本が書き換わって
    // 残る。
    let (outcome, _, written) = apply_over(&sample_files(&SAMPLE_IDS, "priority = \"備考の中\"\n"));
    let err = outcome.expect_err("置き換える行を決められないのに書き戻せてしまった");

    assert!(written.is_empty(), "4 本が決まる前に書き始めている");
    let body = err.to_string();
    assert!(
        body.contains(SAMPLE_IDS[3]),
        "どの項目で決められなかったかが本文に無い: {body}"
    );
}

#[test]
fn a_document_that_cannot_be_read_stops_with_the_absolute_path_it_looked_for() {
    // 1 本でも読めなければ探した絶対パスを添えて止まり、1 バイトも書かない
    // （要件 1.4）。読み手の失敗を包み直して場所を落とすと、ここが赤くなる。
    for missing in [linkage_file(), briefing_file(), LEDGER_FILES[3].to_owned()] {
        let mut files = sample_files(&SAMPLE_IDS, "");
        files.remove(&missing);
        let (outcome, _, written) = apply_over(&files);
        let err = outcome.expect_err("読めない文書があるのに書き戻せてしまった");
        assert!(written.is_empty(), "{missing} が読めないのに書き出している");
        let body = err.to_string().replace('\\', "/");
        assert!(
            body.contains(&missing),
            "探した場所が失敗の本文に無い（{missing}）: {body}"
        );
    }
}
