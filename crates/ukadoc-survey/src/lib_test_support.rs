//! 全モジュールのテストが共用する見本データの組み立て（テスト専用）。
//!
//! ここが作るのは**正常な状態の小さな世界**である。カタログ 12 件・台帳 4 本・
//! ソース文 3 本・ドメイン別報告 4 本が互いに矛盾なく揃っており、整合検査
//! （[`crate::check::run`]）を通しても食い違いは 1 件も出ない。
//!
//! # 壊し方はここに置かない
//!
//! 検査の各判定（タスク 4.2〜4.4）は、この正常な世界の**ちょうど 1 か所**を壊して
//! 所見が 1 件だけ出ることを確かめる。壊し方はそれぞれのテストファイルの中に閉じる
//! ——ここに壊れた見本を並べると、どのテストがどれを使っているかが読めなくなり、
//! 1 つの見本を直すと関係の無いテストが赤くなる。
//!
//! # 報告は台帳から作り直して持つ
//!
//! [`World::domain_reports`] は手書きではなく [`render_domain`] の出力である。手書き
//! にすると新しさの検査（要件 7.4）が最初から赤になる。**台帳を壊したら
//! [`World::refresh_reports`] を呼ぶこと**——呼ばないと、壊した判定の所見に加えて
//! `DomainReportStale` まで出てしまい「1 件だけ出る」が成り立たない。逆に報告の側を
//! 壊す判定（タスク 4.4）は作り直してはいけない。同じことがソース文と証拠の索引
//! （[`World::refresh_evidence`]）にも当てはまる。
//!
//! # ファイルには 1 つも触らない
//!
//! 見本はすべて文字列の定数から組み立てる（要件 6.2・設計 File Structure Plan）。
//! 一時ディレクトリも使わない。ソース文に書いた `// ukadoc:` の行は本物の走査には
//! 拾われない——走査は `crates/ukadoc-survey/` を除くからである（設計 D-3）。

use std::collections::BTreeMap;

use crate::assignment::PageAssignment;
use crate::catalog::{CATALOG_FORMAT, Catalog, CatalogEntry, SnapshotMeta};
use crate::check::CheckInput;
use crate::evidence::EvidenceIndex;
use crate::evidence::extract::extract;
use crate::evidence::resolve::resolve;
use crate::hash::HASH_ALGORITHM;
use crate::ledger::{Ledger, LedgerEntry};
use crate::model::{Domain, EntryId, Link, LinkKind, Status, THEMES};
use crate::report::domain::render_domain;

/// 正典 URL の根。実データと同じ綴りで、フラグメントの前までがページ URL になる。
const URL_BASE: &str = "https://ssp.shillest.net/ukadoc/manual/";

/// 見本のカタログ 1 行。`page` と `url` は id から機械で導く（実データと同じ関係を
/// 保つため。手で書くと id と食い違った見本が作れてしまう）。
struct CatalogRow {
    id: &'static str,
    title: &'static str,
    category: &'static str,
    versions: &'static [&'static str],
    hash: &'static str,
}

