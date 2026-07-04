//! # areka-emo-atlas
//!
//! emo レンダリングエンジンの**素材基盤層**（pure）。emo の三段直列
//! （`emo-atlas` → `emo-compose` → `emo-present`）のうち **1/3** を担う。
//!
//! 責務は bake パイプライン
//! （マニフェスト導出 → デコード → 正規化 → トリム → packing → 焼付）で、
//! shell/balloon の surface 表現から element 画像を列挙し、premultiplied BGRA の
//! アトラス頁へ正規化・トリム・配置し、成果物契約（`AtlasTable` ほか）を正本として
//! 定義する。COM/WIC 依存はデコードポートの WIC 腕にのみ隔離し、正規化以降の
//! 純粋コアは wintf 本体（ECS/D2D/GraphicsCore）へ依存しない。
//!
//! パイプライン各段はモジュールへ分割される（本タスクでは雛形のみ）:
//! - [`manifest`] — マニフェスト導出（列挙・間接参照解決・重複排除）
//! - [`decode`] — デコードポート（trait）＋既定 WIC 腕
//! - [`normalize`] — 透過正規化（premultiplied 統一）
//! - [`trim`] — α トリミング
//! - [`pack`] — packing 座標算出
//! - [`bake`] — 頁バッファへの焼付
//! - [`table`] — 成果物契約型（正本）
//! - [`error`] — 診断可能なエラー型

pub mod bake;
pub mod decode;
pub mod error;
pub mod manifest;
pub mod normalize;
pub mod pack;
pub mod table;
pub mod trim;

// 成果物契約の正本型（D3・R6）。下流 emo-compose はこれらを import する。
pub use table::{
    AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, Placement, Point, Rect, SetId, Size,
};

// デコードポート（D4・R2.3）。既定手段（WIC）を露出せず、trait とデータ型のみ公開。
pub use decode::{DecodeError, DecodedImage, ElementDecoder, MemoryDecoder};

// 既定デコード腕（WIC 経由・COM 隔離・D4）。上位は trait 越しに用いる。
pub use decode::wic_arm::WicDecoderArm;

// マニフェスト導出（列挙層・R1.1–1.6/5.6・D6）。
pub use manifest::{Manifest, ManifestDeriver, SurfaceSet};

// 共有透過パラメータ型（normalize の設計本拠・SurfaceSet が運ぶ・3.6）。
pub use normalize::{AlphaParams, UseSelfAlpha};
