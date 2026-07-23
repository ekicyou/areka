//! 語彙台帳: 台帳型（`VocabEntry` / `BackingLayer` / `M1Status` / `DegradePolicy` / `SetSemantics`）。

pub mod dotted;
pub mod flat;
pub mod shiori_resource;

/// backing 実体層（研究 §9 の 5 層タクソノミー・R3.6）。
///
/// M1 実導出は `StaticConfig` の一部・`ShioriQuery`(username)・`Persistent`。
/// `RuntimeState` / `SystemEnv` は縮退のまま層の存在を型で表現する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackingLayer {
    /// 静的構成（load-time 由来: selfname/keroname/baseware 系）。
    StaticConfig,
    /// リアルタイム運行状態（他エンジンが所有する状態: surface/scope 系）。
    RuntimeState,
    /// システム環境（OS 由来: 時刻・画面系・注入シーム必須）。
    SystemEnv,
    /// SHIORI 照会（username 等）。
    ShioriQuery,
    /// 層別 TOML 永続（窓位置・オフセット・起動記録・vanish 回数）。
    Persistent,
}

/// 縮退政策（R3.1）。
///
/// `ConsumerDefault` は「既定値は消費側の唯一定義点が持つ」ことのマーカーで、sylphya は値を持たない。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DegradePolicy {
    /// `%名前` の原文をそのまま返す素通し。
    PassThroughRaw,
    /// 消費側の唯一定義点の既定値（sylphya は値を持たない）。
    ConsumerDefault,
    /// 不在（NOT_FOUND）。
    NotFound,
}

/// M1 実導出状態（実導出済みか、縮退中か）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum M1Status {
    /// M1 で実値へ導出する。
    Derived,
    /// M1 では決定論的に縮退する（差替シームのみ）。
    Degraded,
}

/// SET の 2 意味論（R3.4）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetSemantics {
    /// 運行コマンド書込（`surface.num` SET＝`\s[]` 等価などランタイムへの命令）。
    RuntimeCommand,
    /// ストア書込（永続値の更新）。
    StoreWrite,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backing_layer_has_five_values() {
        let all = [
            BackingLayer::StaticConfig,
            BackingLayer::RuntimeState,
            BackingLayer::SystemEnv,
            BackingLayer::ShioriQuery,
            BackingLayer::Persistent,
        ];
        assert_eq!(all.len(), 5);
        // Copy + PartialEq
        let x = all[0];
        assert_eq!(x, BackingLayer::StaticConfig);
        assert_ne!(BackingLayer::ShioriQuery, BackingLayer::Persistent);
    }

    #[test]
    fn degrade_policy_variants() {
        assert_ne!(DegradePolicy::PassThroughRaw, DegradePolicy::ConsumerDefault);
        assert_ne!(DegradePolicy::ConsumerDefault, DegradePolicy::NotFound);
    }

    #[test]
    fn m1_status_variants() {
        assert_ne!(M1Status::Derived, M1Status::Degraded);
        let c = M1Status::Derived;
        assert_eq!(c, M1Status::Derived);
    }

    #[test]
    fn set_semantics_variants() {
        assert_ne!(SetSemantics::RuntimeCommand, SetSemantics::StoreWrite);
    }
}