/// 見本のカタログ 12 件。
///
/// 6 ページ・6 カテゴリにまたがり、**アンカー無しの 2 形**（`ukadoc:spec_shiori3`・
/// `ukadoc:manual_shell`）も含む（要件 1.9）。版番号は 0 個・1 個・2 個の 3 通りを
/// 揃えてある——1 通りしか無いと「版番号を 1 つに絞る」取り違えが素通りする。
/// 見出しには逆斜線を含むものを 1 件入れてある（設計 D-10）。
const CATALOG_ROWS: [CatalogRow; 12] = [
    CatalogRow {
        id: "ukadoc:descript_ghost:charset:1",
        title: "charset",
        category: "descript",
        versions: &[],
        hash: "1111111111111111",
    },
    CatalogRow {
        id: "ukadoc:descript_ghost:name:1",
        title: "name",
        category: "descript",
        versions: &["2.3.53"],
        hash: "2222222222222222",
    },
    CatalogRow {
        id: "ukadoc:list_propertysystem:currentghost.name:1",
        title: "currentghost.name",
        category: "propertysystem",
        versions: &[],
        hash: "3333333333333333",
    },
    CatalogRow {
        id: "ukadoc:list_propertysystem:system.month:1",
        title: "system.month",
        category: "propertysystem",
        versions: &["2.3.53", "2.5.60"],
        hash: "4444444444444444",
    },
    CatalogRow {
        id: "ukadoc:list_propertysystem:system.year:1",
        title: "system.year",
        category: "propertysystem",
        versions: &["2.3.53"],
        hash: "5555555555555555",
    },
    CatalogRow {
        id: "ukadoc:list_sakura_script:_5c_5f_71:1",
        title: "選択肢の'既定'を消す",
        category: "sakurascript",
        versions: &[],
        hash: "6666666666666666",
    },
    CatalogRow {
        id: "ukadoc:list_sakura_script:_5c_65:1",
        title: "\\e",
        category: "sakurascript",
        versions: &["2.5.60"],
        hash: "7777777777777777",
    },
    CatalogRow {
        id: "ukadoc:list_sakura_script:_5c_73_5bID_5d:1",
        title: "\\s[ID]",
        category: "sakurascript",
        versions: &["2.3.53", "2.5.60"],
        hash: "8888888888888888",
    },
    CatalogRow {
        id: "ukadoc:list_shiori_event:OnBoot:1",
        title: "OnBoot",
        category: "shiori_event",
        versions: &["2.3.53"],
        hash: "9999999999999999",
    },
    CatalogRow {
        id: "ukadoc:list_shiori_event:OnClose:1",
        title: "OnClose",
        category: "shiori_event",
        versions: &["2.3.53", "2.5.60"],
        hash: "aaaaaaaaaaaaaaaa",
    },
    CatalogRow {
        id: "ukadoc:manual_shell",
        title: "シェルの作り方",
        category: "manual",
        versions: &[],
        hash: "bbbbbbbbbbbbbbbb",
    },
    CatalogRow {
        id: "ukadoc:spec_shiori3",
        title: "SHIORI/3.0 の仕様",
        category: "spec",
        versions: &[],
        hash: "cccccccccccccccc",
    },
];

/// 見本の台帳 1 項目。付録 A.2 の欄をそのまま並べる。
struct LedgerRow {
    id: &'static str,
    status: Status,
    introduced: &'static str,
    alias_of: Option<&'static str>,
    supersedes: &'static [&'static str],
    owner: &'static str,
    priority: &'static str,
    values: &'static [&'static str],
    links: &'static [(LinkKind, &'static str)],
    note: &'static str,
}

/// 欄の大半が既定でよいときの短い書き方。
const fn plain(id: &'static str, status: Status) -> LedgerRow {
    LedgerRow {
        id,
        status,
        introduced: "",
        alias_of: None,
        supersedes: &[],
        owner: "",
        priority: "",
        values: &[],
        links: &[],
        note: "",
    }
}

/// shiori 台帳の 3 項目。
///
/// `OnBoot` は**実装済み＋証拠あり**（要件 6.6 の経路が生きていることの土台）で、
/// 版番号もテーマも関連も持つ。並びは id の byte 厳密昇順。
const SHIORI_ROWS: [LedgerRow; 3] = [
    LedgerRow {
        introduced: "2.3.53",
        values: &["気配"],
        links: &[(LinkKind::SameFeature, "ukadoc:list_shiori_event:OnClose:1")],
        owner: "areka-P0-ukadoc-survey-shiori",
        priority: "A10",
        note: "起動の合図。",
        ..plain("ukadoc:list_shiori_event:OnBoot:1", Status::Implemented)
    },
    LedgerRow {
        introduced: "2.5.60",
        values: &["気配", "更新"],
        ..plain("ukadoc:list_shiori_event:OnClose:1", Status::Absent)
    },
    plain("ukadoc:spec_shiori3", Status::Unclassified),
];

