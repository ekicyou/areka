//! `.nar`（ゴースト／バルーンの配布アーカイブ）を読み、呼び出し側から受け取った
//! ベースウェアの根へ入れるエンジン。
//!
//! 公開面はコンテナ読取・エントリ名の復号と検証・`install.txt` の解釈・配置計画・
//! 原子的な展開と `refresh`・閉じた拒否語彙（`RefuseReason`）で構成する。
//! 根の場所は決めず、`&Path` として受け取る（決めるのは `baseware-root-layout`）。
//! 検体の在処も知らない（窓口は開発専用の `sample-ghost-kit` にある）。

// タスク 4.4 の `NarArchive::open` が繋ぐまで本体からは呼ばれない（兄弟テストだけが使う）。
#[allow(dead_code)]
mod container;
mod crc32;
mod error;
// タスク 4.4 の `NarArchive::install` が繋ぐまで本体からは呼ばれない（兄弟テストだけが使う）。
#[allow(dead_code)]
mod install;
// タスク 4.4 の `NarArchive::open` が繋ぐまで本体からは呼ばれない（兄弟テストだけが使う）。
#[allow(dead_code)]
mod manifest;
// タスク 4.4 の `NarArchive::open` が繋ぐまで本体からは呼ばれない（兄弟テストだけが使う）。
#[allow(dead_code)]
mod names;
// タスク 4.4 の `NarArchive::install` が繋ぐまで本体からは呼ばれない（兄弟テストだけが使う）。
#[allow(dead_code)]
mod plan;

pub use crc32::crc32;
pub use error::{
    ElementKind, ExistingState, InstalledElement, Integrity, IoPhase, ManifestWarning, NarError,
    RefuseReason, UnsafeWhy, Unsupported,
};
// 設計は 4 つとも公開面に置く（後続の `ghost-install` が受け取る形）。結線は
// タスク 4.4 の仕事なので、ここでは型だけ先に出す。
pub use manifest::{Companion, ExistingPolicy, InstallKind, InstallManifest};
