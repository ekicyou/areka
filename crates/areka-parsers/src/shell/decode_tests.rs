//! decode の単体テストの親（`decode(lex(..))` を主題ごとの兄弟ファイルで確かめる）。
//!
//! 本ファイルは共通の import と子の宣言だけを持つ。振り分けと寛容スキップは `skeleton_tests`、
//! element・collision・animation・append・alias の値化はそれぞれの子が確かめる。
//! 期待値はリテラル直書き（sakura 規律）。

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
