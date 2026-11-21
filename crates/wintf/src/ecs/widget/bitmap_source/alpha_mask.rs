//! αマスクデータ構造
//!
//! ビットパック形式（1ビット/ピクセル）でαマスクを保持し、
//! 高速なヒット判定を提供する。

/// 固定閾値（α ≧ 128 でヒット対象）
const ALPHA_THRESHOLD: u8 = 128;

/// αマスクデータ構造
///
/// ビットパック形式でマスクデータを保持:
/// - 1ビット/ピクセル（8ピクセル/バイト）
/// - MSBファースト（ビット7 = 最左ピクセル）
/// - 行ごとに8ピクセル単位でアラインメント
// `PartialEq` は再生成と新規生成の等価比較（data/width/height の全一致）のために derive する。
#[derive(Debug, Clone, PartialEq)]
pub struct AlphaMask {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

/// PBGRA32 の α をビットパック済みバッファへ書き込む（`from_pbgra32` と `regenerate_from_pbgra32` の共通本体）
///
/// # 前提
/// `data` は `row_bytes * height` の長さで、**全バイトが 0 に初期化済み**であること
/// （本関数はセットするビットしか書かないため、残留ビットは消さない）。
fn pack_pbgra32_alpha(
    data: &mut [u8],
    pixels: &[u8],
    width: u32,
    height: u32,
    stride: u32,
    row_bytes: usize,
) {
    for y in 0..height {
        let row_start = (y * stride) as usize;
        for x in 0..width {
            // PBGRA32: 4バイト/ピクセル、α値は4バイト目（オフセット+3）
            let pixel_offset = row_start + (x as usize * 4);
            let alpha = pixels.get(pixel_offset + 3).copied().unwrap_or(0);

            if alpha >= ALPHA_THRESHOLD {
                let byte_index = (y as usize * row_bytes) + (x as usize / 8);
                let bit_index = 7 - (x % 8);
                data[byte_index] |= 1 << bit_index;
            }
        }
    }
}

impl AlphaMask {
    /// PBGRA32ピクセルデータからαマスクを生成
    ///
    /// # Arguments
    /// - `pixels`: PBGRA32形式のピクセルデータ（B, G, R, A の順）
    /// - `width`: 画像幅（ピクセル）
    /// - `height`: 画像高さ（ピクセル）
    /// - `stride`: 行あたりのバイト数
    ///
    /// # Returns
    /// 生成されたαマスク
    pub fn from_pbgra32(pixels: &[u8], width: u32, height: u32, stride: u32) -> Self {
        let row_bytes = width.div_ceil(8) as usize;
        let mut data = vec![0u8; row_bytes * height as usize];

        pack_pbgra32_alpha(&mut data, pixels, width, height, stride, row_bytes);

        Self {
            data,
            width,
            height,
        }
    }

    /// 既存バッファを再利用してαマスクの内容を再生成する
    ///
    /// [`from_pbgra32`](Self::from_pbgra32) と**同一入力に対して同一結果**を作るが、
    /// 新しいバッファを確保せず `self.data` を `clear` + `resize` で作り直す
    /// （容量は保持されるため、同寸または縮小方向の再生成では確保が発生しない）。
    /// 毎コマの適用で当たり判定マスクの新規確保を避けるための口。
    ///
    /// `clear` してから 0 で `resize` するため、**寸法が縮む方向でも前の内容は 1 ビットも
    /// 残らない**（`resize` 単独だと保持された前半バイトのビットが生き残る）。
    ///
    /// # Arguments
    /// - `pixels`: PBGRA32形式のピクセルデータ（B, G, R, A の順）
    /// - `width`: 画像幅（ピクセル）
    /// - `height`: 画像高さ（ピクセル）
    /// - `stride`: 行あたりのバイト数
    pub fn regenerate_from_pbgra32(&mut self, pixels: &[u8], width: u32, height: u32, stride: u32) {
        let row_bytes = width.div_ceil(8) as usize;
        let len = row_bytes * height as usize;

        // clear してから resize: 縮小方向でも前の内容が残らず、len <= capacity の間は確保も起きない。
        self.data.clear();
        self.data.resize(len, 0);

        pack_pbgra32_alpha(&mut self.data, pixels, width, height, stride, row_bytes);

        self.width = width;
        self.height = height;
    }

