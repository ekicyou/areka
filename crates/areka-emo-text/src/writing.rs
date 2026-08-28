//! # writing — 書字方向の唯一の決定点（純粋層）
//!
//! バルーン定義の 2 キー——SSP 正典キー `vertical`（`0`／`1`）と areka 拡張キー
//! `writing_mode`（`horizontal_tb`／`vertical_rl`／`vertical_lr`）——を**独立に分類**し、
//! 確定した 2 つの分類の間で優先順位を裁定して [`WritingMode`] を 1 つ決める。
//! 裁定とその根拠は [`WritingDirectionDecision`] が保持する（正典が再改訂されたときの
//! 唯一の追随点）。方向写像と M2 予約キー名（`text_orientation`／`text_combine_upright`）の
//! 記録も本モジュールが担う。
//!
//! 2 層マージ（`descript.txt` 基層と面別上書き層の後勝ち）は `areka-parsers` の
//! `balloon::parse` が既に確定させたうえで [`BalloonModel`] に届くため、本層はキー間の
//! 優劣だけを見る（層の優劣とキーの優劣を混ぜない）。
//!
//! 本番の唯一の呼出は `actor.rs:153` の [`WritingMode::resolve`]
//! （`ResolvedBalloonText::resolve` の内側）である。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない。`lib.rs` の構造檻
//! `pure_layer_modules_have_no_windows_imports` が本ファイル（`PURE_SOURCES`）を走査して強制する。
//!
//! ## 2 キーの共存規則（要件 2・裁定 1）
//!
//! | 有効な宣言 | 採用する方向 | 記録 |
//! |---|---|---|
//! | どちらも無し | 正典既定の横書き | 無し |
//! | `vertical` のみ | 正典キーの宣言 | 無し |
//! | `writing_mode` のみ | 拡張キーの宣言 | 無し |
//! | 両方・同じ方向 | その方向 | 無し |
//! | 両方・異なる方向 | **拡張キー** `writing_mode` | `debug!` ちょうど 1 件 |
//!
//! 拡張キーが勝つのは、`vertical_lr` という正典キーでは表現できない値を持つ＝表現力で
//! 上位だからである（要件の裁定 1）。値が壊れている宣言（`vertical` が `0`／`1` 以外か空・
//! `writing_mode` が受理語彙外）は `warn!` のうえ「**指定なし**」として上表へ合流する
//! （設計 DD6）。
//!
//! ## M2 予約キー（記録のみ・実装しない・R5.7）
//!
//! CSS 借用の snake_case 予約キー名を定数として記録するに留め、M1 では実挙動を
//! 一切実装しない:
//!
//! - [`RESERVED_KEY_TEXT_ORIENTATION`]（`text_orientation`・欧文の向き）
//! - [`RESERVED_KEY_TEXT_COMBINE_UPRIGHT`]（`text_combine_upright`・縦中横）
use areka_parsers::balloon::BalloonModel;

/// `writing_mode` の CSS 語彙 `horizontal_tb` に対応する受理値（R5.1）。
const VALUE_HORIZONTAL_TB: &str = "horizontal_tb";
/// `writing_mode` の CSS 語彙 `vertical_rl` に対応する受理値（R5.1）。
const VALUE_VERTICAL_RL: &str = "vertical_rl";
/// `writing_mode` の CSS 語彙 `vertical_lr` に対応する受理値（R5.1）。
const VALUE_VERTICAL_LR: &str = "vertical_lr";

/// M2 予約キー: 欧文の向き `text_orientation`（記録のみ・実装しない・R5.7）。
pub const RESERVED_KEY_TEXT_ORIENTATION: &str = "text_orientation";
/// M2 予約キー: 縦中横 `text_combine_upright`（記録のみ・実装しない・R5.7）。
pub const RESERVED_KEY_TEXT_COMBINE_UPRIGHT: &str = "text_combine_upright";

