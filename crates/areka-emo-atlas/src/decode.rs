//! デコードポート定義（`ElementDecoder` trait ＋ `DecodedImage` / `DecodeError`）。
//!
//! デコード手段を差し替え可能にするための trait ポート。既定腕＝WIC（COM 必要・
//! [`wic_arm`] に隔離）／テスト腕＝メモリ PBGRA。正規化以降のコアは COM 非依存で、
//! 既定手段を上位へ露出しない。デコード出力は premultiplied BGRA を
//! 想定し、変換前フレームのピクセルフォーマット由来の α 有無を保持する。
//!
//! 既定の腕は、静止画が [`wic_arm`]、動く絵が `image_arm`。
//!
//! 動く絵（コマが 2 枚以上の APNG・WebP）は「見出しを聞く」（`probe_animation`）→
//! 「全コマを読む」（`decode_frames`）／「動きの 1 枚目だけを読む」（`decode_first_frame`）
//! で読む。3 つとも既定の実装を持つので、静止画しか読まない読み手は書き換え不要。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::table::LoopCount;

mod image_arm;
mod sniff;
pub(crate) mod wic_arm;

/// デコード済み画像（BGRA8・非トリム・原寸）。
///
/// WIC 腕は既に premultiplied を返すが、premultiplied 保証の権威は正規化段。
/// `bgra` の長さは常に `stride * height`（後述 Postcondition）。
#[derive(Clone, Debug)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// 画素バッファ（BGRA8・`len == stride * height`）。
    pub bgra: Vec<u8>,
    /// α チャンネル有無（正規化の腕選択に使用）。
    pub has_alpha: bool,
}

/// 絵の一辺の上限（Direct3D 11 の機能水準 11 の 2D テクスチャの一辺・areka-emo-compose の MAX_EXTENT と同じ値）。
pub(crate) const MAX_SIDE: u32 = 16_384;

/// 一辺の上限を確かめ、BGRA の画素の入れ物の長さ（width * 4 * height）を usize で返す。
/// 幅か高さが MAX_SIDE を越えたら Err（寸法と上限を書いた文）。掛け算はあふれない（最大 2^30）。
pub(crate) fn pixel_len(width: u32, height: u32) -> Result<usize, String> {
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(format!(
            "絵の寸法 {width}x{height} が一辺の上限 {MAX_SIDE} を越える"
        ));
    }
    Ok(width as usize * 4 * height as usize)
}

/// 読み手の答えが約束を守るか: 一辺が MAX_SIDE 以下・stride >= width * 4・bgra.len() == stride * height
/// （比べは u64）。破れば Err（寸法・stride・長さを書いた文）。
pub(crate) fn check_image(img: &DecodedImage) -> Result<(), String> {
    pixel_len(img.width, img.height)?;
    let (w, h, stride, len) = (
        u64::from(img.width),
        u64::from(img.height),
        u64::from(img.stride),
        img.bgra.len() as u64,
    );
    if stride < w * 4 || len != stride * h {
        return Err(format!(
            "読み手の答えが約束を破る（寸法 {w}x{h}・stride {stride}・長さ {len}）"
        ));
    }
    Ok(())
}

