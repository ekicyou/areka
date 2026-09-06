//! 入出力層の束ね。
//!
//! ワークスペース根の解決・ファイルの読み書き・ソースの走査・スナップショットの
//! 読み込みだけを担い、判断は一切持たない（設計 Architecture）。
//! 一時ディレクトリは使わない（設計 File Structure Plan）。

pub mod files;
pub mod paths;
// スナップショットの読み込みだけは crate の中に閉じる。常時走る整合検査
// （`tests/consistency.rs`）はライブラリの外にある別クレートなので、これで
// 「スナップショットの無い環境でも検査が赤にならない」ことを申し合わせではなく
// 型検査で守れる（要件 6.2・設計 Testing Strategy 19）。`cli` は crate の中にあるので
// そのまま引ける。
pub(crate) mod snapshot;
pub mod sources;
