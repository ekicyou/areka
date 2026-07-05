//! `BlitExecutor`: 命令列を CPU 整数演算で転写する execute 段。
//!
//! premultiplied SourceOver（`dst_c' = src_c + div255(dst_c × (255 − src_a))`・
//! `div255(v) = (v + 127) / 255`）を u8/u32 整数で実装し、浮動小数を経路に持ち込まない。
//! 転写先座標は「配置座標＋`trim_offset`」で算出しトリムが見た目を変えないことを保証する。
//! `placement` が None（全透明）のエントリは転写をスキップする。合成先バッファを再利用し
//! アトラス転写を O(elements) で行い、途中アロケーションを発生させない。
