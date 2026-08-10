use areka_sakura::contract::{ActorKey, CueCommand};
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::state::TextLayerConfig;
use super::test_support::{choice_cue, com_world, cue, geo_model, opaque_count, spawn_reserved_slot};

// ══ task 8.3: Clear/ClearAll の原子的無効化（hover リセット＋ヒット行スナップショット無効化・R5.1/5.2/5.4） ══

/// Observable（5.1/5.2/5.4）: `apply_cue(Clear)` は当該 actor の hover を None へリセットし、
/// ヒット行スナップショットを純粋状態の選択肢消去と**原子的**に無効化する（present を待たず
/// `choice_hit_rows` が空・`choice_active` が false へ同時に揃う——表示と hit の片方だけが
/// 古い状態に残らない）。後続の新選択肢集合は stale hover を引き継がず、ハイライト無しで描画される。
#[test]
fn clear_resets_hover_and_invalidates_hit_rows_atomically_no_stale_highlight() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", 0.2, "OnNo", "いいえ", &["r1"]));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    // 選択肢を提示しヒット行を population・ordinal 0 を hover 注入して再提示（ハイライトが載る）。
    present_frame(&mut rt, &mut world, 10.0).expect("提示（population）");
    assert_eq!(rt.choice_hit_rows(&actor).len(), 2, "Clear 前はヒット行 2");
    assert!(rt.choice_active(&actor), "Clear 前は choice_active=true");
    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, 10.0).expect("hover 提示");

    // Clear 注入——present を待たず表示層 state と hit スナップショットが原子的に無効化される。
    rt.apply_cue(&cue("0", 11.0, CueCommand::Clear));
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "Clear は present を待たずヒット行スナップショットを無効化する（5.2 原子性）"
    );
    assert!(
        !rt.choice_active(&actor),
        "Clear は選択肢スパンを消し choice_active=false（5.1）"
    );

    // Clear 後フレーム: 全透明へ戻り、ヒット行は空のまま・choice_active も false のまま。
    present_frame(&mut rt, &mut world, 12.0).expect("Clear 後フレーム");
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "Clear 後の提示もヒット行は空"
    );
    assert!(!rt.choice_active(&actor), "Clear 後も choice_active=false");

    // 新しい選択肢集合を注入——stale hover（Some(0)）を引き継がず、ハイライト無しで描画される（5.4）。
    rt.apply_cue(&choice_cue("0", 13.0, "OnA", "あか", &["a"]));
    rt.apply_cue(&choice_cue("0", 13.2, "OnB", "あお", &["b"]));
    present_frame(&mut rt, &mut world, 20.0).expect("新選択肢提示");
    let rows = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), 2, "新選択肢集合のヒット行は 2");
    assert_eq!(rows[0].id, "OnA", "新選択肢のみ（前選択肢は残らない・5.3）");
    assert_eq!(rows[1].id, "OnB");

    let read = |rt: &TextLayerRuntime| -> usize {
        opaque_count(
            &rt.surface(&actor)
                .expect("供給面")
                .read_back()
                .expect("read_back"),
        )
    };
    let after_new = read(&rt);
    // hover を明示 None にして再提示——Clear が hover を既に None へ揃えていれば NoChange
    // （ハイライトの増減なし）。stale hover が残っていれば present で剥がれてピクセルが減る。
    rt.inject_choice_hover(&actor, None);
    present_frame(&mut rt, &mut world, 20.0).expect("None 再提示");
    let after_none = read(&rt);
    assert_eq!(
        after_new, after_none,
        "Clear が hover を None へリセット済み＝新選択肢に stale ハイライトが載らない（5.4）"
    );
}

