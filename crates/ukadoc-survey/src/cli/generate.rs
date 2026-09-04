//! 生成側の副手続き（`catalog`・`ledger-init`・`report`・`report-summary`）。
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
//!
//! # 出力先
//!
//! 標準出力へ出すのは「どこへ何を書いたか」の結果だけで、本文そのものは出さない。
//! 断りや失敗の本文は標準エラーへ出す（呼び手の `main.rs` が受け持つ）。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::assignment::PageAssignment;
use crate::catalog::Catalog;
use crate::catalog::build::build as build_catalog;
use crate::catalog::read::read as read_catalog;
use crate::catalog::write::write as write_catalog;
use crate::error::SurveyError;
use crate::evidence::extract::extract;
use crate::evidence::resolve::resolve;
use crate::io::{files, paths, snapshot, sources};
use crate::ledger::Ledger;
use crate::ledger::read::read as read_ledger;
use crate::ledger::write::merge_initial;
use crate::model::{Domain, EntryId, PageName, THEMES};
use crate::report::domain::render_domain;
use crate::report::summary::render_summary;

/// 割り当ての無いページを並べるときの区切り（`catalog::build` と同じ綴り）。
const PAGE_SEPARATOR: &str = "・";

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
    announce(&target, &format!("{} 項目", catalog.entries.len()));
    Ok(())
}

/// 初期の台帳を作って既存の台帳へ差し込む（要件 3.3・3.3a）。
///
/// カタログを読み、id をドメインごとに仕分け、4 本の台帳それぞれについて
/// [`merge_initial`] を呼ぶ。台帳が既にあれば既存の塊は 1 バイトも変わらない。
pub fn ledger_init() -> Result<(), SurveyError> {
    let assignment = PageAssignment::canonical();
    let catalog = read_catalog(&files::read_normalized(&paths::catalog_path())?)?;
    let buckets = select_ids_by_domain(&catalog, &assignment)?;

    for domain in Domain::ALL {
        let ids = buckets.get(&domain).map(Vec::as_slice).unwrap_or_default();
        let pages = prologue_pages(&assignment, domain);
        let target = paths::ledger_path(domain);
        let existing = read_if_present(&target)?;
        let body = merge_initial(existing.as_deref(), domain, &pages, ids)?;

        ensure_parent(&target)?;
        files::write_lf(&target, &body)?;
        announce(&target, &format!("{} 項目", ids.len()));
    }
    Ok(())
}

/// ドメイン別の報告 4 本を作り直す（要件 7.1・7.3）。
///
/// 入力はその台帳 1 本とテーマ名だけである（設計 D-11）。カタログもソースも証拠も
/// 読まないので、4 本の調査 spec の編集集合が交わらない。
pub fn report() -> Result<(), SurveyError> {
    for domain in Domain::ALL {
        let ledger = load_ledger(domain)?;
        let body = render_domain(&ledger, &THEMES);
        let target = paths::domain_report_path(domain);
        ensure_parent(&target)?;
        files::write_lf(&target, &body)?;
        announce(&target, &format!("{} 項目", ledger.entries.len()));
    }
    Ok(())
}

/// 全体の報告を作り直す（要件 7.2・7.3）。
///
/// カタログ・台帳 4 本・ソースから集めた証拠の索引を入力に取る。ドメインごとの
/// 証拠の有無だけを載せるので、索引はここで組み立てて渡す。
pub fn report_summary() -> Result<(), SurveyError> {
    let catalog = read_catalog(&files::read_normalized(&paths::catalog_path())?)?;
    let mut ledgers: Vec<Ledger> = Vec::new();
    for domain in Domain::ALL {
        ledgers.push(load_ledger(domain)?);
    }

    let sources = sources::walk(&paths::workspace_root())?;
    let hits: Vec<_> = sources
        .iter()
        .flat_map(|(path, text)| extract(path, text))
        .collect();
    let evidence = resolve(&hits, &sources, &catalog);

    let body = render_summary(&catalog, &ledgers, &evidence, &THEMES);
    let target = paths::summary_report_path();
    ensure_parent(&target)?;
    files::write_lf(&target, &body)?;
    announce(&target, &format!("台帳 {} 本", ledgers.len()));
    Ok(())
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
    let mut buckets: BTreeMap<Domain, Vec<EntryId>> = Domain::ALL
        .into_iter()
        .map(|domain| (domain, Vec::new()))
        .collect();
    let mut unassigned: BTreeSet<&PageName> = BTreeSet::new();

    for (id, entry) in &catalog.entries {
        match assignment.domain_of(&entry.page) {
            Some(domain) => buckets.entry(domain).or_default().push(id.clone()),
            None => {
                unassigned.insert(&entry.page);
            }
        }
    }

    if !unassigned.is_empty() {
        return Err(SurveyError::PageNotAssigned {
            pages: unassigned
                .iter()
                .map(|page| page.as_str())
                .collect::<Vec<&str>>()
                .join(PAGE_SEPARATOR),
        });
    }
    Ok(buckets)
}

/// 台帳の前置きに書く担当ページの並び（要件 3.1・3.3a）。
///
/// **名前順**（[`PageAssignment::pages_of`]）を採る。要件 3.1 の表に書かれた転記順
/// ではない。理由は 3 つある。
///
/// - 前置きは一度書いたら以後バイト列のまま写される（要件 3.3a）ので、後から
///   選び直せない。「表の転記順」は表の書き換えで動くが、名前順は動かない。
/// - 割り当て表の読み出しが名前順を返す唯一の公開の操作である（転記順を持つ配列は
///   `assignment` の中に閉じている）。
/// - 下流の検査（`LedgerPagesMismatch`）は集合として比べるので、どちらを採っても
///   検査は通る。つまり選び直せる機会は最初の 1 度しか無い。
pub(crate) fn prologue_pages(assignment: &PageAssignment, domain: Domain) -> Vec<PageName> {
    assignment.pages_of(domain)
}

/// ドメインの台帳を読む。無ければ探したパスを添えて失敗する（先に `ledger-init`）。
fn load_ledger(domain: Domain) -> Result<Ledger, SurveyError> {
    let path = paths::ledger_path(domain);
    read_ledger(&files::read_normalized(&path)?, domain)
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
fn announce(path: &Path, detail: &str) {
    println!("書き出した: {}（{detail}）", path.display());
}

#[cfg(test)]
#[path = "generate_tests.rs"]
mod tests;
