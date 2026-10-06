//! 動く絵の子の鍵（要件 1.12）と、`always` の見分け・経過 0 の求め方（要件 4.8・4.9）。
//!
//! `NestReport::from_world` の振り分けは `_` の腕で「無い番号」に落ちるので、`ElementKind::Film`
//! の扱いを書き漏らしてもコンパイラは教えない。ここを檻にする。

use areka_parsers::shell::{ElementPath, Interval, parse};

use super::{ElementKind, FilmId, PartKey, element_kind, is_always_interval, rest_index};
use crate::normalized::SurfaceMaster;
use crate::world::{EmoWorld, SurfaceIndex};

fn other(s: &str) -> Interval {
    Interval::Other(s.into())
}

/// `always` の単独・小文字の完全一致だけが真（要件 4.8・4.9）。
#[test]
fn only_exact_lowercase_always_is_always() {
    assert!(is_always_interval(&other("always")));
    for s in [
        "bind+always",
        "always+bind",
        "Always",
        "always ",
        " always",
        "",
    ] {
        assert!(!is_always_interval(&other(s)), "{s:?} は always ではない");
    }
    assert!(!is_always_interval(&Interval::Bind));
    assert!(!is_always_interval(&Interval::Random { k: 2 }));
    assert!(!is_always_interval(&Interval::BindRandom { k: 2 }));
}

/// 経過 0 のコマ＝待ち時間の累積が 0 の最後の番号。先頭から待つなら無い。
#[test]
fn rest_index_is_last_index_with_zero_cumulative_wait() {
    assert_eq!(rest_index([0, 0, 100].into_iter()), Some(1));
    assert_eq!(rest_index([0, 100, 0].into_iter()), Some(0));
    assert_eq!(rest_index([0, 0, 0].into_iter()), Some(2));
    assert_eq!(rest_index([100, 0].into_iter()), None);
    assert_eq!(rest_index(std::iter::empty()), None);
}

/// 作者の欄の読み分けは動く絵の子を返さない（作者は子を書けない・要件 1.12）。
#[test]
fn element_kind_never_reads_as_film() {
    for s in ["0", "7", "0100", "4294967296", "body.png", "", "-1"] {
        let kind = element_kind(&ElementPath::new(s.to_string()));
        assert!(!matches!(kind, ElementKind::Film(_)), "{s:?} → {kind:?}");
    }
}

/// 鍵の 2 つの種類は、同じ数でも等しくならない（番号が当たらない・要件 1.12）。
#[test]
fn part_key_kinds_never_collide() {
    assert_ne!(PartKey::Surface(3), PartKey::Film(FilmId(3)));
}

/// `ElementKind::Film` の element は「無い番号」にも「循環」にも数えない（子は先を持たない）。
///
/// 子を置く element を作るのは分解の仕事なので、ここでは読み込んだ面の表の element を直に替える。
/// 番号 0 は面の表に在る（surface0 自身）ので、`Film` が `Surface` の辺に混ざれば循環に、
/// `_` の腕へ落ちれば「無い番号」に数えられて赤になる。
#[test]
fn nest_report_counts_film_as_neither_missing_nor_cycle() {
    let mut world = EmoWorld::build(&parse(
        "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,eye.png,0,0\n}\n",
    ));
    let entity = world.world().resource::<SurfaceIndex>().0[&0];
    world
        .world_mut()
        .get_mut::<SurfaceMaster>(entity)
        .expect("surface0 は在る")
        .elements[1]
        .kind = ElementKind::Film(FilmId(0));
    assert_eq!(world.nest_report().issues, vec![]);
}
