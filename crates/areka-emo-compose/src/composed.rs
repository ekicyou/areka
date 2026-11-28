//! `ComposedSurface`: 描いた絵の受け渡しの型（premultiplied BGRA・size・stride・`Send` 所有）。
//!
//! 通信機構（channel・async）を介さず値・共有参照として直接返す出力型。surface id→合成結果の
//! キャッシュ・無効化は持たない（それは `emo-present` の責務）。全透明の退化結果も外形どおりの
//! `ComposedSurface` として正常表現する。

/// 描いた絵（premultiplied BGRA・`Send` 所有）。
///
/// 内部フィールドは opaque。不変条件として `bytes.len() == width * 4 * height`・画素は常に premultiplied BGRA を保つ。
/// 描いた絵は下流（画面の外の描き手）が [`from_bytes`] で包む。
///
/// [`from_bytes`]: ComposedSurface::from_bytes
#[derive(Debug, Clone, Default)]
pub struct ComposedSurface {
    width: u32,
    height: u32,
    /// premultiplied BGRA バイト列（`len == width * 4 * height`）。
    bytes: Vec<u8>,
}

impl ComposedSurface {
    /// 指定サイズの全透明（全 0）premultiplied BGRA サーフェスを確保する。
    ///
    /// `bytes.len() = width * 4 * height` を満たす 0 埋めバッファを割り当てる。
    /// 幅と高さは外形の一辺の上限（`plan::MAX_EXTENT`）以下で
    /// あること（`stride()` を u32 であふれさせない・debug で確かめる）。
    pub fn new(width: u32, height: u32) -> Self {
        debug_assert_within_extent(width, height);
        ComposedSurface {
            width,
            height,
            bytes: vec![0u8; byte_len(width, height)],
        }
    }

    /// 描いた絵の乗算済み BGRA（`len == width * 4 * height`）をそのまま包む。
    pub fn from_bytes(width: u32, height: u32, bytes: Vec<u8>) -> Self {
        debug_assert_within_extent(width, height);
        debug_assert_eq!(bytes.len(), byte_len(width, height));
        ComposedSurface {
            width,
            height,
            bytes,
        }
    }

    /// 幅（画素）。
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 高さ（画素）。
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 行ストライド（バイト）。常に `width * 4`。
    pub fn stride(&self) -> u32 {
        // 幅は合成で上限（一辺 `plan::MAX_EXTENT`）以下に抑えるので u32 で足りる。
        self.width * 4
    }

    /// premultiplied BGRA バイト列の参照（`len == stride * height`）。
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// バイト列を所有ごと取り出す（複製なしの受け渡し）。
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// 幅と高さが外形の上限（[`crate::plan::MAX_EXTENT`]）以下か。合成は `build_plan` で越える外形を
/// `ExtentTooLarge` として断るので、越えるのは呼び手の誤り（`stride()` を u32 であふれさせない）。
fn debug_assert_within_extent(width: u32, height: u32) {
    debug_assert!(
        width <= crate::plan::MAX_EXTENT && height <= crate::plan::MAX_EXTENT,
        "外形の上限を越える大きさ: {width}x{height}"
    );
}

/// `width×height` の BGRA の長さ（バイト）。掛け算は `usize` で行い、`u32` であふれさせない。
fn byte_len(width: u32, height: u32) -> usize {
    width as usize * 4 * height as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new` は size/stride 契約を満たす 0 埋めバッファを確保する。
    #[test]
    fn new_allocates_zeroed_premultiplied_buffer() {
        let s = ComposedSurface::new(5, 3);
        assert_eq!(s.width(), 5);
        assert_eq!(s.height(), 3);
        assert_eq!(s.stride(), 5 * 4);
        assert_eq!(s.bytes().len(), 5 * 4 * 3);
        assert!(s.bytes().iter().all(|&b| b == 0));
    }

    /// 0 サイズも外形どおり正常表現する（退化結果）。
    #[test]
    fn new_zero_size_is_empty() {
        let s = ComposedSurface::new(0, 0);
        assert_eq!(s.stride(), 0);
        assert!(s.bytes().is_empty());
    }

    /// `into_bytes` は複製なしにバイト列を取り出す。
    #[test]
    fn into_bytes_yields_buffer() {
        let s = ComposedSurface::new(2, 2);
        let bytes = s.into_bytes();
        assert_eq!(bytes.len(), 2 * 4 * 2);
    }

    /// 外形の上限を越える幅は入口で止める（`stride()` を u32 であふれさせない）。
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "外形の上限")]
    fn new_beyond_max_extent_is_rejected() {
        ComposedSurface::new(crate::plan::MAX_EXTENT + 1, 0);
    }
}
