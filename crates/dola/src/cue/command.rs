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
///
/// `Ord` は内部文字列の辞書順（`BTreeMap<ActorKey, _>` 等の決定論的順序付けを
/// 下流——emo テキスト層の actor 別状態 map など——が要求する）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    /// 表示系（サーフェス消費・seriko が消費: シェル面＋バルーン面）。
    /// Emote, EntityRef, BalloonSurface を主に消費する。
    /// 注: 名前は「シェル」だが分類上バルーン面切替もここへ経路付けられる（名前負債）。
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

/// 演出コマンド（8 バリアント、データ系のみ）。
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
    /// 改行（比率 1.0=全角 1 行）。意味解釈は消費者の責務。
    NewLine { ratio: f32 },
    /// バルーン面切替。key は不透明文字列（数値形・名前形・"-1" 非表示センチネル）。
    /// 解釈（数値化・alias）は消費者（seriko）の責務。dola は状態を持たない。
    /// `Emote { key }` と完全対称の不透明 key 転写語彙。
    BalloonSurface { key: String },
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
    /// この cue の presentation 占有時間（秒）。
    ///
    /// 全表現者が action を処理するか否かに関わらず honor する（duration honor 契約）。
    /// 後続 cue の絶対時刻はこの分だけ上流（sakura compile）で焼き込まれるため、
    /// 表現者はこの値から新たなローカル遅延を生じさせてはならない（二重待ち禁止）。
    ///
    /// 全 presentation cue が本フィールドを保持し、時間を占有しない瞬時コマンドは
    /// **明示的な 0** を持つ（「duration フィールドを持たない cue」という概念を作らない）。
    /// `#[serde(default)]` により、本フィールドを持たない旧シリアライズ資産は 0 として
    /// 従来どおり解釈される（後方互換・既存 variant のワイヤ形は不変）。
    ///
    /// 値は**不透明な秒数**であり、dola は SakuraScript 固有の意味論
    /// （1 文字あたりのウェイト値等）を内包しない——算出は上流（sakura）の責務。
    ///
    /// 注: `CuePayload::Barrier`（動的停止点）／`Routing`（表現者未配送の制御プレーン）は
    /// presentation でなく duration 概念が本質的に非該当のため、静的 duration
    /// タイムラインの外に置かれる（envelope としては一律にフィールドを持ち値は 0）。
    #[serde(default)]
    pub duration: f64,
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
    fn cue_command_eight_variants() {
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
            CueCommand::NewLine { ratio: 1.0 },
            CueCommand::BalloonSurface { key: "2".into() },
        ];
        assert_eq!(cmds.len(), 8);

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
    fn cue_command_newline_serde_roundtrip() {
        let cmd = CueCommand::NewLine { ratio: 1.5 };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: CueCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }

    #[test]
    fn cue_command_balloon_surface_serde_roundtrip() {
        let cmd = CueCommand::BalloonSurface { key: "2".into() };
        let json = serde_json::to_string(&cmd).unwrap();
        // externally tagged で additive（既存 variant のワイヤ形は不変）。
        assert_eq!(json, r#"{"BalloonSurface":{"key":"2"}}"#);
        let parsed: CueCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);

        // 名前形・非表示センチネルも不透明のまま忠実にラウンドトリップする。
        for key in ["バルーン１", "-1", "10"] {
            let cmd = CueCommand::BalloonSurface { key: key.into() };
            let json = serde_json::to_string(&cmd).unwrap();
            let parsed: CueCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, parsed, "BalloonSurface(key={key:?}) must roundtrip");
        }
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

    // ── D3-T 追加ギャップテスト ──

    #[test]
    fn actor_key_new_display_as_str() {
        let key = ActorKey::new("sakura");
        assert_eq!(key.as_str(), "sakura");
        assert_eq!(key.to_string(), "sakura"); // Display 実装

        // From<String> / From<&str> の等価性
        assert_eq!(ActorKey::from("kero"), ActorKey::from("kero".to_string()));
    }

    #[test]
    fn entity_key_namespaces_are_distinct() {
        use std::collections::HashSet;
        // 同名でも名前空間（バリアント）が違えば別キー
        let spot = EntityKey::Spot("x".to_string());
        let balloon = EntityKey::Balloon("x".to_string());
        let actor = EntityKey::Actor(ActorKey::from("x"), CueTarget::Shell);
        assert_ne!(spot, balloon);
        assert_ne!(spot, actor);

        let mut set = HashSet::new();
        set.insert(spot);
        set.insert(balloon);
        set.insert(actor);
        assert_eq!(set.len(), 3);

        // Actor の CueTarget スロット違いも別キー
        let mut slots = HashSet::new();
        slots.insert(EntityKey::Actor(ActorKey::from("a"), CueTarget::Shell));
        slots.insert(EntityKey::Actor(ActorKey::from("a"), CueTarget::Balloon));
        assert_eq!(slots.len(), 2);
    }

    #[test]
    fn entity_key_serde_roundtrip() {
        let keys = vec![
            EntityKey::Actor(ActorKey::from("sakura"), CueTarget::Balloon),
            EntityKey::Spot("spot1".to_string()),
            EntityKey::Balloon("balloon1".to_string()),
        ];
        for key in keys {
            let json = serde_json::to_string(&key).unwrap();
            let parsed: EntityKey = serde_json::from_str(&json).unwrap();
            assert_eq!(key, parsed);
        }
    }

    #[test]
    fn routing_command_serde_roundtrip() {
        let cmds = vec![
            RoutingCommand::RouteAdd {
                target: CueTarget::Balloon,
                to: EntityKey::Actor(ActorKey::from("sakura"), CueTarget::Balloon),
            },
            RoutingCommand::RouteSwitch {
                target: CueTarget::Shell,
                to: EntityKey::Spot("spot1".to_string()),
            },
            RoutingCommand::RouteRemove {
                target: CueTarget::Shell,
            },
        ];
        for cmd in cmds {
            let json = serde_json::to_string(&cmd).unwrap();
            let parsed: RoutingCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, parsed);
        }
    }

    #[test]
    fn cue_payload_and_cue_serde_roundtrip() {
        // CuePayload 3 種のラウンドトリップ
        let payloads = vec![
            CuePayload::Command(CueCommand::Custom {
                command: "fade".to_string(),
                params: DynamicValue::Null,
            }),
            CuePayload::Barrier(BarrierKind::WaitForChoice { timeout: Some(3.0) }),
            CuePayload::Routing(RoutingCommand::RouteRemove {
                target: CueTarget::Balloon,
            }),
        ];
        for payload in payloads {
            let json = serde_json::to_string(&payload).unwrap();
            let parsed: CuePayload = serde_json::from_str(&json).unwrap();
            assert_eq!(payload, parsed);
        }

        // Cue 全体（PartialEq 非導出のためフィールドごとに検証）
        let cue = Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.5,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.0,
        };
        let json = serde_json::to_string(&cue).unwrap();
        let parsed: Cue = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.actor, cue.actor);
        assert_eq!(parsed.start_time, cue.start_time);
        assert_eq!(parsed.payload, cue.payload);
        assert_eq!(parsed.duration, cue.duration);
    }

    // ── D3-V 境界特性化テスト ──

    #[test]
    fn entity_ref_u64_boundary_serde_roundtrip() {
        // EntityRef は u64 全域（Entity::to_bits() 変換値）を保持する。
        // i64::MAX 超の値（上位ビットが立つ Entity generation 等）も JSON 経由で
        // 欠損なくラウンドトリップすることを固定する（serde_json は u64 を直接扱う。
        // なお TOML の整数は i64 のため、TOML 直列化では i64::MAX 超は表現できない —
        // 現行ワークスペースに CueCommand の TOML 直列化経路はない）。
        for bits in [0u64, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX] {
            let cmd = CueCommand::EntityRef(bits);
            let json = serde_json::to_string(&cmd).unwrap();
            let parsed: CueCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, parsed, "EntityRef({bits}) must roundtrip losslessly");
        }
    }

    #[test]
    fn cue_construction() {
        let cue = Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.5,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.0,
        };
        assert_eq!(cue.actor.as_str(), "sakura");
        assert_eq!(cue.start_time, 1.5);
        assert!(matches!(
            cue.payload,
            CuePayload::Command(CueCommand::Text(_))
        ));
    }

    // ── duration（再生時間）envelope 檻 ──

    #[test]
    fn cue_duration_defaults_to_zero_for_legacy_serialized_data() {
        // 再生時間フィールドを持たない旧シリアライズ資産（3 フィールド）を読み込むと
        // duration=0（瞬時）として従来どおり解釈できる（後方互換）。
        let legacy = r#"{"actor":"0","start_time":0.0,"payload":{"Command":{"Text":"hi"}}}"#;
        let parsed: Cue = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.actor, ActorKey::from("0"));
        assert_eq!(parsed.start_time, 0.0);
        assert_eq!(parsed.payload, CueCommand::Text("hi".into()).into());
        assert_eq!(
            parsed.duration, 0.0,
            "duration 欠落の旧資産は 0（瞬時）として復元されねばならない"
        );

        // Barrier / Routing ペイロード（duration 非該当）の旧資産も同様に読める。
        let legacy_barrier = r#"{"actor":"0","start_time":1.0,"payload":{"Barrier":{"WaitForInput":{"timeout":null}}}}"#;
        let parsed: Cue = serde_json::from_str(legacy_barrier).unwrap();
        assert_eq!(parsed.duration, 0.0);
    }

    #[test]
    fn cue_duration_roundtrip_preserves_value() {
        // 新規往復では duration の値が保たれ、既存ペイロードのワイヤ形は変わらない。
        let cue = Cue {
            actor: ActorKey::from("sakura"),
            start_time: 1.5,
            payload: CueCommand::Text("hello".into()).into(),
            duration: 0.25,
        };
        let json = serde_json::to_string(&cue).unwrap();
        assert!(
            json.contains(r#""payload":{"Command":{"Text":"hello"}}"#),
            "既存 variant のワイヤ形は envelope 拡張後も不変: {json}"
        );

        let parsed: Cue = serde_json::from_str(&json).unwrap();
        // Cue は PartialEq 非導出のためフィールドごとに検証する。
        assert_eq!(parsed.actor, cue.actor);
        assert_eq!(parsed.start_time, cue.start_time);
        assert_eq!(parsed.payload, cue.payload);
        assert_eq!(parsed.duration, 0.25);
    }

    #[test]
    fn duration_is_uniform_envelope_field_across_all_payloads() {
        // 「再生時間フィールドを持たない cue」という概念を作らない——
        // 全 presentation cue（CueCommand 全 variant）が envelope duration を保持し、
        // 瞬時コマンドは明示的 0 を持つ（欠落でない）。
        let payloads: Vec<CuePayload> = vec![
            CueCommand::Text("hello".into()).into(),
            CueCommand::Clear.into(),
            CueCommand::Emote {
                key: "smile".into(),
            }
            .into(),
            CueCommand::Choice {
                id: "yes".into(),
                text: "はい".into(),
            }
            .into(),
            CueCommand::EntityRef(42).into(),
            CueCommand::Custom {
                command: "fade".into(),
                params: DynamicValue::Null,
            }
            .into(),
            CueCommand::NewLine { ratio: 1.0 }.into(),
            CueCommand::BalloonSurface { key: "2".into() }.into(),
            // duration 非該当ペイロード（Barrier / Routing）も envelope としては
            // 一律にフィールドを持つ（値は 0・静的 duration タイムラインの外）。
            BarrierKind::Timeout { duration: 5.0 }.into(),
            RoutingCommand::RouteRemove {
                target: CueTarget::Shell,
            }
            .into(),
        ];

        for payload in payloads {
            // 瞬時（明示的 0）
            let instant = Cue {
                actor: ActorKey::from("sakura"),
                start_time: 0.0,
                payload: payload.clone(),
                duration: 0.0,
            };
            let parsed: Cue =
                serde_json::from_str(&serde_json::to_string(&instant).unwrap()).unwrap();
            assert_eq!(parsed.duration, 0.0, "瞬時 cue は明示的 0 を保持する");
            assert_eq!(parsed.payload, instant.payload);

            // 時間占有（不透明秒数）
            let timed = Cue {
                actor: ActorKey::from("sakura"),
                start_time: 0.0,
                payload,
                duration: 1.25,
            };
            let parsed: Cue =
                serde_json::from_str(&serde_json::to_string(&timed).unwrap()).unwrap();
            assert_eq!(parsed.duration, 1.25);
            assert_eq!(parsed.payload, timed.payload);
        }
    }
}
