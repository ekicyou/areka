//! # canvas — emo 共有描画基盤の内容キャンバス（純粋層）
//!
//! 描画面を「キャンバスに置かれる変換行列付き矩形コンテンツ（住人）」の集合として表現する
//! `ContentCanvas`／`Resident`／`ResidentContent`（GlyphRun／Image シーム／Surface シーム）／
//! `RegionTransform`（M1 は恒等/平行移動のみ）／`TextEffects`（M2 予約）を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//! 行列は自前表現（emo-compose の行列原則と収束可能な統一形・emo-compose は改変しない）。
