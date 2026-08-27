//! 追従 Component の書込 4 本の契約のテスト
//! （areka-P0-balloon-offset-dpi・design D14／要件 1.1・3.3・3.5・5.2・9.1）。
//!
//! 純関数 [`rescale_follow_offset`] の判断分岐は兄弟ファイル
//! `follow_offset_space_tests.rs` が全網羅する。本ファイルが固定するのは
//! **状態の側**——「どの書込が基準対を置き換え、どれが置き換えないか」だけである。
//!
//! | 種別 | メソッド | 基準値 | 基準 DPI |
//! |---|---|---|---|
//! | 確立 | `new` | 与えた値 | 与えた値 |
//! | 確立 | `reestablish` | **置き換わる** | **その時の表示 DPI へ係留** |
//! | 追随 | `anchor_base_dpi` | 不変 | 未係留 → 係留 |
//! | 追随 | `apply_rescaled` | **不変** | **不変** |
//!
//! 見出しの受入条件「定義モジュールの外から私有欄へ直接代入するとコンパイルエラーになる」
//! は**コンパイル時の性質**であり、通常の `#[test]` では表現できない（書けば
//! コンパイルが通らず、テストが 1 本も走らない）。本ファイルはその代わりに、
//! 私有化が守ろうとしている**不変量そのもの**——「基準対を置き換える書込と、
//! 置き換えない書込が混ざらない」——を逐語で固定する。

use bevy_ecs::prelude::*;
use wintf::ecs::DPI;

use super::offset_space::{BalloonFollow, OffsetBase};
use crate::placement::resolver::PointPx;

fn dpi(v: u16) -> DPI {
    DPI::from_dpi(v, v)
}

fn pt(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

/// 檻の中だけで使う entity（Component の欄としての意味しか持たない）。
fn some_entity() -> Entity {
    let mut world = World::new();
    world.spawn_empty().id()
}

#[test]
fn new_takes_the_base_pair_and_starts_at_the_base_value() {
    let e = some_entity();
    let base = OffsetBase {
        offset: pt(11, 22),
        dpi: Some(dpi(120)),
    };
    let follow = BalloonFollow::new(e, base);

    assert_eq!(follow.balloon, e);
    assert_eq!(follow.offset(), pt(11, 22));
    assert_eq!(follow.base(), base);
}

#[test]
fn new_with_an_unpinned_base_keeps_the_base_dpi_none() {
    let follow = BalloonFollow::new(some_entity(), OffsetBase::unpinned(pt(-5, 7)));

    assert_eq!(follow.offset(), pt(-5, 7));
    assert_eq!(follow.base().offset, pt(-5, 7));
    assert_eq!(follow.base().dpi, None);
}

#[test]
fn reestablish_stamps_the_current_display_dpi_as_the_new_base() {
    let mut follow = BalloonFollow::new(
        some_entity(),
        OffsetBase {
            offset: pt(10, 20),
            dpi: Some(dpi(96)),
        },
    );

    follow.reestablish(pt(33, -44), dpi(192));

    assert_eq!(follow.offset(), pt(33, -44));
    assert_eq!(
        follow.base(),
        OffsetBase {
            offset: pt(33, -44),
            dpi: Some(dpi(192)),
        },
        "確立点は現在値と基準値の双方を新しい値で焼き直し、基準 DPI をその時の表示 DPI へ係留する"
    );
}

#[test]
fn reestablish_pins_a_previously_unpinned_base() {
    let mut follow = BalloonFollow::new(some_entity(), OffsetBase::unpinned(pt(1, 2)));

    follow.reestablish(pt(3, 4), dpi(144));

    assert_eq!(follow.base().dpi, Some(dpi(144)));
}

#[test]
fn anchor_base_dpi_pins_without_moving_the_value() {
    let mut follow = BalloonFollow::new(some_entity(), OffsetBase::unpinned(pt(-120, 45)));

    follow.anchor_base_dpi(dpi(120));

    assert_eq!(
        follow.base().offset,
        pt(-120, 45),
        "係留は基準値を 1 bit も動かさない（要件 5.2）"
    );
    assert_eq!(follow.base().dpi, Some(dpi(120)));
    assert_eq!(
        follow.offset(),
        pt(-120, 45),
        "係留は現在値も動かさない（永続値の腕はそのまま採用される）"
    );
}

#[test]
fn apply_rescaled_moves_the_value_but_never_the_base() {
    let base = OffsetBase {
        offset: pt(100, -50),
        dpi: Some(dpi(96)),
    };
    let mut follow = BalloonFollow::new(some_entity(), base);

    follow.apply_rescaled(pt(200, -100));

    assert_eq!(follow.offset(), pt(200, -100));
    assert_eq!(
        follow.base(),
        base,
        "追随は基準対を変えない——出力を入力へ戻さないので誤差が連鎖しない（要件 3.1）"
    );
}

#[test]
fn repeated_apply_rescaled_always_derives_from_the_same_base() {
    let base = OffsetBase {
        offset: pt(64, 32),
        dpi: Some(dpi(96)),
    };
    let mut follow = BalloonFollow::new(some_entity(), base);

    follow.apply_rescaled(pt(128, 64));
    follow.apply_rescaled(pt(80, 40));
    follow.apply_rescaled(pt(64, 32));

    assert_eq!(follow.base(), base);
    assert_eq!(
        follow.offset(),
        pt(64, 32),
        "往復して元の値へ戻れば現在値も元へ戻る（基準が動かないため）"
    );
}

#[test]
fn apply_rescaled_after_reestablish_derives_from_the_reestablished_base() {
    let mut follow = BalloonFollow::new(
        some_entity(),
        OffsetBase {
            offset: pt(10, 10),
            dpi: Some(dpi(96)),
        },
    );

    follow.reestablish(pt(70, 70), dpi(120));
    follow.apply_rescaled(pt(140, 140));

    assert_eq!(
        follow.base(),
        OffsetBase {
            offset: pt(70, 70),
            dpi: Some(dpi(120)),
        },
        "ドラッグ由来の基準が確立したあとの追随は、その新しい基準を起点にする（要件 3.5）"
    );
}

#[test]
fn the_follow_target_entity_survives_every_write() {
    let e = some_entity();
    let mut follow = BalloonFollow::new(e, OffsetBase::unpinned(pt(0, 0)));

    follow.reestablish(pt(1, 1), dpi(96));
    follow.anchor_base_dpi(dpi(120));
    follow.apply_rescaled(pt(2, 2));

    assert_eq!(follow.balloon, e, "追従先は書込 4 本のいずれでも変わらない");
}
