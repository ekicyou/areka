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

    /// バイト列**そのものの確保量**（`Vec::capacity`・バイト）。
    ///
    /// 消費側（`areka-emo-present` の `FrameBudget`）が「この呼び出しで確保が起きたか」を
    /// **厳密に**観測するための口である。`Vec` の容量は再確保でしか増えず縮みもしないため、
    /// 呼び出しの前後で読み比べれば「増えた＝確保した／変わらない＝確保していない」が
    /// 過大にも過小にもならずに決まる（[`ResampleScratch::capacity`] と同じ形）。
    ///
    /// [`bytes`] の長さ（`stride * height`）とは別物である——長さは外形が決めるが、容量は
    /// 「これまでに到達した最大」を保つ。**役割ではなく実体に紐づく**値ゆえ、バッファが
    /// 複数本で入れ替わる経路（キャッシュの追い出しバッファを回して使う形）でも、器側で
    /// 到達済み寸法を覚える必要が無くなる。
    ///
    /// [`bytes`]: ComposedSurface::bytes
    /// [`ResampleScratch::capacity`]: crate::scale::ResampleScratch::capacity
    pub fn bytes_capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// バッファを `w×h`（`stride = w*4`）へ再確保し、全画素を 0（全透明）へクリアする。
    ///
    /// 既存の `Vec<u8>` 容量を**再利用**する（要件 10.3・毎フレーム経路のゼロアロケーション）:
    /// 必要バイト数が現容量以下なら再割り当てせず `truncate`＋`resize`／不足時のみ伸長する。
    /// クリアは全 0 上書き（前フレームの残像が混ざらない）。blit 実行器（[`crate::blit`]）が
    /// 転写前に呼び、以後 [`bytes_mut`] へ premultiplied BGRA を書き込む。
    ///
    /// [`bytes_mut`]: ComposedSurface::bytes_mut
    pub(crate) fn resize_and_clear(&mut self, width: u32, height: u32) {
        let stride = width * 4;
        let len = stride as usize * height as usize;
        self.width = width;
        self.height = height;
        self.stride = stride;
        // 容量再利用: 既存要素を 0 で埋め直し、長さを len へ合わせる（>len は捨て・<len は 0 追加）。
        // Vec::resize は容量が足りれば再割り当てしない（10.3）。
        self.bytes.clear();
        self.bytes.resize(len, 0u8);
    }

    /// **直後に全バイトが上書きされる**用途向けの寸法合わせ（追補 §A2・要件 3.1）。
    ///
    /// 外形フィールド（`width`／`height`／`stride`）は**常に**更新する。バイト列は必要長が
    /// 現在の長さと一致していれば**一切触らない**——ゼロ埋めも再割り当ても行わない。
    /// 長さが違うときだけ従来どおり `clear`＋`resize` で 0 初期化する（容量は再利用され、
    /// 伸長した領域は 0 で埋まるので未初期化メモリは決して露出しない・`unsafe` 不使用）。
    ///
    /// # 呼んでよい条件（呼び手の責務）
    ///
    /// 呼び出し直後に `0..stride*height` の**全バイト**が書かれること。これを満たさない
    /// 呼び手が使うと、使い回したバッファの前回の内容がそのまま出力へ残る。現在の唯一の
    /// 呼び手は [`crate::scale::resample_with`] で、恒等 k は `copy_from_slice`・非恒等 k は
    /// 内側 2 重ループが `out_h × out_w × 4` バイトを全て代入する（`stride == width*4` ゆえ
    /// 行内の余白は存在しない）。**画布へ重ねる**意味論の [`resize_and_clear`]（`blit` 用）とは
    /// 用途が違い、そちらは 0 初期化が必須なので置き換えてはならない。
    ///
    /// [`resize_and_clear`]: ComposedSurface::resize_and_clear
    pub(crate) fn resize_for_full_overwrite(&mut self, width: u32, height: u32) {
        let stride = width * 4;
        let len = stride as usize * height as usize;
        // 外形は長さが一致していても必ず更新する（8×6 と 6×8 は同じ 192 バイト）。
        self.width = width;
        self.height = height;
        self.stride = stride;
        if self.bytes.len() != len {
            self.bytes.clear();
            self.bytes.resize(len, 0u8);
        }
    }

    /// premultiplied BGRA バイト列の可変参照（`len == stride * height`）。
    ///
    /// blit 実行器が転写先として書き込む（本クレート内専用）。
    pub(crate) fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
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

    /// 追補 §A2 の**本題**: 長さが一致する呼び出しではバイト列に一切触れない。
    ///
    /// この 1 本が、除去したゼロ埋めが本当に消えていることの構造的な証拠である
    /// （`resize_and_clear` へ戻すと事前に塗った値が 0 になり赤になる）。所要の計測は
    /// 実時間ゆえ恒久テストの合否条件にできない（要件 6.2）ので、代わりに
    /// 「書き込みが起きていない」ことを内容の不動で固定する。
    #[test]
    fn resize_for_full_overwrite_leaves_bytes_untouched_when_the_length_matches() {
        let mut s = ComposedSurface::new(4, 3);
        s.bytes_mut().fill(0xAA);
        let before = s.bytes().to_vec();

        s.resize_for_full_overwrite(4, 3);
        assert_eq!(
            s.bytes(),
            before.as_slice(),
            "同じ外形の呼び出しでバイトが書き換わった（ゼロ埋めが残っている）"
        );

        // バイト長が同じで外形だけ違う場合も、初期化は走らない（触るのは外形フィールドだけ）。
        s.resize_for_full_overwrite(3, 4);
        assert_eq!(
            s.bytes(),
            before.as_slice(),
            "長さが同じ別外形の呼び出しでバイトが書き換わった"
        );
    }

    /// 外形フィールドは**長さが一致していても**必ず更新する。
    ///
    /// `4×3` と `3×4` はどちらも 48 バイトである。「長さが同じなら何もしない」を
    /// 外形の更新まで飛ばして読むと、まっさらな器では長さが違うため必ず作り直されて
    /// **正しく動き**、使い回しの側でしか壊れない。
    #[test]
    fn resize_for_full_overwrite_always_updates_the_extent_fields() {
        let mut s = ComposedSurface::new(4, 3);
        s.resize_for_full_overwrite(3, 4);
        assert_eq!((s.width(), s.height()), (3, 4), "外形が更新されていない");
        assert_eq!(s.stride(), 3 * 4, "stride が新しい幅に追随していない");
        assert_eq!(s.bytes().len(), 3 * 4 * 4, "不変条件 len == stride*height");
    }

    /// 伸長・縮小のどちらでも、事後に長さ・外形が厳密で、**新しい領域は 0 で初期化される**
    /// （安全な Rust ゆえ未初期化メモリは露出しないが、前の内容が残らないことを固定する）。
    /// 縮小方向では容量が再利用され、再確保が起きない（要件 3.1・5.1 の容量引き継ぎの前提）。
    #[test]
    fn resize_for_full_overwrite_reinitializes_on_growth_and_drops_the_tail_on_shrink() {
        // 伸長: 前の内容は 1 バイトも残らない。
        let mut s = ComposedSurface::new(2, 2);
        s.bytes_mut().fill(0xAA);
        s.resize_for_full_overwrite(4, 4);
        assert_eq!((s.width(), s.height(), s.stride()), (4, 4, 16));
        assert_eq!(s.bytes().len(), 4 * 4 * 4);
        assert!(
            s.bytes().iter().all(|&b| b == 0),
            "伸長した器に前の内容が残っている（未初期化・残像の混入経路）"
        );

        // 縮小: 末尾が捨てられ、容量は据え置かれる（同じ確保が回る）。
        s.bytes_mut().fill(0xBB);
        let ptr = s.bytes().as_ptr();
        s.resize_for_full_overwrite(2, 2);
        assert_eq!((s.width(), s.height(), s.stride()), (2, 2, 8));
        assert_eq!(s.bytes().len(), 2 * 4 * 2);
        assert!(
            s.bytes().iter().all(|&b| b == 0),
            "縮小した器に前の内容が残っている"
        );
        assert_eq!(
            s.bytes().as_ptr(),
            ptr,
            "縮小で再確保が起きた（回収した容量が引き継がれない）"
        );

        // 外形ゼロへ縮めても不変条件が保たれ、前の内容は残らない。
        s.resize_for_full_overwrite(0, 0);
        assert_eq!((s.width(), s.height(), s.stride()), (0, 0, 0));
        assert!(s.bytes().is_empty(), "外形ゼロで空にならない");
    }
}
