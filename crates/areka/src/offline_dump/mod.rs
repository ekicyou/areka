//! オフラインの書き出し（`--offline-dump`）の走り（spec: areka-P0-offline-surface-dump）。
//!
//! ゴーストを起こさず、窓も出さずに、指定のシェルの面を PNG のファイルへ書き出し、その間の
//! 記録を CSV に残して終了コードを返す。使ってよい部品は design.md の許す依存の一覧だけで、
//! areka の記憶・置き場・MCP・窓の部品は呼ばない（要件 9 の構造の上の担保）。

/// `--dump-bind` の読みと、スコープの着せ替えへの当て方。
mod bind_spec;
/// 面 1 つを時点 T まで進めて組み、シェル全体の下地に置いて PNG のファイルにする。
mod render;
/// 写した記録から CSV の中身・終了コード・CSV のファイルを作る。
mod report;
/// `--dump-surface-list` の読みと、シェルの面の番号への展開。
mod surface_list;

/// 検体のゴーストを `target\` の下に組む部品（試験専用）。
#[cfg(test)]
mod offline_dump_test_support;

#[cfg(test)]
#[path = "offline_dump_tests.rs"]
mod offline_dump_tests;