/// Observable（5.1/5.2/5.4・ClearAll 全スコープ）: `apply_cue(ClearAll)` は cue が名指ししない
/// actor を含む**全** actor の hover を None へリセットし、各 actor のヒット行スナップショットを
/// 純粋状態の全スコープ消去と原子的に無効化する。後続の新選択肢集合は stale hover を引き継がない。
#[test]
fn clear_all_resets_hover_and_hit_rows_for_every_actor() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");
    let mut world = World::new();
    let (window0, slot0) = spawn_reserved_slot(&mut world);
    let (window1, slot1) = spawn_reserved_slot(&mut world);
    world.insert_resource(core);
    world.insert_resource(wuc);

    let a0 = ActorKey::from("0");
    let a1 = ActorKey::from("1");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let image = (120u32, 60u32);
    // 両 actor に選択肢＋装着。
    rt.apply_cue(&choice_cue("0", 0.0, "Q0", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("1", 0.0, "Q1", "いいえ", &["r1"]));
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

    present_frame(&mut rt, &mut world, 10.0).expect("提示（population）");
    assert_eq!(rt.choice_hit_rows(&a0).len(), 1, "ClearAll 前 actor0 ヒット行 1");
    assert_eq!(rt.choice_hit_rows(&a1).len(), 1, "ClearAll 前 actor1 ヒット行 1");
    // 両 actor に hover 注入して再提示（両供給面へハイライトが載る）。
    rt.inject_choice_hover(&a0, Some(0));
    rt.inject_choice_hover(&a1, Some(0));
    present_frame(&mut rt, &mut world, 10.0).expect("hover 提示");

    // ClearAll（cue.actor="0"）——名指ししない actor(1) を含む全スコープが原子的に無効化される。
    rt.apply_cue(&cue("0", 11.0, CueCommand::ClearAll));
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
    present_frame(&mut rt, &mut world, 12.0).expect("ClearAll 後フレーム");

    // 新選択肢集合を両 actor へ——stale hover を引き継がずハイライト無しで描画（5.4）。
    rt.apply_cue(&choice_cue("0", 13.0, "N0", "あか", &["a"]));
    rt.apply_cue(&choice_cue("1", 13.0, "N1", "あお", &["b"]));
    present_frame(&mut rt, &mut world, 20.0).expect("新選択肢提示");
    let read = |rt: &TextLayerRuntime, a: &ActorKey| -> usize {
        opaque_count(
            &rt.surface(a)
                .expect("供給面")
                .read_back()
                .expect("read_back"),
        )
    };
    assert_eq!(rt.choice_hit_rows(&a0).len(), 1, "actor0 新選択肢ヒット行 1");
    assert_eq!(rt.choice_hit_rows(&a1).len(), 1, "actor1 新選択肢ヒット行 1");
    let after_new_0 = read(&rt, &a0);
    let after_new_1 = read(&rt, &a1);
    // 両 actor の hover を明示 None にして再提示——ClearAll が既に None へ揃えていれば NoChange。
    rt.inject_choice_hover(&a0, None);
    rt.inject_choice_hover(&a1, None);
    present_frame(&mut rt, &mut world, 20.0).expect("None 再提示");
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

// ══ task 9.4: ライフサイクル無効化の同一フレーム原子性（画素消滅＋契約無効化の同時観測・R5.1/5.2/5.3/7.3/7.4） ══

/// 指定バンド（y0..y1・全幅）内の非透明画素数（draw.rs α≠0 述語・9.1 の ink_in_rect と同型）。
fn ink_in_band(bytes: &[u8], width: u32, y0: u32, y1: u32) -> usize {
    let mut n = 0usize;
    for y in y0..y1 {
        for x in 0..width {
            if bytes[((y * width + x) * 4) as usize + 3] != 0 {
                n += 1;
            }
        }
    }
    n
}

/// Observable（5.1/5.2/7.3/7.4・シナリオA=Clear）: Clear 注入→FullClear フレームの**同一 post-present 観測**で、
/// 選択肢行バンドのインク消滅・`choice_hit_rows` 空・`choice_active=false` の三者が**同時**に成立する
/// （8.3 は present 前の契約レベル原子性——9.4 は FullClear 提示後の画素消滅を契約空と同一フレームで束ねる）。
#[test]
fn clear_fullclear_pixels_vanish_with_empty_hit_rows_same_frame_atomic() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", 0.2, "OnNo", "いいえ", &["r1"]));
    let image = (120u32, 60u32);
    let width = image.0;
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&geo_model(), image),
    );

    // ── ベースライン: 提示で選択肢を population。選択肢行バンドを捕捉し、三者の生存を確認。
    present_frame(&mut rt, &mut world, 10.0).expect("提示（population）");
    let base_rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(base_rows.len(), 2, "Clear 前はヒット行 2");
    assert!(rt.choice_active(&actor), "Clear 前は choice_active=true");
    // 選択肢行バンド（各ヒット行の y レンジ）を捕捉。
    let bands: Vec<(u32, u32)> = base_rows
        .iter()
        .map(|r| (r.rect.top as u32, r.rect.bottom.ceil() as u32))
        .collect();
    let base_bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    for (i, &(y0, y1)) in bands.iter().enumerate() {
        assert!(
            ink_in_band(&base_bytes, width, y0, y1) > 0,
            "ベースライン: 選択肢行 {i} バンドにインクがある"
        );
    }

    // ── Clear 注入→FullClear フレーム提示。この直後の**同一観測**で三者が同時に揃う。
    rt.apply_cue(&cue("0", 11.0, CueCommand::Clear));
    present_frame(&mut rt, &mut world, 12.0).expect("FullClear フレーム提示");

    // 同一 post-present 観測①: 選択肢行バンドのインクが消滅（FullClear＝全域透明）。
    let cleared_bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    for (i, &(y0, y1)) in bands.iter().enumerate() {
        assert_eq!(
            ink_in_band(&cleared_bytes, width, y0, y1),
            0,
            "FullClear 後: 選択肢行 {i} バンドのインクが消滅する（7.3/7.4）"
        );
    }
    // 同一観測②: 供給面は全域透明（FullClear＝描画 0 件の全域リセット）。
    assert_eq!(
        opaque_count(&cleared_bytes),
        0,
        "FullClear 後の供給面は全域透明"
    );
    // 同一観測③: 契約ヒット行が空。
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "FullClear 後の同一フレームで choice_hit_rows が空（5.2）"
    );
    // 同一観測④: choice_active=false。
    assert!(
        !rt.choice_active(&actor),
        "FullClear 後の同一フレームで choice_active=false（5.1）"
    );
}

