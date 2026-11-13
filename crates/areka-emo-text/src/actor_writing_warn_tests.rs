//! # actor_writing_warn_tests — 書字方向の警告は登録口が書く（結線層・兄弟テスト）
//!
//! spec `emo-text-reveal-incremental` 件 3（要件 3.6）。書字方向の宣言の警告と記録を、解決の
//! 内側でなく actor の登録口で数え、件数と文言が今と同じで、同じ宣言を繰り返さないことを
//! 確かめる（`actor_region_warn_tests.rs` と同じ部品と流儀）。
//!
//! [`super`]（`actor.rs`）の子モジュールとして接続してある。
