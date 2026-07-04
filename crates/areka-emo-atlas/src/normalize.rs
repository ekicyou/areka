//! 透過正規化（`use_self_alpha` 解釈・premultiplied BGRA 統一）。
//!
//! 設計決定 **D5 / D8**（要件 **R3**）。
//!
//! 伺かの透過規則（優先順位 α ＞ `.pna` ＞ キーカラー）を契約として定義する。
//! ukadoc 2×2（`use_self_alpha` × `.pna` 有無）動作表のうち emo2 実装腕＝
//! `use_self_alpha=1` かつ `.pna` 無し（α チャンネル採用）のみを実装し、他はシーム。
//! 契約は「Normalizer 出力は常に premultiplied BGRA」で、premultiplied 統一点は
//! デコード腕出力（WIC PBGRA）で既に成立、シーム腕実装時は本段末尾で premultiply する。
//!
//! 現状は本モジュールの共有透過パラメータ型（`AlphaParams` / `UseSelfAlpha`）のみを
//! 定義する。これらは `SurfaceSet` が出所単位で運ぶため列挙タスク（2.1）先行で必要となり、
//! ここ（設計上の定義本拠）に置く。**正規化ロジック本体**（`normalize()` / `NormalizedImage` /
//! `AlphaSource` / `NormalizeError`）は後続の Normalizer タスク（2.3）が本モジュールへ追加する。

/// 上流由来の透過パラメータ（`SurfaceSet` 単位で注入・自ら読まない・3.6）。
///
/// descript（shell/balloon 別定義）由来の透過設定を束ねる。ManifestDeriver は本値を
/// 運ぶのみで解釈せず、解釈は Normalizer（task 2.3）が担う。
#[derive(Clone, Copy, Debug)]
pub struct AlphaParams {
    /// `use_self_alpha` の解釈（α 採用 / 完全不透明扱い / 無効）。
    pub use_self_alpha: UseSelfAlpha,
}

/// `use_self_alpha` の 3 値（ukadoc 動作表・1／true・full・0）。
#[derive(Clone, Copy, Debug)]
pub enum UseSelfAlpha {
    /// `1` / `true`（α チャンネル採用）。
    On,
    /// `full`（全面不透明扱い）。
    Full,
    /// `0`（無効）。
    Off,
}
