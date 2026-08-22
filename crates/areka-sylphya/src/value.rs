//! 値・解決結果型（`FlatResolution` / `DottedResolution` / `DegradePolicy` 適用）。
//!
//! 読み口 2 形（[`crate::reader::SylphyaReader`]）が返す解決結果型を定義する。
//!
//! ## 由来非露出（R2.4・型で保証）
//!
//! 解決結果型は値の由来（どの backing 実体層〔[`crate::vocab::BackingLayer`]〕から来たか）を
//! **一切保持しない**。[`FlatResolution::Value`] / [`DottedResolution::Value`] は生の `String` の
//! みを運び、[`FlatResolution::Degraded`] は縮退政策マーカー（[`crate::vocab::DegradePolicy`]）
//! のみを運ぶ。`DegradePolicy` は「不在時に消費側がどう振る舞うか」の政策指定であって、
//! 値がどの層に実在したかという backing 同一性ではない（design §「鏡像＋SylphyaReader」
//! Responsibilities「解決結果に由来（backing/層）を含めない」）。由来はログにのみ記録する。

use crate::vocab::DegradePolicy;

/// フラット窓（`%名前`）の解決結果（R3.1）。
///
/// 由来非露出（R2.4）: 値の backing 層情報を型に持たない。`Value` は生値のみ、
/// `Degraded` は縮退政策マーカーのみを運ぶ。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlatResolution {
    /// 鏡像に値あり（R3.1 の「値源が値を提供」）。
    Value(String),
    /// 不在→台帳の縮退政策を返す（適用は消費側契約に従う。`PassThroughRaw` は `%名前` 素通し、
    /// `ConsumerDefault` は消費側の唯一定義点の既定値、を意味する）。台帳外の名は
    /// `Degraded(PassThroughRaw)`。
    Degraded(DegradePolicy),
}

/// 点付き窓（`system.*` 等）の解決結果（R3.2 / R5.2）。
///
/// 由来非露出（R2.4）: `Value` は生値のみを運び、backing 層情報を型に持たない。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DottedResolution {
    /// 鏡像に値あり。
    Value(String),
    /// 不在（NOT_FOUND・決定論的）。
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 由来非露出（R2.4・構造保証）: 解決結果型に backing 由来フィールドが無いことを構築で示す。
    /// `Value` は `String` 1 個、`Degraded` は `DegradePolicy` 1 個のみで完全に構築できる
    /// ——backing 層を要求するフィールドは存在しない（存在すればこの構築がコンパイルしない）。
    #[test]
    fn resolution_types_carry_no_backing_source() {
        let _v: FlatResolution = FlatResolution::Value("x".into());
        let _d: FlatResolution = FlatResolution::Degraded(DegradePolicy::PassThroughRaw);
        let _dv: DottedResolution = DottedResolution::Value("y".into());
        let _dn: DottedResolution = DottedResolution::NotFound;
    }

    #[test]
    fn flat_resolution_eq() {
        assert_eq!(
            FlatResolution::Value("a".into()),
            FlatResolution::Value("a".into())
        );
        assert_ne!(
            FlatResolution::Value("a".into()),
            FlatResolution::Value("b".into())
        );
        assert_ne!(
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw),
            FlatResolution::Degraded(DegradePolicy::ConsumerDefault)
        );
    }

    #[test]
    fn dotted_resolution_eq() {
        assert_eq!(
            DottedResolution::Value("a".into()),
            DottedResolution::Value("a".into())
        );
        assert_ne!(
            DottedResolution::Value("a".into()),
            DottedResolution::NotFound
        );
    }
}