/// assets 台帳の 3 項目。
///
/// `charset` が `name` の**別名**で、`name` はその逆向きを `supersedes` に持つ。
/// 別名の連鎖（要件 6.7）はここを壊して作る。
const ASSETS_ROWS: [LedgerRow; 3] = [
    LedgerRow {
        alias_of: Some("ukadoc:descript_ghost:name:1"),
        ..plain("ukadoc:descript_ghost:charset:1", Status::Alias)
    },
    LedgerRow {
        introduced: "2.3.53",
        supersedes: &["ukadoc:descript_ghost:charset:1"],
        values: &["装い"],
        links: &[(LinkKind::Configures, "ukadoc:manual_shell")],
        ..plain("ukadoc:descript_ghost:name:1", Status::VocabularyOnly)
    },
    plain("ukadoc:manual_shell", Status::NotApplicable),
];

/// sakura-script 台帳の 3 項目。
const SAKURA_SCRIPT_ROWS: [LedgerRow; 3] = [
    plain(
        "ukadoc:list_sakura_script:_5c_5f_71:1",
        Status::Unclassified,
    ),
    LedgerRow {
        introduced: "2.5.60",
        ..plain("ukadoc:list_sakura_script:_5c_65:1", Status::Degraded)
    },
    LedgerRow {
        introduced: "2.3.53",
        values: &["触れ合い"],
        ..plain(
            "ukadoc:list_sakura_script:_5c_73_5bID_5d:1",
            Status::Implemented,
        )
    },
];

/// property 台帳の 3 項目。
///
/// `system.year` の証拠は**語彙表の経路**で付く（設計 D-5）。URL の直書きだけの見本に
/// すると、名前の突き合わせが 1 度も走らないまま緑になる。
const PROPERTY_ROWS: [LedgerRow; 3] = [
    LedgerRow {
        values: &["記憶"],
        ..plain(
            "ukadoc:list_propertysystem:currentghost.name:1",
            Status::VocabularyOnly,
        )
    },
    plain(
        "ukadoc:list_propertysystem:system.month:1",
        Status::Unclassified,
    ),
    LedgerRow {
        introduced: "2.3.53",
        values: &["記憶"],
        ..plain(
            "ukadoc:list_propertysystem:system.year:1",
            Status::Implemented,
        )
    },
];

/// 見本のソース 1 本目——`OnBoot` の証拠（URL の直書き）。
const SOURCE_EVENTS: (&str, &str) = (
    "crates/areka-kanade/src/schedule/events.rs",
    r#"//! 送出するイベントの定義。

/// ゴーストを起こしたときに一度だけ送る。
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBoot:1
pub const ON_BOOT: &str = "OnBoot";

/// ここには URL の付かない ukadoc の語がある（証拠にはならない・要件 5.6）。
pub const ON_CLOSE: &str = "OnClose";
"#,
);

/// 見本のソース 2 本目——さくらスクリプトのタグの証拠（URL の直書き）。
const SOURCE_TAG: (&str, &str) = (
    "crates/areka-sakura/src/tag/surface.rs",
    r#"//! 立ち絵を切り替えるタグ。

/// 立ち絵の番号を差し替える。
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_73_5bID_5d:1
pub fn surface(id: u32) -> u32 {
    id
}
"#,
);

/// 見本のソース 3 本目——語彙表の経路（設計 D-5）。
///
/// 目印のページ URL の**直後**にスライス定数が始まり、要素ごとの最初の文字列リテラルが
/// 名前になる。実物と同じくタプルの形を採ってある。
const SOURCE_VOCAB: (&str, &str) = (
    "crates/areka-sylphya/src/vocab/dotted.rs",
    r#"//! 点付きプロパティの語彙表。

/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html
const SET_EFFECTIVE: &[(&str, SetSemantics)] = &[
    ("system.year", SetSemantics::ReadOnly),
    ("currentghost.name", SetSemantics::ReadOnly),
];
"#,
);

/// 見本のソース 3 本。
const SOURCE_FILES: [(&str, &str); 3] = [SOURCE_EVENTS, SOURCE_TAG, SOURCE_VOCAB];

