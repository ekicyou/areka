use areka_sakura::contract::{ActorKey, CueCommand};
use dola::time::testing::ms;

use super::test_support::{
    choice_cue, com_world, count_px, cue, geo_model, inked, opaque_count, read_back, row_px,
    spawn_reserved_slot,
};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::state::{SpanKind, TextLayerConfig};

// ══ Clear/ClearAll の原子的無効化（hover リセット＋ヒット行スナップショット無効化） ══

/// `apply_cue(Clear)` は当該 actor の hover を None へ戻し、ヒット行スナップショットを
/// 選択肢の消去と**原子的**に無効化する。present を待たず `choice_hit_rows` が空・
/// `choice_active` が false に同時に揃い、表示と hit の片方だけが古い状態に残らない。
/// 続く新しい選択肢は古い hover を引き継がず、ハイライト無しで描画される。
#[test]
fn clear_resets_hover_and_invalidates_hit_rows_atomically_no_stale_highlight() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", ms(0), "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", ms(200), "OnNo", "いいえ", &["r1"]));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    present_frame(&mut rt, &mut world, ms(10000)).expect("提示（population）");
    assert_eq!(rt.choice_hit_rows(&actor).len(), 2, "Clear 前はヒット行 2");
    assert!(rt.choice_active(&actor), "Clear 前は choice_active=true");
    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover 提示");

    rt.apply_cue(&cue("0", ms(11000), CueCommand::Clear));
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "Clear は present を待たずヒット行スナップショットを無効化する（5.2 原子性）"
    );
    assert!(
        !rt.choice_active(&actor),
        "Clear は選択肢スパンを消し choice_active=false（5.1）"
    );

    // 提示の後もヒット行が空のままなのは
    // `clear_fullclear_pixels_vanish_with_empty_hit_rows_same_frame_atomic` が見る。
    present_frame(&mut rt, &mut world, ms(12000)).expect("Clear 後フレーム");

    rt.apply_cue(&choice_cue("0", ms(13000), "OnA", "あか", &["a"]));
    rt.apply_cue(&choice_cue("0", ms(13200), "OnB", "あお", &["b"]));
    present_frame(&mut rt, &mut world, ms(20000)).expect("新選択肢提示");
    let rows = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 2, "新選択肢集合のヒット行は 2");
    assert_eq!(rows[0].id, "OnA", "新選択肢のみ（前選択肢は残らない・5.3）");
    assert_eq!(rows[1].id, "OnB");

    let read = |rt: &TextLayerRuntime| -> usize { opaque_count(&read_back(rt, &actor)) };
    let after_new = read(&rt);
    // Clear が hover を既に None にしていれば再提示は NoChange。古い hover が残っていれば
    // 再提示で剥がれて画素が減る。
    rt.inject_choice_hover(&actor, None);
    present_frame(&mut rt, &mut world, ms(20000)).expect("None 再提示");
    let after_none = read(&rt);
    assert_eq!(
        after_new, after_none,
        "Clear が hover を None へリセット済み＝新選択肢に stale ハイライトが載らない（5.4）"
    );
}

