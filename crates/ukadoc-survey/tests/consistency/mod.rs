//! repo の実データを読み込む共通処理（設計「入口 / tests/consistency」）。
//!
//! 読むのは 5 種類——カタログ 1 本・台帳 4 本・テーマ定義 1 本・ソース全域・
//! ドメイン別報告 4 本である。読んだ値をそのまま [`CheckInput`] へ束ね、判定そのものは
//! 純粋層に委ねる。**ここは 1 つも判断を持たない**——所見を数える・並べ替える・
//! 選り分ける、のいずれもしない。
//!
//! # スナップショットには構造として届かない（要件 6.2・設計 Testing Strategy 19）
//!
//! この一群は統合テストで、ライブラリとは別のクレートである。スナップショットを読む
//! `io::snapshot` は crate 内公開なので、ここからは名前を書いても解決しない——
//! 「呼ばない」という申し合わせではなく、型検査が拒む。**スナップショットの場所を
//! 組み立てる `io::snapshot::default_path` も同じ理由で 1 度も呼べない**ので、
//! 環境変数 `AREKA_UKADOC_SNAPSHOT` の有無でこの一群の合否が動くことはない。
//!
//! だから環境変数を書き換えるテストは置かない（テストは同一プロセスで並行に走り、
//! Rust 2024 では `set_var` が unsafe である）。スナップショットへ手が届くのは crate の
//! 中にある実行ファイルの入口（`cli`）だけなので、**ここからは `cli` の名前も引かない**。
//!
//! # 全体報告 `summary.md` は読まない（要件 7.6）
//!
//! 読むのはドメイン別報告 4 本だけである。`summary.md` は 4 台帳を跨ぐ成果物で、
//! 常時赤にすると並走 4 本が同じファイルを取り合う（開発者裁定 2026-09-02 議題 2）。
//! `paths::summary_report_path()` はこのファイルから 1 度も呼ばない。
//!
//! # ファイルを 1 つも作らない
//!
//! 読むだけである。書き出しも一時ディレクトリも使わない（設計 File Structure Plan）。
//! 一時ディレクトリを使うと `log-capture-kit` の例外表への追記が要り、要件 9.1
//! （既存クレート非接触）を破る。
//!
//! # テーマ名の出どころは `model::THEMES`（要件 6.8・タスク 6.3 の申し送り）
//!
//! テーマ定義の正本は `doc/ukadoc-coverage/values.md` だが、**検査へ渡すのは
//! [`THEMES`] である**。理由は新しさの検査（要件 7.4）が本文をバイトで比べることに
//! ある——repo にあるドメイン別報告は `report` の副手続きが [`THEMES`] で組み立てて
//! 書き出したものなので、突き合わせ相手を `values.md` の見出しから作ると、`values.md`
//! を書き換えただけで `DomainReportStale`（「報告を作り直せ」）が**誤った理由で**赤く
//! なる。突き合わせの両側は同じ出どころでなければならない。
//!
//! `values.md` の本文は [`RepoData::values`] として持つ（タスク 8.4 が見出しと
//! [`THEMES`] の順序まで一致することを釘付けする）。台帳のテーマ名が定義に実在する
//! ことは、⑴ 台帳のテーマ名が [`THEMES`] にあること（要件 6.8 の判定）と
//! ⑵ [`THEMES`] が `values.md` の見出しと一致すること（タスク 8.4）の 2 つで守る。
//!
//! # テストの本体はこのディレクトリの兄弟に置く
//!
//! 実データへの主張は [`checks`]（`checks.rs`）に、検査の対象が 0 件でないことの主張は
//! [`non_vacuity`]（`non_vacuity.rs`）に、自前の道具の較正は [`values_md`]
//! （`values_md.rs`）に、要件と README の記入例が実データに実在することの主張は
//! [`examples`]（`examples.rs`）にある。[`checks`] の摂動が使う道具（実データの写しを
//! 1 か所だけ壊す型と、所見の見方）は [`perturb`]（`perturb.rs`）にあり、そこには
//! テストの本体を 1 つも置かない。どれも同じディレクトリの兄弟で、宣言は
//! このファイルへ下の `mod checks;` と同じく素の `mod` で置ける（`#[path]` で
//! 読み込まれたファイルの子モジュールは、そのファイル自身のディレクトリを基準に
//! 解決される——`structure.md:141`）。

mod checks;
mod examples;
mod non_vacuity;
mod perturb;
mod values_md;

use std::collections::BTreeMap;
use std::path::Path;

