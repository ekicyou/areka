//! 表のコマの欄の単体テスト（spec: areka-P0-animated-image-decode 要件 2.5・4.2〜4.6）。
//!
//! `with_frames` で組んだ表が、親の鍵で 0 番のコマを返し、親の番号でだけコマの並びを
//! 答えることを、手組みの小さな表で確かめる。ファイルにも検体にも依存しない。
use super::*;
use std::num::NonZeroU32;

fn key(rel_path: &str) -> AtlasKey {
    AtlasKey {
        set: SetId(0),
        rel_path: rel_path.into(),
    }
}

/// 原寸の幅でエントリを見分ける（配置は持たない）。
fn entry(w: u32) -> AtlasEntry {
    AtlasEntry {
        original: Size { w, h: 1 },
        placement: None,
    }
}

fn three_frames() -> Animation {
    Animation {
        frames: vec![ElementId(1), ElementId(2), ElementId(3)],
        delays_ms: vec![100, 50, 0],
        loop_count: LoopCount::Finite(NonZeroU32::new(2).unwrap()),
    }
}

/// 0 番＝静止画・1 番＝動く絵の親（0 番のコマ）・2, 3 番＝後ろに足した 1, 2 番のコマ。
fn frames_table(animation: Animation) -> AtlasTable {
    AtlasTable::with_frames(
        vec![
            key("still.png"),
            key("anim.png"),
            key("anim.png"),
            key("anim.png"),
        ],
        vec![entry(10), entry(20), entry(21), entry(22)],
        vec![],
        vec![(ElementId(1), animation)],
    )
}

/// 4.3: 親の鍵はいちばん小さい番号（0 番のコマ）を返す。静止画の番号は今と同じ。
#[test]
fn parent_key_resolves_to_frame_zero() {
    let t = frames_table(three_frames());
    let id = t.resolve(SetId(0), "anim.png").unwrap();
    assert_eq!(id, ElementId(1));
    assert_eq!(t.entry(id).original.w, 20);
    assert_eq!(t.resolve(SetId(0), "still.png"), Some(ElementId(0)));
    assert_eq!(t.len(), 4);
}

/// 4.2/4.5/2.5: 親の番号でコマの並び・待ち時間・繰り返し回数が答えられる。
#[test]
fn animation_is_some_for_parent() {
    let t = frames_table(three_frames());
    let a = t.animation(ElementId(1)).expect("parent has animation");
    assert_eq!(a.frames, vec![ElementId(1), ElementId(2), ElementId(3)]);
    assert_eq!(a.delays_ms, vec![100, 50, 0]);
    assert_eq!(a.loop_count, LoopCount::Finite(NonZeroU32::new(2).unwrap()));
    // 4.4: 各コマは普通のエントリとして引ける。
    assert_eq!(t.entry(a.frames[2]).original.w, 22);
}

/// 4.6: 静止画と 2 枚目以降のコマの番号では無し。
#[test]
fn animation_is_none_for_still_and_later_frames() {
    let t = frames_table(three_frames());
    assert!(t.animation(ElementId(0)).is_none());
    assert!(t.animation(ElementId(2)).is_none());
    assert!(t.animation(ElementId(3)).is_none());
}

/// 2 枚目以降のコマの鍵は親の鍵（作り物のパスは作らない）。
#[test]
fn later_frame_keys_are_parent_key() {
    let t = frames_table(three_frames());
    assert_eq!(t.key(ElementId(2)), &key("anim.png"));
    assert_eq!(t.key(ElementId(3)), &key("anim.png"));
}

/// 今の組み立て口で作った表は、動く絵が全部無し。
#[test]
fn new_table_has_no_animations() {
    let t = AtlasTable::new(
        vec![key("a.png"), key("b.png")],
        vec![entry(1), entry(2)],
        vec![],
    );
    assert!(t.animation(ElementId(0)).is_none());
    assert!(t.animation(ElementId(1)).is_none());
}

/// 繰り返し回数「終わりなし」もそのまま渡る。
#[test]
fn infinite_loop_count_round_trips() {
    let mut a = three_frames();
    a.loop_count = LoopCount::Infinite;
    let t = frames_table(a);
    assert_eq!(
        t.animation(ElementId(1)).unwrap().loop_count,
        LoopCount::Infinite
    );
}

#[test]
#[should_panic]
fn panics_when_keys_and_entries_differ() {
    AtlasTable::with_frames(vec![key("a.png")], vec![], vec![], vec![]);
}

#[test]
#[should_panic]
fn panics_when_single_frame() {
    let mut a = three_frames();
    a.frames.truncate(1);
    a.delays_ms.truncate(1);
    frames_table(a);
}

#[test]
#[should_panic]
fn panics_when_delays_length_differs() {
    let mut a = three_frames();
    a.delays_ms.pop();
    frames_table(a);
}

#[test]
#[should_panic]
fn panics_when_frame_outside_table() {
    let mut a = three_frames();
    a.frames[2] = ElementId(4);
    frames_table(a);
}
