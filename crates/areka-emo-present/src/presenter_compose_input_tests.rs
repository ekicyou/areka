use super::*;

use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::BindSet;
use areka_parsers::shell::{Animation, AppendTarget, DrawMethod, Interval, Pattern, Surface};

use wintf::ecs::{HitTest, HitTestMode, Visual};

use super::test_support::{
    build_two_face_assets, elem, make_world_with_gpu, pattern_overlay_at, shell_of,
    spawn_window_with_dpi, surface,
};

/// surface 1000（`w×h` 全不透明 element ＋ bind animation 2000 が surface 5000 を (0,0) に重ねる）
/// の `(EmoWorld, AtlasTable)` と、bind 無し／bind 有りそれぞれの直接合成 golden を返す。
///
/// 5000 の part（1×1 不透明・base と異色）は base 内に収まるため、bind 有無で**外形は不変・
/// バイトのみ変わる**（供給面リサイズ経路を踏まずに bind 差分の表示反映だけを固定できる）。
fn build_target_assets_with_bind(
    w: u32,
    h: u32,
    salt: u8,
) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
    let base = Path::new("shell/master");
    let bind_part = Surface {
        id: 5000,
        targets: vec![AppendTarget::Single(5000)],
        elements: vec![elem("q.png", 0, 0)],
        collisions: Vec::new(),
        animations: Vec::new(),
    };
    let base_surface = Surface {
        id: 1000,
        targets: vec![AppendTarget::Single(1000)],
        elements: vec![elem("p.png", 0, 0)],
        collisions: Vec::new(),
        animations: vec![Animation {
            id: 2000,
            interval: Interval::Bind,
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 5000,
                wait: 0,
                x: 0,
                y: 0,
            }],
        }],
    };
    let surfaces = vec![base_surface, bind_part];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            img.extend_from_slice(&[b, g, r, 0xFF]);
        }
    }
    dec.insert(base.join("p.png"), w, h, stride, img, true);
    // 1×1 の不透明 part（base 左上と必ず異なる色 → bind 有無でバイトが必ず変わる）。
    dec.insert(
        base.join("q.png"),
        1,
        1,
        4,
        vec![0xFF, 0xFF, 0xFF, 0xFF],
        true,
    );

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(
        baked.errors.is_empty(),
        "atlas bake セットアップは失敗しない"
    );

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    let atlas = baked.table;

    let mut composer = Composer::new();
    let golden_plain = composer
        .compose(
            &world,
            &atlas,
            1000,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("bind 無し合成は Ok")
        .bytes()
        .to_vec();
    let golden_bound = composer
        .compose(
            &world,
            &atlas,
            1000,
            &BindSet::from_ids([2000]),
            &PatternState::default(),
        )
        .expect("bind 有り合成は Ok")
        .bytes()
        .to_vec();
    assert_ne!(
        golden_plain, golden_bound,
        "fixture 前提: bind 有無で合成バイトが異ならなければ回帰檻にならない"
    );

    (world, atlas, golden_plain, golden_bound)
}

/// 回帰檻（キャッシュ仕様バグ・実表示レベル）: **同一 surface id で bind 集合だけ変えた**
/// `ShowSurface` が必ず再合成され、`read_back` が各 bind 状態の直接合成 golden とバイト一致する。
///
/// 旧設計（surface id のみキー）では 2 回目以降が古い合成にヒットし、着せ替え・まばたきの
/// bind 差分が表示に反映されなかった（2026-07-09 まばたきデモで顕在化）。往復（無し→有り→無し）
/// で両方向の再合成を固定する。
#[test]
fn bind_change_on_same_surface_updates_display() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, golden_plain, golden_bound) = build_target_assets_with_bind(4, 3, 0x2B);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let show = |presenter: &mut EmoPresenter, world: &mut World, binds: BindSet| {
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds,
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "ShowSurface が Ok でない"
        );
    };

    // bind 無し → golden_plain。
    show(&mut presenter, &mut world, BindSet::default());
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_plain,
        "bind 無し表示が直接合成 golden と一致しない"
    );

    // 同一 surface・bind 有り → 再合成されて golden_bound（旧設計はここで古い絵を返した）。
    show(&mut presenter, &mut world, BindSet::from_ids([2000]));
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_bound,
        "bind 追加が表示へ反映されない（合成入力キーの回帰＝着せ替えバグ再発）"
    );

    // bind 無しへ戻す → 再合成されて golden_plain（往復の両方向を固定）。
    show(&mut presenter, &mut world, BindSet::default());
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_plain,
        "bind 除去が表示へ反映されない（合成入力キーの回帰＝着せ替えバグ再発）"
    );
}

