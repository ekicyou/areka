// =============================================================================
// `\![move]` の引数の読みの決定論檻（design.md Testing Strategy「move_cue_args_tests.rs」・
// 要件 1.1〜1.13・3.8・4.4〜4.6・5.1）
// =============================================================================

use super::*;
use crate::emo2_boot::move_cue::{Axis, AxisSpec, MoveBase, MoveDirective, RefPoint, RefX, RefY};

fn toks(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn parse(scope: u32, items: &[&str]) -> ParsedMove {
    parse_move_directive(scope, &toks(items))
}

/// 既定（要件 1.3）: X・Y は今の座標・time 0・基準 screen・両参照点 left.top・option なし。
fn defaults(scope: u32) -> MoveDirective {
    MoveDirective {
        scope,
        x: AxisSpec::Fix,
        y: AxisSpec::Fix,
        duration_ms: 0,
        base: MoveBase::Screen,
        base_offset: RefPoint::LEFT_TOP,
        move_offset: RefPoint::LEFT_TOP,
        ignore_sticky_window: false,
    }
}

const fn rp(x: RefX, y: RefY) -> RefPoint {
    RefPoint { x, y }
}

/// 要件 1.1・1.4: 名前付きの形と旧い位置の形で同じ値を書くと同じ指令になり、記録は 0 件。
#[test]
fn named_and_positional_read_to_same_directive() {
    let named = parse(
        1,
        &[
            "--X=10",
            "--Y=-20",
            "--time=0",
            "--base=0",
            "--base-offset=right.bottom",
            "--move-offset=center.base",
        ],
    );
    let positional = parse(1, &["10", "-20", "0", "0", "right.bottom", "center.base"]);
    assert_eq!(named.notes, vec![]);
    assert_eq!(positional.notes, vec![]);
    assert_eq!(named.directive, positional.directive);
    assert_eq!(
        named.directive,
        MoveDirective {
            x: AxisSpec::Px(10),
            y: AxisSpec::Px(-20),
            base: MoveBase::Scope(0),
            base_offset: rp(RefX::Right, RefY::Bottom),
            move_offset: rp(RefX::Center, RefY::Base),
            ..defaults(1)
        }
    );
}

/// 要件 1.2: キーの大小・並び順を問わない。
#[test]
fn named_keys_are_case_and_order_insensitive() {
    let a = parse(
        0,
        &[
            "--X=10",
            "--Y=-20",
            "--BASE=Me",
            "--Base-Offset=Left.Bottom",
        ],
    );
    let b = parse(
        0,
        &[
            "--base-offset=left.bottom",
            "--base=me",
            "--y=-20",
            "--x=10",
        ],
    );
    assert_eq!(a.notes, vec![]);
    assert_eq!(b.notes, vec![]);
    assert_eq!(a.directive, b.directive);
    assert_eq!(a.directive.base, MoveBase::Me);
    assert_eq!(a.directive.base_offset, rp(RefX::Left, RefY::Bottom));
}

/// 要件 1.3・1.4: どのキー・欄も省略でき、空の値は省略と同じ既定へ落ちる（両方の形）。
#[test]
fn omitted_and_empty_fields_take_the_same_defaults() {
    for items in [
        &[][..],
        &["", "", "", "", "", ""][..],
        &[
            "--X=",
            "--Y=",
            "--time=",
            "--base=",
            "--base-offset=",
            "--move-offset=",
            "--option=",
        ][..],
    ] {
        let p = parse(2, items);
        assert_eq!(p.directive, defaults(2), "入力 {items:?}");
        assert_eq!(p.notes, vec![], "入力 {items:?}");
    }
}

/// 要件 1.5・1.6: `fix`（大小を問わない）は今の座標を保ち、負の整数を受ける（両方の形）。
#[test]
fn fix_and_negative_numbers_in_both_forms() {
    let positional = parse(0, &["-353", "FIX"]);
    let named = parse(0, &["--X=-353", "--Y=Fix"]);
    for p in [positional, named] {
        assert_eq!(p.directive.x, AxisSpec::Px(-353));
        assert_eq!(p.directive.y, AxisSpec::Fix);
        assert_eq!(p.notes, vec![]);
    }
}

/// 要件 1.7: 点の無い 1 語は横と縦の両方へ当たる（両方の形・`base` は `base.base`）。
#[test]
fn bare_word_applies_to_both_axes() {
    let positional = parse(0, &["", "", "", "0", "base", "center"]);
    let named = parse(
        0,
        &["--base=0", "--base-offset=BASE", "--move-offset=center"],
    );
    for p in [positional, named] {
        assert_eq!(p.directive.base_offset, RefPoint::BASE_BASE);
        assert_eq!(p.directive.move_offset, rp(RefX::Center, RefY::Center));
        assert_eq!(p.notes, vec![]);
    }
}

/// 要件 1.8・1.9・1.10・1.11: `--option` の繰り返しは全部受け、`ignore-sticky-window` は印を立て、
/// 知らない値は記録し、重複とは数えない。
#[test]
fn option_repeats_are_all_read() {
    let p = parse(
        0,
        &[
            "--option=foo",
            "--OPTION=Ignore-Sticky-Window",
            "--option=bar",
        ],
    );
    assert!(p.directive.ignore_sticky_window);
    assert_eq!(
        p.notes,
        vec![
            MoveNote::UnknownOption {
                value: "foo".to_string()
            },
            MoveNote::UnknownOption {
                value: "bar".to_string()
            },
        ]
    );
    // 印の無い指令は false のまま。
    assert!(!parse(0, &["--option=foo"]).directive.ignore_sticky_window);
}

/// 要件 1.10: 知らないキー・`=` の無い引数・`--` で始まらない引数はその部分だけ無視して記録し、
/// 残りで指令を作る。
#[test]
fn ignored_parts_are_recorded_and_the_rest_is_read() {
    let p = parse(1, &["--colour=red", "--X", "plain", "--X=5", "--=7", "--"]);
    assert_eq!(
        p.directive,
        MoveDirective {
            x: AxisSpec::Px(5),
            ..defaults(1)
        }
    );
    assert_eq!(
        p.notes,
        vec![
            MoveNote::UnknownKey {
                key: "colour".to_string()
            },
            MoveNote::MalformedArg {
                arg: "--X".to_string()
            },
            MoveNote::MalformedArg {
                arg: "plain".to_string()
            },
            MoveNote::UnknownKey { key: String::new() },
            MoveNote::MalformedArg {
                arg: "--".to_string()
            },
        ]
    );
}

/// 要件 1.11: `option` 以外の同じキーは後勝ちで、重複を記録する（SSP 実測 M4-j と同じ値）。
#[test]
fn duplicate_key_last_wins_and_is_recorded() {
    let p = parse(1, &["--X=10", "--x=20", "--Y=fix", "--base=me"]);
    assert_eq!(p.directive.x, AxisSpec::Px(20));
    assert_eq!(p.directive.y, AxisSpec::Fix);
    assert_eq!(p.directive.base, MoveBase::Me);
    assert_eq!(
        p.notes,
        vec![MoveNote::DuplicateKey {
            key: "x".to_string()
        }]
    );
}

/// 要件 1.12: 旧い形の欄の並びは今のままで、7 欄目以降は読まない（記録もしない）。
#[test]
fn positional_reads_six_fields_and_ignores_the_rest() {
    let p = parse(
        0,
        &[
            "1",
            "2",
            "30",
            "1",
            "right.top",
            "left.bottom",
            "garbage",
            "x",
        ],
    );
    assert_eq!(
        p.directive,
        MoveDirective {
            x: AxisSpec::Px(1),
            y: AxisSpec::Px(2),
            duration_ms: 30,
            base: MoveBase::Scope(1),
            base_offset: rp(RefX::Right, RefY::Top),
            move_offset: rp(RefX::Left, RefY::Bottom),
            ..defaults(0)
        }
    );
    assert_eq!(
        p.notes,
        vec![MoveNote::TimedMoveImmediate { duration_ms: 30 }]
    );
}

/// 要件 1.13: `--` で始まる引数が 1 つでもあれば名前付きの形、`-353` だけなら旧い形。
#[test]
fn form_is_named_iff_some_arg_starts_with_double_dash() {
    let positional = parse(0, &["-353", "", "", "screen"]);
    assert_eq!(positional.directive.x, AxisSpec::Px(-353));
    assert_eq!(positional.notes, vec![]);

    // 1 つでも `--` があれば名前付き。`-353` は `--キー=値` の形でないので無視して記録する。
    let mixed = parse(0, &["-353", "--base=screen"]);
    assert_eq!(mixed.directive.x, AxisSpec::Fix);
    assert_eq!(
        mixed.notes,
        vec![MoveNote::MalformedArg {
            arg: "-353".to_string()
        }]
    );
}

/// 要件 4.5: 読めない X・Y は `fix` へ置き換えて記録する（両方の形）。
#[test]
fn unparsable_axis_falls_back_to_fix() {
    let positional = parse(0, &["abc", "1.5"]);
    let named = parse(0, &["--X=abc", "--Y=1.5"]);
    for p in [positional, named] {
        assert_eq!(p.directive.x, AxisSpec::Fix);
        assert_eq!(p.directive.y, AxisSpec::Fix);
        assert_eq!(
            p.notes,
            vec![
                MoveNote::UnparsableAxis {
                    axis: Axis::X,
                    token: "abc".to_string()
                },
                MoveNote::UnparsableAxis {
                    axis: Axis::Y,
                    token: "1.5".to_string()
                },
            ]
        );
    }
}

/// 要件 4.6・5.1: 読めない time は 0 へ置き換えて記録し、time>0 は値を保って記録する。
#[test]
fn duration_unparsable_is_zero_and_positive_is_recorded() {
    for p in [parse(0, &["", "", "-5"]), parse(0, &["--time=soon"])] {
        assert_eq!(p.directive.duration_ms, 0);
        assert!(
            matches!(&p.notes[..], [MoveNote::UnparsableDuration { .. }]),
            "{:?}",
            p.notes
        );
    }
    for p in [parse(0, &["", "", "2500"]), parse(0, &["--time=2500"])] {
        assert_eq!(p.directive.duration_ms, 2500);
        assert_eq!(
            p.notes,
            vec![MoveNote::TimedMoveImmediate { duration_ms: 2500 }]
        );
    }
}

/// 基準の語（大小を問わない）と数のスコープ。要件 3.8: 他の語（他のゴーストの名前）はスコープ 0
/// へ置き換えて記録する。
#[test]
fn base_vocabulary_and_other_ghost_name() {
    for (word, expected) in [
        ("screen", MoveBase::Screen),
        ("PrimaryScreen", MoveBase::PrimaryScreen),
        ("GLOBAL", MoveBase::Global),
        ("me", MoveBase::Me),
        ("0", MoveBase::Scope(0)),
        ("2", MoveBase::Scope(2)),
    ] {
        for p in [
            parse(0, &["", "", "", word]),
            parse(0, &[&format!("--base={word}")]),
        ] {
            assert_eq!(p.directive.base, expected, "基準 {word}");
            assert_eq!(p.notes, vec![], "基準 {word}");
        }
    }
    for p in [
        parse(0, &["", "", "", "さくら"]),
        parse(0, &["--base=さくら"]),
    ] {
        assert_eq!(p.directive.base, MoveBase::Scope(0));
        assert_eq!(
            p.notes,
            vec![MoveNote::OtherGhostBase {
                name: "さくら".to_string()
            }]
        );
    }
}

/// 要件 4.4: 知らない参照点の語は軸ごとに横 `left`・縦 `top` へ置き換えて記録する。
#[test]
fn unknown_ref_words_fall_back_per_axis() {
    let p = parse(0, &["--base-offset=foo.bottom", "--move-offset=right.bar"]);
    assert_eq!(p.directive.base_offset, rp(RefX::Left, RefY::Bottom));
    assert_eq!(p.directive.move_offset, rp(RefX::Right, RefY::Top));
    assert_eq!(
        p.notes,
        vec![
            MoveNote::UnknownRefWord {
                side: OffsetSide::Base,
                axis: Axis::X,
                word: "foo".to_string()
            },
            MoveNote::UnknownRefWord {
                side: OffsetSide::Move,
                axis: Axis::Y,
                word: "bar".to_string()
            },
        ]
    );

    // 点の無い 1 語は両軸へ当たるので、縦にしか無い語は横で、横にしか無い語は縦で置き換わる。
    let bare = parse(0, &["", "", "", "0", "bottom", "right"]);
    assert_eq!(bare.directive.base_offset, rp(RefX::Left, RefY::Bottom));
    assert_eq!(bare.directive.move_offset, rp(RefX::Right, RefY::Top));
    assert_eq!(
        bare.notes,
        vec![
            MoveNote::UnknownRefWord {
                side: OffsetSide::Base,
                axis: Axis::X,
                word: "bottom".to_string()
            },
            MoveNote::UnknownRefWord {
                side: OffsetSide::Move,
                axis: Axis::Y,
                word: "right".to_string()
            },
        ]
    );
}

/// emo2 の 2 つの台本（2026-10-17 版の位置調整と旧い `-353,,,0,base,base`）は記録 0 件で読める。
#[test]
fn emo2_scripts_read_without_notes() {
    let new = parse(
        1,
        &[
            "--X=0",
            "--Y=0",
            "--base=0",
            "--base-offset=left.bottom",
            "--move-offset=right.bottom",
        ],
    );
    assert_eq!(new.notes, vec![]);
    assert_eq!(
        new.directive,
        MoveDirective {
            x: AxisSpec::Px(0),
            y: AxisSpec::Px(0),
            base: MoveBase::Scope(0),
            base_offset: rp(RefX::Left, RefY::Bottom),
            move_offset: rp(RefX::Right, RefY::Bottom),
            ..defaults(1)
        }
    );

    let old = parse(1, &["-353", "", "", "0", "base", "base"]);
    assert_eq!(old.notes, vec![]);
    assert_eq!(
        old.directive,
        MoveDirective {
            x: AxisSpec::Px(-353),
            base: MoveBase::Scope(0),
            base_offset: RefPoint::BASE_BASE,
            move_offset: RefPoint::BASE_BASE,
            ..defaults(1)
        }
    );
}

/// 同じ入力には同じ出力（決定論）。
#[test]
fn parse_is_deterministic() {
    let items = ["--X=1", "--bogus=2", "--X=3", "--base-offset=zz"];
    assert_eq!(parse(0, &items), parse(0, &items));
}