/// `apply_cue(ClearAll)` は cue が名指ししない actor を含む**全** actor の hover を None へ戻し、
/// 各 actor のヒット行スナップショットを全スコープの消去と原子的に無効化する。
/// 続く新しい選択肢は古い hover を引き継がない。
#[test]
fn clear_all_resets_hover_and_hit_rows_for_every_actor() {
    let (mut world, window0, slot0) = com_world();
    let (window1, slot1) = spawn_reserved_slot(&mut world);

    let a0 = ActorKey::from("0");
    let a1 = ActorKey::from("1");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let image = (120u32, 60u32);
    rt.apply_cue(&choice_cue("0", ms(0), "Q0", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("1", ms(0), "Q1", "いいえ", &["r1"]));
    rt.register_actor(
        a0.clone(),
        TextSlotBinding::new(slot0, window0, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );
    rt.register_actor(
        a1.clone(),
        TextSlotBinding::new(slot1, window1, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    present_frame(&mut rt, &mut world, ms(10000)).expect("提示（population）");
    assert_eq!(
        rt.choice_hit_rows(&a0).len(),
        1,
        "ClearAll 前 actor0 ヒット行 1"
    );
    assert_eq!(
        rt.choice_hit_rows(&a1).len(),
        1,
        "ClearAll 前 actor1 ヒット行 1"
    );
    rt.inject_choice_hover(&a0, Some(0));
    rt.inject_choice_hover(&a1, Some(0));
    present_frame(&mut rt, &mut world, ms(10000)).expect("hover 提示");

    rt.apply_cue(&cue("0", ms(11000), CueCommand::ClearAll));
    for a in [&a0, &a1] {
        assert!(
            rt.choice_hit_rows(a).is_empty(),
            "ClearAll は present を待たず全 actor のヒット行を無効化する（5.2）"
        );
        assert!(
            !rt.choice_active(a),
            "ClearAll は全 actor の choice_active=false（5.1）"
        );
    }
    present_frame(&mut rt, &mut world, ms(12000)).expect("ClearAll 後フレーム");

    rt.apply_cue(&choice_cue("0", ms(13000), "N0", "あか", &["a"]));
    rt.apply_cue(&choice_cue("1", ms(13000), "N1", "あお", &["b"]));
    present_frame(&mut rt, &mut world, ms(20000)).expect("新選択肢提示");
    let read = |rt: &TextLayerRuntime, a: &ActorKey| -> usize { opaque_count(&read_back(rt, a)) };
    assert_eq!(
        rt.choice_hit_rows(&a0).len(),
        1,
        "actor0 新選択肢ヒット行 1"
    );
    assert_eq!(
        rt.choice_hit_rows(&a1).len(),
        1,
        "actor1 新選択肢ヒット行 1"
    );
    let after_new_0 = read(&rt, &a0);
    let after_new_1 = read(&rt, &a1);
    rt.inject_choice_hover(&a0, None);
    rt.inject_choice_hover(&a1, None);
    present_frame(&mut rt, &mut world, ms(20000)).expect("None 再提示");
    assert_eq!(
        read(&rt, &a0),
        after_new_0,
        "ClearAll が actor0 の hover を None へリセット済み＝stale ハイライト無し（5.4）"
    );
    assert_eq!(
        read(&rt, &a1),
        after_new_1,
        "ClearAll が名指ししない actor1 の hover も None へリセット済み＝stale ハイライト無し（5.4）"
    );
}

// ══ ライフサイクル無効化の同一フレーム原子性（画素消滅＋契約無効化の同時観測） ══

/// Clear を注入した FullClear フレームの提示後に、選択肢行バンドのインク消滅・
/// `choice_hit_rows` 空・`choice_active=false` が**同時**に成立する。
/// 上の節は present 前の契約の原子性で、この節は FullClear 提示後の画素消滅を同じ時点で束ねる。
#[test]
fn clear_fullclear_pixels_vanish_with_empty_hit_rows_same_frame_atomic() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", ms(0), "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", ms(200), "OnNo", "いいえ", &["r1"]));
    let image = (120u32, 60u32);
    let width = image.0;
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    present_frame(&mut rt, &mut world, ms(10000)).expect("提示（population）");
    let base_rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(base_rows.len(), 2, "Clear 前はヒット行 2");
    assert!(rt.choice_active(&actor), "Clear 前は choice_active=true");
    let bands: Vec<(u32, u32)> = base_rows
        .iter()
        .map(|r| {
            let (_, y0, _, y1) = row_px(r);
            (y0, y1)
        })
        .collect();
    let base_bytes = read_back(&rt, &actor);
    for (i, &(y0, y1)) in bands.iter().enumerate() {
        assert!(
            count_px(&base_bytes, width, (0, y0, width, y1), inked) > 0,
            "ベースライン: 選択肢行 {i} バンドにインクがある"
        );
    }

    rt.apply_cue(&cue("0", ms(11000), CueCommand::Clear));
    present_frame(&mut rt, &mut world, ms(12000)).expect("FullClear フレーム提示");

    // FullClear は描画 0 件の全域リセットなので、供給面は全域透明（選択肢行の帯のインクも消える）。
    let cleared_bytes = read_back(&rt, &actor);
    assert_eq!(
        opaque_count(&cleared_bytes),
        0,
        "FullClear 後の供給面は全域透明"
    );
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "FullClear 後の同一フレームで choice_hit_rows が空（5.2）"
    );
    assert!(
        !rt.choice_active(&actor),
        "FullClear 後の同一フレームで choice_active=false（5.1）"
    );
}

/// ClearAll と別 id の新しい選択肢を注入して提示した同一フレームで、旧選択肢が画素・契約とも
/// 消え、**新しい選択肢だけ**が残る。ClearAll 直後（提示前）に旧ヒット行が残らないことは
/// `clear_all_resets_hover_and_hit_rows_for_every_actor` が見る。
#[test]
fn new_talk_clearall_then_new_choice_set_retains_only_new_set_atomic() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let image = (120u32, 60u32);
    let width = image.0;
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    rt.apply_cue(&choice_cue("0", ms(0), "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", ms(200), "OnNo", "いいえ", &["r1"]));
    present_frame(&mut rt, &mut world, ms(10000)).expect("提示（集合1 population）");
    let old_rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(old_rows.len(), 2, "集合1 はヒット行 2");
    assert_eq!(
        (old_rows[0].id.as_str(), old_rows[1].id.as_str()),
        ("OnYes", "OnNo"),
        "集合1 の id"
    );
    assert!(
        rt.choice_active(&actor),
        "集合1 提示後は choice_active=true"
    );
    let old_bytes = read_back(&rt, &actor);
    assert!(opaque_count(&old_bytes) > 0, "集合1 は画素が描かれる");

    rt.apply_cue(&cue("0", ms(11000), CueCommand::ClearAll));

    present_frame(&mut rt, &mut world, ms(12000)).expect("FullClear フレーム提示");
    let cleared_bytes = read_back(&rt, &actor);
    assert_eq!(
        opaque_count(&cleared_bytes),
        0,
        "FullClear で旧集合（集合1）の画素は全域消滅する（stale 残留なし）"
    );

    rt.apply_cue(&choice_cue("0", ms(13000), "OnA", "あか", &["a"]));
    rt.apply_cue(&choice_cue("0", ms(13200), "OnB", "あお", &["b"]));
    present_frame(&mut rt, &mut world, ms(20000)).expect("提示（集合2）");

    let new_rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(new_rows.len(), 2, "集合2 はヒット行 2（新集合のみ）");
    assert_eq!(
        (new_rows[0].id.as_str(), new_rows[1].id.as_str()),
        ("OnA", "OnB"),
        "集合2 の新 id のみ（旧 OnYes/OnNo は消える・5.3）"
    );
    assert!(
        rt.choice_active(&actor),
        "集合2 提示後は choice_active=true"
    );

    let new_bytes = read_back(&rt, &actor);
    for (i, r) in new_rows.iter().enumerate() {
        let (_, y0, _, y1) = row_px(r);
        assert!(
            count_px(&new_bytes, width, (0, y0, width, y1), inked) > 0,
            "集合2: 新選択肢行 {i} バンドにインクがある（描画==ヒット・7.3/7.4）"
        );
    }
}

// ══ 当たりの行は範囲の種類を持つ ══

/// 当たりの行は、元の範囲の種類（選択肢／アンカー）をそのまま写して持つ。
#[test]
fn hit_rows_carry_the_kind_of_their_span() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    // 「あい」をアンカーの範囲（通し番号 0）にし、続けて選択肢（通し番号 1）を置く。
    rt.apply_cue(&cue(
        "0",
        ms(0),
        CueCommand::AnchorBegin {
            id: "x".into(),
            references: vec![],
        },
    ));
    rt.apply_cue(&cue("0", ms(0), CueCommand::Text("あい".into())));
    rt.apply_cue(&cue("0", ms(0), CueCommand::AnchorEnd));
    rt.apply_cue(&choice_cue("0", ms(200), "OnYes", "はい", &[]));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    present_frame(&mut rt, &mut world, ms(10000)).expect("提示");
    let mut kinds: Vec<(usize, SpanKind)> = rt
        .choice_hit_rows(&actor)
        .iter()
        .map(|r| (r.ordinal, r.kind))
        .collect();
    kinds.sort_by_key(|(ordinal, _)| *ordinal);
    assert_eq!(kinds, vec![(0, SpanKind::Anchor), (1, SpanKind::Choice)]);
}

