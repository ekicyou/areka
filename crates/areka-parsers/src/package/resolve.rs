//! resolve — ツリー解決 + I/O（唯一の I/O 保有点）。
//!
//! **スタブ**: このタスク（1.1・module 接ぎ木）では公開 facade の
//! シグネチャのみを確定させ、本体は未実装とする。実体（`std::fs` 存在確認・
//! descript 読込合成・既定フォールバック）はタスク 3.1/3.2 が本実装で埋める。

use std::path::Path;

use crate::charset::DefaultEncoding;

use super::model::{MountError, MountModel};

/// 展開済みゴーストパッケージのルートから、descript.txt 起点で
/// SHIORI/shell の 2 点マウントを解決する。
///
/// - `ghost_root`: 展開済みゴーストパッケージのルート（`ghost/` `shell/` を含む階層）。
/// - `default_encoding`: descript.txt に `charset` 宣言が無い場合に用いる既定エンコード。
///   本 module は既定をハードコードせず（固定 Utf8 はレガシー ANSI ゴーストを誤読する）、
///   呼び出し側が指定する（SSP 準拠の既定は ANSI）。非 UTF-8 拒否のエンフォースは
///   下流の SHIORI 層（設計ディスカッション #1）。
///
/// 成功時 `MountModel`、致命的欠落時 `MountError` を返す。
///
/// **スタブ**: 本体はタスク 3.1/3.2 が実装する。
#[allow(unused_variables)]
pub fn resolve(
    ghost_root: &Path,
    default_encoding: DefaultEncoding,
) -> Result<MountModel, MountError> {
    unimplemented!("package::resolve はタスク 3.1/3.2 で実装する")
}
