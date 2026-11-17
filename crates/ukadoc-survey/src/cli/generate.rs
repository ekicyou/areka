//! 生成側の副手続き（`catalog`・`ledger-init`・`report`・`report-summary`・
//! `priority-apply`）。
//!
//! どれも「読む → 組み立てる → 丸ごと書き出す」の 3 段でできている。判断は純粋層に
//! あり、ここが持つのは**順番**だけである（設計「入口 / cli」）。
//!
//! # 読めてから初めて置き場を作る（要件 1.8）
//!
//! `doc/ukadoc-coverage/` は新しい checkout には無いので、書き出しの前にこちらで
//! 作る（[`crate::io::files::write_lf`] は親ディレクトリを作らない）。ただし**作るのは
//! 読み終えた後**である。スナップショットが読めないときに 1 バイトも書かないという
//! 取り決めは、空のディレクトリを掘ることも含む——先に作ると、実在しない場所を指して
//! 失敗させただけで repo に空の木が残る。
//!
//! # 書き出しは丸ごと入れ替える
//!
//! カタログと報告は機械生成の文書なので、部分更新はしない（設計「System Flows /
//! カタログ再生成」）。台帳だけは人が書く文書なので、既存の塊をバイト列のまま写して
//! 欠けた id だけを差し込む（要件 3.3a。差し込みの判断は [`merge_initial`] が持つ）。
//! 確定値の書き戻し（[`priority_apply`]）も同じ流儀で、塊のバイト列を写したまま
//! 当たった `priority` の 1 行だけを差し替える（判断は [`patch::replace_priority`]）。
//!
//! # 出力先
//!
//! 標準出力へ出すのは「どこへ何を書いたか」の結果だけで、本文そのものは出さない。
//! 断りや失敗の本文は標準エラーへ出す（呼び手の `main.rs` が受け持つ）。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{load_catalog, load_ledgers, load_ledgers_with, write_stdout};

use crate::assignment::PageAssignment;
use crate::catalog::Catalog;
use crate::catalog::build::{build as build_catalog, page_not_assigned};
use crate::catalog::write::write as write_catalog;
use crate::documents::{derive, parse};
use crate::error::SurveyError;
use crate::io::{files, paths, snapshot};
use crate::ledger::Ledger;
use crate::ledger::patch;
use crate::ledger::write::merge_initial;
use crate::model::{Domain, EntryId, THEMES};
use crate::report::domain::render_domain;
use crate::report::summary::render_summary;

/// 正典のカタログを作り直す（要件 1.1〜1.9）。
///
/// スナップショットを読み、正典由来の項目だけでカタログを組み立て、本文を丸ごと
/// 書き換える。読めなければ探した絶対パスと理由を載せて失敗し、既存のカタログには
/// 触れない（要件 1.8）。場所は `AREKA_UKADOC_SNAPSHOT` が既定より優先する（要件 1.7）。
pub fn catalog() -> Result<(), SurveyError> {
    // 読む段が全部済むまで、書き出しにも置き場作りにも進まない。
    let source = snapshot::default_path()?;
    let doc = snapshot::load(&source)?;
    let catalog = build_catalog(&doc, &PageAssignment::canonical())?;
    let body = write_catalog(&catalog);

    let target = paths::catalog_path();
    ensure_parent(&target)?;
    files::write_lf(&target, &body)?;
    announce(&target, &format!("{} 項目", catalog.entries.len()))?;
    Ok(())
}

/// 台帳 1 本ぶんの書き出しの計画（要件 3.3a）。
///
/// [`plan_ledger_init`] が組み立て、[`ledger_init`] がそのとおりに書く。値だけを持ち、
/// ファイルには触らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LedgerPlan {
    /// 書き出す先。
    pub(crate) target: PathBuf,
    /// 書き出す本文（既存があればそれを写したもの）。
    pub(crate) body: String,
    /// 標準出力へ添える件数（カタログ側のこのドメインの id の数）。
    pub(crate) count: usize,
}