/// `writing_mode` 宣言の解決結果（3 語彙→3 方向の 1:1 写像・R5.1/R5.5）。
///
/// 語彙（snake_case・CSS `writing-mode` 借用）と方向の対応は 1:1:
///
/// | 語彙 | variant | 方向（行内／行送り） |
/// |---|---|---|
/// | `horizontal_tb` | [`HorizontalTb`](Self::HorizontalTb) | 左→右／上→下 |
/// | `vertical_rl` | [`VerticalRl`](Self::VerticalRl) | 上→下／右→左 |
/// | `vertical_lr` | [`VerticalLr`](Self::VerticalLr) | 上→下／左→右 |
///
/// 軸読み替え（折返し軸・スクロール軸・書字開始角）の適用は layout 層の領分
/// （design.md「軸読み替え正準表」）。DirectWrite への設定写像
/// （ReadingDirection／FlowDirection）は COM 層 draw.rs が本型を消費して行う。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WritingMode {
    /// 横書き（行内 左→右・行送り 上→下）。SSP 互換の既定（マーカー無し・R5.3）。
    #[default]
    HorizontalTb,
    /// 日本語縦書き（行内 上→下・行送り 右→左）。
    VerticalRl,
    /// 縦書き左送り（行内 上→下・行送り 左→右）。
    VerticalLr,
}

impl WritingMode {
    /// `BalloonModel`（2 層マージ済み）から有効な書字方向を解釈する。
    ///
    /// [`WritingDirectionDecision::resolve`] へ委譲し、その [`mode`](WritingDirectionDecision::mode)
    /// を返すだけの薄い経路である（戻り値型は不変）。決定の**根拠**（採用出所・両キーの分類・
    /// 矛盾併記の有無）が要るときは [`WritingDirectionDecision`] を直接使うこと。
    pub fn resolve(model: &BalloonModel) -> WritingMode {
        WritingDirectionDecision::resolve(model).mode()
    }
}

/// SSP 正典キー `vertical` の「横書き」を表す受理値（R1.2）。
const VALUE_VERTICAL_OFF: &str = "0";
/// SSP 正典キー `vertical` の「縦書き」を表す受理値（R1.1）。
const VALUE_VERTICAL_ON: &str = "1";

/// 正典キー `vertical` の宣言の分類（未宣言・不正値を潰さない）。
///
/// 未宣言（[`Undeclared`](Self::Undeclared)）と `0` の宣言（[`Horizontal`](Self::Horizontal)）を
/// 区別して保つのは、共存規則の裁定に**宣言の有無**が要るためである（R1.4）。
/// 表示結果としては両者とも横書きで同一になる（R1.3）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VerticalDecl {
    /// キーが無い。
    Undeclared,
    /// `0`＝横書きの宣言。
    Horizontal,
    /// `1`＝縦書きの宣言。
    Vertical,
    /// `0`／`1` 以外または空文字列（警告済み・共存規則では「指定なし」として扱う）。
    Invalid,
}

impl VerticalDecl {
    /// 有効な宣言であれば、それが意味する書字方向を返す。
    ///
    /// `1` は日本語縦書き＝右から左へ列送り（[`WritingMode::VerticalRl`]）へ写す（R2.2）。
    /// [`Invalid`](Self::Invalid) は [`Undeclared`](Self::Undeclared) と同じく `None`——
    /// 壊れた値は「指定なし」として共存規則へ合流する（設計 DD6）。
    fn declared_mode(self) -> Option<WritingMode> {
        match self {
            VerticalDecl::Horizontal => Some(WritingMode::HorizontalTb),
            VerticalDecl::Vertical => Some(WritingMode::VerticalRl),
            VerticalDecl::Undeclared | VerticalDecl::Invalid => None,
        }
    }
}

/// areka 拡張キー `writing_mode` の宣言の分類。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WritingModeDecl {
    /// キーが無い。
    Undeclared,
    /// 受理語彙 3 種のいずれかの宣言。
    Declared(WritingMode),
    /// 受理語彙外（警告済み・「指定なし」として扱う・要件 2.7）。
    Unknown,
}

impl WritingModeDecl {
    /// 有効な宣言であれば、それが意味する書字方向を返す。
    ///
    /// [`Unknown`](Self::Unknown) は [`Undeclared`](Self::Undeclared) と同じく `None`
    /// （要件 2.7・[`VerticalDecl::Invalid`] と対称）。
    fn declared_mode(self) -> Option<WritingMode> {
        match self {
            WritingModeDecl::Declared(mode) => Some(mode),
            WritingModeDecl::Undeclared | WritingModeDecl::Unknown => None,
        }
    }
}

/// どちらの宣言を採ったか。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DirectionSource {
    /// 有効な宣言が無く正典既定（横書き）を用いた。
    CanonDefault,
    /// 正典キー `vertical` を採った。
    CanonKey,
    /// areka 拡張キー `writing_mode` を採った（矛盾併記の解決を含む）。
    ///
    /// 両キーが有効に宣言されているときは**常に**本出所になる（併記が同じ方向を意味する
    /// 場合も含む）——優先順位は方向の一致・不一致に依らず一定であり、一致・不一致は
    /// 記録の有無だけを変える（要件 2.4／2.5）。
    ExtensionKey,
}

