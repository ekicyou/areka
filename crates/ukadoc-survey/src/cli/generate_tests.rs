//! 生成の副手続きが持つ「判断」と「順番」を釘付けする。
//!
//! カタログ再生成はスナップショットが要るので、ここでは走らせない（完了条件は
//! 実データでの通し確認の側にある）。台帳の初期化は取り寄せと書き手を引数で受けるので、
//! 書き手を「控えるだけ」に差し替えればファイルを 1 つも作らずに走らせられる。
//! このファイルが確かめるのは次の箇条で、はじめの 2 つは入出力に 1 度も触れずに決まる。
//!
//! - どの id をどの台帳へ入れるか（[`select_ids_by_domain`]）
//! - 既存の台帳の本文を差し込みへ渡しているか（[`plan_ledger_init`]）
//! - 既存の台帳を実際に取り寄せられるか（[`read_if_present`]）
//! - repo の台帳 4 本が再実行で 1 バイトも変わらないか（[`ledger_init_with`]）
//! - 4 本の本文が全部決まるまで 1 本も書き始めないか（[`ledger_init_with`]）
//! - 書き手の失敗を握り潰さず、そのまま返すか（[`ledger_init_with`]）
//! - 4 本を全部書き終えてから告げるか（[`ledger_init_with`]）
//!
//! 台帳の前置きに担当ページを並べる順（名前順）は `assignment_tests.rs` が逐語で
//! 固定している。2 つ目と 3 つ目が守る壊れ方は 4 つ目の再実行の主張も捕まえるが、
//! それは repo の台帳に手で書いた値が載っているからで、台帳の中身に頼らずに立つよう
//! 手で書いた見本と追跡ファイルで別に確かめる——理由はこのファイルの後半に書いた。
//!
//! # 作らない。ただし repo の追跡ファイルは読む
//!
//! ファイルを 1 つも作らず、一時ディレクトリも使わない（禁じてあるのは**作ること**である）。
//! スナップショットも読まない。
//!
//! **repo に追跡されているファイルを読むことは禁じられていない。** スナップショットが
//! その環境に無くても整合検査は赤にならない（repo 内のカタログを正本として検査する）
//! ので、repo のファイルを読むことをむしろ前提にしている。
//! 在中テストが repo の追跡ファイルを読む前例も既にある（`io/files_tests.rs` が
//! `Cargo.toml` を読む）。3 つ目から後の主張はこの読みだけを使う。

// `priority-apply` の節。共有の定数と小道具は子のこのモジュールにだけ見える。
#[path = "generate_priority_tests.rs"]
mod priority;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{
    LedgerPlan, ledger_init_with, plan_ledger_init, read_if_present, select_ids_by_domain,
};
use crate::assignment::PageAssignment;
use crate::catalog::{Catalog, CatalogEntry};
use crate::error::SurveyError;
use crate::io::{files, paths};
use crate::ledger::write::render_initial_entry;
use crate::lib_test_support;
use crate::model::{Domain, EntryId, PageName};

/// 仕分けの結果を「ドメインの綴り → id の綴りの一覧」に均す小道具。
fn spelled(buckets: &BTreeMap<Domain, Vec<EntryId>>) -> BTreeMap<&'static str, Vec<&str>> {
    buckets
        .iter()
        .map(|(domain, ids)| {
            (
                domain.as_key(),
                ids.iter().map(EntryId::as_str).collect::<Vec<&str>>(),
            )
        })
        .collect()
}

#[test]
fn each_sample_id_lands_in_the_ledger_that_owns_its_page() {
    // 件数だけを確かめると、12 件が全部よそのドメインへ入っていても緑になる。
    // どの id がどの台帳へ行くかを綴りで書く。
    let catalog = lib_test_support::catalog();
    let buckets = select_ids_by_domain(&catalog, &PageAssignment::canonical())
        .expect("見本のカタログはすべて割り当て済みのページのはず");

    let expected: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
        (
            "shiori",
            vec![
                "ukadoc:list_shiori_event:OnBoot:1",
                "ukadoc:list_shiori_event:OnClose:1",
                "ukadoc:spec_shiori3",
            ],
        ),
        (
            "assets",
            vec![
                "ukadoc:descript_ghost:charset:1",
                "ukadoc:descript_ghost:name:1",
                "ukadoc:manual_shell",
            ],
        ),
        (
            "sakura-script",
            vec![
                "ukadoc:list_sakura_script:_5c_5f_71:1",
                "ukadoc:list_sakura_script:_5c_65:1",
                "ukadoc:list_sakura_script:_5c_73_5bID_5d:1",
            ],
        ),
        (
            "property",
            vec![
                "ukadoc:list_propertysystem:currentghost.name:1",
                "ukadoc:list_propertysystem:system.month:1",
                "ukadoc:list_propertysystem:system.year:1",
            ],
        ),
    ]);
    assert_eq!(spelled(&buckets), expected, "id の行き先が割り当て表と違う");
}

