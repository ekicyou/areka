//! ゴースト／シェル／バルーン／追加ファイルの `.nar` を入れる手続き（spec: areka-P0-ghost-install）。
//!
//! メニュー・台本・後続の投げ込みが呼ぶ受付の口は後続のタスクがここに置く。書庫の読み取り・
//! 検査・展開は `areka-nar` に任せる。子のモジュール（判断・利用条件・手続き・背景の
//! スレッド・UI 側の窓口・ファイルを選ぶ画面・置換語の値）は、それを作るタスクが
//! 宣言を 1 行ずつ足す。

use std::path::PathBuf;

mod judge;
mod procedure;
mod terms;

/// インストールの依頼（書庫のパスを 1 本以上・並んだ順に扱う＝要件 9.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstallOrder {
    pub archives: Vec<PathBuf>,
    pub origin: InstallOrigin,
}

/// 依頼の出どころ（記録の語彙・手続きは分岐しない）。
// 組み立てる呼び手（メニュー・台本の入口）は後続のタスクが結ぶ。結んだら外す。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallOrigin {
    Menu,
    Script,
}
