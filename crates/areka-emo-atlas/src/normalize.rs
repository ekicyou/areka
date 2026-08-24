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
//! 共有透過パラメータ型（`AlphaParams` / `UseSelfAlpha`）は `SurfaceSet` が出所単位で
//! 運ぶため列挙タスク（2.1）先行で定義された。正規化ロジック本体（`normalize()` /
//! `NormalizedImage` / `AlphaSource` / `NormalizeError`）は Normalizer タスク（2.3）が
//! 本モジュールへ追加した。

use crate::decode::DecodedImage;

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

/// 採用された透過ソース（型シーム・emo2 は `AlphaChannel` のみ実装・3.5）。
///
/// 動作表（D5）の選択結果を表す。`AlphaChannel` 以外の腕は実装せず型の口のみで、
/// 到達時は `NormalizeError::Unsupported` にこの値を載せて「どの腕が選択されたか」を
/// 呼び出し側へ示す（優先順位 3.3 の検証に用いる）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaSource {
    /// 画像自身の α チャンネルを透明度として採用（emo2 実装腕・3.1）。
    AlphaChannel,
    /// 同名 `.pna` グレースケールを α として採用（シーム・未実装）。
    Pna,
    /// 画像左上ピクセルをキー色とするキーカラー透過（シーム・未実装）。
    KeyColor,
    /// 全面不透明（`full` かつ α 無し・キー色透過しない）（シーム・未実装）。
    Opaque,
}

/// 正規化済み画像（常に premultiplied BGRA・3.4/D8）。
///
/// `pbgra` の長さは常に `stride * height`。α チャンネル採用腕はデコード出力
/// （WIC PBGRA）を素通しするため実質恒等で、premultiplied のまま保持される
/// （straight α 混入禁止＝にじみ/暗縁防止・D8）。
#[derive(Clone, Debug)]
pub struct NormalizedImage {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// premultiplied BGRA 画素バッファ（`len == stride * height`）。
    pub pbgra: Vec<u8>,
}

/// 正規化失敗（未実装の透過腕への到達）。
#[derive(Debug)]
pub enum NormalizeError {
    /// 未実装の透過腕に到達（型シームの誤使用検出・3.5）。
    ///
    /// 載せる `AlphaSource` は動作表（D5）で**選択された**ソースで、優先順位
    /// （α ＞ `.pna` ＞ キーカラー・3.3）に従い決まる。emo2 経路
    /// （`use_self_alpha=On` かつ α 有り）では発生しない。
    Unsupported(AlphaSource),
}

impl std::fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NormalizeError::Unsupported(src) => {
                write!(f, "normalize: unsupported alpha source: {src:?}")
            }
        }
    }
}

impl std::error::Error for NormalizeError {}

/// 透過正規化器（`use_self_alpha` 解釈・premultiplied BGRA 統一）。
///
/// 上流由来の `AlphaParams` を入力として受け、自ら設定ファイルを読まない（3.6）。
pub struct Normalizer;

impl Normalizer {
    /// 動作表（D5）から採用透過ソースを選択する。
    ///
    /// 優先順位 α ＞ `.pna` ＞ キーカラー（3.3）に従う。純粋関数（副作用なし・
    /// ファイル I/O なし・3.6）。実装腕・シーム腕の判定は呼び出し側で行う。
    fn select_source(params: AlphaParams, has_alpha: bool, has_pna: bool) -> AlphaSource {
        match params.use_self_alpha {
            // 1 / true: α ＞ .pna ＞ キーカラー。
            UseSelfAlpha::On => {
                if has_alpha {
                    AlphaSource::AlphaChannel
                } else if has_pna {
                    AlphaSource::Pna
                } else {
                    AlphaSource::KeyColor
                }
            }
            // full: α 有りは α 採用・α 無しは全面不透明（キー色透過しない）。
            UseSelfAlpha::Full => {
                if has_alpha {
                    AlphaSource::AlphaChannel
                } else {
                    AlphaSource::Opaque
                }
            }
            // 0（既定・旧挙動）: 自身の α チャンネルは無視し .pna ＞ キーカラー。
            UseSelfAlpha::Off => {
                if has_pna {
                    AlphaSource::Pna
                } else {
                    AlphaSource::KeyColor
                }
            }
        }
    }

