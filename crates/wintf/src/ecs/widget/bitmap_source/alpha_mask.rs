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
#[derive(Debug, Clone)]
pub struct AlphaMask {
    data: Vec<u8>,
    width: u32,
    height: u32,
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
        // 行あたりのバイト数（8ピクセル単位でアラインメント）
        let row_bytes = width.div_ceil(8) as usize;
        let mut data = vec![0u8; row_bytes * height as usize];

        for y in 0..height {
            let row_start = (y * stride) as usize;
            for x in 0..width {
                // PBGRA32: 4バイト/ピクセル、α値は4バイト目（オフセット+3）
                let pixel_offset = row_start + (x as usize * 4);
                let alpha = pixels.get(pixel_offset + 3).copied().unwrap_or(0);

                // 閾値128で2値化
                if alpha >= ALPHA_THRESHOLD {
                    // MSBファースト: ビット7 = 最左ピクセル
                    let byte_index = (y as usize * row_bytes) + (x as usize / 8);
                    let bit_index = 7 - (x % 8);
                    data[byte_index] |= 1 << bit_index;
                }
            }
        }

        Self {
            data,
            width,
            height,
        }
    }

    /// 指定座標がヒット対象かを判定
    ///
    /// # Returns
    /// - `true`: α ≧ 128 のピクセル（ヒット対象）
    /// - `false`: α < 128 または範囲外
    pub fn is_hit(&self, x: u32, y: u32) -> bool {
        // 範囲外チェック
        if x >= self.width || y >= self.height {
            return false;
        }

        let row_bytes = self.width.div_ceil(8) as usize;
        let byte_index = (y as usize * row_bytes) + (x as usize / 8);
        let bit_index = 7 - (x % 8);

        // 不変条件: well-formed な AlphaMask（from_pbgra32 が生成）は
        // `data.len() == row_bytes * height` を満たし、直前の範囲チェック
        // （x < width, y < height）と併せて byte_index は常に data 範囲内
        // （byte_index <= height*row_bytes - 1 < data.len()）。リリースでは
        // この添字は常に安全。debug_assert は data 長と width/height が乖離した
        // 不整合な手構築マスクのみを検出する（リリース挙動は不変）。
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
}

// Send + Sync は自動導出（Vec<u8> と u32 のみ）

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

    /// サイズアクセサのテスト
    #[test]
    fn test_size_accessors() {
        let pixels = vec![0u8; 100 * 50 * 4];
        let mask = AlphaMask::from_pbgra32(&pixels, 100, 50, 400);

        assert_eq!(mask.width(), 100);
        assert_eq!(mask.height(), 50);
    }

    // === W5b-T 追加: from_pbgra32 / is_hit の境界ロジック特性化 ===

    /// width=0 の退化ケース: 空マスクが生成され、いかなる座標もヒットしない
    #[test]
    fn test_zero_width_produces_empty_mask() {
        let pixels: Vec<u8> = Vec::new();
        let mask = AlphaMask::from_pbgra32(&pixels, 0, 0, 0);

        assert_eq!(mask.width(), 0);
        assert_eq!(mask.height(), 0);
        // 範囲外チェック（x >= 0 が常に真）により必ず false
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
    ///
    /// stride を width*4 より大きく取り（行末に未使用バイト）、
    /// pixel_offset = y*stride + x*4 の stride ベース行頭計算を固定する。
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
        assert!(mask.is_hit(1, 1), "(1,1) should hit (padded stride row offset)");
    }

    /// バイト境界跨ぎのビットパック: x=7（1バイト目末尾）と x=8（2バイト目先頭）
    ///
    /// row_bytes = ceil(width/8)。16px 幅で x=7 と x=8 が別バイト・MSBファースト
    /// で正しく配置されることを固定する。
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
        // 直近の範囲外はヒットしない
        assert!(!mask.is_hit(3, 1), "x == width is out of bounds");
        assert!(!mask.is_hit(2, 2), "y == height is out of bounds");
    }

    // === W5b-V 追加: is_hit 添字不変条件の特性化（debug_assert の保護対象固定） ===

    /// `is_hit` が範囲外座標で添字アクセス手前に return すること（panic 経路なし）
    ///
    /// 非バイト境界幅（width=9 → row_bytes=2）で、範囲外座標が `data[byte_index]` に
    /// 到達せず常に false を返すことを固定する。範囲チェック（x>=width||y>=height）が
    /// debug_assert / 添字の手前で全ての越境を吸収する安全鎖を特性化する。
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
    ///
    /// 非バイト境界幅（width=17 → row_bytes=3）の最右下を含む全座標を走査し、
    /// `byte_index < data.len()` が常に成立する（debug ビルドでも panic しない）ことを
    /// 固定する。これは is_hit に追加した debug_assert が well-formed マスクで決して
    /// 発火しないこと（＝リリース挙動不変）の回帰検知器となる。
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

        // 全有効座標で is_hit が panic せず評価できる（debug_assert 非発火）
        let mut hit_count = 0u32;
        for y in 0..h {
            for x in 0..w {
                if mask.is_hit(x, y) {
                    hit_count += 1;
                }
            }
        }
        // 最右下のみヒット（不変条件を満たしつつ最大 byte_index が安全に読まれた証拠）
        assert_eq!(hit_count, 1, "only the bottom-right pixel should hit");
        assert!(mask.is_hit(w - 1, h - 1), "max-index pixel should hit");
    }
}