#[test]
fn a_domain_that_owns_no_entry_still_gets_a_bucket() {
    // 台帳は 4 本と決まっている。項目 0 件のドメインの鍵が消えると、
    // その台帳だけが書き出されずに黙って欠ける。
    let mut catalog = lib_test_support::catalog();
    catalog
        .entries
        .retain(|id, _| id.page() == PageName::new("list_propertysystem"));
    let buckets = select_ids_by_domain(&catalog, &PageAssignment::canonical())
        .expect("見本のカタログはすべて割り当て済みのページのはず");

    for domain in Domain::ALL {
        let ids = buckets
            .get(&domain)
            .unwrap_or_else(|| panic!("ドメイン {} の鍵が無い", domain.as_key()));
        let expected = if domain == Domain::Property { 3 } else { 0 };
        assert_eq!(
            ids.len(),
            expected,
            "ドメイン {} の件数が違う",
            domain.as_key()
        );
    }
}

#[test]
fn an_entry_whose_page_belongs_to_no_ledger_is_named_and_refused() {
    // 割り当ての無いページの id を黙って落とすと、件数が減るだけで何も言わない。
    let mut catalog: Catalog = lib_test_support::catalog();
    let stray = EntryId::parse("ukadoc:list_no_such_page:thing:1").expect("2 形のはず");
    catalog.entries.insert(
        stray.clone(),
        CatalogEntry {
            page: stray.page(),
            title: "thing".to_owned(),
            category: "misc".to_owned(),
            versions: Vec::new(),
            hash: "9999999999999999".to_owned(),
            url: "https://ssp.shillest.net/ukadoc/manual/list_no_such_page.html#thing:1".to_owned(),
            id: stray,
        },
    );

    let err = select_ids_by_domain(&catalog, &PageAssignment::canonical())
        .expect_err("割り当ての無いページが黙って落とされた");
    assert!(
        matches!(err, SurveyError::PageNotAssigned { .. }),
        "別の失敗になっている: {err}"
    );
    let body = err.to_string();
    assert!(
        body.contains("list_no_such_page"),
        "どのページが割り当て無しなのかが本文に無い: {body}"
    );
}

// ---------------------------------------------------------------------------
// 既存本文を渡している配線
// ---------------------------------------------------------------------------
//
// 既存本文を渡し忘れると、手で書き込んだ状態・担当・優先度を次の `ledger-init` が
// 黙って初期値へ戻す。下の再実行の主張（repo の台帳との比較）もこれを捕まえるが、
// それは repo の台帳に手で書いた値が載っているあいだだけである。
//
// そこで [`plan_ledger_init`] は既存本文の取り寄せを引数で受ける。ここでは手で
// 書いた本文を渡し、その綴りが返る本文に生き残ることを確かめる。ファイルは
// 1 つも作らず、一時ディレクトリも使わない。

/// 台帳の置き場（実装の `io::paths` を引かず、独立した綴りで書く）。
const LEDGER_FILES: [&str; 4] = [
    "doc/ukadoc-coverage/ledger/shiori.toml",
    "doc/ukadoc-coverage/ledger/assets.toml",
    "doc/ukadoc-coverage/ledger/sakura-script.toml",
    "doc/ukadoc-coverage/ledger/property.toml",
];

