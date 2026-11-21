//! SHIORI ABI 定義クレート（最小依存・`wintf`/`dola`/`bevy_ecs` 非依存）。
//!
//! `IShiori`（脳側が実装する COM 境界）と `IShioriHost`（areka 本体が実装する sink）を
//! カスタム COM インターフェイスとして定義するための独立 ABI クレート。
//! 下流の 32bit ホストが UI 基盤（`wintf`）を引き込まず同 ABI を共有できるよう、依存を
//! `windows-core` / `windows`(`Win32_System_Com`) / `thiserror` に限定する。x64 / CPU ネイティブ前提。

pub mod ergonomic;
pub mod error;
pub mod interface;
pub mod outcome;