/// Observable（5.1/5.2/5.3/7.3/7.4・シナリオB=新 talk）: ClearAll＋新 Choice 集合（別 id）注入→提示の
/// 同一フレームで、旧選択肢が画素・契約とも消え、**新集合のみ**が保持される（5.3）。ClearAll 直後（提示前）に
/// 旧集合のヒット行が残らないこと（原子的無効化）も併せて確認する。
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

    // ── 集合1（旧）: OnYes/OnNo を population。ベースラインの id・画素を確認。
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0"]));
    rt.apply_cue(&choice_cue("0", 0.2, "OnNo", "いいえ", &["r1"]));
    present_frame(&mut rt, &mut world, 10.0).expect("提示（集合1 population）");
    let old_rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(old_rows.len(), 2, "集合1 はヒット行 2");
    assert_eq!(
        (old_rows[0].id.as_str(), old_rows[1].id.as_str()),
        ("OnYes", "OnNo"),
        "集合1 の id"
    );
    assert!(rt.choice_active(&actor), "集合1 提示後は choice_active=true");
    let old_bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert!(opaque_count(&old_bytes) > 0, "集合1 は画素が描かれる");

    // 旧集合の行バンド（後段で旧画素の消滅を確認するため捕捉）。
    let old_bands: Vec<(u32, u32)> = old_rows
        .iter()
        .map(|r| (r.rect.top as u32, r.rect.bottom.ceil() as u32))
        .collect();

    // ── ClearAll 注入——present を待たず旧集合のヒット行が残らない（原子的無効化・5.2）。
    rt.apply_cue(&cue("0", 11.0, CueCommand::ClearAll));
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "ClearAll 直後（提示前）に旧集合のヒット行は残らない（5.2）"
    );
    assert!(
        !rt.choice_active(&actor),
        "ClearAll 直後に choice_active=false（5.1）"
    );

    // ── FullClear フレーム提示（2 段階クリアの第 1 相・8.3 と同型）——旧集合の画素が消える。
    present_frame(&mut rt, &mut world, 12.0).expect("FullClear フレーム提示");
    let cleared_bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert_eq!(
        opaque_count(&cleared_bytes),
        0,
        "FullClear で旧集合（集合1）の画素は全域消滅する（stale 残留なし）"
    );
    for (i, &(y0, y1)) in old_bands.iter().enumerate() {
        assert_eq!(
            ink_in_band(&cleared_bytes, width, y0, y1),
            0,
            "旧選択肢行 {i} バンドの画素が消滅する（7.3/7.4）"
        );
    }

    // ── 集合2（新・別 id）: OnA/OnB を注入し提示。この同一フレームで新集合のみが保持される。
    rt.apply_cue(&choice_cue("0", 13.0, "OnA", "あか", &["a"]));
    rt.apply_cue(&choice_cue("0", 13.2, "OnB", "あお", &["b"]));
    present_frame(&mut rt, &mut world, 20.0).expect("提示（集合2）");

    // 契約: 新集合のみ（旧 id は一切残らない・5.3）。
    let new_rows: Vec<super::ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(new_rows.len(), 2, "集合2 はヒット行 2（新集合のみ）");
    assert_eq!(
        (new_rows[0].id.as_str(), new_rows[1].id.as_str()),
        ("OnA", "OnB"),
        "集合2 の新 id のみ（旧 OnYes/OnNo は消える・5.3）"
    );
    assert!(
        !new_rows.iter().any(|r| r.id == "OnYes" || r.id == "OnNo"),
        "旧集合の id はヒット行に残らない（5.3）"
    );
    assert!(rt.choice_active(&actor), "集合2 提示後は choice_active=true");

    // 画素: 新集合の行バンドにインクがある＝新選択肢が描かれている（7.3/7.4）。
    let new_bytes = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert!(
        opaque_count(&new_bytes) > 0,
        "集合2 は画素が描かれる（stale 空表示でない）"
    );
    for (i, r) in new_rows.iter().enumerate() {
        let (y0, y1) = (r.rect.top as u32, r.rect.bottom.ceil() as u32);
        assert!(
            ink_in_band(&new_bytes, width, y0, y1) > 0,
            "集合2: 新選択肢行 {i} バンドにインクがある（描画==ヒット・7.3/7.4）"
        );
    }
}
