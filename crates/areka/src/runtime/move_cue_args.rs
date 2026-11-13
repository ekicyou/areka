//! `\![move]` の引数の読み（design.md「MoveArgs」・要件 1.1〜1.13・3.8・4.4〜4.6・5.1）。
//!
//! 名前付きの形（`--キー=値`）と旧い位置の形の両方を、同じ既定・同じ欄の読みで 1 つの
//! [`MoveDirective`] へ読む。読めない値・知らない語は SSP 2.9.18 の実測と同じ値へ置き換え、
//! 置き換えたことを [`MoveNote`] として 1 件ずつ返す（記録は受け口が `warn!` で出す）。
//!
//! 純粋（`World`・`tracing`・wintf の窓を綴らない）・決定論・panic しない。
//!
//! # 形の判定（要件 1.13）
//!
//! 引数のどれかが `--` で始まれば名前付きの形、それ以外は旧い位置の形（`-353` は旧い形）。
//!
//! # 旧い位置の形（要件 1.12・今のまま）
//!
//! | idx | 欄 | 省略時 |
//! |----:|----|--------|
//! | 0 | X | `fix` |
//! | 1 | Y | `fix` |
//! | 2 | time（ms） | 0 |
//! | 3 | base | `screen` |
//! | 4 | base-offset | `left.top` |
//! | 5 | move-offset | `left.top` |
//!
//! 7 欄目以降は読まない（記録もしない）。
//!
//! # 名前付きの形（要件 1.1・1.2・1.8・1.10・1.11）
//!
//! 最初の `=` でキーと値に分け、キーは小文字にそろえて引く。正典のキーは `x`・`y`・`time`・
//! `base`・`base-offset`・`move-offset`・`option`。値が空のキーは省略と同じ（重複とも数えない）。
//! 空の引数（`\![move,--X=0,]` の末尾など）は何も書かれていないので記録せずに読み飛ばす。

use super::{Axis, AxisSpec, MoveBase, MoveDirective, RefPoint, RefX, RefY};

/// 読みの結果（design.md Service Interface）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMove {
    pub directive: MoveDirective,
    /// 読みで置き換えた・無視した・記録すべき点（出現順）。空なら台本どおり。
    pub notes: Vec<MoveNote>,
}

/// 参照点がどちら側か（[`MoveNote::UnknownRefWord`] の識別）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetSide {
    /// `base-offset`（基準の矩形の参照点）。
    Base,
    /// `move-offset`（動かす窓の参照点）。
    Move,
}

/// 読みの置き換えの記録（受け口が `move_arg_substituted` の `warn!` で 1 件ずつ出す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveNote {
    /// `--` で始まらない・`=` の無い引数を無視した（名前付きの形・要件 1.10）。
    MalformedArg { arg: String },
    /// 正典に無いキーを無視した（要件 1.10）。`key` は書かれたままの綴り。
    UnknownKey { key: String },
    /// `--option` の知らない値を無視した（要件 1.10）。
    UnknownOption { value: String },
    /// `option` 以外の同じキーが重なり後の値を使った（要件 1.11）。`key` は小文字。
    DuplicateKey { key: String },
    /// X・Y が整数として読めず `fix` にした（要件 4.5）。
    UnparsableAxis { axis: Axis, token: String },
    /// time が整数として読めず 0 にした（要件 4.6）。
    UnparsableDuration { token: String },
    /// 参照点の知らない語を横 `left`・縦 `top` にした（要件 4.4）。
    UnknownRefWord {
        side: OffsetSide,
        axis: Axis,
        word: String,
    },
    /// 正典の語でも数でもない基準（他のゴーストの名前など）をスコープ 0 にした（要件 3.8）。
    OtherGhostBase { name: String },
    /// time>0 を補間せず最終の位置へ即時に置く（要件 5.1）。
    TimedMoveImmediate { duration_ms: u32 },
}

/// `\![move]` の引数列を [`ParsedMove`] へ読む（両方の形・常に指令を返す）。
///
/// `scope` は動かす窓（cue.actor 由来）。`tokens` はキャリアの `params`（コマンド名を除く）。
pub fn parse_move_directive(scope: u32, tokens: &[String]) -> ParsedMove {
    let mut notes = Vec::new();
    let fields = if tokens.iter().any(|t| t.starts_with("--")) {
        named_fields(tokens, &mut notes)
    } else {
        let tok = |i: usize| tokens.get(i).map(String::as_str).unwrap_or("");
        Fields {
            x: tok(0),
            y: tok(1),
            time: tok(2),
            base: tok(3),
            base_offset: tok(4),
            move_offset: tok(5),
            ignore_sticky_window: false,
        }
    };

    let directive = MoveDirective {
        scope,
        x: read_axis(fields.x, Axis::X, &mut notes),
        y: read_axis(fields.y, Axis::Y, &mut notes),
        duration_ms: read_duration(fields.time, &mut notes),
        base: read_base(fields.base, &mut notes),
        base_offset: read_ref_point(fields.base_offset, OffsetSide::Base, &mut notes),
        move_offset: read_ref_point(fields.move_offset, OffsetSide::Move, &mut notes),
        ignore_sticky_window: fields.ignore_sticky_window,
    };
    ParsedMove { directive, notes }
}

