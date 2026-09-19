//! 登記の口（[`MenuRegistry`]）の決定論テスト（要件 6.2・6.3・6.4）。
//!
//! 確かめること: 置き換えが記録されること・取り消した枠が写しから消えること・写しが登記順
//! ではなく枠の並び順になること・供給関数が写しを取るたびに呼ばれること。

use std::cell::Cell;

use log_capture_kit::{LineFormat, capture_lines};

use super::*;

/// 何もしない動作を持つ、有効な葉の項目。
fn leaf(label: impl Into<String>) -> MenuItem {
    MenuItem {
        label: label.into(),
        caption_resource: None,
        enabled: true,
        checked: None,
        body: ItemBody::Action(Rc::new(|_, _| {})),
    }
}

/// 毎回同じ名前の項目を返す供給関数。
fn fixed(label: &'static str) -> Supplier {
    Rc::new(move |_, _| leaf(label))
}

/// 写しを「枠と既定名」の列へ縮める（`MenuItem` は閉包を持つので比較できない）。
fn labels(registry: &MenuRegistry) -> Vec<(Frame, String)> {
    registry
        .snapshot(&World::new(), &MenuContext { scope: 0 })
        .into_iter()
        .map(|(frame, item)| (frame, item.label))
        .collect()
}

/// 登記の置き換えを記録した行だけを拾う。
fn replaced_lines(lines: &[String]) -> Vec<&String> {
    lines
        .iter()
        .filter(|l| l.contains("menu_registration_replaced"))
        .collect()
}

/// 並びを決める定数と登記の添字が食い違わない: 枠は 7 つ、①〜⑦の順、判別値が添字に一致する。
#[test]
fn frame_order_is_the_seven_frames_and_matches_slot_indices() {
    assert_eq!(
        Frame::ORDER,
        [
            Frame::Ghost,
            Frame::Shell,
            Frame::Balloon,
            Frame::Update,
            Frame::Install,
            Frame::Readme,
            Frame::Close,
        ],
        "並びは ①ゴースト ②シェル ③バルーン ④ネットワーク更新 ⑤インストール ⑥説明書 ⑦終了（要件 2.1）"
    );
    assert_eq!(
        MenuRegistry::default().slots.len(),
        Frame::ORDER.len(),
        "登記の席の数は枠の数と同じ"
    );
    for (i, frame) in Frame::ORDER.iter().enumerate() {
        assert_eq!(*frame as usize, i, "{frame:?} の判別値が並びの位置と違う");
    }
}

/// 何も登記していなければ写しは空（未登記の枠は現れない）。
#[test]
fn snapshot_of_empty_registry_is_empty() {
    assert!(labels(&MenuRegistry::default()).is_empty());
}

/// 写しは登記順ではなく枠の並び順。登記していない枠は現れない。
#[test]
fn snapshot_follows_frame_order_not_registration_order() {
    let mut registry = MenuRegistry::default();
    registry.register(Frame::Close, fixed("終了"));
    registry.register(Frame::Ghost, fixed("ゴースト"));
    registry.register(Frame::Readme, fixed("説明書"));
    registry.register(Frame::Shell, fixed("シェル"));

    assert_eq!(
        labels(&registry),
        [
            (Frame::Ghost, "ゴースト".to_string()),
            (Frame::Shell, "シェル".to_string()),
            (Frame::Readme, "説明書".to_string()),
            (Frame::Close, "終了".to_string()),
        ]
    );
}

/// 供給関数は登記のときには呼ばれず、写しを取るたびに呼ばれる（値を使い回さない・要件 6.2）。
/// 文脈（スコープ番号）もそのまま渡る。
#[test]
fn snapshot_invokes_supplier_every_time_with_the_given_context() {
    let calls = Rc::new(Cell::new(0u32));
    let seen = Rc::clone(&calls);
    let mut registry = MenuRegistry::default();
    registry.register(
        Frame::Ghost,
        Rc::new(move |_, ctx| {
            seen.set(seen.get() + 1);
            leaf(format!("呼出{}・scope{}", seen.get(), ctx.scope))
        }),
    );
    assert_eq!(calls.get(), 0, "登記しただけでは供給関数を呼ばない");

    let world = World::new();
    let first = registry.snapshot(&world, &MenuContext { scope: 0 });
    let second = registry.snapshot(&world, &MenuContext { scope: 1 });

    assert_eq!(calls.get(), 2, "写しを取るたびに 1 回ずつ呼ぶ");
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].1.label, "呼出1・scope0");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].1.label, "呼出2・scope1");
}

/// 同じ枠への 2 度目の登記は後勝ちで置き換わり、`warn!` で 1 回記録される（要件 6.3）。
/// 最初の登記と、別の枠への登記は記録されない。
#[test]
fn second_registration_replaces_and_warns_once() {
    let mut registry = MenuRegistry::default();

    let ((), first) = capture_lines(LineFormat::LevelFields, || {
        registry.register(Frame::Shell, fixed("先"));
        registry.register(Frame::Balloon, fixed("別の枠"));
    });
    assert!(
        replaced_lines(&first).is_empty(),
        "空の枠への登記は置き換えではない: {first:?}"
    );

    let ((), second) = capture_lines(LineFormat::LevelFields, || {
        registry.register(Frame::Shell, fixed("後"));
    });
    let replaced = replaced_lines(&second);
    assert_eq!(replaced.len(), 1, "置き換えの記録は 1 回: {second:?}");
    let line = replaced[0];
    assert!(line.contains("level=WARN"), "置き換えは warn: {line}");
    assert!(line.contains("frame=Shell"), "どの枠かを載せる: {line}");
    assert!(line.contains("[menu]"), "接頭辞 [menu] を付ける: {line}");

    assert_eq!(
        labels(&registry),
        [
            (Frame::Shell, "後".to_string()),
            (Frame::Balloon, "別の枠".to_string()),
        ],
        "後から登記したものだけが残る（枠 1 つに供給関数は 1 つ）"
    );
}

/// 取り消した枠は写しに現れない（要件 6.4）。取り消した後の登記は置き換えではない。
#[test]
fn unregister_removes_the_frame_and_frees_the_slot() {
    let mut registry = MenuRegistry::default();
    registry.register(Frame::Install, fixed("インストール"));
    registry.register(Frame::Close, fixed("終了"));
    assert_eq!(labels(&registry).len(), 2, "取り消す前は 2 枠とも現れる");

    registry.unregister(Frame::Install);
    assert_eq!(labels(&registry), [(Frame::Close, "終了".to_string())]);

    // 登記の無い枠の取り消しは何も起こさない。
    registry.unregister(Frame::Install);
    registry.unregister(Frame::Ghost);
    assert_eq!(labels(&registry), [(Frame::Close, "終了".to_string())]);

    let ((), lines) = capture_lines(LineFormat::LevelFields, || {
        registry.register(Frame::Install, fixed("再登記"));
    });
    assert!(
        replaced_lines(&lines).is_empty(),
        "取り消した枠は空なので、次の登記は置き換えとして記録されない: {lines:?}"
    );
    assert_eq!(
        labels(&registry),
        [
            (Frame::Install, "再登記".to_string()),
            (Frame::Close, "終了".to_string()),
        ]
    );
}
