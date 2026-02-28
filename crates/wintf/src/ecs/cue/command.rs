//! CueCommand — 型安全な演出コマンド体系（11バリアント）。
//!
//! さくらスクリプトのタグに相当する型安全な enum。
//! 3カテゴリー: データ（ブロードキャスト）、バリア（入力待ち）、ルーティング（配送制御）。

use bevy_ecs::entity::Entity;
use dola::DynamicValue;

use super::CueTarget;

/// CueCommand の配送先スロット内でのキー識別子。
/// EntityRegistry の名前空間を型で分離する。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityKey {
    /// アクターの特定スロット
    Actor(super::ActorKey, CueTarget),
    /// 物理スポットエンティティ (P1 拡張)
    Spot(String),
    /// 物理バルーンエンティティ (P1 拡張)
    Balloon(String),
}

/// 演出コマンド。さくらスクリプトの各タグに相当する型安全な enum。
///
/// # カテゴリー
///
/// | カテゴリー | コマンド | 配信モデル |
/// |-----------|---------|-----------|
/// | データ | Text, Clear, Emote, Choice, EntityRef, Custom | ブロードキャスト |
/// | バリア | WaitForChoice, WaitForClick | ブロードキャスト |
/// | ルーティング | RouteAdd, RouteSwitch, RouteRemove | dispatch 層のみ消費 |
#[derive(Clone, Debug)]
pub enum CueCommand {
    // ── データコマンド（ブロードキャスト） ──
    /// テキスト表示。意味解釈（縦書き、装飾等）は消費者の責務。
    Text(String),
    /// コンテンツクリア
    Clear,
    /// 演技発現。key の意味解釈は消費者が担う。
    /// Spot: サーフェスアニメーション選択、Balloon: フォントセット切替。
    Emote { key: String },
    /// 選択肢データ。WaitForChoice の前に連続投入する先積みプロトコル。
    Choice { id: String, text: String },
    /// ECS エンティティ参照渡し（消費者が解釈）
    EntityRef(Entity),
    /// 消費者固有コマンド。DynamicValue は JSON 互換辞書型。
    Custom {
        command: String,
        params: DynamicValue,
    },

    // ── バリアコマンド（ブロードキャスト） ──
    /// 選択肢バリア。直前の Choice 群を提示してブロック。
    WaitForChoice { timeout: Option<f64> },
    /// クリック待ちバリア。全体配信のため関係するどこをクリックしても応答される。
    WaitForClick { timeout: Option<f64> },

    // ── ルーティングコマンド（dispatch 層のみ消費） ──
    /// スロット追加（既存ルーティングを維持したまま追加先を登録）
    RouteAdd { target: CueTarget, to: EntityKey },
    /// スロット切替（既存ルーティングを上書き）
    RouteSwitch { target: CueTarget, to: EntityKey },
    /// スロット除去（指定ターゲットのルーティングを削除）
    RouteRemove { target: CueTarget },
}

impl CueCommand {
    /// バリアコマンドか判定
    pub fn is_barrier(&self) -> bool {
        matches!(self, Self::WaitForChoice { .. } | Self::WaitForClick { .. })
    }

    /// ルーティングコマンドか判定（dispatch 層で消費、CueQueue に入らない）
    pub fn is_routing_command(&self) -> bool {
        matches!(
            self,
            Self::RouteAdd { .. } | Self::RouteSwitch { .. } | Self::RouteRemove { .. }
        )
    }
}