/// 見本の id を作る。
fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は要件 1.9 の 2 形のいずれかのはず")
}

/// id から正典 URL を導く。アンカー付きなら `#アンカー:連番` を付ける。
fn url_of(entry: &EntryId) -> String {
    let fields: Vec<&str> = entry.as_str().split(':').collect();
    match fields.as_slice() {
        [_, page] => format!("{URL_BASE}{page}.html"),
        [_, page, anchor, seq] => format!("{URL_BASE}{page}.html#{anchor}:{seq}"),
        _ => panic!("見本の id は 2 形のいずれかのはず: {}", entry.as_str()),
    }
}

/// 見本のカタログ。
pub(crate) fn catalog() -> Catalog {
    let entries = CATALOG_ROWS
        .iter()
        .map(|row| {
            let entry_id = id(row.id);
            let entry = CatalogEntry {
                page: entry_id.page(),
                title: row.title.to_owned(),
                category: row.category.to_owned(),
                versions: row.versions.iter().map(|v| (*v).to_owned()).collect(),
                hash: row.hash.to_owned(),
                url: url_of(&entry_id),
                id: entry_id.clone(),
            };
            (entry_id, entry)
        })
        .collect();
    Catalog {
        snapshot: snapshot_meta(),
        entries,
    }
}

/// 見本の冒頭情報。
///
/// `total_entries` と `ukadoc_entries` は**違う値**にしてある。同じ値の見本では
/// 2 つの欄を取り違えても気づけない（タスク 2.1 の教訓）。
fn snapshot_meta() -> SnapshotMeta {
    SnapshotMeta {
        package: "ukagaka-doc-mcp".to_owned(),
        package_version: "0.4.2".to_owned(),
        snapshot_version: 3,
        generated_at: "2026-08-31T02:03:04Z".to_owned(),
        total_entries: 20,
        ukadoc_entries: CATALOG_ROWS.len(),
        catalog_format: CATALOG_FORMAT,
        hash_algorithm: HASH_ALGORITHM.to_owned(),
    }
}

/// ドメインごとの見本の行。
fn rows_of(domain: Domain) -> &'static [LedgerRow] {
    match domain {
        Domain::Shiori => &SHIORI_ROWS,
        Domain::Assets => &ASSETS_ROWS,
        Domain::SakuraScript => &SAKURA_SCRIPT_ROWS,
        Domain::Property => &PROPERTY_ROWS,
    }
}

/// 見本の台帳 4 本（[`Domain::ALL`] の順）。
///
/// 前置きの担当ページは [`PageAssignment::pages_of`] から取る。手で書くと
/// `LedgerPagesMismatch` が最初から赤になる。**その台帳に項目が 1 件も無いページも
/// 前置きには並ぶ**（shiori は 12 ページを宣言して 2 ページ分の項目しか持たない）。
pub(crate) fn ledgers() -> Vec<Ledger> {
    let assignment = PageAssignment::canonical();
    Domain::ALL
        .iter()
        .map(|domain| ledger_of(*domain, &assignment))
        .collect()
}

/// 1 本の台帳を組む。
fn ledger_of(domain: Domain, assignment: &PageAssignment) -> Ledger {
    let entries: Vec<LedgerEntry> = rows_of(domain).iter().map(entry_of).collect();
    let file_order = entries.iter().map(|entry| entry.id.clone()).collect();
    Ledger {
        domain,
        pages: assignment.pages_of(domain),
        entries: entries
            .into_iter()
            .map(|entry| (entry.id.clone(), entry))
            .collect(),
        file_order,
    }
}

/// 見本の 1 行から台帳の項目を作る。
fn entry_of(row: &LedgerRow) -> LedgerEntry {
    LedgerEntry {
        id: id(row.id),
        status: row.status,
        introduced: row.introduced.to_owned(),
        alias_of: row.alias_of.map(id),
        supersedes: row.supersedes.iter().map(|raw| id(raw)).collect(),
        owner: row.owner.to_owned(),
        priority: row.priority.to_owned(),
        values: row.values.iter().map(|v| (*v).to_owned()).collect(),
        links: row
            .links
            .iter()
            .map(|(kind, to)| Link {
                kind: *kind,
                to: id(to),
            })
            .collect(),
        note: row.note.to_owned(),
    }
}