/// 書字方向の決定と、その決定の記録（正典再改訂に対する唯一の追随点）。
///
/// [`resolve`](Self::resolve) が 2 キーを独立に分類してから裁定し、結果とともに
/// **なぜその結果になったか**（採用出所・両キーの分類・矛盾併記の有無）を保持する。
/// 生の宣言文字列は持たない（[`Copy`] を保つため）——文字列を要する記録は
/// [`resolve`](Self::resolve) の内側でその場で発行する。
///
/// 本型は `ResolvedBalloonText` へ配線していない（現時点で消費者が居ないため・
/// design.md C2 Responsibilities）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WritingDirectionDecision {
    mode: WritingMode,
    source: DirectionSource,
    vertical_declaration: VerticalDecl,
    writing_mode_declaration: WritingModeDecl,
    conflicting: bool,
}

impl WritingDirectionDecision {
    /// 2 層マージ済み `BalloonModel` から解決する（副作用はログのみ）。
    ///
    /// 記録水準（design.md「記録水準の割当」が正本）:
    ///
    /// - `vertical` が `0`／`1` 以外または空 → `warn!` 1 件（R1.6／R1.7）
    /// - `writing_mode` が受理語彙外 → `warn!` 1 件（現行の文言のまま・R2.7）
    /// - 両キーが有効宣言かつ**異なる**方向 → `debug!` 1 件（両キーの生値を構造化
    ///   フィールドで記録・R2.5）
    /// - 両キーが有効宣言かつ同じ方向／両キーとも宣言なし → **記録しない**（R2.4／R1.3）
    pub fn resolve(model: &BalloonModel) -> WritingDirectionDecision {
        let vertical_declaration = classify_vertical(model.vertical_raw());
        let writing_mode_declaration = classify_writing_mode(model.writing_mode());

        let (mode, source, conflicting) = match (
            vertical_declaration.declared_mode(),
            writing_mode_declaration.declared_mode(),
        ) {
            // 有効な宣言が 1 つも無い——正典既定の横書き（正常系につき記録しない）。
            (None, None) => (
                WritingMode::HorizontalTb,
                DirectionSource::CanonDefault,
                false,
            ),
            // 正典キー単独。
            (Some(canon), None) => (canon, DirectionSource::CanonKey, false),
            // 拡張キー単独（`writing_mode` 単独指定時の結果は現行と 1 ビットも変わらない・R2.3）。
            (None, Some(extension)) => (extension, DirectionSource::ExtensionKey, false),
            // 併記——優先順位は拡張キー（要件の裁定 1）。方向が異なるときだけ記録する。
            (Some(canon), Some(extension)) => {
                let conflicting = canon != extension;
                if conflicting {
                    tracing::debug!(
                        vertical = model.vertical_raw().unwrap_or_default(),
                        writing_mode = model.writing_mode().unwrap_or_default(),
                        "vertical と writing_mode が異なる書字方向を指すため areka 拡張キー writing_mode を採用する（正典キー vertical は採らない）"
                    );
                }
                (extension, DirectionSource::ExtensionKey, conflicting)
            }
        };

        WritingDirectionDecision {
            mode,
            source,
            vertical_declaration,
            writing_mode_declaration,
            conflicting,
        }
    }

    /// 実際に適用される書字方向。
    pub fn mode(&self) -> WritingMode {
        self.mode
    }

    /// 採用した宣言の出所。
    pub fn source(&self) -> DirectionSource {
        self.source
    }

    /// 正典キーの宣言の分類。
    pub fn vertical_declaration(&self) -> VerticalDecl {
        self.vertical_declaration
    }

    /// 拡張キーの宣言の分類。
    pub fn writing_mode_declaration(&self) -> WritingModeDecl {
        self.writing_mode_declaration
    }

    /// 双方が有効に宣言され、かつ異なる方向を意味していたか。
    ///
    /// 判定は**両宣言が意味する [`WritingMode`] の相違**で行う。したがって
    /// `vertical,1` ＋ `writing_mode,vertical_lr` は「異なる方向」であり
    /// （正典キーが意味する [`VerticalRl`](WritingMode::VerticalRl) を採らないため）、
    /// `debug!` の対象になる。
    pub fn conflicting(&self) -> bool {
        self.conflicting
    }

