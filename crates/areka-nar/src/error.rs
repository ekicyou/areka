//! 拒否の閉じた語彙・マニフェストの警告・I/O 失敗の形と表示（要件 9.1・9.2）。
//!
//! ここではログを出さない。記録は `lib.rs` の `open`／`install` が `Err` を返す
//! 直前に 1 回だけ出す（二重記録を避ける＝設計「Monitoring」）。そのため
//! 各変種の表示は、その 1 行だけで診断できる情報（どのエントリか・期待と実際）を
//! 落とさずに持つ。

use std::path::PathBuf;

/// 拒否語彙を 1 か所の宣言から組み立てる。
///
/// `kind()` の分岐と `ALL_KINDS` の並びを同じ宣言から展開するので、
/// 二重管理による食い違いが起こり得ない。変種を足せば `ALL_KINDS` も
/// 宣言順のまま自動で伸びる（ワイルドカードの腕は生成しないので、
/// 手書きの `kind()` に足し忘れるという事故も起こらない）。
macro_rules! refuse_reasons {
    ($(
        $(#[$variant_attr:meta])*
        $variant:ident { $( $(#[$field_attr:meta])* $field:ident : $ty:ty ),* $(,)? }
    ),* $(,)?) => {
        /// 拒否の理由。14 変種で閉じる（要件 9.2）。
        ///
        /// `kind()` が返す短い語をそのまま `OnInstallFailure` の理由に写せる。
        #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
        pub enum RefuseReason {
            $(
                $(#[$variant_attr])*
                $variant { $( $(#[$field_attr])* $field : $ty ),* }
            ),*
        }

        impl RefuseReason {
            /// 変種名（`OnInstallFailure` に写す短い語）。ASCII・安定。
            pub fn kind(&self) -> &'static str {
                match self {
                    $( RefuseReason::$variant { .. } => stringify!($variant) ),*
                }
            }

            /// 拒否語彙の全数。宣言順に並ぶ。
            pub const ALL_KINDS: &'static [&'static str] = &[ $( stringify!($variant) ),* ];
        }
    };
}

refuse_reasons! {
    /// zip の構造そのものが読めない（EOCD 不在・範囲外・署名不一致）。
    #[error("アーカイブが壊れている: {detail}")]
    CorruptArchive { detail: String },

    /// 伸長の結果が宣言と合わない（要件 2.6）。
    #[error("エントリ {index}（{name}）の内容が壊れている: {what}")]
    IntegrityMismatch { index: usize, name: String, what: Integrity },

    /// 読めることになっていない形式のエントリ（要件 2.5）。
    #[error("エントリ {index}（{name}）は対応していない: {what}")]
    UnsupportedEntry { index: usize, name: String, what: Unsupported },

    /// エントリ名を復号できない（置換文字が出た＝要件 2.4）。
    #[error("エントリ {index} の名前を {encoding} として復号できない（生バイト {raw_hex}）")]
    NameUndecodable { index: usize, raw_hex: String, encoding: &'static str },

    /// エントリ名が長すぎる（要件 1.1〜1.4）。名前の全体は持たず、先頭の有界の一部だけを持つ。
    #[error("エントリ {index} の名前が長すぎる（{length} 単位・上限 {limit}・先頭 {head}…）")]
    PathTooLong { index: usize, length: usize, limit: usize, head: String },

    /// シンボリックリンクのエントリ（要件 2.7）。
    #[error("エントリ {index}（{name}）はシンボリックリンク")]
    SymlinkEntry { index: usize, name: String },

    /// 根の外へ出得る、または Windows で扱えないパス（要件 4.2〜4.6）。
    #[error("エントリ {index}（{name}）のパスが安全でない: {why}")]
    UnsafePath { index: usize, name: String, why: UnsafeWhy },

    /// 大文字小文字だけが違う名前が同じ宛先で重なる（要件 4.7）。
    #[error("大文字小文字だけが違う名前が重なっている: {a} と {b}")]
    CaseCollision { a: String, b: String },

    /// 最上位に `install.txt` が無い（包みフォルダ 1 段を含む＝要件 3.2）。
    #[error("最上位に install.txt が無い（最上位 {top_level:?}）")]
    MissingInstallTxt { top_level: Vec<String> },

    /// `type` が無い、または対応していない値（要件 3.6）。
    #[error("対応していない種別: {}", found.as_deref().unwrap_or("（指定なし）"))]
    UnsupportedType { found: Option<String> },

    /// `name`／`directory` のどちらかが無い、または空（要件 3.7）。
    #[error("必須キー {key} が無い")]
    MissingRequiredKey { key: &'static str },

    /// `directory` の値が 1 階層のフォルダ名として使えない（要件 3.8）。
    #[error("{key} のフォルダ名が使えない: {value}")]
    InvalidDirectoryName { key: String, value: String },

    /// 同梱バルーンの取り出し元フォルダに 1 件もエントリが無い（要件 5.4）。
    #[error("{key} の取り出し元フォルダが無い: {source_directory}")]
    CompanionSourceMissing { key: String, source_directory: String },

    /// `shell`／`supplement` の宛先ゴーストが無い（要件 5.7）。
    #[error("宛先のゴーストが無い: {}", target.as_deref().unwrap_or("（指定なし）"))]
    TargetGhostMissing { target: Option<String> },
}

/// `IntegrityMismatch` の中身。期待と実際を必ず両方持つ。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Integrity {
    /// CRC-32 の突合が合わない。
    #[error("CRC-32 が一致しない（期待 {expected:08x}・実際 {actual:08x}）")]
    Crc { expected: u32, actual: u32 },
    /// 伸長後の長さが中央ディレクトリの宣言と合わない。
    #[error("伸長後の長さが一致しない（期待 {expected}・実際 {actual}）")]
    Size { expected: u64, actual: u64 },
    /// deflate の伸長そのものが失敗した。
    #[error("伸長に失敗した")]
    Inflate,
}

/// `UnsupportedEntry` の中身。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Unsupported {
    /// 暗号化ビットが立っている。
    #[error("暗号化されている")]
    Encrypted,
    /// 無圧縮（0）と deflate（8）以外の圧縮方式。
    #[error("対応していない圧縮方式 {0}")]
    Compression(u16),
    /// zip64 の印がある。
    #[error("zip64 の印がある")]
    Zip64,
    /// 複数ディスクに分かれている。
    #[error("複数ディスクに分かれている")]
    MultiDisk,
    /// 宣言サイズの総和が受け入れ上限を超える。
    #[error("大きすぎる")]
    TooLarge,
}

/// `UnsafePath` の理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnsafeWhy {
    /// `/` 始まり、またはドライブレターつき。
    #[error("絶対パス")]
    Absolute,
    /// 親へ遡る要素を含む。
    #[error(".. を含む")]
    DotDot,
    /// NUL を含む。
    #[error("NUL を含む")]
    Nul,
    /// 区切りとして `\\` を含む。
    #[error("\\ を含む")]
    Backslash,
    /// Windows の予約名・末尾ドット・禁止文字など。
    #[error("Windows で使えない名前: {0}")]
    InvalidWindowsName(String),
}

/// I/O 失敗がどの段階で起きたか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPhase {
    /// `.nar` を読む段階（宛先には触れていない）。
    Read,
    /// 作業フォルダへ組み上げる段階（宛先には触れていない）。
    Stage,
    /// 作業フォルダと宛先を入れ替えて確定する段階。
    Commit,
    /// 確定済みの配置を元へ戻す段階。
    Rollback,
}

/// インストールした要素の種別。
///
/// 設計は `plan` の節に置いているが、`NarError::Io.committed` が持つので
/// 語彙と同じ場所に宣言する。`plan`／`install` はここから使う（公開面は同じ）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElementKind {
    Ghost,
    Balloon,
    Shell,
    Supplement,
}

/// 宛先が確定前にどうなっていたか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExistingState {
    /// 宛先が無かった。
    New,
    /// 宛先が在り、重ね置きした。
    Overlaid,
    /// 宛先が在り、mask 以外を消して入れ替えた。
    Refreshed,
}

/// 配置の確定結果。1 つのインストール済みフォルダにつき 1 つ。
///
/// 設計は `install` の節に置いているが、`NarError::Io.committed` が持つので
/// 語彙と同じ場所に宣言する。中身を作るのはタスク 4.3。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledElement {
    pub kind: ElementKind,
    /// ghost／shell／supplement／balloon 本体は `manifest.name`・同梱バルーンは `directory`。
    pub name: String,
    /// 置いたフォルダの絶対パス。
    pub path: PathBuf,
    pub target_ghost: Option<String>,
    pub existing: ExistingState,
}

/// 巻き戻せなかった宛先 1 つぶんの、元の中身が生き残っている場所（要件 2.1）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurvivingTree {
    /// 元へ戻せなかった宛先（利用者から見えるインストール済みフォルダ）。
    pub destination: PathBuf,
    /// その宛先の確定前の中身がそのまま残っているフォルダ（作業フォルダの直下 old-<k>）。
    pub path: PathBuf,
}