/// R6.1 観測完了（同寸・異 id 再 Show ＝ 新面提示 ＋ 文字スロット安定）: バルーン target が既に
/// ある面（1000）を表示中に、**同寸の異なる面 id（3000）**を `ShowSurface` すると——(a) reply Ok・
/// (b) 可視維持・(c) `HitTest::AlphaMask` 維持・(d) `read_back` が **新面 3000 の golden** と一致
/// （新面が実際に提示された証跡）・(e) `text_slot_view()`（slot/window/surface_size/scale）が切替の
/// 前後で**完全一致**（文字スロットが安定＝TextSlotView が不変）——をすべて満たす。
///
/// 同寸ゆえ供給面（chain）と装着（mount）は再生成されず（apply_show の `chain.is_none()` 分岐を
/// 踏まない）、予約 text スロット entity は据え置かれる＝emo-text の描画資源を破壊しない
/// （design §emo-present 回帰・文字層＝同寸保持）。本 crate 本体は無改変（test-only・R6.3）。
#[test]
fn reshow_same_size_different_face_keeps_text_slot_stable() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, golden_1000, golden_3000) = build_two_face_assets(6, 5);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 面 1000 を表示確立（可視・αマスク判定・供給面/装着を遅延生成）。
    let (tx0, rx0) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx0),
        },
    );
    assert!(
        matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "面 1000 の初回 ShowSurface が Ok でない"
    );
    // 前提: 初回表示は面 1000 の golden（切替前の基準）。
    assert_eq!(
        presenter
            .read_back(TargetId(0))
            .expect("read_back（面 1000）失敗"),
        golden_1000,
        "初回表示が面 1000 の golden と一致しない（前提が崩れている）"
    );

    let surface_entity = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.mount.as_ref())
        .expect("初回表示後は mount が生成済み")
        .surface_entity();

    // 切替前の文字スロット表示スナップショット（TextSlotView は Copy＝値で退避）。
    let slot_before = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");

    // 同寸・異 id 再 Show（面 3000）。
    let (tx1, rx1) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 3000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx1),
        },
    );
    // (a) reply Ok。
    assert!(
        matches!(rx1.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "同寸・異 id（面 3000）の再 ShowSurface が Ok でない"
    );

    // (b) 可視維持。
    assert!(
        world.get::<Visual>(surface_entity).unwrap().is_visible,
        "同寸・異 id 再表示後も可視のまま"
    );
    assert!(
        presenter.targets.get(&TargetId(0)).unwrap().visible,
        "同寸・異 id 再表示後も target.visible=true"
    );
    // (c) HitTest::AlphaMask 維持。
    assert_eq!(
        world.get::<HitTest>(surface_entity).unwrap().mode,
        HitTestMode::AlphaMask,
        "同寸・異 id 再表示後も αマスク判定を維持"
    );

    // (d) read_back が新面 3000 の golden と一致（新面が実際に提示された証跡・R6.1）。
    let rb = presenter
        .read_back(TargetId(0))
        .expect("read_back（面 3000）失敗");
    assert_eq!(
        rb, golden_3000,
        "再表示のバイトが新面 3000 の golden と一致しない（新面が提示されていない）"
    );
    assert_ne!(
        rb, golden_1000,
        "再表示のバイトが旧面 1000 のまま（面切替が表示へ反映されていない）"
    );

    // (e) 文字スロット表示が切替の前後で完全一致（slot/window/surface_size/scale が不変・R6.1）。
    let slot_after = presenter
        .text_slot_view(TargetId(0))
        .expect("再表示後の text_slot_view は Some");
    assert_eq!(
        slot_before, slot_after,
        "同寸・異 id 再表示で文字スロット表示（slot/window/surface_size/scale）が変化した（TextSlotView が不安定）"
    );
}

