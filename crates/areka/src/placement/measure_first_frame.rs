//! 採寸の読み手: 動く絵を 1 枚目だけの静止画として焼く段へ見せる包み
//! （spec: placement-measure-bake-once 要件 1.1・1.4・2.2・3.7、design「FirstFrameDecoder」・D4）。
//!
//! - `probe_animation` は既定（`None`）のまま。焼く段は動く絵の道に入らず、全コマを読まない
//!   （上限の判定と動く絵の 3 つの `warn!` も採寸の道では起きない）
//! - `decode` は資産の組み立ての縮め方の段 1・段 2 と同じ順（1 枚目 → 1 枚読み）なので、
//!   採寸が焼く絵は資産の 0 番のコマと同じ寸法になる
//! - 例外（D4）: 1 枚目が全透明の動く絵は静止画として焼かれ、「全透明」の `warn!` が 1 行増える
//!
//! examples が配置を `#[path]` で取り込むので `crate::` を使わない。
use std::path::Path;

use areka_emo_atlas::{DecodeError, DecodedImage, ElementDecoder};

/// 動く絵を 1 枚目だけで読む読み手（採寸の道だけで使う）。
pub(super) struct FirstFrameDecoder<'a, D: ElementDecoder> {
    inner: &'a D,
}

impl<'a, D: ElementDecoder> FirstFrameDecoder<'a, D> {
    pub(super) fn new(inner: &'a D) -> Self {
        Self { inner }
    }
}

impl<D: ElementDecoder> ElementDecoder for FirstFrameDecoder<'_, D> {
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        match self.inner.probe_animation(path) {
            Some(info) => self
                .inner
                .decode_first_frame(path, info)
                .or_else(|_| self.inner.decode(path)),
            None => self.inner.decode(path),
        }
    }

    fn probe_pna(&self, path: &Path) -> bool {
        self.inner.probe_pna(path)
    }
}