/// 見本のソース文（パスと本文の組）。
pub(crate) fn sources() -> Vec<(String, String)> {
    SOURCE_FILES
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

/// ソース文から証拠の索引を作る。
///
/// 実行ファイルと同じ経路（取り出し → 解決）を通す。索引を手で組むと、ソース文を
/// 壊しても索引が変わらない見本ができてしまう。
pub(crate) fn evidence(catalog: &Catalog, sources: &[(String, String)]) -> EvidenceIndex {
    let hits: Vec<_> = sources
        .iter()
        .flat_map(|(path, text)| extract(path, text))
        .collect();
    resolve(&hits, sources, catalog)
}

/// 台帳 4 本からドメイン別報告 4 本を作る（要件 7.4 の突き合わせ相手）。
pub(crate) fn domain_reports(ledgers: &[Ledger]) -> BTreeMap<Domain, String> {
    ledgers
        .iter()
        .map(|ledger| (ledger.domain, render_domain(ledger, &THEMES)))
        .collect()
}

/// 正常な状態の見本の世界ひとそろい。
///
/// [`CheckInput`] は借りた値の束なので、借り元をどこかが持っていなければならない。
/// その持ち主がこの型である。
pub(crate) struct World {
    pub(crate) catalog: Catalog,
    pub(crate) ledgers: Vec<Ledger>,
    pub(crate) assignment: PageAssignment,
    pub(crate) sources: Vec<(String, String)>,
    pub(crate) evidence: EvidenceIndex,
    pub(crate) domain_reports: BTreeMap<Domain, String>,
}

impl World {
    /// 食い違いが 1 件も無い状態の世界を組む。
    pub(crate) fn normal() -> Self {
        let catalog = catalog();
        let ledgers = ledgers();
        let sources = sources();
        let evidence = evidence(&catalog, &sources);
        let domain_reports = domain_reports(&ledgers);
        Self {
            catalog,
            ledgers,
            assignment: PageAssignment::canonical(),
            sources,
            evidence,
            domain_reports,
        }
    }

    /// 検査に渡す入力。
    pub(crate) fn input(&self) -> CheckInput<'_> {
        CheckInput {
            catalog: &self.catalog,
            ledgers: &self.ledgers,
            assignment: &self.assignment,
            themes: &THEMES,
            evidence: &self.evidence,
            domain_reports: &self.domain_reports,
        }
    }

    /// そのドメインの台帳を書き換えるために借りる。
    pub(crate) fn ledger_mut(&mut self, domain: Domain) -> &mut Ledger {
        self.ledgers
            .iter_mut()
            .find(|ledger| ledger.domain == domain)
            .expect("見本の台帳は 4 ドメインすべてを持つ")
    }

    /// そのドメインの報告の本文を書き換えるために借りる。
    pub(crate) fn report_mut(&mut self, domain: Domain) -> &mut String {
        self.domain_reports
            .get_mut(&domain)
            .expect("見本の報告は 4 ドメインすべてを持つ")
    }

    /// そのパスのソース文を書き換えるために借りる。
    pub(crate) fn source_mut(&mut self, path: &str) -> &mut String {
        &mut self
            .sources
            .iter_mut()
            .find(|(name, _)| name == path)
            .expect("見本のソース文にそのパスは無い")
            .1
    }

    /// いまのソース文から証拠の索引を作り直す。
    pub(crate) fn refresh_evidence(&mut self) {
        self.evidence = evidence(&self.catalog, &self.sources);
    }

    /// いまの台帳からドメイン別報告を作り直す。
    ///
    /// 台帳を壊す判定（タスク 4.2・4.3）はこれを呼ぶ。呼ばないと報告の側も古くなり、
    /// 所見が 2 種類出る。
    pub(crate) fn refresh_reports(&mut self) {
        self.domain_reports = domain_reports(&self.ledgers);
    }
}