/// property の台帳に手で書いた本文。生成器はこの綴りを 1 つも作らない
/// （欄の並び替え・余分な空白・複数行の備考・末尾の改行なし）。
///
/// 3 件のうち **真ん中** の `system.month` だけが欠けている。
const HAND_PROPERTY: &str = concat!(
    "# 手で書いた台帳。生成器が作らない綴りを混ぜてある。\n",
    "\n",
    "[ledger]\n",
    "domain   = \"property\"\n",
    "pages    = [ \"list_propertysystem\" ]\n",
    "\n",
    "[entry.\"ukadoc:list_propertysystem:currentghost.name:1\"]\n",
    "owner = \"areka-P0-ukadoc-survey-property\"\n",
    "status = \"implemented\"\n",
    "introduced = \"2.4.03\"\n",
    "priority = \"A1\"\n",
    "values = [ \"対話の手触り\" ]\n",
    "links = []\n",
    "note = \"\"\"\n",
    "手で書いた備考の 1 行目。\n",
    "2 行目。\n",
    "\"\"\"\n",
    "\n",
    "[entry.\"ukadoc:list_propertysystem:system.year:1\"]\n",
    "status=\"degraded\"\n",
    "owner=\"areka-P0-ukadoc-survey-property\"\n",
    "priority=\"C3\"\n",
    "note=\"末尾に改行を置かない書き方\"",
);

/// 差し込みの位置になる見出し行（`HAND_PROPERTY` の 2 つ目の塊の先頭）。
const PROPERTY_SECOND_HEAD: &str = "[entry.\"ukadoc:list_propertysystem:system.year:1\"]";

/// sakura-script の台帳に手で書いた本文。塊と塊の間に空行を置かず、末尾にも
/// 改行を置かない。3 件のうち **最後** の `_5c_73_5bID_5d` だけが欠けている。
const HAND_SAKURA: &str = concat!(
    "# 手で書いた台帳（空行を 1 つも置かない書き方）。\n",
    "[ledger]\n",
    "domain = \"sakura-script\"\n",
    "pages = [\"list_sakura_script\"]\n",
    "[entry.\"ukadoc:list_sakura_script:_5c_5f_71:1\"]\n",
    "status = \"absent\"\n",
    "owner = \"areka-P0-ukadoc-survey-sakura-script\"\n",
    "note = \"手書き 1\"\n",
    "[entry.\"ukadoc:list_sakura_script:_5c_65:1\"]\n",
    "status = \"vocabulary-only\"\n",
    "owner = \"areka-P0-ukadoc-survey-sakura-script\"\n",
    "note = \"手書き 2\"",
);

/// パスを「区切りが `/` の綴り」に均す。どの計算機でも同じ比較になる。
fn spelled_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// `doc/ukadoc-coverage/` から先だけを取り出す。
///
/// 場所はワークスペース根からの絶対パスで来るので、計算機ごとに違う頭を落とす。
/// その綴りが 1 度も現れなければ、落とさずに全部を返す——別の場所を指す壊し方を
/// 「合っている」と誤魔化さないためである。
fn under_coverage(path: &Path) -> String {
    let spelled = spelled_path(path);
    match spelled.find("doc/ukadoc-coverage/") {
        Some(at) => spelled[at..].to_owned(),
        None => spelled,
    }
}

/// 見本の表（`doc/ukadoc-coverage/` から先の綴り → 本文）から読む読み手。表に無い場所は読めない。
fn reader_over(
    files: &BTreeMap<String, String>,
) -> impl Fn(&Path) -> Result<String, SurveyError> + '_ {
    move |path| {
        files
            .get(&under_coverage(path))
            .cloned()
            .ok_or_else(|| files::io_error(path, "見本に無い"))
    }
}

/// 書き手（場所と本文を受ける）。
type WriteFn<'a> = dyn FnMut(&Path, &str) -> Result<(), SurveyError> + 'a;

/// 告げ手（場所と件数を受ける）。
type SayFn<'a> = dyn FnMut(&Path, usize) -> Result<(), SurveyError> + 'a;

/// 告げ手が 2 本目で落ちる（出力の先が途中で閉じた）ときに、書き手が受け取った場所。
fn written_when_the_second_announcement_fails(
    run: &mut dyn FnMut(&mut WriteFn<'_>, &mut SayFn<'_>) -> Result<(), SurveyError>,
) -> (Result<(), SurveyError>, Vec<String>) {
    let mut written: Vec<String> = Vec::new();
    let mut said = 0_usize;
    let outcome = {
        let mut record = |target: &Path, _: &str| -> Result<(), SurveyError> {
            written.push(under_coverage(target));
            Ok(())
        };
        let mut say = |target: &Path, _: usize| -> Result<(), SurveyError> {
            said += 1;
            if said == 2 {
                return Err(files::io_error(target, "告げる先が閉じた"));
            }
            Ok(())
        };
        run(&mut record, &mut say)
    };
    (outcome, written)
}