/// 4 本の台帳の本文を決める（要件 3.3・3.3a）。
///
/// **既存本文の取り寄せを引数で受ける**のがこの関数の要点である。呼び出し側が渡した
/// 取り寄せの結果をそのまま [`merge_initial`] へ渡すので、「既存を渡している」という
/// 配線そのものを、ファイルを 1 つも作らずに、repo の台帳の中身に頼らずに確かめられる
/// （要件 3.3a）。既存を渡し忘れると、手で書いた状態・担当・優先度が黙って初期値へ戻る。
///
/// `read_existing` にはその台帳の置き場を渡し、本文があれば `Some`、無ければ `None` を
/// 返してもらう。
pub(crate) fn plan_ledger_init(
    catalog: &Catalog,
    assignment: &PageAssignment,
    read_existing: &dyn Fn(&Path) -> Result<Option<String>, SurveyError>,
) -> Result<Vec<LedgerPlan>, SurveyError> {
    let buckets = select_ids_by_domain(catalog, assignment)?;

    let mut plans: Vec<LedgerPlan> = Vec::new();
    for domain in Domain::ALL {
        let ids = buckets.get(&domain).map(Vec::as_slice).unwrap_or_default();
        // 名前順を採る（前置きは以後バイト列のまま写されるので、表の転記順でなく動かない順）。
        let pages = assignment.pages_of(domain);
        let target = paths::ledger_path(domain);
        let existing = read_existing(&target)?;
        let body = merge_initial(existing.as_deref(), domain, &pages, ids)?;
        plans.push(LedgerPlan {
            target,
            body,
            count: ids.len(),
        });
    }
    Ok(plans)
}

/// 初期の台帳を作って既存の台帳へ差し込む（要件 3.3・3.3a）。
///
/// 実際の取り寄せ（[`read_if_present`]）と実際の書き手（置き場を作って
/// [`files::write_lf`] を呼ぶ）を [`ledger_init_with`] へ渡すだけの 1 行である。
///
/// # 常時テストが届かない 1 か所
///
/// 残るのは**この 1 行が何を渡すか**だけである。判断も順番も [`ledger_init_with`] に
/// あり、そちらは取り寄せも書き手も引数で受けるので、ファイルを 1 つも作らずに
/// 確かめてある（`generate_tests.rs`）。そのテストは [`ledger_init_with`] を直に
/// 呼ぶので、ここが渡す `&read_if_present` を「いつも `None`」を返す関数へ差し替えても
/// 常時テストは緑のままである。捕まえるのは実データでの通し確認——`ledger-init` を
/// 走らせて台帳 4 本が 1 バイトも変わらないことを見る——だけである。
pub fn ledger_init() -> Result<(), SurveyError> {
    ledger_init_with(&read_if_present, &mut |target: &Path, body: &str| {
        ensure_parent(target)?;
        files::write_lf(target, body)
    })
}

/// 台帳 4 本の本文を決めてから、渡された書き手でそのとおりに書く（要件 3.3・3.3a・1.8）。
///
/// カタログを読み、[`plan_ledger_init`] に 4 本の本文を決めさせ、`write` へ「書き出す先」
/// と「本文」をそのまま渡す。台帳が既にあれば既存の塊は 1 バイトも変わらない。
///
/// 判断は計画側に全部あるので、ここに残るのは**順番**だけである——4 本ぶんの本文が
/// 全部決まってから書き始める。途中の 1 本で落ちたときに、その前の台帳だけが
/// 書き換わって残ることが無い（要件 1.8）。
///
/// 取り寄せと書き手を引数で受けるのは、この順番をファイルを 1 つも作らずに確かめる
/// ためである（設計 File Structure Plan）。書き手を「控えるだけ」に差し替えれば、
/// repo の台帳を 1 本も書かずに走らせられる。
pub(crate) fn ledger_init_with(
    read_existing: &dyn Fn(&Path) -> Result<Option<String>, SurveyError>,
    write: &mut dyn FnMut(&Path, &str) -> Result<(), SurveyError>,
) -> Result<(), SurveyError> {
    // 読む段が全部済むまで、書き出しに進まない。
    let assignment = PageAssignment::canonical();
    let catalog = load_catalog()?;
    let plans = plan_ledger_init(&catalog, &assignment, read_existing)?;

    for plan in &plans {
        write(&plan.target, &plan.body)?;
        announce(&plan.target, &format!("{} 項目", plan.count))?;
    }
    Ok(())
}

/// ドメイン別の報告 4 本を作り直す（要件 7.1・7.3）。
///
/// 実際の読み手（[`files::read_normalized`]）と実際の書き手（置き場を作って
/// [`files::write_lf`] を呼び、結果を 1 行告げる）を [`report_with`] へ渡すだけの
/// 1 行である。
pub fn report() -> Result<(), SurveyError> {
    report_with(
        &files::read_normalized,
        &mut |target: &Path, body: &str, count: usize| {
            ensure_parent(target)?;
            files::write_lf(target, body)?;
            announce(target, &format!("{count} 項目"))
        },
    )
}

