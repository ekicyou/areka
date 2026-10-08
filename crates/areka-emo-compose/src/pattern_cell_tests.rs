//! `PatternState` の欄の 3 つの意味と動く絵の子の欄（task 2.2・要件 1.10, 4.5, 7.4）。
//!
//! 欄の読み `Cell` は「載っていない（経過 0）」「サーフェスを指すコマ」「絵を指すコマ」「消えている」の
//! 4 つ。等しさは足した欄も比べる（`ComposeKey` に自動で入る）。ここでは 4 つの読みが区別されること、
//! 等しさが入れた順に依らないこと、空の欄が「全部が経過 0」と等しいこと、同じ番号の一番上の欄・
//! 部品の欄・子の欄が混ざらないことを固定する。

use super::*;

fn frame(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

const TOP: Option<PartKey> = None;
fn part(n: u32) -> Option<PartKey> {
    Some(PartKey::Surface(n))
}
fn film(n: u32) -> Option<PartKey> {
    Some(PartKey::Film(FilmId(n)))
}

/// 4 つの読みが区別される（一番上の欄）。
#[test]
fn four_readings_are_distinct() {
    let mut s = PatternState::default();
    assert_eq!(s.cell(TOP, 1), Cell::Rest);
    s.set(1, frame(10));
    assert_eq!(s.cell(TOP, 1), Cell::Frame(&frame(10)));
    s.set_blank(1);
    assert_eq!(s.cell(TOP, 1), Cell::Blank);
    // 「消えている」はコマではない（今までの読み口には現れない）。
    assert_eq!(s.get(1), None);
    assert_eq!(s.iter().count(), 0);
    s.set_film(FilmId(7), 3);
    assert_eq!(s.cell(film(7), 0), Cell::Picture(3));
    // `remove` は「載っていない」へ戻す（「消えている」も消える）。
    s.remove(1);
    assert_eq!(s.cell(TOP, 1), Cell::Rest);
    s.remove_film(FilmId(7));
    assert_eq!(s.cell(film(7), 0), Cell::Rest);
    assert!(s.is_empty());
}

/// 「消えている」と「載っていない」は等しさでも区別され、空ではない。
#[test]
fn blank_is_not_rest() {
    let mut top = PatternState::default();
    top.set_blank(2);
    assert_ne!(top, PatternState::default());
    assert!(!top.is_empty());

    let mut p = PatternState::default();
    p.set_part_blank(1400, 2);
    assert_eq!(p.cell(part(1400), 2), Cell::Blank);
    assert_eq!(p.part_get(1400, 2), None);
    assert_eq!(p.part(1400).count(), 0);
    assert_ne!(p, PatternState::default());
    assert!(!p.is_empty());
}

/// コマと「消えている」は同じ欄で入れ替わる（両方が残ることは無い）。
#[test]
fn frame_and_blank_replace_each_other() {
    let mut a = PatternState::default();
    a.set_blank(1);
    a.set(1, frame(10));
    let mut b = PatternState::default();
    b.set(1, frame(10));
    assert_eq!(a, b);

    let mut c = PatternState::default();
    c.set_part(1400, 0, frame(10));
    c.set_part_blank(1400, 0);
    let mut d = PatternState::default();
    d.set_part_blank(1400, 0);
    assert_eq!(c, d);
}

/// 入れた順に依らず等しい（全部の欄を使う）。
#[test]
fn eq_is_insertion_order_stable_over_all_cells() {
    let mut a = PatternState::default();
    a.set(1, frame(10));
    a.set_blank(2);
    a.set_part(1400, 0, frame(20));
    a.set_part_blank(1400, 1);
    a.set_film(FilmId(5), 1);
    a.set_film(FilmId(3), 2);

    let mut b = PatternState::default();
    b.set_film(FilmId(3), 2);
    b.set_part_blank(1400, 1);
    b.set_film(FilmId(5), 1);
    b.set_blank(2);
    b.set_part(1400, 0, frame(20));
    b.set(1, frame(10));

    assert_eq!(a, b);
    assert_eq!(a.cells().collect::<Vec<_>>(), b.cells().collect::<Vec<_>>());
}

/// 子の欄だけが違えば等しくない（合成の鍵が分かれる）。
#[test]
fn film_cell_enters_equality() {
    let mut a = PatternState::default();
    a.set_film(FilmId(1), 1);
    let mut b = PatternState::default();
    b.set_film(FilmId(1), 2);
    assert_ne!(a, b);
    assert_ne!(a, PatternState::default());
}

/// 空の欄は「全部が経過 0」と等しい: 置いて外した後の状態は既定値と等しい（空の内側の表を持たない）。
#[test]
fn emptied_state_equals_default() {
    let mut s = PatternState::default();
    s.set_blank(1);
    s.set_part_blank(1400, 0);
    s.set_part(1401, 0, frame(1));
    s.set_film(FilmId(9), 4);
    s.remove(1);
    s.clear_parts();
    assert_eq!(s, PatternState::default());
    assert!(s.is_empty());
    for key in [TOP, part(1400), part(1401), film(9)] {
        assert_eq!(s.cell(key, 0), Cell::Rest);
    }
}

/// `clear_parts` は部品のコマ・部品の「消えている」・子の欄を消し、一番上の欄は残す。
#[test]
fn clear_parts_keeps_top_only() {
    let mut s = PatternState::default();
    s.set(1, frame(10));
    s.set_blank(2);
    s.set_part(1400, 0, frame(20));
    s.set_part_blank(1400, 1);
    s.set_film(FilmId(5), 1);
    s.clear_parts();

    let mut top = PatternState::default();
    top.set(1, frame(10));
    top.set_blank(2);
    assert_eq!(s, top);
}

/// 同じ番号の一番上の欄・部品の欄・子の欄は混ざらない。
#[test]
fn same_number_in_top_part_and_film_never_mix() {
    let mut s = PatternState::default();
    s.set(0, frame(100));
    s.set_part_blank(0, 0);
    s.set_film(FilmId(0), 7);

    assert_eq!(s.cell(TOP, 0), Cell::Frame(&frame(100)));
    assert_eq!(s.cell(part(0), 0), Cell::Blank);
    assert_eq!(s.cell(film(0), 0), Cell::Picture(7));

    s.remove_film(FilmId(0));
    assert_eq!(s.cell(TOP, 0), Cell::Frame(&frame(100)));
    assert_eq!(s.cell(part(0), 0), Cell::Blank);
    assert_eq!(s.cell(film(0), 0), Cell::Rest);

    // 子の欄は animation 0 だけ（他の番号は載っていない）。
    s.set_film(FilmId(0), 7);
    assert_eq!(s.cell(film(0), 1), Cell::Rest);
}

/// 走査の口は「載っていない」以外の欄を、一番上 → 部品 → 子、各々番号の昇順で返す。
#[test]
fn cells_scans_every_non_rest_cell_in_canonical_order() {
    let mut s = PatternState::default();
    s.set_film(FilmId(2), 9);
    s.set_part_blank(1400, 1);
    s.set_part(1400, 0, frame(20));
    s.set_blank(3);
    s.set(1, frame(10));
    let f10 = frame(10);
    let f20 = frame(20);
    assert_eq!(
        s.cells().collect::<Vec<_>>(),
        vec![
            (TOP, 1, Cell::Frame(&f10)),
            (TOP, 3, Cell::Blank),
            (part(1400), 0, Cell::Frame(&f20)),
            (part(1400), 1, Cell::Blank),
            (film(2), 0, Cell::Picture(9)),
        ]
    );
    assert_eq!(PatternState::default().cells().count(), 0);
}
