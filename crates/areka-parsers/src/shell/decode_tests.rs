//! decode の単体テスト（タスク 4.1: surface ブロックの枠組みとヘッダの寛容スキップ）。
//!
//! 本タスクが検証するのは decode スケルトンの骨格挙動のみ:
//! - 空入力・ヘッダのみ（charset/descript）・ヘッダ欠落が失敗せず既定状態で継続する（要件 3.1/3.2/3.3）。
//! - `surfaceNNN` ブロックから surface ID と（当面空の）構成要素枠を取り出す（要件 4.1）。
//! - ディスパッチ優先順位により `kero.surface.alias` / `surface.append*` が
//!   `surfaceNNN` と誤分類されない（設計 System Flows のブロックディスパッチ）。
//!
//! element/collision/animation の値化・append/alias の値化はタスク 4.2〜4.6 の領分ゆえ
//! ここでは検証しない（本タスクは枠組みのみ）。期待値はリテラル直書き（sakura 規律）。

use super::decode::decode;
use super::lexer::lex;
use super::model::{
    AliasKey, Animation, AppendTarget, Collision, CollisionName, DefRef, DrawMethod, Element,
    ElementPath, Interval, Pattern, Shell, SortOrder, Surface, SurfaceAlias,
};

// テーマ単位のテストモジュール接続宣言（areka-P0-file-slimming タスク 8.5・要件 1.7 / 3.1 / 3.2）。
// 本ファイルは module doc と共通 import のみを保持し、テストの本体は元ファイルの区画バナーが
// 宣言していたテーマ境界どおりに兄弟ファイル `decode_tests_<テーマ>_tests.rs` へ置く
// （子は `use super::{…}` で本ファイルの import 束縛を引く）。
#[cfg(test)]
#[path = "decode_tests_alias_tests.rs"]
mod alias_tests;
#[cfg(test)]
#[path = "decode_tests_animation_tests.rs"]
mod animation_tests;
#[cfg(test)]
#[path = "decode_tests_append_tests.rs"]
mod append_tests;
#[cfg(test)]
#[path = "decode_tests_element_collision_tests.rs"]
mod element_collision_tests;
#[cfg(test)]
#[path = "decode_tests_lenient_input_tests.rs"]
mod lenient_input_tests;
#[cfg(test)]
#[path = "decode_tests_method_matrix_tests.rs"]
mod method_matrix_tests;
#[cfg(test)]
#[path = "decode_tests_pattern_method_tests.rs"]
mod pattern_method_tests;
#[cfg(test)]
#[path = "decode_tests_skeleton_tests.rs"]
mod skeleton_tests;
#[cfg(test)]
#[path = "decode_tests_transcription_gap_tests.rs"]
mod transcription_gap_tests;
