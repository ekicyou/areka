//! `ComposedSurface`: 合成コアの出力契約（premultiplied BGRA・size・stride・`Send` 所有）。
//!
//! 通信機構（channel・async）を介さず値・共有参照として直接返す出力型。surface id→合成結果の
//! キャッシュ・無効化は持たない（それは `emo-present` の責務）。全透明の退化結果も外形どおりの
//! `ComposedSurface` として正常表現する。

/// 合成結果（premultiplied BGRA・`Send` 所有・無変換で WUC upload / AlphaMask 生成可能）。
///
/// 内部フィールドは opaque。不変条件として `stride == width * 4`・
/// `bytes.len() == stride * height`・画素は常に premultiplied BGRA を保つ。
/// [`new`] は全 0（全透明）バッファを確保して構築し、下流 `blit` が転写で埋める。
///
/// [`new`]: ComposedSurface::new
#[derive(Debug, Clone, Default)]
pub struct ComposedSurface {
    width: u32,
    height: u32,
    /// 常に `width * 4`（BGRA・1 画素 4 バイト）。
    stride: u32,
    /// premultiplied BGRA バイト列（`len == stride * height`）。
    bytes: Vec<u8>,
}

impl ComposedSurface {
    /// 指定サイズの全透明（全 0）premultiplied BGRA サーフェスを確保する。
    ///
    /// `stride = width * 4`・`bytes.len() = stride * height` を満たす 0 埋めバッファを割り当てる。
    /// 下流 `blit` 実行器がこのバッファへ画素を転写する。
    pub fn new(width: u32, height: u32) -> Self {
        let stride = width * 4;
        let len = stride as usize * height as usize;
        ComposedSurface {
            width,
            height,
            stride,
            bytes: vec![0u8; len],
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
        self.stride
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
}
