//! cue モジュール — 離散コマンド配信エンジン基盤
//!
//! dola クレートの 2 つめのエンジン。連続値アニメ宣言エンジン（`DolaRuntime`）に対して、
//! 離散コマンドの時刻ベース配信を担う。
//!
//! # 提供する型
//!
//! | 型 | 役割 |
//! |---|---|
//! | `TimedSchedule<T>` | 0 ベース相対オフセットの汎用配信エンジン |
//! | `CueCommand` | データ系 7 バリアント演出コマンド |
//! | `RoutingCommand` | 配送制御 3 バリアント |
//! | `CuePayload` | CueSheet 記述時の統一型 |
//! | `CueSheet` | 相対時刻コマンド列（演出台本） |
//! | `ActorKey` / `CueTarget` / `EntityKey` / `Cue` | 演出ドメイン型 |
//! | `TalkCue` | 配送エンベロープ（`Cue` の実行時投影・serde 非依存） |

mod command;
pub mod schedule;
pub mod sheet;

pub use command::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueTarget, EntityKey, RoutingCommand,
    TalkCue,
};
pub use schedule::{Entry, TimedSchedule};
pub use sheet::{CompiledCue, CueSheet, compile_sheet};