/// 4 本ぶんの計画を組み立てる。渡した本文と、取り寄せに聞かれた場所を返す。
fn plan_with(hand: &[(&str, &str)]) -> (Vec<LedgerPlan>, Vec<String>) {
    let asked: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let bodies: Vec<(String, String)> = hand
        .iter()
        .map(|(file, body)| ((*file).to_owned(), (*body).to_owned()))
        .collect();

    let read = |path: &Path| -> Result<Option<String>, SurveyError> {
        let spelled = under_coverage(path);
        asked.borrow_mut().push(spelled.clone());
        Ok(bodies
            .iter()
            .find(|(file, _)| spelled.ends_with(file.as_str()))
            .map(|(_, body)| body.clone()))
    };

    let catalog = lib_test_support::catalog();
    let plans = plan_ledger_init(&catalog, &PageAssignment::canonical(), &read)
        .expect("見本のカタログと手書きの本文なら計画は立つはず");
    let asked = asked.into_inner();
    (plans, asked)
}

#[test]
fn the_plan_asks_for_each_ledger_at_its_own_place_and_writes_back_there() {
    // 取り寄せに渡す場所を間違える壊し方（カタログの場所を渡す・4 本を同じ場所で
    // 読む）は、本文の比較だけでは見えないことがある。聞かれた場所そのものを見る。
    let (plans, asked) = plan_with(&[]);

    assert_eq!(
        asked, LEDGER_FILES,
        "既存本文を取り寄せた場所が台帳 4 本と違う"
    );
    let targets: Vec<String> = plans
        .iter()
        .map(|plan| under_coverage(&plan.target))
        .collect();
    assert_eq!(targets, LEDGER_FILES, "書き出す先が台帳 4 本と違う");
    let counts: Vec<usize> = plans.iter().map(|plan| plan.count).collect();
    assert_eq!(
        counts,
        vec![3, 3, 3, 3],
        "見本のカタログはドメインごとに 3 件のはず"
    );
}

#[test]
fn a_hand_written_ledger_survives_the_plan_byte_for_byte() {
    // 既存本文を渡し忘れると、repo の台帳の中身に関わらずこの
    // 主張が赤くなる。
    let (plans, _) = plan_with(&[
        (LEDGER_FILES[3], HAND_PROPERTY),
        (LEDGER_FILES[2], HAND_SAKURA),
    ]);

    // property: 真ん中の 1 件が、手書きの塊を 1 バイトも動かさずに差し込まれる。
    let month = EntryId::parse("ukadoc:list_propertysystem:system.month:1").expect("2 形のはず");
    let inserted = render_initial_entry(&month);
    let expected = HAND_PROPERTY.replace(
        PROPERTY_SECOND_HEAD,
        &format!("{inserted}{PROPERTY_SECOND_HEAD}"),
    );
    assert_eq!(
        plans[3].body, expected,
        "property の手書き本文が写されていない"
    );

    // sakura-script: 最後の 1 件。末尾に改行が無い本文へ改行 1 つだけを足して繋ぐ。
    let last = EntryId::parse("ukadoc:list_sakura_script:_5c_73_5bID_5d:1").expect("2 形のはず");
    let expected = format!("{HAND_SAKURA}\n{}", render_initial_entry(&last));
    assert_eq!(
        plans[2].body, expected,
        "sakura-script の手書き本文が写されていない"
    );
}

#[test]
fn a_ledger_with_no_existing_body_gets_the_generated_prologue() {
    // 既存が無いドメインは新規に組み立てる。前置きの担当ページを
    // 渡し忘れる壊し方をここで捕まえる。
    let (plans, _) = plan_with(&[(LEDGER_FILES[3], HAND_PROPERTY)]);

    assert!(
        plans[0]
            .body
            .starts_with("# doc/ukadoc-coverage/ledger/shiori.toml\n"),
        "shiori の前置きが生成されていない: {}",
        plans[0].body.chars().take(80).collect::<String>()
    );
    assert!(
        plans[0].body.contains("\"list_plugin_event\""),
        "shiori の前置きに担当ページが入っていない"
    );
    assert!(
        !plans[3]
            .body
            .starts_with("# doc/ukadoc-coverage/ledger/property.toml\n"),
        "手書きの前置きが生成された前置きへ置き換わっている"
    );
}