/// 台帳 4 本を全部読み、4 本の報告を全部描いてから書く（要件 7.1・7.3・1.8）。
///
/// 各報告の入力はその台帳 1 本とテーマ名だけである（設計 D-11）。カタログもソースも
/// 証拠も読まない。書くのは読む段と描く段が全部済んでからなので、3 本目の台帳が
/// 読めない走行は 1 本も書かない——1 本ずつ「読む→書く」だと、前の 2 本だけが
/// 書き換わって新旧の混ざった報告が残る。書き手が受け取る 3 つ目は台帳の項目数で、
/// 読み手と書き手を引数で受けるのは [`priority_apply_with`] と同じ理由である。
pub(crate) fn report_with(
    read: &dyn Fn(&Path) -> Result<String, SurveyError>,
    write: &mut dyn FnMut(&Path, &str, usize) -> Result<(), SurveyError>,
) -> Result<(), SurveyError> {
    let ledgers = load_ledgers_with(read)?;
    let plans: Vec<(PathBuf, String, usize)> = Domain::ALL
        .into_iter()
        .zip(&ledgers)
        .map(|(domain, ledger)| {
            (
                paths::domain_report_path(domain),
                render_domain(ledger, &THEMES),
                ledger.entries.len(),
            )
        })
        .collect();

    for (target, body, count) in &plans {
        write(target, body, *count)?;
    }
    Ok(())
}

/// 全体の報告を作り直す（要件 7.2・7.3）。
///
/// 入力はカタログと台帳 4 本だけである（要件 11.2・設計 D-5）。ソース木は歩かない
/// ——証拠の件数は本文から外して `evidence` 副手続きの出力に一本化したので、他の spec
/// が正典 URL のコメントを 1 行足しても本文は変わらない。
pub fn report_summary() -> Result<(), SurveyError> {
    let catalog = load_catalog()?;
    let ledgers = load_ledgers()?;

    let body = render_summary(&catalog, &ledgers, &THEMES);
    let target = paths::summary_report_path();
    ensure_parent(&target)?;
    files::write_lf(&target, &body)?;
    announce(&target, &format!("台帳 {} 本", ledgers.len()))?;
    Ok(())
}

/// 確定した `priority` を台帳 4 本へ書き戻す（要件 7.1・7.2・7.6・1.4）。
///
/// 実際の読み手（[`files::read_normalized`]）と実際の書き手（[`files::write_lf`] を
/// 呼んで結果を 1 行告げる）を [`priority_apply_with`] へ渡すだけの 1 行である。
/// 置き場は作らない——読む台帳が既にそこに在る以上、親は必ず在る。
pub fn priority_apply() -> Result<(), SurveyError> {
    priority_apply_with(
        &files::read_normalized,
        &mut |target: &Path, body: &str, changed: usize| {
            files::write_lf(target, body)?;
            announce(target, &format!("変更 {changed} 項目"))?;
            Ok(())
        },
    )
}

/// 2 文書と台帳 4 本を読み、`priority` を導き、4 本の本文を決めてから書く
/// （要件 7.1・7.2・1.4・設計「入口 / cli::generate::priority_apply」）。
///
/// 判断は全部純粋層にある——帰属と順位から値を決めるのは
/// [`crate::documents::derive::priorities`]、塊のバイト列を保って `priority` の 1 行
/// だけを差し替えるのは [`patch::replace_priority`] である。ここに残るのは**順番**
/// だけである。
///
/// 1. `linkage.md`・`briefing.md`・台帳 4 本の 6 本を読む。1 本でも読めなければ、
///    読み手が添えた**探した絶対パス**をそのまま返して止まる（要件 1.4）。
/// 2. 全項目の `priority` を導く。導けない id があれば id を挙げて止まる。
/// 3. 4 本ぶんの本文を決める。決められない台帳が 1 本でもあれば止まる。
/// 4. **4 本が全部決まってから**書き始める。
///
/// 3 と 4 を分けてあるので、途中で落ちた走行は 1 バイトも書かない。読み手と書き手を
/// 引数で受けるのは、この順番をファイルを 1 つも作らずに確かめるためである
/// （[`ledger_init_with`] と同じ形）。書き手が受け取る 3 つ目は**変更した項目数**で、
/// 2 度目の走行が 0 になることが冪等の主張である。
pub(crate) fn priority_apply_with(
    read: &dyn Fn(&Path) -> Result<String, SurveyError>,
    write: &mut dyn FnMut(&Path, &str, usize) -> Result<(), SurveyError>,
) -> Result<(), SurveyError> {
    // 読む段が全部済むまで、導出にも書き出しにも進まない。
    let linkage = parse::read_linkage(&read(&paths::linkage_path())?)?;
    let briefing = parse::read_briefing(&read(&paths::briefing_path())?)?;

    // 台帳の本文は差し替えの元になるので、読み手に渡した場所と本文を控えておく。
    let sources: RefCell<Vec<(PathBuf, String)>> = RefCell::new(Vec::new());
    let ledgers = load_ledgers_with(&|path: &Path| {
        let text = read(path)?;
        sources
            .borrow_mut()
            .push((path.to_path_buf(), text.clone()));
        Ok(text)
    })?;
    let sources = sources.into_inner();

    // 導けない id が 1 つでもあれば、ここで止まる（何も書いていない）。
    let wanted = derive::priorities(&linkage, &briefing, &ledgers)?;

    // 4 本ぶんの本文を決める。ここも書き出しではない。
    let mut plans: Vec<(&Path, String, usize)> = Vec::new();
    for ((target, text), ledger) in sources.iter().zip(&ledgers) {
        let body = patch::replace_priority(text, &wanted)?;
        plans.push((target.as_path(), body, changed_count(ledger, &wanted)));
    }

    for (target, body, changed) in &plans {
        write(target, body, *changed)?;
    }
    Ok(())
}

