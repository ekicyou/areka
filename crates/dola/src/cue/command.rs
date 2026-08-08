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
    /// 窓/placement 系（窓移動等）— `\!` 汎用キャリアのうち、消費側が名前で自己選別して
    /// ここへ割り当てるコマンド名（M1: `"move"`）を消費する演者スロット。
    /// additive unit variant（既存 variant のワイヤ形不変・`EntityKey` 参照非破壊）。
    Window,
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

/// 演出コマンド（10 バリアント、データ系のみ）。
///
/// バリアは `BarrierKind` として、ルーティングは `RoutingCommand` として、
/// それぞれ `Entry` レベルで分離済み。
///
/// 各コマンドは **action の種別のみ**を表し、時間は常に `Cue` envelope の
/// `duration` が担う（`Wait` も同様——コマンド側に時間値を埋め込まない）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CueCommand {
    /// テキスト表示。意味解釈（縦書き、装飾等）は消費者の責務。
    Text(String),
    /// コンテンツクリア（**対象スコープのみ**消去）。全スコープ消去は `ClearAll`。
    Clear,
    /// 演技発現。key の意味解釈は消費者が担う。
    Emote { key: String },
    /// 選択肢データ。WaitForChoice の前に連続投入する先積みプロトコル。
    ///
    /// `references` は `\q` の第 3 引数以降（参照列）を**記述順を保った不透明文字列列**として
    /// 保持する（ID 解釈なし・1.3/1.4）。`#[serde(default, skip_serializing_if = "Vec::is_empty")]`
    /// により、空のときは `references` キーを出力せず既存ワイヤ形とバイト同一を保ち、
    /// `references` を持たない旧資産も `default` で空 vec として読める（8.1・後方互換）。
    Choice {
        id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        references: Vec<String>,
    },
    /// ECS エンティティ参照渡し（u64 = Entity::to_bits() 変換済み）
    EntityRef(u64),
    /// `\!` ベースウェアコマンド名前空間全体の汎用キャリア（正典 183 コマンドの全転写）。
    ///
    /// コマンドごとの typed cue 語彙は**新設しない**——`move` も `bind` も同一の本キャリアに
    /// 乗り（正準形は [`CueCommand::command_carrier`] が単一箇所で構築する）、消費は
    /// **消費者の名前自己選別**へ委譲される（dola はコマンド名の語彙を持たない）。ゆえに
    /// 本 variant の型レベル分類 `cue_target_of(Custom)=None` は「誰も action しない」ではなく
    /// 「消費側が名前で自己選別する」ことを意味する（R8.7）。
    ///
    /// `params` は JSON 互換辞書型。汎用キャリアとしての正準形は
    /// `DynamicValue::Array([String…])`（トークン列）だが、それ以外の消費者固有形も
    /// 型としては表現可能で、非正準 params は [`CueCommand::as_command_carrier`] が
    /// `None` を返し消費側は記録付き良性スキップへ縮退する。
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
    /// カーソル位置指定（さくらスクリプト `\_l` の不透明転写）。
    ///
    /// x・y は**記述通りの不透明な文字列**であり、単位付き（`5em`/`2lh`/`50%`）・
    /// 裸数値・相対（`@` 前置）・空（省略）の区別を失わない（R3.2）。dola は
    /// 単位換算・座標解決・原点解釈を**行わない**——これらは消費（表示）側の責務（R3.3）。
    /// 双方が空でも発行され得る（記述の存在を台本から失わせない・R3.5）。
    Cursor { x: String, y: String },
    /// 純粋な待ち（**action を持たない**第一級コマンド）。
    ///
    /// 待ち時間は本 variant でなく `Cue` envelope の `duration` が保持する
    /// （envelope 一律ゆえ、表現者はコマンドを解釈せず duration を honor できる）。
    /// 上流（sakura compile）が明示ウェイトを offset へ吸収して消さず本 cue として
    /// 台本に残すことで、末尾・単独の待ちも失われない自己完結した楽譜になる。
    /// action がないため、どの表現者にとっても担当外（duration のみ honor する）。
    Wait,
    /// **全スコープ**のコンテンツクリア（`Clear`＝対象スコープのみ、との峻別）。
    ///
    /// 上流は残存スコープを列挙できないため、"全消し"を表現者が自らの全スコープを
    /// 消す自己完結コマンドとして表現する。テキスト表現者（バルーン）が消費する。
    ClearAll,
}