use ukadoc_survey::assignment::PageAssignment;
use ukadoc_survey::catalog::Catalog;
use ukadoc_survey::catalog::read::read as read_catalog;
use ukadoc_survey::check::CheckInput;
use ukadoc_survey::evidence::EvidenceIndex;
use ukadoc_survey::evidence::extract::extract;
use ukadoc_survey::evidence::resolve::resolve;
use ukadoc_survey::io::{files, paths, sources};
use ukadoc_survey::ledger::Ledger;
use ukadoc_survey::ledger::read::read as read_ledger;
use ukadoc_survey::model::{Domain, THEMES};

/// repo から読み込んだ実データ一式。
///
/// [`CheckInput`] は何も所有しない借り物の束なので、値の持ち主がどこかに要る。それが
/// この型で、[`RepoData::input`] が借り方を 1 か所に決める——検査へ渡す組み合わせを
/// テストごとに書くと、渡し忘れた欄が黙って別の値になる。
pub struct RepoData {
    /// 正典の写し（`doc/ukadoc-coverage/catalog.toml`）。
    pub catalog: Catalog,
    /// 台帳 4 本（並びは [`Domain::ALL`]）。
    pub ledgers: Vec<Ledger>,
    /// ページ→ドメインの割り当て（担当の正本）。
    pub assignment: PageAssignment,
    /// ソース全域から集めた証拠の索引。
    pub evidence: EvidenceIndex,
    /// ドメイン別報告 4 本の本文（復帰文字を落としたもの・設計 D-6）。
    pub domain_reports: BTreeMap<Domain, String>,
    /// 走査したソース（ワークスペース根からの相対パスと本文）。
    pub sources: Vec<(String, String)>,
    /// テーマ定義（`doc/ukadoc-coverage/values.md`）の本文。
    pub values: String,
}

impl RepoData {
    /// repo の実データを読む。
    ///
    /// 読めないファイルが 1 つでもあれば、**探した絶対パスと理由を添えて**止まる
    /// （要件 6.12。黙って飛ばすと、その 1 本を見なかった検査が緑になる）。
    pub fn load() -> Self {
        let catalog_path = paths::catalog_path();
        let catalog = read_catalog(&read_text(&catalog_path)).unwrap_or_else(|err| {
            panic!("カタログを読めない: {}（{err}）", catalog_path.display())
        });

        let mut ledgers = Vec::with_capacity(Domain::ALL.len());
        for domain in Domain::ALL {
            let path = paths::ledger_path(domain);
            let ledger = read_ledger(&read_text(&path), domain)
                .unwrap_or_else(|err| panic!("台帳を読めない: {}（{err}）", path.display()));
            ledgers.push(ledger);
        }

        let mut domain_reports = BTreeMap::new();
        for domain in Domain::ALL {
            domain_reports.insert(domain, read_text(&paths::domain_report_path(domain)));
        }

        let values = read_text(&paths::values_path());

        let root = paths::workspace_root();
        let sources = sources::walk(&root)
            .unwrap_or_else(|err| panic!("ソースを走査できない: {}（{err}）", root.display()));
        let hits: Vec<_> = sources
            .iter()
            .flat_map(|(path, text)| extract(path, text))
            .collect();
        let evidence = resolve(&hits, &sources, &catalog);

        Self {
            catalog,
            ledgers,
            assignment: PageAssignment::canonical(),
            evidence,
            domain_reports,
            sources,
            values,
        }
    }

    /// 純粋層の判定へ渡す入力を組む。
    ///
    /// テーマ名は [`THEMES`] である（このファイルの冒頭の理由）。`values.md` の見出しを
    /// ここへ流し込んではならない——報告を書き出した側と出どころが割れる。
    pub fn input(&self) -> CheckInput<'_> {
        CheckInput {
            catalog: &self.catalog,
            ledgers: &self.ledgers,
            assignment: &self.assignment,
            themes: &THEMES,
            evidence: &self.evidence,
            domain_reports: &self.domain_reports,
        }
    }
}

/// 1 本読む。復帰文字は入出力層が落とす（設計 D-6）。
///
/// 失敗したら `io::files` の本文をそのまま出して止まる（要件 6.12）。その本文には既に
/// 探した絶対パスと理由が入っており、パスの末尾（`catalog.toml`・`ledger/<ドメイン>.toml`・
/// `report/<ドメイン>.md`・`values.md`）がそのまま役目を名指すので、ここで包み直しても
/// 同じパスを 2 度書くだけになる。だから包まない。
fn read_text(path: &Path) -> String {
    files::read_normalized(path).unwrap_or_else(|err| panic!("{err}"))
}