    /// 指定座標がヒット対象かを判定
    ///
    /// # Returns
    /// - `true`: α ≧ 128 のピクセル（ヒット対象）
    /// - `false`: α < 128 または範囲外
    pub fn is_hit(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }

        let row_bytes = self.width.div_ceil(8) as usize;
        let byte_index = (y as usize * row_bytes) + (x as usize / 8);
        let bit_index = 7 - (x % 8);

        // well-formed なマスクは `data.len() == row_bytes * height` で、上の範囲チェックと併せて添字は常に範囲内。
        // debug_assert は data 長と width/height が乖離した手構築マスクだけを検出する。
        debug_assert!(
            byte_index < self.data.len(),
            "AlphaMask invariant violated: byte_index {} out of bounds (data.len()={}, width={}, height={})",
            byte_index,
            self.data.len(),
            self.width,
            self.height
        );

        (self.data[byte_index] >> bit_index) & 1 == 1
    }

    /// マスク幅を取得
    pub fn width(&self) -> u32 {
        self.width
    }

    /// マスク高さを取得
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 詰めビット列**そのものの確保量**（`Vec::capacity`・バイト）を取得
    /// [`regenerate_from_pbgra32`] の前後で読み比べ、再生成で確保が起きたかを観測するための口。
    /// 値はマスクの実体に紐づくため、複数のマスクを入れ替えて使う呼び手も到達済み寸法を覚えなくてよい。
    ///
    /// [`regenerate_from_pbgra32`]: AlphaMask::regenerate_from_pbgra32
    pub fn packed_capacity(&self) -> usize {
        self.data.capacity()
    }
}