    /// 正典プロパティ `currentghost.balloon.scope(ID).vertical` の導出規則（要件 7.1・**語彙**）。
    ///
    /// 当該スコープに実際に適用されている書字方向（[`mode`](Self::mode)）から一意に定まる:
    /// 縦書きで `1`・横書きで `0`。**`vertical_lr`（areka 拡張の縦書き左送り）も `1`** である
    /// ——正典の語彙は縦横 2 値であり、列送りの向きを区別しない。
    ///
    /// 本仕様はこの値を publish しない（語彙表登録も照会経路の新設も行わない）。
    /// プロパティの実導出は追跡 spec `areka-P0-currentghost-property-tree` が所有する（DD7）。
    pub fn vertical_property_value(&self) -> u8 {
        match self.mode {
            WritingMode::HorizontalTb => 0,
            WritingMode::VerticalRl | WritingMode::VerticalLr => 1,
        }
    }
}

/// 正典キー `vertical` の生値を分類する（不正値はここで `warn!` 1 件・R1.6／R1.7）。
fn classify_vertical(raw: Option<&str>) -> VerticalDecl {
    match raw {
        None => VerticalDecl::Undeclared,
        Some(VALUE_VERTICAL_OFF) => VerticalDecl::Horizontal,
        Some(VALUE_VERTICAL_ON) => VerticalDecl::Vertical,
        Some(invalid) => {
            tracing::warn!(
                value = invalid,
                "vertical の値が 0／1 のいずれでもないため指定なしとして扱う（受理値: 0 / 1）"
            );
            VerticalDecl::Invalid
        }
    }
}

/// 拡張キー `writing_mode` の生値を分類する（未知値はここで `warn!` 1 件・R5.4／R2.7）。
///
/// 語彙は snake_case 完全一致（trim は parser の kv 層で済んでいる前提・R5.1）。
/// 警告の文言と件数は現行のまま（既存インラインテストが逐語固定している）。
fn classify_writing_mode(raw: Option<&str>) -> WritingModeDecl {
    match raw {
        None => WritingModeDecl::Undeclared,
        Some(VALUE_HORIZONTAL_TB) => WritingModeDecl::Declared(WritingMode::HorizontalTb),
        Some(VALUE_VERTICAL_RL) => WritingModeDecl::Declared(WritingMode::VerticalRl),
        Some(VALUE_VERTICAL_LR) => WritingModeDecl::Declared(WritingMode::VerticalLr),
        Some(unknown) => {
            tracing::warn!(
                value = unknown,
                "未知の writing_mode 値のため horizontal_tb へフォールバックする（受理語彙: horizontal_tb / vertical_rl / vertical_lr）"
            );
            WritingModeDecl::Unknown
        }
    }
}

