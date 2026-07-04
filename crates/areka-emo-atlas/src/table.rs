//! 成果物契約型（正本）: `AtlasTable` / `AtlasEntry` / `AtlasKey` / `AtlasPage`。
//!
//! 設計決定 **D3**（要件 **R6**）。
//!
//! 本層が emo-compose と共有する成果物契約を**正本として定義**する。識別子は二層で、
//! ランタイムキー＝`ElementId(u32)`（密 index・決定的採番・毎フレーム O(1) 引き）／
//! ソースキー＝`AtlasKey{ set, rel_path }`（無改変相対パス・重複排除／golden／
//! デバッグ逆引き用にテーブル保持）。空エントリは
//! `AtlasEntry.placement: Option<Placement>`（`None`＝転写スキップ）で表現する。
//! 頁バッファは `AtlasPage{ bytes: Arc<[u8]>, width, height, stride }`
//! （premultiplied BGRA・stride 明示）。
//!
//! （本タスクは雛形。型定義は後続タスクで追加する。）