    /// デコード済み画像を透過解釈し premultiplied BGRA へ統一する。
    ///
    /// emo2 実装腕＝`use_self_alpha=On` かつ α チャンネル有り（採用ソース＝
    /// `AlphaChannel`）のみ実装（3.1）。この腕はデコード出力が既に premultiplied
    /// （WIC PBGRA・D8）ゆえ実質恒等で、画素変換を一切行わずバッファを move する。
    /// 他の全組合せはシーム（`.pna`／キーカラー／`full`／不透明）で、選択された
    /// `AlphaSource` を載せた `NormalizeError::Unsupported` を返す（3.2/3.5）。
    /// 透過パラメータは入力として受け、自ら設定を読みに行かない（3.6）。
    pub fn normalize(
        &self,
        img: DecodedImage,
        params: AlphaParams,
        has_pna: bool,
    ) -> Result<NormalizedImage, NormalizeError> {
        let source = Self::select_source(params, img.has_alpha, has_pna);
        // 実装腕（emo2）は動作表（D5）でただ 1 行＝`use_self_alpha=On` かつ
        // α チャンネル採用のみ。`full` が α を選ぶ行は表上シームゆえ、採用ソースが
        // `AlphaChannel` でも `On` 以外は実装しない（未実装で明示エラー）。
        match (params.use_self_alpha, source) {
            // 実装腕: WIC PBGRA を素通し＝恒等（D8・3.4）。画素変換なし。
            (UseSelfAlpha::On, AlphaSource::AlphaChannel) => Ok(NormalizedImage {
                width: img.width,
                height: img.height,
                stride: img.stride,
                pbgra: img.bgra,
            }),
            // シーム腕（未実装）: 選択ソースを載せて明示エラー（3.2/3.5）。
            (_, other) => Err(NormalizeError::Unsupported(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::DecodedImage;

    /// 既知の premultiplied BGRA バイト列を持つ 2×2 画像を作る。
    ///
    /// premultiplied 例: 半透明赤（straight R=255,α=128）→ premultiplied では
    /// R が α で乗算され R≈128。ここでは検証用に「明らかに premultiplied な」
    /// 決め打ちバイト列を使い、恒等腕がこれを一切変えないことを確認する。
    fn img(has_alpha: bool) -> DecodedImage {
        let (w, h) = (2u32, 2u32);
        let stride = w * 4;
        // BGRA・premultiplied（B,G,R <= A）。4 画素分・16 byte。
        let bgra: Vec<u8> = vec![
            0, 0, 128, 128, // px0: premul 赤 α=128
            0, 64, 0, 64, // px1: premul 緑 α=64
            255, 255, 255, 255, // px2: 不透明白
            0, 0, 0, 0, // px3: 完全透明
        ];
        assert_eq!(bgra.len(), (stride * h) as usize);
        DecodedImage {
            width: w,
            height: h,
            stride,
            bgra,
            has_alpha,
        }
    }

    fn params(use_self_alpha: UseSelfAlpha) -> AlphaParams {
        AlphaParams { use_self_alpha }
    }

    /// 主経路（3.1/3.4/D8）: On + α → Ok・premultiplied を恒等で素通し。
    ///
    /// 出力 `pbgra` が入力 `bgra` とバイト単位で一致（straight α 混入なし）し、
    /// 寸法・stride が保存され、`len == stride * height` を満たすことを確認する。
    #[test]
    fn on_with_alpha_is_identity_premultiplied() {
        let src = img(true);
        let expected = src.bgra.clone();
        let (w, h, stride) = (src.width, src.height, src.stride);

        let out = Normalizer
            .normalize(src, params(UseSelfAlpha::On), false)
            .expect("On + alpha is the implemented emo2 arm");

        // 恒等: 画素は 1 byte も変換されない（premultiplied 一貫性・D8）。
        assert_eq!(out.pbgra, expected);
        assert_eq!(out.width, w);
        assert_eq!(out.height, h);
        assert_eq!(out.stride, stride);
        // 不変: len == stride * height。
        assert_eq!(out.pbgra.len(), (out.stride * out.height) as usize);
    }

    /// 優先順位 α ＞ `.pna`（3.3）: On + α + .pna 有り → なお AlphaChannel 腕（Ok）。
    ///
    /// `.pna` が在っても α が勝ち、`Unsupported(Pna)` にはならないことを示す。
    #[test]
    fn on_alpha_beats_pna() {
        let out = Normalizer
            .normalize(img(true), params(UseSelfAlpha::On), true)
            .expect("alpha wins over .pna (priority 3.3)");
        assert_eq!(out.pbgra.len(), (out.stride * out.height) as usize);
    }

    /// シーム（3.2/3.5）: On + α 無し + .pna 有り → Unsupported(Pna)。
    #[test]
    fn on_no_alpha_with_pna_selects_pna_seam() {
        match Normalizer.normalize(img(false), params(UseSelfAlpha::On), true) {
            Err(NormalizeError::Unsupported(src)) => assert_eq!(src, AlphaSource::Pna),
            other => panic!("expected Unsupported(Pna), got {other:?}"),
        }
    }

    /// シーム（3.2/3.3/3.5）: On + α 無し + .pna 無し → Unsupported(KeyColor)。
    #[test]
    fn on_no_alpha_no_pna_selects_keycolor_seam() {
        match Normalizer.normalize(img(false), params(UseSelfAlpha::On), false) {
            Err(NormalizeError::Unsupported(src)) => assert_eq!(src, AlphaSource::KeyColor),
            other => panic!("expected Unsupported(KeyColor), got {other:?}"),
        }
    }

    /// シーム: full + α 有り → Unsupported(AlphaChannel)（full 腕は未実装）。
    #[test]
    fn full_with_alpha_selects_alphachannel_but_seam() {
        match Normalizer.normalize(img(true), params(UseSelfAlpha::Full), false) {
            Err(NormalizeError::Unsupported(src)) => assert_eq!(src, AlphaSource::AlphaChannel),
            other => panic!("expected Unsupported(AlphaChannel), got {other:?}"),
        }
    }

    /// シーム: full + α 無し → Unsupported(Opaque)（全面不透明・キー色透過しない）。
    #[test]
    fn full_no_alpha_selects_opaque_seam() {
        // .pna 有無に依らず Opaque（full 行は .pna を参照しない）。
        for has_pna in [false, true] {
            match Normalizer.normalize(img(false), params(UseSelfAlpha::Full), has_pna) {
                Err(NormalizeError::Unsupported(src)) => assert_eq!(src, AlphaSource::Opaque),
                other => panic!("expected Unsupported(Opaque), got {other:?}"),
            }
        }
    }

    /// シーム（legacy）: Off + .pna 有り → Unsupported(Pna)（自身の α を無視）。
    ///
    /// α 有りでも Off は α チャンネルを採用せず .pna を選ぶことを示す。
    #[test]
    fn off_with_pna_ignores_own_alpha_selects_pna_seam() {
        for has_alpha in [false, true] {
            match Normalizer.normalize(img(has_alpha), params(UseSelfAlpha::Off), true) {
                Err(NormalizeError::Unsupported(src)) => assert_eq!(src, AlphaSource::Pna),
                other => panic!("expected Unsupported(Pna), got {other:?}"),
            }
        }
    }

    /// シーム（legacy）: Off + .pna 無し → Unsupported(KeyColor)。
    #[test]
    fn off_no_pna_selects_keycolor_seam() {
        for has_alpha in [false, true] {
            match Normalizer.normalize(img(has_alpha), params(UseSelfAlpha::Off), false) {
                Err(NormalizeError::Unsupported(src)) => assert_eq!(src, AlphaSource::KeyColor),
                other => panic!("expected Unsupported(KeyColor), got {other:?}"),
            }
        }
    }

    /// 3.6（構造的）: normalize は `AlphaParams` と `has_pna` のみを入力に取り、
    /// 設定ファイルを読まない。本テスト群は手組みデータのみで走り、いかなる
    /// ファイルの存在にも依存しない（ファイル I/O 不在の実証）。
    #[test]
    fn normalize_reads_no_config_pure_from_inputs() {
        // 手組み画像＋注入パラメータのみで完結する（外部状態への依存ゼロ）。
        let out = Normalizer
            .normalize(img(true), params(UseSelfAlpha::On), false)
            .expect("pure normalize from injected inputs only");
        assert_eq!(out.pbgra.len(), (out.stride * out.height) as usize);
    }
}