/// デコード失敗の診断可能なエラー（不在・破損）。失敗パスを常に保持する。
#[derive(Debug)]
pub enum DecodeError {
    /// 対象パスが存在しない。
    NotFound { path: PathBuf },
    /// 対象は在るがデコードできない（破損等）。`source` は原因の説明文字列。
    Decode { path: PathBuf, source: String },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::NotFound { path } => {
                write!(f, "decode: file not found: {}", path.display())
            }
            DecodeError::Decode { path, source } => {
                write!(f, "decode: failed to decode {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// 動く絵の見出し（全コマを読む前に分かること）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationInfo {
    /// 絵の全体の幅。
    pub width: u32,
    /// 絵の全体の高さ。
    pub height: u32,
    /// 動きのコマの枚数（2 以上）。
    pub frame_count: u32,
}

/// 重ね済みの 1 コマ。
#[derive(Clone, Debug)]
pub struct AnimationFrame {
    /// 乗算済み BGRA・絵の全体の寸法。
    pub image: DecodedImage,
    /// 待ち時間（ミリ秒・0 は 0 のまま）。
    pub delay_ms: u32,
}

/// 動く絵の全部。
#[derive(Clone, Debug)]
pub struct AnimatedImage {
    /// コマはファイルの順（0 番から）。
    pub frames: Vec<AnimationFrame>,
    pub loop_count: LoopCount,
}

/// 動く絵を読めない読み手の既定の答え（`Decode`・パス付き）。
fn not_animated(path: &Path) -> DecodeError {
    DecodeError::Decode {
        path: path.to_path_buf(),
        source: "this decoder does not read animated images".into(),
    }
}

/// 乗算しない読みを持たない読み手の既定の答え（`Decode`・パス付き）。
fn no_unmultiplied(path: &Path) -> DecodeError {
    DecodeError::Decode {
        path: path.to_path_buf(),
        source: "this decoder has no non-premultiplied read".into(),
    }
}

/// 差替可能デコードポート。既定手段（WIC）を上位へ露出しない。
///
/// trait 面は「パス→デコード済み BGRA 画像」の最小面のみで、WIC/COM 型を一切公開しない。
pub trait ElementDecoder {
    /// 実パスから画素バッファを得る。成功時 `bgra.len() == stride*height`。
    /// 失敗（不在・破損）は `DecodeError`（パス付き）で返す。副作用はファイル読取のみ。
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError>;

    /// 同名 `.pna` の有無（焼きの段が数えるだけで、透過の決定には使わない・emo2 は常に false）。
    fn probe_pna(&self, _path: &Path) -> bool {
        false
    }

    /// 動く絵（コマが 2 枚以上の APNG・WebP）なら見出しを返す。既定は None（動かない）。
    fn probe_animation(&self, _path: &Path) -> Option<AnimationInfo> {
        None
    }

    /// 動く絵の全コマを読む。`info` は `probe_animation` が返した見出し（読むのは
    /// `info.frame_count` 枚まで）。`Ok` ならコマは `info.frame_count` 枚ちょうどで、全コマ
    /// `info` と同じ寸法・同じ `has_alpha`。1 コマでも読めなければ `Err`。
    /// 既定は `Err`（この読み手は動く絵を読めない）。
    fn decode_frames(
        &self,
        path: &Path,
        _info: AnimationInfo,
    ) -> Result<AnimatedImage, DecodeError> {
        Err(not_animated(path))
    }

    /// 動く絵の動きの 1 枚目だけを読む（縮めるときの段 1）。全コマは読まない・抱えない。
    /// 返す絵は、`decode_frames` が `Ok` のときの `frames[0].image` と同じ画素・同じ
    /// `has_alpha`。既定は `Err`。
    fn decode_first_frame(
        &self,
        path: &Path,
        _info: AnimationInfo,
    ) -> Result<DecodedImage, DecodeError> {
        Err(not_animated(path))
    }

    /// 1 枚目のコマを、色に α を掛けない BGRA8 で読む。抜き色・`seriko.use_self_alpha`・
    /// `.pna` は当てない。α の無い絵は全部の画素が α＝255。一辺の上限は `decode` と同じ。
    /// 既定は `Err`（この読み手は乗算しない読みを持たない）。
    fn decode_unmultiplied(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        Err(no_unmultiplied(path))
    }
}

/// `MemoryDecoder::insert_animated` の登録 1 件。
struct AnimatedEntry {
    info: AnimationInfo,
    first: Result<DecodedImage, String>,
    frames: Result<AnimatedImage, String>,
}

/// メモリ上の登録データから画素バッファを返すテスト腕（COM 非依存）。
///
/// パス→画像 / パス→破損マーカ / `.pna` 有りパスの各写像を保持し、`ElementDecoder`
/// 契約に従って応答する。後続 bake 単体テストやポート契約テストが差替可能な test double
/// として再利用できるよう `pub`（`#[cfg(test)]` にしない）。
#[derive(Default)]
pub struct MemoryDecoder {
    images: HashMap<PathBuf, DecodedImage>,
    /// 破損として登録したパス→`Decode.source` 文字列。
    corrupt: HashMap<PathBuf, String>,
    pnas: HashSet<PathBuf>,
    animated: HashMap<PathBuf, AnimatedEntry>,
    unmultiplied: HashMap<PathBuf, DecodedImage>,
}

impl MemoryDecoder {
    /// 空の腕を作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// パスに対する画像を登録する。以降 `decode(path)` はこの画像を返す。
    pub fn insert(
        &mut self,
        path: impl Into<PathBuf>,
        width: u32,
        height: u32,
        stride: u32,
        bgra: Vec<u8>,
        has_alpha: bool,
    ) {
        self.images.insert(
            path.into(),
            DecodedImage {
                width,
                height,
                stride,
                bgra,
                has_alpha,
            },
        );
    }

    /// パスを「破損（デコード不能）」として登録する。`decode` は `Decode` を返す。
    pub fn insert_corrupt(&mut self, path: impl Into<PathBuf>, source: impl Into<String>) {
        self.corrupt.insert(path.into(), source.into());
    }

    /// パスに同名 `.pna` が在るものとして登録する。`probe_pna` が true を返す。
    pub fn insert_pna(&mut self, path: impl Into<PathBuf>) {
        self.pnas.insert(path.into());
    }

    /// 動く絵として登録する。`probe_animation` は `info` を返す。`frames` が `Err` なら
    /// `decode_frames` は `Decode` の失敗を返す（渡された info は見ない）。`first` は
    /// `decode_first_frame` の答え（段 1）。見出しと中身は照合せず登録どおりに返すので、
    /// 「見出しと枚数が食い違う」も作れる。段 2 で読まれる 1 枚（今までの `decode` の答え）は
    /// `insert`／`insert_corrupt` で別に登録する。
    pub fn insert_animated(
        &mut self,
        path: impl Into<PathBuf>,
        info: AnimationInfo,
        first: Result<DecodedImage, String>,
        frames: Result<AnimatedImage, String>,
    ) {
        self.animated.insert(
            path.into(),
            AnimatedEntry {
                info,
                first,
                frames,
            },
        );
    }

    /// パスに対する乗算しない絵を登録する。以降 `decode_unmultiplied(path)` はこの絵を返す。
    /// `decode` の答え（`insert`）とは別に持つ。
    pub fn insert_unmultiplied(&mut self, path: impl Into<PathBuf>, image: DecodedImage) {
        self.unmultiplied.insert(path.into(), image);
    }

    /// 登録の答えを、パス付きの `Decode` の失敗へ包んで返す。未登録は既定と同じ失敗。
    fn animated_answer<T: Clone>(
        &self,
        path: &Path,
        pick: impl Fn(&AnimatedEntry) -> &Result<T, String>,
    ) -> Result<T, DecodeError> {
        let Some(entry) = self.animated.get(path) else {
            return Err(not_animated(path));
        };
        pick(entry).clone().map_err(|source| DecodeError::Decode {
            path: path.to_path_buf(),
            source,
        })
    }
}

impl ElementDecoder for MemoryDecoder {
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        if let Some(img) = self.images.get(path) {
            return Ok(img.clone());
        }
        if let Some(source) = self.corrupt.get(path) {
            return Err(DecodeError::Decode {
                path: path.to_path_buf(),
                source: source.clone(),
            });
        }
        Err(DecodeError::NotFound {
            path: path.to_path_buf(),
        })
    }

    fn probe_pna(&self, path: &Path) -> bool {
        self.pnas.contains(path)
    }

    fn probe_animation(&self, path: &Path) -> Option<AnimationInfo> {
        self.animated.get(path).map(|e| e.info)
    }

    fn decode_frames(
        &self,
        path: &Path,
        _info: AnimationInfo,
    ) -> Result<AnimatedImage, DecodeError> {
        self.animated_answer(path, |e| &e.frames)
    }

    fn decode_first_frame(
        &self,
        path: &Path,
        _info: AnimationInfo,
    ) -> Result<DecodedImage, DecodeError> {
        self.animated_answer(path, |e| &e.first)
    }

    /// 乗算しない絵 → その絵・破損 → `Decode`・`insert` だけ → 既定と同じ失敗・未登録 → `NotFound`。
    fn decode_unmultiplied(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        if let Some(img) = self.unmultiplied.get(path) {
            return Ok(img.clone());
        }
        if self.images.contains_key(path) {
            return Err(no_unmultiplied(path));
        }
        self.decode(path)
    }
}

#[cfg(test)]
#[path = "decode_animation_tests.rs"]
mod animation_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// 登録に使う既知のパス。
    fn known_path() -> PathBuf {
        PathBuf::from("shell/surface0.png")
    }