/// 両方の形に共通の欄（空＝省略）。
struct Fields<'a> {
    x: &'a str,
    y: &'a str,
    time: &'a str,
    base: &'a str,
    base_offset: &'a str,
    move_offset: &'a str,
    ignore_sticky_window: bool,
}

/// 名前付きの形の引数列を欄へ振り分ける（無視・重複・知らない option を記録）。
fn named_fields<'a>(tokens: &'a [String], notes: &mut Vec<MoveNote>) -> Fields<'a> {
    let mut f = Fields {
        x: "",
        y: "",
        time: "",
        base: "",
        base_offset: "",
        move_offset: "",
        ignore_sticky_window: false,
    };
    for arg in tokens.iter().filter(|t| !t.is_empty()) {
        let Some((key, value)) = arg.strip_prefix("--").and_then(|kv| kv.split_once('=')) else {
            notes.push(MoveNote::MalformedArg { arg: arg.clone() });
            continue;
        };
        let lower = key.to_ascii_lowercase();
        let slot = match lower.as_str() {
            "x" => &mut f.x,
            "y" => &mut f.y,
            "time" => &mut f.time,
            "base" => &mut f.base,
            "base-offset" => &mut f.base_offset,
            "move-offset" => &mut f.move_offset,
            "option" => {
                if value.eq_ignore_ascii_case("ignore-sticky-window") {
                    f.ignore_sticky_window = true;
                } else if !value.is_empty() {
                    notes.push(MoveNote::UnknownOption {
                        value: value.to_string(),
                    });
                }
                continue;
            }
            _ => {
                notes.push(MoveNote::UnknownKey {
                    key: key.to_string(),
                });
                continue;
            }
        };
        if value.is_empty() {
            continue;
        }
        if !slot.is_empty() {
            notes.push(MoveNote::DuplicateKey { key: lower });
        }
        *slot = value;
    }
    f
}

/// X・Y: 空か `fix` → `Fix`・`i32` → `Px`・他 → `Fix`＋記録（要件 1.5・1.6・4.5）。
fn read_axis(token: &str, axis: Axis, notes: &mut Vec<MoveNote>) -> AxisSpec {
    if token.is_empty() || token.eq_ignore_ascii_case("fix") {
        return AxisSpec::Fix;
    }
    token.parse::<i32>().map(AxisSpec::Px).unwrap_or_else(|_| {
        notes.push(MoveNote::UnparsableAxis {
            axis,
            token: token.to_string(),
        });
        AxisSpec::Fix
    })
}

/// time: 空 → 0・`u32` → その値（>0 は記録）・他 → 0＋記録（要件 4.6・5.1）。
fn read_duration(token: &str, notes: &mut Vec<MoveNote>) -> u32 {
    if token.is_empty() {
        return 0;
    }
    match token.parse::<u32>() {
        Ok(0) => 0,
        Ok(duration_ms) => {
            notes.push(MoveNote::TimedMoveImmediate { duration_ms });
            duration_ms
        }
        Err(_) => {
            notes.push(MoveNote::UnparsableDuration {
                token: token.to_string(),
            });
            0
        }
    }
}

/// base: 空 → `Screen`・正典の語（大小を問わない）・`u32` → `Scope`・他 → `Scope(0)`＋記録（要件 3.8）。
fn read_base(token: &str, notes: &mut Vec<MoveNote>) -> MoveBase {
    if token.is_empty() {
        return MoveBase::Screen;
    }
    match token.to_ascii_lowercase().as_str() {
        "screen" => MoveBase::Screen,
        "primaryscreen" => MoveBase::PrimaryScreen,
        "me" => MoveBase::Me,
        "global" => MoveBase::Global,
        _ => token
            .parse::<u32>()
            .map(MoveBase::Scope)
            .unwrap_or_else(|_| {
                notes.push(MoveNote::OtherGhostBase {
                    name: token.to_string(),
                });
                MoveBase::Scope(0)
            }),
    }
}

/// 参照点: 空 → `left.top`・`横.縦`・点の無い 1 語は両軸へ（要件 1.3・1.7）。知らない語は軸ごとに
/// 横 `left`・縦 `top`＋記録（要件 4.4）。
fn read_ref_point(token: &str, side: OffsetSide, notes: &mut Vec<MoveNote>) -> RefPoint {
    if token.is_empty() {
        return RefPoint::LEFT_TOP;
    }
    let (xs, ys) = token.split_once('.').unwrap_or((token, token));
    let mut unknown = |axis: Axis, word: &str| {
        notes.push(MoveNote::UnknownRefWord {
            side,
            axis,
            word: word.to_string(),
        })
    };
    let x = RefX::parse(xs).unwrap_or_else(|| {
        unknown(Axis::X, xs);
        RefX::Left
    });
    let y = RefY::parse(ys).unwrap_or_else(|| {
        unknown(Axis::Y, ys);
        RefY::Top
    });
    RefPoint { x, y }
}

#[cfg(test)]
#[path = "move_cue_args_tests.rs"]
mod tests;
