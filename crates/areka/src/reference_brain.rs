//! 製品コード（非 `#[cfg(test)]`）のリファレンス脳。
//!
//! 上流 `areka-P0-shiori-com` の `IShiori` を `#[implement(IShiori)]` で実装する
//! 最小の native リファレンス脳と、`IShiori` を生成する唯一の純粋C コンストラクタ
//! `pub unsafe extern "system" fn shiori_create(out)` を所有する（要件 1・2・3・4・5・7・9）。
//!
//! 本モジュールは下流仕様（`areka-P0-shiori-host-32` の DLL ホスト、
//! `areka-P0-reference-ghost` の pasta 旗艦脳）が `IShiori` 実装と `shiori_create`
//! コンストラクタを参照する**正解見本**であり、content（リクエスト・応答・通知の本文）は
//! 不透明のまま固定／エコーで取り回す（要件 4.4・7.2・8.1）。
//!
//! 本タスク（1.1）は宣言のみのスタブ。`IShiori` 実装・`shiori_create` は後続タスク
//! （2.x／3.x／9.x）で配線する。