    /// 2×2・stride=8・16byte の小画像を作る（値はダミー）。
    fn small_image(has_alpha: bool) -> crate::test_support::ImageSpec {
        let (w, h) = (2u32, 2u32);
        let stride = w * 4;
        let bgra = vec![0u8; (stride * h) as usize];
        (w, h, stride, bgra, has_alpha)
    }

    /// 契約: 登録済みパスの decode は Ok を返し、寸法・stride・長さ・has_alpha が一致。
    #[test]
    fn port_contract_success() {
        let (w, h, stride, bgra, has_alpha) = small_image(true);
        let mut dec = MemoryDecoder::new();
        dec.insert(known_path(), w, h, stride, bgra.clone(), has_alpha);

        let img = dec.decode(&known_path()).expect("registered path decodes");
        assert_eq!(img.width, w);
        assert_eq!(img.height, h);
        assert_eq!(img.stride, stride);
        assert_eq!(img.has_alpha, has_alpha);
        assert_eq!(img.bgra.len(), (img.stride * img.height) as usize);
        assert_eq!(img.bgra, bgra);
    }

    /// 未登録パスは NotFound（パス付き）で失敗。
    #[test]
    fn not_found_error_carries_path() {
        let dec = MemoryDecoder::new();
        let missing = PathBuf::from("shell/missing.png");
        match dec.decode(&missing) {
            Err(DecodeError::NotFound { path }) => assert_eq!(path, missing),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    /// 破損として登録したパスは Decode（パス＋source 付き）で失敗。
    #[test]
    fn decode_error_carries_path_and_source() {
        let mut dec = MemoryDecoder::new();
        let corrupt = PathBuf::from("shell/corrupt.png");
        dec.insert_corrupt(corrupt.clone(), "broken PNG header");
        match dec.decode(&corrupt) {
            Err(DecodeError::Decode { path, source }) => {
                assert_eq!(path, corrupt);
                assert_eq!(source, "broken PNG header");
            }
            other => panic!("expected Decode, got {other:?}"),
        }
    }

    /// probe_pna: 登録あり→true・なし→false（既定）。
    #[test]
    fn probe_pna_reflects_registration() {
        let (w, h, stride, bgra, _) = small_image(true);
        let mut dec = MemoryDecoder::new();
        let with = PathBuf::from("shell/surface1.png");
        let without = PathBuf::from("shell/surface2.png");
        dec.insert(with.clone(), w, h, stride, bgra.clone(), true);
        dec.insert(without.clone(), w, h, stride, bgra, true);
        dec.insert_pna(with.clone());

        assert!(dec.probe_pna(&with));
        assert!(!dec.probe_pna(&without));
        assert!(!dec.probe_pna(Path::new("shell/nowhere.png")));
    }

    /// 乗算しない読み: 登録した絵を返し、`decode` の登録とは混ぜない。登録が無ければ
    /// 理由つきの失敗（既定の答え・破損・不在）で、空の絵を返さない。
    #[test]
    fn unmultiplied_registration_and_failures() {
        let (w, h, stride, _, _) = small_image(true);
        let straight = DecodedImage {
            width: w,
            height: h,
            stride,
            bgra: [50, 100, 200, 128].repeat(4),
            has_alpha: true,
        };
        let mut dec = MemoryDecoder::new();
        dec.insert_unmultiplied(known_path(), straight.clone());
        let only_premul = PathBuf::from("shell/premul_only.png");
        dec.insert(only_premul.clone(), w, h, stride, vec![0; 16], true);
        let corrupt = PathBuf::from("shell/corrupt.png");
        dec.insert_corrupt(corrupt.clone(), "broken PNG header");

        let img = dec.decode_unmultiplied(&known_path()).expect("registered");
        assert_eq!(img.bgra, straight.bgra);
        assert!(matches!(
            dec.decode(&known_path()),
            Err(DecodeError::NotFound { .. })
        ));
        match dec.decode_unmultiplied(&only_premul) {
            Err(DecodeError::Decode { path, source }) => {
                assert_eq!(path, only_premul);
                assert_eq!(source, "this decoder has no non-premultiplied read");
            }
            other => panic!("expected Decode, got {other:?}"),
        }
        match dec.decode_unmultiplied(&corrupt) {
            Err(DecodeError::Decode { source, .. }) => assert_eq!(source, "broken PNG header"),
            other => panic!("expected Decode, got {other:?}"),
        }
        assert!(matches!(
            dec.decode_unmultiplied(Path::new("shell/nowhere.png")),
            Err(DecodeError::NotFound { .. })
        ));
    }

    /// 既定の実装は理由つきの失敗を返す（ほかの読み手は書き換えない）。
    #[test]
    fn unmultiplied_default_is_error() {
        struct Plain;
        impl ElementDecoder for Plain {
            fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
                Err(DecodeError::NotFound {
                    path: path.to_path_buf(),
                })
            }
        }
        match Plain.decode_unmultiplied(&known_path()) {
            Err(DecodeError::Decode { path, source }) => {
                assert_eq!(path, known_path());
                assert_eq!(source, "this decoder has no non-premultiplied read");
            }
            other => panic!("expected Decode, got {other:?}"),
        }
    }

    /// 抽象境界: trait object 経由でのみ decode/probe_pna を呼べる。
    /// 署名は WIC/COM 型を露出しない（std::path とプレーン型のみ）ことをコンパイルで示す。
    #[test]
    fn port_hides_concrete_means() {
        fn takes_decoder(d: &dyn ElementDecoder, p: &Path) -> Result<DecodedImage, DecodeError> {
            let _ = d.probe_pna(p);
            d.decode(p)
        }

        let (w, h, stride, bgra, has_alpha) = small_image(false);
        let mut dec = MemoryDecoder::new();
        dec.insert(known_path(), w, h, stride, bgra, has_alpha);

        let img = takes_decoder(&dec, &known_path()).expect("decodes via dyn port");
        assert_eq!(img.has_alpha, has_alpha);
        assert_eq!(img.bgra.len(), (img.stride * img.height) as usize);
    }

    /// 長さの関数は上限の端で u32 ならあふれる値も usize で正しく出し、越えたら寸法と上限を書いて断る。
    #[test]
    fn pixel_len_at_max_does_not_overflow_and_refuses_over() {
        // 上限の端の長さは 2^30。u32 の掛け算ならあふれる寸法（u32::MAX 四方）は掛ける前に断る。
        assert_eq!(pixel_len(MAX_SIDE, MAX_SIDE), Ok(1usize << 30));
        assert_eq!(pixel_len(0, 0), Ok(0));
        for (w, h) in [
            (MAX_SIDE + 1, 1),
            (1, MAX_SIDE + 1),
            (40_000, 30_000),
            (u32::MAX, u32::MAX),
        ] {
            let err = pixel_len(w, h).expect_err("over max side");
            assert!(
                err.contains(&format!("{w}x{h}")) && err.contains("16384"),
                "{err}"
            );
        }
    }

    /// 約束（一辺・stride・長さ）を破る答えは断り、守る答えは通す。
    #[test]
    fn check_image_rejects_broken_contract() {
        let img = |width, height, stride, len| DecodedImage {
            width,
            height,
            stride,
            bgra: vec![0u8; len],
            has_alpha: true,
        };
        assert_eq!(check_image(&img(2, 2, 8, 16)), Ok(()));
        assert_eq!(check_image(&img(2, 2, 12, 24)), Ok(()));
        assert!(check_image(&img(2, 2, 8, 15)).is_err(), "short buffer");
        assert!(check_image(&img(2, 2, 4, 8)).is_err(), "stride < width * 4");
        assert!(
            check_image(&img(MAX_SIDE + 1, 0, 0, 0)).is_err(),
            "side over max"
        );
        // stride * height が u32 であふれる答えも u64 で比べて断る（panic しない）。
        assert!(check_image(&img(1, MAX_SIDE, u32::MAX, 0)).is_err());
    }
}
