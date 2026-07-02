//! 旧 `ShioriExt::request` 即時往復の結合テストは Task 8.1 で撤去された。
//!
//! 新 ABI（factory 経由 create → `get`/`notify` の safe 面往復・所有権/Drop 実証）の roundtrip 統合
//! テストは **Task 8.4 が再構築**する（design.md §Testing Strategy）。本ファイルは撤去済みを明示する
//! プレースホルダで、テストを含まない（`cargo test -p shiori-abi` は本ファイルから 0 件を収集する）。