/// `.nar` の読取・展開が返す失敗。
///
/// `std::io::Error` を持つので `Clone`／`PartialEq` は導出できない。
/// 比べたいときは `kind()` か中の `RefuseReason` を比べる。
#[derive(Debug, thiserror::Error)]
pub enum NarError {
    /// 書く前の検査で撥ねた（宛先には一切触れていない）。
    #[error("{archive}: 拒否: {reason}")]
    Refused {
        archive: PathBuf,
        reason: RefuseReason,
    },
    /// 読み書きに失敗した。どこまで確定したかと巻き戻せたかを持つ（要件 6.4）。
    #[error("{archive}: {phase:?} で I/O に失敗: {path}: {source}")]
    Io {
        archive: PathBuf,
        phase: IoPhase,
        path: PathBuf,
        #[source]
        source: std::io::Error,
        /// 失敗までに確定した配置。
        committed: Vec<InstalledElement>,
        /// `committed` を元に戻せたか。
        rolled_back: bool,
        /// 元へ戻せなかった宛先ごとの生き残り。巻き戻せた・掘る前・新規の宛先を消す手だけが
        /// 躓いた場合は空。全て失敗の記録の work の配下に在る。`Vec` でなく `Box<[_]>` なのは、
        /// `NarError` を包む呼び手の失敗の型を clippy の `result_large_err` の閾値の下に保つため。
        survivors: Box<[SurvivingTree]>,
    },
}