impl CueCommand {
    /// `\![name,args...]` の汎用キャリア正準形を構築する（生成はこの一点を通す）。
    ///
    /// `Custom { command: name, params: DynamicValue::Array([String…]) }` を組む。
    /// トークンは記述順のまま無変形で載せ、空トークン（省略スロット）・`--key=value`
    /// トークンも素通しで保持する（R4.2）。往復同一は [`Self::as_command_carrier`] が保証する。
    pub fn command_carrier(name: impl Into<String>, tokens: Vec<String>) -> CueCommand {
        CueCommand::Custom {
            command: name.into(),
            params: DynamicValue::Array(tokens.into_iter().map(DynamicValue::String).collect()),
        }
    }

    /// キャリア正準形の抽出子（消費はこの一点を通す）。
    ///
    /// `Custom` かつ `params` が全要素 String の `Array`（＝正準形）のときのみ
    /// `Some((name, tokens))` を返す。`Custom` 以外・非 `Array`・非 String 要素混入の
    /// いずれも `None`＝消費側は記録付き良性スキップへ縮退する（R4.5）。空トークンは保持される。
    pub fn as_command_carrier(&self) -> Option<(&str, Vec<&str>)> {
        let CueCommand::Custom { command, params } = self else {
            return None;
        };
        let DynamicValue::Array(items) = params else {
            return None;
        };
        let mut tokens = Vec::with_capacity(items.len());
        for item in items {
            match item {
                DynamicValue::String(s) => tokens.push(s.as_str()),
                // 非 String 要素が 1 つでもあれば非正準 → None（記録付き良性スキップ）。
                _ => return None,
            }
        }
        Some((command.as_str(), tokens))
    }
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

// ============================================================================
// 配送エンベロープ
// ============================================================================

/// 1 発火の配送エンベロープ（搬送体）— cue 再生ランタイムから演者への受け渡し単位。
///
/// [`Cue`] の**実行時投影**であり serde 非依存（ワイヤ型は [`Cue`]・本型は配送経路の
/// 通貨）。上流の canonical 変換が [`Cue`] の各フィールドを**無変形**で複写して組み、
/// 登録された全出力先へ broadcast される。
///
/// 演者は受け取った任意の搬送体について、[`Self::command`] の action を処理するか否かに
/// 関わらず [`Self::duration`] を honor する（duration honor 契約）。duration が
/// コマンド種別を問わない一律フィールドであることが、「コマンドを解釈せずに時間を読める」
/// ＝honor 契約が例外なく回る前提を担保する。
#[derive(Clone, Debug, PartialEq)]
pub struct TalkCue {
    /// 発火時刻（f64 秒＝dola ドメイン）。上流が確定した値をそのまま運ぶ（導出しない）。
    pub at: f64,
    /// 対象演者の識別子（話者スコープ）。
    pub actor: ActorKey,
    /// 演出コマンド（action の種別のみ・時間は本 envelope の `duration` が担う）。
    pub command: CueCommand,
    /// この cue の presentation 占有時間（秒）— [`Cue::duration`] の無変形複写。
    ///
    /// 全演者が action 可否に関わらず honor する。後続 cue の発火時刻は上流で既に
    /// この分だけ焼き込まれているため、演者はこの値から**新たなローカル遅延を
    /// 生じさせてはならない**（二重待ち禁止）。瞬時コマンドは明示的な 0 を持つ
    /// （「duration を持たない搬送体」という概念は作らない）。
    pub duration: f64,
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