#[cfg(test)]
mod tests {

    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };
    use log_capture_kit::count_levels;

    use super::{RESERVED_KEY_TEXT_COMBINE_UPRIGHT, RESERVED_KEY_TEXT_ORIENTATION, WritingMode};

    /// テスト用 BalloonModel 生成ヘルパ（writing_mode 以外は全成分未指定）。
    fn model(writing_mode: Option<&str>) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(None, None),
            WordWrapPoint::new(None, None),
            ValidRect::new(None, None, None, None),
            Font::new(None, None, FontColor::new(None, None, None)),
            writing_mode.map(str::to_owned),
            None,
        )
    }

    /// resolve を共有のログ捕捉窓の中で実行し、（解決結果, WARN 件数）を返す。
    ///
    /// 件数の集計は硬化機構の唯一の定義元 `log-capture-kit` の [`count_levels`] に委ねる。
    /// 戻り値の組は移行前と同一で、呼出側の判定内容は変わらない。
    fn resolve_counting_warns(writing_mode: Option<&str>) -> (WritingMode, usize) {
        let (mode, counts) = count_levels(|| WritingMode::resolve(&model(writing_mode)));
        (mode, counts.warn)
    }

    // ── R5.1/R5.5: 3 語彙の受理と方向写像 1:1（CSS 語彙 snake_case→3 variant 単射） ──

    #[test]
    fn horizontal_tb_resolves_to_horizontal_tb() {
        assert_eq!(
            WritingMode::resolve(&model(Some("horizontal_tb"))),
            WritingMode::HorizontalTb
        );
    }

    #[test]
    fn vertical_rl_resolves_to_vertical_rl() {
        assert_eq!(
            WritingMode::resolve(&model(Some("vertical_rl"))),
            WritingMode::VerticalRl
        );
    }

    #[test]
    fn vertical_lr_resolves_to_vertical_lr() {
        assert_eq!(
            WritingMode::resolve(&model(Some("vertical_lr"))),
            WritingMode::VerticalLr
        );
    }

    /// 方向写像は 1:1（R5.5）——3 語彙は互いに異なる variant へ写る（単射）。
    #[test]
    fn three_vocabulary_values_map_injectively() {
        let modes = [
            WritingMode::resolve(&model(Some("horizontal_tb"))),
            WritingMode::resolve(&model(Some("vertical_rl"))),
            WritingMode::resolve(&model(Some("vertical_lr"))),
        ];
        assert_ne!(modes[0], modes[1]);
        assert_ne!(modes[0], modes[2]);
        assert_ne!(modes[1], modes[2]);
    }

    /// 既知 3 語彙の受理は警告を出さない（warn は未知値専用・R5.1/R5.4）。
    #[test]
    fn known_values_resolve_without_warning() {
        for value in ["horizontal_tb", "vertical_rl", "vertical_lr"] {
            let (_, warns) = resolve_counting_warns(Some(value));
            assert_eq!(warns, 0, "value {value:?} は warn なしで受理される");
        }
    }

    // ── R5.3: マーカー無し→横書き既定（SSP 互換） ──

    #[test]
    fn missing_marker_defaults_to_horizontal_tb() {
        let (mode, warns) = resolve_counting_warns(None);
        assert_eq!(mode, WritingMode::HorizontalTb);
        // 未指定は正常系（SSP 互換既定）——warn は記録しない。
        assert_eq!(warns, 0);
    }

    /// 型の Default も横書き（design.md の #[default] 正準と一致）。
    #[test]
    fn writing_mode_default_is_horizontal_tb() {
        assert_eq!(WritingMode::default(), WritingMode::HorizontalTb);
    }

    // ── R5.4: 未知値→warn ログ＋横書きフォールバック ──

    #[test]
    fn unknown_value_falls_back_to_horizontal_tb_with_warn() {
        // parsers 側の未知値素通しテストと同じ値（diagonal_bt）で連続性を持たせる。
        let (mode, warns) = resolve_counting_warns(Some("diagonal_bt"));
        assert_eq!(mode, WritingMode::HorizontalTb);
        assert_eq!(warns, 1, "未知値はちょうど 1 回 warn を記録する");
    }

    /// 語彙は snake_case 完全一致（R5.1）——ハイフン形（CSS 原表記）や大文字・空白は未知値扱い。
    #[test]
    fn near_miss_values_are_unknown_and_fall_back() {
        for value in ["vertical-rl", "VERTICAL_RL", "vertical_rl ", ""] {
            let (mode, warns) = resolve_counting_warns(Some(value));
            assert_eq!(
                mode,
                WritingMode::HorizontalTb,
                "value {value:?} は横書きへフォールバックする"
            );
            assert_eq!(warns, 1, "value {value:?} は warn を記録する");
        }
    }

    // ── R5.2: 2層マージ後の転記値を読むだけ（マージ済み単一値で解決） ──

    /// 2層マージは balloon-parse 側で解決済み——本層は `parse` が転記した後勝ち単一値を
    /// 読むだけで、画像別上書きの結果がそのまま有効値になる。
    #[test]
    fn resolves_from_two_layer_merged_transcription() {
        use areka_parsers::balloon::parse;
        use std::collections::BTreeMap;

        let descript: BTreeMap<String, String> =
            [("writing_mode".to_owned(), "horizontal_tb".to_owned())].into();
        let image: BTreeMap<String, String> =
            [("writing_mode".to_owned(), "vertical_rl".to_owned())].into();
        let merged = parse(&descript, Some(&image));
        assert_eq!(WritingMode::resolve(&merged), WritingMode::VerticalRl);
    }

    // ── R5.7: M2 予約キーは名前の記録のみ（実挙動なし） ──

    #[test]
    fn m2_reserved_key_names_are_recorded_as_constants() {
        assert_eq!(RESERVED_KEY_TEXT_ORIENTATION, "text_orientation");
        assert_eq!(RESERVED_KEY_TEXT_COMBINE_UPRIGHT, "text_combine_upright");
    }
}

#[cfg(test)]
#[path = "writing_decision_tests.rs"]
mod decision_tests;
