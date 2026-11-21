//! 食い違い 1 件の型と出力の整形。
//!
//! 検査は「1 件目で止めて失敗する」形を採らない。見つけた食い違いを全部 [`Finding`]
//! として集め、[`render`] が 1 つの本文にしてから赤にする。直す人は 1 度の実行で全部を
//! 読める。
//!
//! # 行番号を持たない
//!
//! [`Finding`] の欄は 4 つだけで、行番号はどこにも無い。整理や作り替えで行が上下に
//! 動いても、同じ食い違いは同じ本文になる。場所は [`Finding::place`]——ファイルパスで、
//! それ以上細かくは言わない。
//!
//! # 本文そのものが契約
//!
//! この本文はテストの失敗メッセージであり、実行ファイルの標準出力でもある。版面（件数の行・
//! 種類ごとの塊・字下げ 2 段）は在中テストが逐語で釘付けしている。

use crate::model::EntryId;

/// 食い違い 1 件。
///
/// [`Self::id`] が `None` になるのは、食い違いの主語が項目でないとき（割り当ての無い
/// ページや、報告 1 本まるごとの古さのような場合）である。主語が項目でないものに
/// 作り物の id を付けるとかえって読めなくなる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// 食い違いの種類。
    pub kind: FindingKind,
    /// 該当する項目 id（主語が項目でなければ `None`）。
    pub id: Option<EntryId>,
    /// 場所。ワークスペース根からの相対パス（区切りは `/`）。
    pub place: String,
    /// 何がどう食い違ったか。
    pub detail: String,
}

impl Finding {
    /// 食い違いを 1 件作る。
    pub fn new(
        kind: FindingKind,
        id: Option<EntryId>,
        place: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            id,
            place: place.into(),
            detail: detail.into(),
        }
    }
}

/// 食い違いの種類（13 種）。
///
/// **並びが出力の並びである**。[`render`] は種類ごとに塊を作り、その塊を
/// [`Self::ALL`] の順に並べる。ここを並べ替えると本文が変わる。
///
/// `[ledger].domain` の食い違いの種類は無い。それは `ledger::read` の段で落ちる
/// （`Ledger` は `domain` を 1 つしか持たず、その値はファイル名から来るため）。
/// テーマ名の綴り違いの種類も無い。それも `ledger::read` が `model::parse_theme` で
/// 落とす（検査へ渡すテーマ名と同じ `THEMES` に完全一致で照らすので、検査の段に届く
/// 台帳は綴り違いを持てない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FindingKind {
    /// 台帳の id がカタログに無い。
    LedgerIdNotInCatalog,
    /// カタログの id がどの台帳にも無い。
    CatalogIdMissingFromLedgers,
    /// カタログの id が 2 つ以上の台帳に現れる。
    CatalogIdInMultipleLedgers,
    /// id のページがその台帳の担当でない。
    LedgerIdPageMismatch,
    /// `[ledger].pages` が割り当て表の担当ページと一致しない。
    LedgerPagesMismatch,
    /// 台帳の項目が id の byte 厳密昇順でない。
    LedgerOutOfOrder,
    /// カタログにあるページに割り当てが無い。
    PageNotAssigned,
    /// ソースの正典 URL がカタログに無い。
    SourceUrlNotInCatalog,
    /// `implemented` の id に証拠が 1 件も無い。
    ImplementedWithoutEvidence,
    /// 関連・別名・置き換えの相手がカタログに無い。
    LinkEndpointMissing,
    /// `alias_of` の指す先の状態が `alias`。
    AliasChain,
    /// 台帳の登場版がカタログの版番号の中に無い。
    IntroducedNotInCatalogVersions,
    /// ドメイン別報告か全体報告が台帳から作り直した本文と一致しない。
    DomainReportStale,
}

impl FindingKind {
    /// 13 種すべて。**出力の並びの正本**でもある。
    pub const ALL: [Self; 13] = [
        Self::LedgerIdNotInCatalog,
        Self::CatalogIdMissingFromLedgers,
        Self::CatalogIdInMultipleLedgers,
        Self::LedgerIdPageMismatch,
        Self::LedgerPagesMismatch,
        Self::LedgerOutOfOrder,
        Self::PageNotAssigned,
        Self::SourceUrlNotInCatalog,
        Self::ImplementedWithoutEvidence,
        Self::LinkEndpointMissing,
        Self::AliasChain,
        Self::IntroducedNotInCatalogVersions,
        Self::DomainReportStale,
    ];

