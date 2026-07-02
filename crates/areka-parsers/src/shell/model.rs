//! model — 下流共有 I/O 契約型。
//!
//! 後続タスク 2.1 で Shell ルート型と各サブ型、opaque NewType を定義する。
//! フラット enum は `#[non_exhaustive]` + `#[derive(Clone, Debug, PartialEq)]`
//! （`serde` / `Eq` / `Hash` なし）、opaque NewType はフィールド非公開・
//! `new()` ＋ read-only accessor（dola `ActorKey` 流儀）で構成する。
//! 依存方向 `model ← lexer ← decode ← parse` の最上流ゆえ他層に依存しない。