// ---------------------------------------------------------------------------
// 取り寄せそのもの
// ---------------------------------------------------------------------------
//
// 上の一群は「呼び手が渡した取り寄せの結果を差し込みへ回しているか」を見る。渡す中身
// ——実在すれば本文を返し、無ければ `None` を返す——は [`read_if_present`] が持つ。
//
// ここは判断の分岐なので釘付けする。`Ok(true)` の枝を `Ok(None)` へ潰すと、実在する
// 台帳に「無い」と答えるようになり、次の `ledger-init` が既存の記入をすべて初期値へ
// 戻す。
//
// repo に追跡されている台帳を **読むだけ** である。ファイルは作らず、一時ディレクトリも
// 使わない。

#[test]
fn read_if_present_returns_the_committed_ledger_and_none_for_a_missing_sibling() {
    let path = paths::ledger_path(Domain::Property);
    let body = read_if_present(&path)
        .expect("repo に追跡されている台帳は読めるはず")
        .unwrap_or_else(|| panic!("実在する台帳に None が返った: {}", path.display()));

    // 空の本文やよそのファイルで誤魔化されないよう、その台帳自身の 1 行目を見る。
    assert!(
        body.starts_with("# doc/ukadoc-coverage/ledger/property.toml\n"),
        "返った本文が property の台帳のものではない: {}",
        body.lines().next().unwrap_or_default()
    );
    assert!(
        body.contains("domain = \"property\""),
        "返った本文に台帳の前置きが無い"
    );

    // 1 つの台帳だけを見ると、置き場の実在だけ確かめて本文はいつも property から読む
    // 取り違えを通してしまう。読む先が引数で決まっていることを 2 本目で確かめる。
    let other = paths::ledger_path(Domain::Shiori);
    let other_body = read_if_present(&other)
        .expect("repo に追跡されている台帳は読めるはず")
        .unwrap_or_else(|| panic!("実在する台帳に None が返った: {}", other.display()));
    assert!(
        other_body.starts_with("# doc/ukadoc-coverage/ledger/shiori.toml\n"),
        "返った本文が shiori の台帳のものではない: {}",
        other_body.lines().next().unwrap_or_default()
    );

    // 実在しない兄弟は `None`。ここが `Some` になると、無いはずの本文を写す。
    let missing = path.with_file_name("property.toml.not-a-file");
    assert!(
        !missing.exists(),
        "この置き場は実在してはならない: {}",
        missing.display()
    );
    assert_eq!(
        read_if_present(&missing).expect("実在しない置き場でも失敗しない"),
        None,
        "実在しない置き場に本文が返った"
    );
}

// ---------------------------------------------------------------------------
// 4 本を決めてから書く
// ---------------------------------------------------------------------------
//
// ここまでの一群が見るのは計画の中身だけである。残るのは [`ledger_init_with`] が持つ
// **順番**——4 本ぶんの本文が全部決まってから書き始め、4 本を書き終えてから告げる——と、
// 決めた本文をそのまま書き手へ渡していることである。
//
// 書き手を引数で受けるので、ファイルを 1 つも作らずに走らせられる。書き出しの実物
// （置き場を作って `files::write_lf` を呼ぶ）を渡すのは `ledger_init()` の 1 行だけで、
// そちらはここから呼ばない——呼べば repo の台帳 4 本を実際に書く。

#[test]
fn re_running_the_init_writes_the_committed_ledgers_with_the_very_same_bytes() {
    // 不動点。`ledger-init` を今もう一度走らせても、repo にある台帳は
    // 1 バイトも変わらない。実際のカタログと実際の取り寄せを使い、書き手だけを
    // 「控えるだけ」に差し替える。ファイルは 1 つも作らない。
    //
    // 空振りしていない理由: カタログの id が 1 つでも台帳に欠けていれば、計画は
    // その塊を差し込むので本文が食い違う。逆にここが押さえないのは `ledger_init()`
    // の 1 行——どの取り寄せとどの書き手を渡すか——である。
    let mut written: Vec<(PathBuf, String)> = Vec::new();
    {
        let mut record = |target: &Path, body: &str| -> Result<(), SurveyError> {
            written.push((target.to_path_buf(), body.to_owned()));
            Ok(())
        };
        ledger_init_with(&read_if_present, &mut record, &mut |_: &Path, _: usize| {
            Ok(())
        })
        .expect("repo のカタログと台帳なら計画も書き出しも通るはず");
    }

    assert_eq!(written.len(), 4, "書き出した本数が台帳 4 本でない");
    let places: Vec<String> = written
        .iter()
        .map(|(target, _)| under_coverage(target))
        .collect();
    assert_eq!(places, LEDGER_FILES, "書き出す先が台帳 4 本と違う");

    for (index, domain) in Domain::ALL.into_iter().enumerate() {
        let (target, body) = &written[index];
        assert_eq!(
            target,
            &paths::ledger_path(domain),
            "{} の書き出し先が違う",
            domain.as_key()
        );

        // 空の本文どうしを比べて緑になる形を先に潰す。
        let head = format!("# doc/ukadoc-coverage/ledger/{}.toml\n", domain.as_key());
        assert!(
            body.starts_with(&head),
            "{} の本文の 1 行目が台帳のものでない: {}",
            domain.as_key(),
            body.lines().next().unwrap_or_default()
        );

        let committed =
            files::read_normalized(target).expect("repo に追跡されている台帳は読めるはず");
        assert_eq!(
            body,
            &committed,
            "{} の台帳が書き換わる（再実行で不動でない）",
            domain.as_key()
        );
    }
}