/// `regenerate_from_pbgra32` の等価檻とバッファ再利用檻
#[cfg(test)]
#[path = "alpha_mask_regenerate_tests.rs"]
mod regenerate_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// 透明部分（α < 128）でヒットしないことを確認
    #[test]
    fn test_transparent_pixel_not_hit() {
        // 2x2 PBGRA32: 全て透明（α = 0）
        let pixels = vec![
            0, 0, 0, 0, // (0,0) 透明
            0, 0, 0, 0, // (1,0) 透明
            0, 0, 0, 0, // (0,1) 透明
            0, 0, 0, 0, // (1,1) 透明
        ];
        let mask = AlphaMask::from_pbgra32(&pixels, 2, 2, 8);

        assert!(!mask.is_hit(0, 0));
        assert!(!mask.is_hit(1, 0));
        assert!(!mask.is_hit(0, 1));
        assert!(!mask.is_hit(1, 1));
    }

    /// 不透明部分（α ≧ 128）でヒットすることを確認
    #[test]
    fn test_opaque_pixel_hit() {
        // 2x2 PBGRA32: 全て不透明（α = 255）
        let pixels = vec![
            0, 0, 0, 255, // (0,0) 不透明
            0, 0, 0, 255, // (1,0) 不透明
            0, 0, 0, 255, // (0,1) 不透明
            0, 0, 0, 255, // (1,1) 不透明
        ];
        let mask = AlphaMask::from_pbgra32(&pixels, 2, 2, 8);

        assert!(mask.is_hit(0, 0));
        assert!(mask.is_hit(1, 0));
        assert!(mask.is_hit(0, 1));
        assert!(mask.is_hit(1, 1));
    }

    /// 境界値テスト: α = 127 → ヒットしない
    #[test]
    fn test_threshold_boundary_127() {
        let pixels = vec![0, 0, 0, 127]; // α = 127
        let mask = AlphaMask::from_pbgra32(&pixels, 1, 1, 4);

        assert!(!mask.is_hit(0, 0));
    }

    /// 境界値テスト: α = 128 → ヒットする
    #[test]
    fn test_threshold_boundary_128() {
        let pixels = vec![0, 0, 0, 128]; // α = 128
        let mask = AlphaMask::from_pbgra32(&pixels, 1, 1, 4);

        assert!(mask.is_hit(0, 0));
    }

    /// 範囲外座標でfalseを返すことを確認
    #[test]
    fn test_out_of_bounds() {
        let pixels = vec![0, 0, 0, 255];
        let mask = AlphaMask::from_pbgra32(&pixels, 1, 1, 4);

        assert!(!mask.is_hit(1, 0)); // x = width
        assert!(!mask.is_hit(0, 1)); // y = height
        assert!(!mask.is_hit(100, 100));
    }

    /// 混在パターン: 透明/不透明が混在
    #[test]
    fn test_mixed_pattern() {
        // 2x2: チェッカーパターン
        let pixels = vec![
            0, 0, 0, 255, // (0,0) 不透明
            0, 0, 0, 0, // (1,0) 透明
            0, 0, 0, 0, // (0,1) 透明
            0, 0, 0, 255, // (1,1) 不透明
        ];
        let mask = AlphaMask::from_pbgra32(&pixels, 2, 2, 8);

        assert!(mask.is_hit(0, 0));
        assert!(!mask.is_hit(1, 0));
        assert!(!mask.is_hit(0, 1));
        assert!(mask.is_hit(1, 1));
    }

    /// 8ピクセル以上の幅でビットパックが正しく動作することを確認
    #[test]
    fn test_wide_image_bit_packing() {
        // 10x1: 最初と最後のピクセルが不透明
        let mut pixels = vec![0u8; 10 * 4];
        pixels[3] = 255; // (0,0) 不透明
        pixels[9 * 4 + 3] = 255; // (9,0) 不透明

        let mask = AlphaMask::from_pbgra32(&pixels, 10, 1, 40);

        assert!(mask.is_hit(0, 0));
        for x in 1..9 {
            assert!(!mask.is_hit(x, 0), "x={} should not hit", x);
        }
        assert!(mask.is_hit(9, 0));
    }

    // === from_pbgra32 / is_hit の境界ロジック ===

    /// width=0 の退化ケース: 空マスクが生成され、いかなる座標もヒットしない
    #[test]
    fn test_zero_width_produces_empty_mask() {
        let pixels: Vec<u8> = Vec::new();
        let mask = AlphaMask::from_pbgra32(&pixels, 0, 0, 0);

        assert_eq!(mask.width(), 0);
        assert_eq!(mask.height(), 0);
        assert!(!mask.is_hit(0, 0));
        assert!(!mask.is_hit(5, 5));
    }

    /// height=0（幅あり）の退化ケース: 行が存在せずヒットなし
    #[test]
    fn test_zero_height_produces_empty_mask() {
        let pixels: Vec<u8> = Vec::new();
        let mask = AlphaMask::from_pbgra32(&pixels, 4, 0, 16);

        assert_eq!(mask.width(), 4);
        assert_eq!(mask.height(), 0);
        assert!(!mask.is_hit(0, 0));
        assert!(!mask.is_hit(3, 0));
    }

    /// パディング付き stride: 行末パディングを跨いで各行のαが正しく読まれること
    #[test]
    fn test_padded_stride_reads_correct_rows() {
        // 2x2 画像、stride=12（= 行に必要な 8 バイト + 4 バイトのパディング）
        let stride = 12u32;
        let mut pixels = vec![0u8; (stride * 2) as usize];
        // (0,0) 不透明
        pixels[3] = 255;
        // (1,1) 不透明: 2行目先頭 = y*stride = 12、x=1 → +4、α は +3
        pixels[(1 * stride as usize) + (1 * 4) + 3] = 255;

        let mask = AlphaMask::from_pbgra32(&pixels, 2, 2, stride);

        assert!(mask.is_hit(0, 0), "(0,0) should hit");
        assert!(!mask.is_hit(1, 0), "(1,0) should not hit");
        assert!(!mask.is_hit(0, 1), "(0,1) should not hit");
        assert!(
            mask.is_hit(1, 1),
            "(1,1) should hit (padded stride row offset)"
        );
    }

    /// バイト境界跨ぎのビットパック: x=7（1バイト目末尾）と x=8（2バイト目先頭）
    #[test]
    fn test_bit_packing_across_byte_boundary() {
        // 16x1: x=7 と x=8 のみ不透明
        let mut pixels = vec![0u8; 16 * 4];
        pixels[7 * 4 + 3] = 255; // x=7（バイト0の bit0）
        pixels[8 * 4 + 3] = 255; // x=8（バイト1の bit7）

        let mask = AlphaMask::from_pbgra32(&pixels, 16, 1, 64);

        for x in 0..16 {
            let expected = x == 7 || x == 8;
            assert_eq!(
                mask.is_hit(x, 0),
                expected,
                "x={x} hit mismatch (boundary packing)"
            );
        }
    }

    /// is_hit が最終有効ピクセル（width-1, height-1）で正しく判定すること
    #[test]
    fn test_is_hit_at_last_valid_pixel() {
        // 3x2: 右下 (2,1) のみ不透明
        let w = 3u32;
        let h = 2u32;
        let stride = w * 4;
        let mut pixels = vec![0u8; (stride * h) as usize];
        pixels[((h - 1) * stride + (w - 1) * 4 + 3) as usize] = 255;

        let mask = AlphaMask::from_pbgra32(&pixels, w, h, stride);

        assert!(mask.is_hit(2, 1), "last valid pixel should hit");
        assert!(!mask.is_hit(3, 1), "x == width is out of bounds");
        assert!(!mask.is_hit(2, 2), "y == height is out of bounds");
    }

    // === is_hit の添字不変条件 ===

    /// `is_hit` が範囲外座標で添字アクセス手前に return すること（panic 経路なし）
    #[test]
    fn test_is_hit_out_of_bounds_returns_before_indexing() {
        // 9x3: row_bytes = ceil(9/8) = 2、data.len() = 6
        let w = 9u32;
        let h = 3u32;
        let stride = w * 4;
        let pixels = vec![0u8; (stride * h) as usize];
        let mask = AlphaMask::from_pbgra32(&pixels, w, h, stride);

        // x == width / y == height / 両方超過 / 極大値: いずれも false（panic なし）
        assert!(!mask.is_hit(w, 0));
        assert!(!mask.is_hit(0, h));
        assert!(!mask.is_hit(w, h));
        assert!(!mask.is_hit(u32::MAX, u32::MAX));
        assert!(!mask.is_hit(u32::MAX, 0));
        assert!(!mask.is_hit(0, u32::MAX));
    }

    /// well-formed マスクの全有効座標で `is_hit` が添字不変条件を満たす（debug_assert 非発火）
    #[test]
    fn test_is_hit_all_valid_coords_satisfy_index_invariant() {
        // 17x5: row_bytes = ceil(17/8) = 3、data.len() = 15。最右下 (16,4) を含む全走査
        let w = 17u32;
        let h = 5u32;
        let stride = w * 4;
        let mut pixels = vec![0u8; (stride * h) as usize];
        // 最右下ピクセルのみ不透明にして最大 byte_index 到達を確実にする
        pixels[((h - 1) * stride + (w - 1) * 4 + 3) as usize] = 255;
        let mask = AlphaMask::from_pbgra32(&pixels, w, h, stride);

        let mut hit_count = 0u32;
        for y in 0..h {
            for x in 0..w {
                if mask.is_hit(x, y) {
                    hit_count += 1;
                }
            }
        }
        assert_eq!(hit_count, 1, "only the bottom-right pixel should hit");
        assert!(mask.is_hit(w - 1, h - 1), "max-index pixel should hit");
    }
}