/// surface 1000（`w×h` 全不透明 element）＋ pattern の現在コマが参照する overlay surface 5000
/// （1×1 不透明・base と異色・base 左上に収まる）を同一 world へ載せた `(EmoWorld, AtlasTable)` と、
/// 空 pattern／非空 pattern それぞれの直接合成 golden を返す。
///
/// surface 5000 は surface 1000 の **bind animation ではなく**（1000 に animation を定義しない）、
/// pattern の現在コマ（`PatternFrame{ surface_id: 5000, Overlay, (0,0) }`）としてのみ top-level 合流
/// する（plan.rs: 合流対象 = 有効 bind pattern0 の id ∪ PatternState に現在コマを持つ id）。5000 は
/// 定義層（extent 母集合＝全 element ＋全 bind animation pattern0）に寄与しないため合成外形は base の
/// `w×h` のまま不変で、pattern 有無で**外形は不変・バイトのみ変わる**（chain リサイズ経路を踏まず
/// 「pattern が compose へ届いたか」だけを固定できる）。
fn build_target_assets_with_pattern(
    w: u32,
    h: u32,
    salt: u8,
) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
    let base = Path::new("shell/master");
    // surface 1000: 全不透明 element 1 本（animation は持たない＝bind 非依存）。
    // surface 5000: 1×1 不透明 part（pattern の現在コマが参照する overlay 源）。
    let surfaces = vec![
        surface(1000, vec![elem("p.png", 0, 0)]),
        surface(5000, vec![elem("q.png", 0, 0)]),
    ];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            img.extend_from_slice(&[b, g, r, 0xFF]);
        }
    }
    dec.insert(base.join("p.png"), w, h, stride, img, true);
    // 1×1 の不透明 part（base 左上と必ず異なる色 → pattern 有無でバイトが必ず変わる）。
    dec.insert(
        base.join("q.png"),
        1,
        1,
        4,
        vec![0xFF, 0xFF, 0xFF, 0xFF],
        true,
    );

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(
        baked.errors.is_empty(),
        "atlas bake セットアップは失敗しない"
    );

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    let atlas = baked.table;

    let mut composer = Composer::new();
    let golden_plain = composer
        .compose(
            &world,
            &atlas,
            1000,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("空 pattern 合成は Ok")
        .bytes()
        .to_vec();
    let golden_pattern = composer
        .compose(
            &world,
            &atlas,
            1000,
            &BindSet::default(),
            &pattern_overlay(2000, 5000),
        )
        .expect("非空 pattern 合成は Ok")
        .bytes()
        .to_vec();
    assert_ne!(
        golden_plain, golden_pattern,
        "fixture 前提: pattern 有無で合成バイトが異ならなければ回帰檻にならない"
    );

    (world, atlas, golden_plain, golden_pattern)
}

/// animation `anim_id` に surface `surf` の `Overlay` 現在コマ 1 枚を持つ非空 `PatternState`。
/// `PatternState::default()`（空）と等価でないことを保証する pattern 差分の実体。
fn pattern_overlay(anim_id: u32, surf: u32) -> PatternState {
    pattern_overlay_at(anim_id, surf, 0, 0)
}

/// Task 8.2 完了檻（pattern が presenter → compose ＋ cache を実際に貫く・R5.1/5.2/5.4）: 同一
/// `(target, surface_id, binds)` でも `ShowSurface` が運ぶ **pattern が変われば表示が変わる**。
///
/// (1) 空 pattern の Show → `read_back` が空 pattern 直接合成 golden と一致（R5.4: 拡張前と観測等価）。
/// (2) 同一 id・binds のまま **非空 pattern** の Show → `read_back` が非空 pattern 直接合成 golden と
///     一致し、かつ空 pattern の絵と**異なる**（pattern が compose へ届き ComposeKey も pattern 分だけ
///     ミスして再合成された証跡・R5.1/5.2）。(3) 再び空 pattern の Show → 空 golden へ戻る（pattern が
///     キー要素として往復両方向で効く）。presenter が pattern を既定（空）で握り潰していれば (2) が
///     空 golden のままとなり本テストは RED になる。
#[test]
fn show_surface_pattern_flows_through_to_compose_and_cache() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);

    let (emo_world, atlas, golden_plain, golden_pattern) =
        build_target_assets_with_pattern(4, 3, 0x3C);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let show = |presenter: &mut EmoPresenter, world: &mut World, pattern: PatternState| {
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern,
                reply: Some(tx),
            },
        );
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "ShowSurface が Ok でない"
        );
    };

    // (1) 空 pattern → golden_plain（拡張前と観測等価・R5.4）。
    show(&mut presenter, &mut world, PatternState::default());
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_plain,
        "空 pattern の表示が空 pattern 直接合成 golden と一致しない（R5.4）"
    );

    // (2) 同一 id・binds・非空 pattern → 再合成されて golden_pattern（pattern が compose＋cache を貫く証跡）。
    show(&mut presenter, &mut world, pattern_overlay(2000, 5000));
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb, golden_pattern,
        "非空 pattern が表示へ反映されない（pattern が compose へ届いていない＝presenter が握り潰している）"
    );
    assert_ne!(
        rb, golden_plain,
        "非空 pattern の表示が空 pattern と同一（ComposeKey が pattern を無視＝古い絵に衝突している）"
    );

    // (3) 空 pattern へ戻す → golden_plain（pattern がキー要素として往復両方向で効く）。
    show(&mut presenter, &mut world, PatternState::default());
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_plain,
        "空 pattern へ戻した表示が空 golden と一致しない（pattern キー要素の往復が壊れている）"
    );
}
