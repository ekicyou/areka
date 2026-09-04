//! この道具の失敗を表す型。
//!
//! 黙って失敗する経路は作らない（設計 Error Handling）。失敗は必ず値として返し、
//! 呼び出し側——実行ファイルなら標準エラー、テストなら失敗メッセージ——が本文を出す。
//! どの変種も「探した絶対パス」「読めない理由」「形が違う場所」のいずれかを本文に
//! 載せる（要件 1.8・6.10・6.12）。
//!
//! 整合検査で見つかる食い違いはここには入らない。あちらは 1 件目で止めずに全部を
//! 並べる必要があるので、`check::finding` の値として集める。

/// この道具の失敗。
#[derive(Debug, thiserror::Error)]
pub enum SurveyError {
    /// スナップショットのファイルが読めない。探した絶対パスと理由を必ず添える。
    #[error("スナップショットが読めない: {path}（{reason}）")]
    SnapshotUnreadable { path: String, reason: String },
    /// スナップショットの JSON の形が想定と違う。
    #[error("スナップショットの形が違う: {detail}")]
    SnapshotShape { detail: String },
    /// 既定の場所を組み立てるための環境変数が無い。
    #[error(
        "環境変数 {name} が無いので既定の場所を組み立てられない。AREKA_UKADOC_SNAPSHOT で場所を指定してほしい"
    )]
    MissingEnv { name: &'static str },
    /// 項目 id が 2 形（`ukadoc:<ページ>` か `ukadoc:<ページ>:<アンカー>:<連番>`）の
    /// どちらでもない。
    #[error("項目 id の形が違う: {raw}")]
    BadEntryId { raw: String },
    /// 台帳の欄の値が凍結された語彙に無い。どのファイルのどの id のどの欄かを添える。
    #[error("{file} の {id}: {field} の値 {value} は語彙に無い")]
    BadVocabulary {
        file: String,
        id: String,
        field: &'static str,
        value: String,
    },
    /// 台帳の項目が id の byte 昇順に並んでいない（付録 A・設計 D-12）。
    #[error("{file} の項目が id の順に並んでいない: {id}")]
    LedgerOutOfOrder { file: String, id: String },
    /// どの台帳にも割り当てが無いページがある（要件 3.5）。
    #[error("どの台帳にも割り当てが無いページ: {pages}")]
    PageNotAssigned { pages: String },
    /// 台帳本文の塊への切り分けと `toml` の読み取りが食い違う（設計 D-12 の較正）。
    #[error("台帳の切り分けと読み取りが食い違う: {detail}")]
    LedgerSplitMismatch { detail: String },
    /// ファイルの読み書きに失敗した。パスと理由を必ず添える。
    #[error("読み書きに失敗: {path}（{reason}）")]
    Io { path: String, reason: String },
    /// TOML として読めなかった。パスと理由を必ず添える。
    #[error("TOML の読み取りに失敗: {path}（{reason}）")]
    TomlParse { path: String, reason: String },
    /// 整合検査が食い違いを見つけた（要件 6.12・設計「Error Handling / 見張り」）。
    ///
    /// 食い違いそのものは `check::finding` の値として集まり、本文は既に標準出力へ
    /// 出ている。この変種が運ぶのは**件数だけ**で、役目は実行ファイルを終了コード 1
    /// で終わらせることである（`cli::inspect::verdict`）。本文をここへ写すと、
    /// 読み手が同じ一覧を標準出力と標準エラーで 2 度読むことになる。
    #[error("整合検査で食い違いが {count} 件見つかった（本文は標準出力）")]
    CheckFindings { count: usize },
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