/// 確定値が今の台帳と違う項目の数（要件 7.1 の「変更 n 項目」）。
///
/// 台帳に在って `wanted` に無い id は数えない——[`patch::replace_priority`] がその塊を
/// 1 バイトも変えないので、変更にも当たらない。
fn changed_count(ledger: &Ledger, wanted: &BTreeMap<EntryId, String>) -> usize {
    ledger
        .entries
        .iter()
        .filter(|(id, entry)| {
            wanted
                .get(*id)
                .is_some_and(|value| value != &entry.priority)
        })
        .count()
}

/// カタログの id を担当ドメインごとに仕分ける（要件 3.1・3.2・3.5）。
///
/// 返る表は **4 ドメインすべてを鍵に持つ**。項目が 1 件も無いドメインの鍵を落とすと、
/// その台帳だけが書き出されずに黙って欠けるからである。各ドメインの id は
/// カタログの並び＝id の byte 昇順で入る。
///
/// # 落ちる場合
///
/// どの台帳にも割り当ての無いページが 1 つでもあれば、そのページ名を挙げて失敗する
/// （[`SurveyError::PageNotAssigned`]・要件 3.5）。落とさずに黙って捨てると、台帳の
/// 件数が減るだけで何も言わない。
pub(crate) fn select_ids_by_domain(
    catalog: &Catalog,
    assignment: &PageAssignment,
) -> Result<BTreeMap<Domain, Vec<EntryId>>, SurveyError> {
    let unassigned = assignment.unassigned(catalog.entries.values().map(|entry| &entry.page));
    if !unassigned.is_empty() {
        return Err(page_not_assigned(&unassigned));
    }

    let mut buckets: BTreeMap<Domain, Vec<EntryId>> = Domain::ALL
        .into_iter()
        .map(|domain| (domain, Vec::new()))
        .collect();
    for (id, entry) in &catalog.entries {
        // 割り当ての無いページは上で落としたので、ここではどのページにも担当がある。
        if let Some(domain) = assignment.domain_of(&entry.page) {
            buckets.entry(domain).or_default().push(id.clone());
        }
    }
    Ok(buckets)
}

/// 置き場が無ければ作る。
///
/// [`files::write_lf`] は親ディレクトリを作らないので、書き出しの直前にここを通す。
/// 呼ぶ位置が大事で、**読む段を全部終えてから**でなければならない（要件 1.8）。
fn ensure_parent(path: &Path) -> Result<(), SurveyError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|err| files::io_error(parent, &err.to_string()))
}

/// ファイルがあれば本文を返す。無ければ `None`。
///
/// 「あるか無いか分からない」場合は黙って `None` にせず、パスを添えて失敗する
/// ——無いことにして進むと、読めるはずの台帳を新規に書き潰す。
fn read_if_present(path: &Path) -> Result<Option<String>, SurveyError> {
    match path.try_exists() {
        Ok(true) => Ok(Some(files::read_normalized(path)?)),
        Ok(false) => Ok(None),
        Err(err) => Err(files::io_error(path, &err.to_string())),
    }
}

/// 何をどこへ書いたかを標準出力へ 1 行出す。本文そのものは出さない。
fn announce(path: &Path, detail: &str) -> Result<(), SurveyError> {
    write_stdout(
        &mut std::io::stdout().lock(),
        &format!("書き出した: {}（{detail}）\n", path.display()),
    )
}

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;
