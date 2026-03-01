//! 演出コマンド・ドメイン型定義。
//!
//! dola cue モジュールの演出コマンド体系と演出パイプラインのドメイン概念を
//! ECS 非依存な型で提供する。

use serde::{Deserialize, Serialize};

use crate::value::DynamicValue;

// ============================================================================
// ドメイン型
// ============================================================================

/// 演者識別子。NewType パターンにより型安全性を確保。
///
/// さくらスクリプトの `\0` (さくら) / `\1` (うにゅう) に相当するが、
/// 文字列ベースで任意の名前を許容する。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorKey(String);

impl ActorKey {
    /// 新しい ActorKey を生成する。
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// 文字列スライスとして取得
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ActorKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ActorKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for ActorKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// CueCommand の配送先スロット。
/// 1 ActorKey に対して複数の CueTarget スロットが存在する。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CueTarget {
    /// シェル（キャラクター描画）— Emote, EntityRef を主に消費
    Shell,
    /// バルーン（テキスト表示）— Text, Clear, Choice, WaitForChoice を主に消費
    Balloon,
}

/// CueCommand の配送先スロット内でのキー識別子。
/// EntityRegistry の名前空間を型で分離する。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKey {
    /// アクターの特定スロット
    Actor(ActorKey, CueTarget),
    /// 物理スポットエンティティ (P1 拡張)
    Spot(String),
    /// 物理バルーンエンティティ (P1 拡張)
    Balloon(String),
}

// ============================================================================
// バリア種別
// ============================================================================

/// バリア種別（3 種）。
///
/// `TimedSchedule<T>` の `Entry::Barrier` に格納される。
/// `CueCommand` enum とは独立し、スケジューリング層が直接消費する。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BarrierKind {
    /// クリック/キー入力待ち（旧 WaitForClick を統合）
    WaitForInput { timeout: Option<f64> },
    /// 選択肢待ち
    WaitForChoice { timeout: Option<f64> },
    /// 指定時間経過待ち（新規）
    Timeout { duration: f64 },
}

// ============================================================================
// ルーティングコマンド
// ============================================================================

/// ルーティングコマンド（3 バリアント）。
///
/// `TimedSchedule<T>` の `Entry::Routing` に格納される。
/// CueQueue 層が `next_routing()` で消費し、`ready()` 利用側には届かない。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RoutingCommand {
    /// スロット追加（既存ルーティング維持で追加先登録）
    RouteAdd { target: CueTarget, to: EntityKey },
    /// スロット切替（既存ルーティング上書き）
    RouteSwitch { target: CueTarget, to: EntityKey },
    /// スロット除去
    RouteRemove { target: CueTarget },
}

// ============================================================================
// 演出コマンド
// ============================================================================

/// 演出コマンド（6 バリアント、データ系のみ）。
///
/// バリアは `BarrierKind` として、ルーティングは `RoutingCommand` として、
/// それぞれ `Entry` レベルで分離済み。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CueCommand {
    /// テキスト表示。意味解釈（縦書き、装飾等）は消費者の責務。
    Text(String),
    /// コンテンツクリア
    Clear,
    /// 演技発現。key の意味解釈は消費者が担う。
    Emote { key: String },
    /// 選択肢データ。WaitForChoice の前に連続投入する先積みプロトコル。
    Choice { id: String, text: String },
    /// ECS エンティティ参照渡し（u64 = Entity::to_bits() 変換済み）
    EntityRef(u64),
    /// 消費者固有コマンド。DynamicValue は JSON 互換辞書型。
    Custom {
        command: String,
        params: DynamicValue,
    },
}

// ============================================================================
// 統合ペイロード型
// ============================================================================

/// CueSheet 記述時の統合型（3 種）。
///
/// コマンド・バリア・ルーティングを同一インターフェースで記述可能にする。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CuePayload {
    /// データコマンド
    Command(CueCommand),
    /// バリア（進行停止点）
    Barrier(BarrierKind),
    /// ルーティング（配送制御）
    Routing(RoutingCommand),
}

impl From<CueCommand> for CuePayload {
    fn from(cmd: CueCommand) -> Self {
        Self::Command(cmd)
    }
}

impl From<BarrierKind> for CuePayload {
    fn from(barrier: BarrierKind) -> Self {
        Self::Barrier(barrier)
    }
}