// ══ アンカーの範囲の消去 ══

/// アンカーだけの列も、本文の消去（`Clear`）と同時に押せなくなる: 提示を待たずに当たりの行が
/// 空になり、「押せる範囲があるか」が偽になる。選択肢が 1 つも無い列なので、消去の無効化が
/// 選択肢の有無に寄りかかっていれば古い当たりの行が残る。
#[test]
fn clear_drops_anchor_hit_rows_and_hit_active_atomically() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue(
        "0",
        ms(0),
        CueCommand::AnchorBegin {
            id: "x".into(),
            references: vec![],
        },
    ));
    rt.apply_cue(&cue("0", ms(0), CueCommand::Text("あい".into())));
    rt.apply_cue(&cue("0", ms(0), CueCommand::AnchorEnd));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    present_frame(&mut rt, &mut world, ms(10000)).expect("提示");
    let kinds: Vec<SpanKind> = rt.choice_hit_rows(&actor).iter().map(|r| r.kind).collect();
    assert_eq!(kinds, vec![SpanKind::Anchor], "消去の前は当たりの行 1");
    assert!(rt.hit_active(&actor), "消去の前は押せる範囲がある");

    rt.apply_cue(&cue("0", ms(11000), CueCommand::Clear));
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "消去は提示を待たずに当たりの行を空にする"
    );
    assert!(!rt.hit_active(&actor), "消去で押せる範囲が無くなる");

    // 消去の後の提示でも戻らない。
    present_frame(&mut rt, &mut world, ms(12000)).expect("消去の後の提示");
    assert!(rt.choice_hit_rows(&actor).is_empty());
    assert!(!rt.hit_active(&actor));
}