    /// 本文の見出しに出る綴り。既定の腕は置かない（種類を増やすと綴りを書き足すまで
    /// コンパイルが通らない）。
    pub fn as_key(&self) -> &'static str {
        match self {
            Self::LedgerIdNotInCatalog => "LedgerIdNotInCatalog",
            Self::CatalogIdMissingFromLedgers => "CatalogIdMissingFromLedgers",
            Self::CatalogIdInMultipleLedgers => "CatalogIdInMultipleLedgers",
            Self::LedgerIdPageMismatch => "LedgerIdPageMismatch",
            Self::LedgerPagesMismatch => "LedgerPagesMismatch",
            Self::LedgerOutOfOrder => "LedgerOutOfOrder",
            Self::PageNotAssigned => "PageNotAssigned",
            Self::SourceUrlNotInCatalog => "SourceUrlNotInCatalog",
            Self::ImplementedWithoutEvidence => "ImplementedWithoutEvidence",
            Self::LinkEndpointMissing => "LinkEndpointMissing",
            Self::AliasChain => "AliasChain",
            Self::IntroducedNotInCatalogVersions => "IntroducedNotInCatalogVersions",
            Self::DomainReportStale => "DomainReportStale",
        }
    }
}

/// 食い違いの一覧を本文にする。
///
/// 版面は次のとおり。**所見が 1 件も無ければ空の本文**を返す。検査が緑のときに何かを書くと、
/// 呼び手が「本文があるかどうか」で合否を判じられなくなる。
///
/// ```text
/// 食い違い 3 件
///
/// [LedgerIdNotInCatalog] 2 件
///   doc/ukadoc-coverage/ledger/property.toml
///     ukadoc:list_propertysystem:balloon.scope(ID).width:1  カタログに無い id
///     ukadoc:list_propertysystem:system.zzz:1  カタログに無い id
/// [ImplementedWithoutEvidence] 1 件
///   doc/ukadoc-coverage/ledger/shiori.toml
///     ukadoc:list_shiori_event:OnBoot:1  正典 URL がソースに 1 件も無い
/// ```
///
/// 並びは入力の順に依らない。塊は [`FindingKind::ALL`] の順、塊の中は場所の名前順、
/// 同じ場所の中は id と詳細の昇順である。id を持たない所見は id の欄ごと落として詳細だけを
/// 書く。
pub fn render(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return String::new();
    }

    let mut out = format!("食い違い {} 件\n\n", findings.len());
    for kind in FindingKind::ALL {
        let mut of_kind: Vec<&Finding> = findings
            .iter()
            .filter(|finding| finding.kind == kind)
            .collect();
        if of_kind.is_empty() {
            continue;
        }
        of_kind.sort_by(|left, right| sort_key(left).cmp(&sort_key(right)));

        out.push_str(&format!("[{}] {} 件\n", kind.as_key(), of_kind.len()));
        let mut written_place: Option<&str> = None;
        for finding in of_kind {
            if written_place != Some(finding.place.as_str()) {
                out.push_str(&format!("  {}\n", finding.place));
                written_place = Some(&finding.place);
            }
            match &finding.id {
                Some(id) => out.push_str(&format!("    {}  {}\n", id.as_str(), finding.detail)),
                None => out.push_str(&format!("    {}\n", finding.detail)),
            }
        }
    }
    out
}

/// 塊の中の並びを決める鍵。
///
/// 場所を先頭に置くのは、場所ごとに 1 行だけ見出しを書くためである（場所が飛び飛びだと
/// 同じ場所の行が 2 度出る）。
fn sort_key(finding: &Finding) -> (&str, &str, &str) {
    (
        finding.place.as_str(),
        finding.id.as_ref().map(EntryId::as_str).unwrap_or_default(),
        finding.detail.as_str(),
    )
}

#[cfg(test)]
#[path = "finding_tests.rs"]
mod tests;