/// マニフェストを読んで読み飛ばしたもの。拒否ではないが黙って通さない（要件 3.9）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestWarning {
    /// 対応していない同梱の種別（`headline*` など）。
    #[error("対応していない同梱の種別なので読み飛ばした: {key}")]
    UnsupportedCompanionKind { key: String },
    /// ゴースト以外に同梱の指定があった。
    #[error("ゴースト以外に同梱の指定があるので読み飛ばした: {key}")]
    CompanionOnNonGhost { key: String },
    /// 知らないキー。
    #[error("知らないキーなので読み飛ばした: {key}")]
    IgnoredKey { key: String },
    /// `supplement` では `refresh` を見ない。
    #[error("supplement では refresh を無視した")]
    RefreshIgnoredForSupplement,
    /// mask の要素が 1 階層のファイル名として使えない。
    #[error("mask の要素が不正なので読み飛ばした: {key} = {value}")]
    InvalidMaskEntry { key: String, value: String },
}

/// 書庫の 1 要素の相対パスと `install.txt` のフォルダ名に共通の、パスの長さの
/// 上限（UTF-16 の単位・要件 1.1・1.2・1.5・1.6）。要素ごとの上限は持たない
/// （全体 ≤ 200 < 255 なので NTFS の要素の上限は自動で満たす＝要件 1.7）。
pub(crate) const MAX_ENTRY_PATH_UTF16: usize = 200;

/// 理由と警告に載せる名前の先頭の長さ（UTF-16 の単位）。表示のための長さで、上限ではない。
const HEAD_UTF16: usize = 32;

/// 生バイトを小文字の 16 進に写す（区切り無し）。拒否の理由に入れる。
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// UTF-16 の単位で数えた長さ（BMP の文字は 1・それ以外は 2）。
pub(crate) fn utf16_len(name: &str) -> usize {
    name.encode_utf16().count()
}

/// 名前の先頭の有界の一部（最大 [`HEAD_UTF16`] 単位）。文字の途中では切らない。
pub(crate) fn head_utf16(name: &str) -> String {
    let mut used = 0;
    name.chars()
        .take_while(|ch| {
            used += ch.len_utf16();
            used <= HEAD_UTF16
        })
        .collect()
}

/// 理由と警告に載せる値（要件 1.4・1.5）。上限の内側なら全体、超えていれば
/// 先頭と測った長さと上限だけを綴り、全体は載せない。
pub(crate) fn bounded_value(name: &str) -> String {
    let length = utf16_len(name);
    if length <= MAX_ENTRY_PATH_UTF16 {
        name.to_owned()
    } else {
        format!(
            "{}…（{length} 単位・上限 {MAX_ENTRY_PATH_UTF16}）",
            head_utf16(name)
        )
    }
}

/// 理由に載せる生バイトの 16 進（要件 1.4 と同じ規則）。上限の内側なら全体、超えていれば
/// 先頭 [`HEAD_UTF16`] バイトと測った長さだけを綴る（長さの検査は復号の後なので、ここで縛る）。
pub(crate) fn bounded_hex(bytes: &[u8]) -> String {
    if bytes.len() <= MAX_ENTRY_PATH_UTF16 {
        to_hex(bytes)
    } else {
        format!(
            "{}…（{} バイト）",
            to_hex(&bytes[..HEAD_UTF16]),
            bytes.len()
        )
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