impl From<RoutingCommand> for CuePayload {
    fn from(routing: RoutingCommand) -> Self {
        Self::Routing(routing)
    }
}

// ============================================================================
// 個別演出指示
// ============================================================================

/// 個々の演出指示（相対時刻）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cue {
    /// 対象演者の識別子
    pub actor: ActorKey,
    /// CueSheet 開始時点からの相対秒数
    pub start_time: f64,
    /// 演出ペイロード（コマンド / バリア / ルーティング）
    pub payload: CuePayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrier_kind_clone_debug_partial_eq() {
        let barrier = BarrierKind::WaitForInput { timeout: Some(5.0) };
        let clone = barrier.clone();
        assert_eq!(barrier, clone);
        let debug = format!("{:?}", barrier);
        assert!(debug.contains("WaitForInput"));
    }

    #[test]
    fn barrier_kind_three_variants() {
        let _ = BarrierKind::WaitForInput { timeout: None };
        let _ = BarrierKind::WaitForChoice {
            timeout: Some(30.0),
        };
        let _ = BarrierKind::Timeout { duration: 5.0 };
    }

    #[test]
    fn routing_command_three_variants() {
        let actor = ActorKey::from("sakura");
        let _ = RoutingCommand::RouteAdd {
            target: CueTarget::Balloon,
            to: EntityKey::Actor(actor.clone(), CueTarget::Balloon),
        };
        let _ = RoutingCommand::RouteSwitch {
            target: CueTarget::Shell,
            to: EntityKey::Spot("spot1".into()),
        };
        let _ = RoutingCommand::RouteRemove {
            target: CueTarget::Balloon,
        };
    }

    #[test]
    fn cue_command_six_variants() {
        let cmds = vec![
            CueCommand::Text("hello".into()),
            CueCommand::Clear,
            CueCommand::Emote {
                key: "smile".into(),
            },
            CueCommand::Choice {
                id: "yes".into(),
                text: "はい".into(),
            },
            CueCommand::EntityRef(42),
            CueCommand::Custom {
                command: "fade".into(),
                params: DynamicValue::Null,
            },
        ];
        assert_eq!(cmds.len(), 6);

        // Clone + Debug + PartialEq
        for cmd in &cmds {
            let clone = cmd.clone();
            assert_eq!(*cmd, clone);
            let _ = format!("{:?}", cmd);
        }
    }

    #[test]
    fn cue_payload_from_conversions() {
        let cmd = CueCommand::Text("test".into());
        let payload: CuePayload = cmd.into();
        assert!(matches!(payload, CuePayload::Command(CueCommand::Text(_))));

        let barrier = BarrierKind::WaitForInput { timeout: None };
        let payload: CuePayload = barrier.into();
        assert!(matches!(payload, CuePayload::Barrier(_)));

        let routing = RoutingCommand::RouteRemove {
            target: CueTarget::Shell,
        };
        let payload: CuePayload = routing.into();
        assert!(matches!(payload, CuePayload::Routing(_)));
    }

    #[test]
    fn domain_types_serde_roundtrip() {
        let actor = ActorKey::from("sakura");
        let json = serde_json::to_string(&actor).unwrap();
        let parsed: ActorKey = serde_json::from_str(&json).unwrap();
        assert_eq!(actor, parsed);

        let target = CueTarget::Balloon;
        let json = serde_json::to_string(&target).unwrap();
        let parsed: CueTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(target, parsed);

        let cmd = CueCommand::Text("hello".into());
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: CueCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);

        let barrier = BarrierKind::Timeout { duration: 3.0 };
        let json = serde_json::to_string(&barrier).unwrap();
        let parsed: BarrierKind = serde_json::from_str(&json).unwrap();
        assert_eq!(barrier, parsed);
    }

    #[test]
    fn actor_key_hash_eq() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ActorKey::from("sakura"));
        set.insert(ActorKey::from("sakura"));
        assert_eq!(set.len(), 1);
        set.insert(ActorKey::from("kero"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn cue_construction() {
        let cue = Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.5,
            payload: CueCommand::Text("hello".into()).into(),
        };
        assert_eq!(cue.actor.as_str(), "sakura");
        assert_eq!(cue.start_time, 1.5);
        assert!(matches!(
            cue.payload,
            CuePayload::Command(CueCommand::Text(_))
        ));
    }
}