// ══ 全消しの後の作りたての場所 ══

/// `\p[0]a`〜`\p[999]a` の後に全消しを 2 回すると、場所の数は登録した 2 つのスコープへ戻る
/// （作りたてと同じで面を持たない場所を外す）。消した回数は外さないので減らない。
#[test]
fn clear_all_forgets_pristine_places_but_keeps_clear_counts() {
    let mut world = bevy_ecs::prelude::World::new();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let image = (120u32, 60u32);
    for actor in ["0", "1"] {
        let (window, slot) = spawn_reserved_slot(&mut world);
        rt.register_actor(
            ActorKey::from(actor),
            TextSlotBinding::new(slot, window, 1.0, image, image),
            ResolvedBalloonText::resolve(&geo_model(), image),
        );
    }
    for n in 0..1000 {
        rt.apply_cue(&cue(&n.to_string(), ms(0), CueCommand::Text("a".into())));
    }
    assert_eq!(rt.state().places().count(), 1000);

    rt.apply_cue(&cue("0", ms(1000), CueCommand::ClearAll));
    let after_first: Vec<u64> = ["0", "1", "5", "999"]
        .iter()
        .map(|a| rt.state().clear_count(&ActorKey::from(*a)))
        .collect();
    rt.apply_cue(&cue("0", ms(2000), CueCommand::ClearAll));

    assert_eq!(
        rt.state().places().count(),
        2,
        "全消しの後は登録したスコープの場所だけが残る"
    );
    let after_second: Vec<u64> = ["0", "1", "5", "999"]
        .iter()
        .map(|a| rt.state().clear_count(&ActorKey::from(*a)))
        .collect();
    assert_eq!(after_first, vec![1, 1, 1, 1]);
    assert_eq!(after_second, vec![2, 2, 1, 1], "消した回数は減らない");
}