#[test]
fn a_failure_on_the_fourth_ledger_leaves_the_first_three_unwritten() {
    // 4 本目（property）の取り寄せで落ちたとき、先に本文が決まっている 3 本を
    // 書いてしまうと、走行は失敗したのに repo の台帳だけが書き換わって残る。
    // 取り寄せに聞かれた場所を控えて、「3 本ぶんは決まっていた」と
    // 「それでも 1 本も書いていない」を対で書く。
    let asked: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let read = |path: &Path| -> Result<Option<String>, SurveyError> {
        asked.borrow_mut().push(under_coverage(path));
        if spelled_path(path).ends_with("property.toml") {
            return Err(SurveyError::Io {
                path: path.display().to_string(),
                reason: "4 本目の取り寄せをここで落とした".to_owned(),
            });
        }
        read_if_present(path)
    };

    let mut writes = 0_usize;
    let err = {
        let mut count = |_: &Path, _: &str| -> Result<(), SurveyError> {
            writes += 1;
            Ok(())
        };
        ledger_init_with(&read, &mut count, &mut |_: &Path, _: usize| Ok(()))
            .expect_err("4 本目が読めないのに成功した")
    };

    assert_eq!(
        asked.into_inner(),
        LEDGER_FILES,
        "4 本目の取り寄せまで進んでいない（3 本ぶんは決まっていたはず）"
    );
    assert_eq!(writes, 0, "4 本が決まる前に書き始めている");
    let body = err.to_string();
    assert!(
        body.contains("property.toml"),
        "どの台帳で落ちたかが本文に無い: {body}"
    );
}

#[test]
fn a_failure_from_the_writer_comes_back_unchanged() {
    // 書き手の失敗を握り潰して次の台帳へ進むと、書けなかった台帳があることを
    // 誰も知らないまま終了コード 0 で終わる。
    const MARKER: &str = "書き手をここで落とした";

    let mut attempts: Vec<String> = Vec::new();
    let err = {
        let mut refuse = |target: &Path, _: &str| -> Result<(), SurveyError> {
            attempts.push(under_coverage(target));
            Err(SurveyError::Io {
                path: target.display().to_string(),
                reason: MARKER.to_owned(),
            })
        };
        ledger_init_with(&read_if_present, &mut refuse, &mut |_: &Path, _: usize| {
            Ok(())
        })
        .expect_err("書き手が落ちたのに成功した")
    };

    assert_eq!(
        attempts,
        vec![LEDGER_FILES[0]],
        "書き手を 1 本目で止めていない"
    );
    match err {
        SurveyError::Io { path, reason } => {
            assert_eq!(reason, MARKER, "書き手の理由が別の本文へ差し替わっている");
            assert!(
                spelled_path(Path::new(&path)).ends_with(LEDGER_FILES[0]),
                "書き手へ渡した場所が失敗に載っていない: {path}"
            );
        }
        other => panic!("書き手の失敗が別の失敗に化けている: {other}"),
    }
}

#[test]
fn a_closed_output_on_the_second_announcement_still_leaves_all_four_ledgers_written() {
    // 1 本書くたびに告げると、2 本目の告知で落ちたとき（出力の先が途中で閉じた）に
    // 3・4 本目が書かれずに残る。告げるのは 4 本を書き終えてから。
    let (outcome, written) = written_when_the_second_announcement_fails(&mut |write, say| {
        ledger_init_with(&read_if_present, write, say)
    });
    outcome.expect_err("告げる先が閉じたのに成功した");
    assert_eq!(written, LEDGER_FILES, "告げる前に書き終えていない");
}
