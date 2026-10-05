//! 読むだけの口 3 本（`has_surface`／`last_shown`／`compose_alone`・spec `areka-P0-mcp-dump-images`
//! タスク 2.2）と写しの口 2 本（`alias_snapshot`／`surface_ids`・spec `areka-P0-mcp-author-tools`
//! タスク 2.2）の決定論テスト（GPU なし）。
//!
//! # GPU なしで「表示」を成立させる方法
//!
//! `ShowSurface` が GPU を要するのは合成メモに外れたときの表示の記録（`GraphicsCore` の
//! DeviceContext）だけで、当たったときは純 ECS の装着だけで表示が成立する。ゆえに本ファイルは
//! 同じ入力の合成結果を先に target の合成メモへ入れ（表示の記録は `GraphicsCommandList::empty()`・
//! `cache_tests.rs` と同じ扱い）、**本物の `ShowSurface` 指令**を当たりの経路で通す。
//! `last_show`・`current_surface_id`・`visible` を書くのは本番の表示成立点そのものである。

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, ComposedSurface, Composer, EmoWorld, PatternState};
use areka_parsers::shell::{
    AliasKey, Animation, AppendTarget, DefRef, DrawMethod, Interval, Pattern, Shell, Surface,
    SurfaceAlias,
};
use bevy_ecs::world::World;
use tracing::Level;
use wintf::ecs::GraphicsCommandList;
use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::super::test_support::{
    capture, elem, native_golden, native_golden_with, pattern_overlay_at, spawn_window_with_dpi,
};
use super::super::{EmoPresenter, PresentCommand, PresentOutcome, TargetId};

const SHOWN: TargetId = TargetId(0);
const NEVER_SHOWN: TargetId = TargetId(1);
const UNREGISTERED: TargetId = TargetId(9);

/// 生の surface（土台）。
const BASE: u32 = 1000;
/// 着せ替えの部品の surface（生の ID としても在る）。
const PART: u32 = 5000;
/// 着せ替えの animation ID。
const BIND: u32 = 2000;
/// 別名の表（`kero.surface.alias`）のキーにだけ在る番号（生の surface には無い）。
const ALIAS_ONLY: u32 = 7000;
/// どこにも無い番号。
const ABSENT: u32 = 9999;

/// surface 1000（4×3 全不透明・着せ替え 2000 が surface 5000 を (0,0) に重ねる）と surface 5000
/// （1×1 白）に、別名の表 `7000 → [1000]` を足したシェルの `(EmoWorld, AtlasTable)`。
fn build_assets() -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![
        Surface {
            id: BASE,
            targets: vec![AppendTarget::Single(BASE)],
            elements: vec![elem("p.png", 0, 0)],
            collisions: Vec::new(),
            animations: vec![Animation {
                id: BIND,
                interval: Interval::Bind,
                patterns: vec![Pattern {
                    index: 0,
                    method: DrawMethod::new("overlay".to_string()),
                    surface_id: i64::from(PART),
                    wait: 0,
                    x: 0,
                    y: 0,
                }],
            }],
        },
        Surface {
            id: PART,
            targets: vec![AppendTarget::Single(PART)],
            elements: vec![elem("q.png", 0, 0)],
            collisions: Vec::new(),
            animations: Vec::new(),
        },
    ];

    let (w, h) = (4u32, 3u32);
    let mut img = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            img.extend_from_slice(&[(x * 40) as u8, (y * 60) as u8, 0x30, 0xFF]);
        }
    }
    let mut dec = MemoryDecoder::new();
    dec.insert(base.join("p.png"), w, h, w * 4, img, true);
    dec.insert(base.join("q.png"), 1, 1, 4, vec![0xFF; 4], true);

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "atlas bake は失敗しない");

    let shell = Shell {
        surfaces,
        appends: Vec::new(),
        aliases: vec![SurfaceAlias {
            key: AliasKey::new(ALIAS_ONLY.to_string()),
            ids: vec![BASE],
        }],
        animation_sort: None,
        collision_sort: None,
        definitions: vec![DefRef::Surface(0), DefRef::Surface(1), DefRef::Alias(0)],
    };
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// `SHOWN` と `NEVER_SHOWN` の 2 target を登録した presenter と World。
fn setup() -> (EmoPresenter, World) {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    for target in [SHOWN, NEVER_SHOWN] {
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas) = build_assets();
        presenter
            .attach_target(&mut world, target, window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
    }
    (presenter, world)
}

/// 合成メモへ先に入れてから本物の `ShowSurface` を当たりの経路で通す（モジュール doc 参照）。
fn show(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
) {
    let t = presenter.targets.get_mut(&target).expect("登録済み target");
    let composed = Composer::new()
        .compose(&t.emo_world, &t.atlas, surface_id, &binds, &pattern)
        .expect("表示の入力の合成は Ok");
    let mask = Arc::new(AlphaMask::from_pbgra32(
        composed.bytes(),
        composed.width(),
        composed.height(),
        composed.stride(),
    ));
    t.cache.insert(
        surface_id,
        binds.clone(),
        pattern.clone(),
        composed,
        mask,
        GraphicsCommandList::empty(),
    );

    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id,
            binds,
            pattern,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "ShowSurface（surface {surface_id}）が Ok でない"
    );
}

fn hide(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::Hide {
            target,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "Hide が Ok でない"
    );
}

fn bound() -> BindSet {
    BindSet::from_ids([BIND])
}

/// 着せ替えの部品を (2,1) に重ねるアニメーションのコマ（表示の入力に載せて、単体の合成が
/// それを使わないことを見るため）。
fn animated() -> PatternState {
    pattern_overlay_at(3, PART, 2, 1)
}

/// `SHOWN` の World と atlas での合成の答え（期待値）。
fn golden(
    presenter: &EmoPresenter,
    surface_id: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> Vec<u8> {
    let t = presenter.targets.get(&SHOWN).expect("登録済み target");
    native_golden_with(&t.emo_world, &t.atlas, surface_id, binds, pattern).0
}

fn bytes_of(c: &ComposedSurface) -> (Vec<u8>, u32, u32) {
    (c.bytes().to_vec(), c.width(), c.height())
}

/// ⑴ 一度も表示していない target は「最後に表示した surface」が無く、「surface の有無」は答える。
#[test]
fn never_shown_target_has_no_last_shown_but_answers_has_surface() {
    let (presenter, _world) = setup();

    assert!(presenter.last_shown(NEVER_SHOWN).is_none());
    assert_eq!(presenter.has_surface(NEVER_SHOWN, BASE), Some(true));
    assert_eq!(presenter.has_surface(NEVER_SHOWN, ABSENT), Some(false));
}

/// ⑵ 表示の後は ID と絵が返り、隠した後も同じ ID と同じ絵が返る。
#[test]
fn last_shown_returns_id_and_picture_and_keeps_them_after_hide() {
    let (mut presenter, mut world) = setup();
    let plain = golden(
        &presenter,
        BASE,
        &BindSet::default(),
        &PatternState::default(),
    );
    let dressed = golden(&presenter, BASE, &bound(), &PatternState::default());
    assert_ne!(plain, dressed, "前提: 着せ替えの有無で絵が変わる");

    // 着せ替えなしの後に着せ替えありを表示する（メモには両方残る＝最後の入力で引けているかを見る）。
    show(
        &mut presenter,
        &mut world,
        SHOWN,
        BASE,
        BindSet::default(),
        PatternState::default(),
    );
    show(
        &mut presenter,
        &mut world,
        SHOWN,
        BASE,
        bound(),
        PatternState::default(),
    );

    let (id, picture) = presenter.last_shown(SHOWN).expect("表示の後は Some");
    assert_eq!(id, BASE);
    let before = bytes_of(picture.expect("表示の後はメモに絵が在る"));
    assert_eq!(before, (dressed, 4, 3), "最後に表示した入力の絵が返る");

    hide(&mut presenter, &mut world, SHOWN);
    assert_eq!(
        presenter.current_surface_id(SHOWN),
        None,
        "前提: 隠れている"
    );
    assert_eq!(
        presenter.target_visible(SHOWN),
        Some(false),
        "前提: 隠れている"
    );

    let (id, picture) = presenter.last_shown(SHOWN).expect("隠した後も Some");
    assert_eq!(id, BASE);
    assert_eq!(bytes_of(picture.expect("隠した後も絵が在る")), before);
}

/// ⑶ 「surface の有無」は生の ID にだけ真で、別名の表にだけ在る番号と無い番号は偽、未登録は無し。
#[test]
fn has_surface_is_true_only_for_raw_ids() {
    let (presenter, _world) = setup();
    let alias = presenter
        .targets
        .get(&SHOWN)
        .expect("登録済み target")
        .emo_world
        .resolve_alias(&ALIAS_ONLY.to_string());
    assert_eq!(alias, Some(&[BASE][..]), "前提: 番号 7000 は別名の表に在る");

    assert_eq!(presenter.has_surface(SHOWN, BASE), Some(true));
    assert_eq!(presenter.has_surface(SHOWN, PART), Some(true));
    assert_eq!(presenter.has_surface(SHOWN, ALIAS_ONLY), Some(false));
    assert_eq!(presenter.has_surface(SHOWN, ABSENT), Some(false));
    assert_eq!(presenter.has_surface(UNREGISTERED, BASE), None);
    assert!(presenter.last_shown(UNREGISTERED).is_none());
    assert!(presenter.compose_alone(UNREGISTERED, BASE).is_none());
}

/// ⑷ 単体の合成は着せ替えなし・アニメーションなしの合成と一致し、表示の後は最後に表示したときの
/// 着せ替えが掛かる（アニメーションは掛からない）。
#[test]
fn compose_alone_uses_last_shown_binds_and_no_animation() {
    let (mut presenter, mut world) = setup();
    let plain = golden(
        &presenter,
        BASE,
        &BindSet::default(),
        &PatternState::default(),
    );
    let dressed = golden(&presenter, BASE, &bound(), &PatternState::default());
    let dressed_animated = golden(&presenter, BASE, &bound(), &animated());
    assert_ne!(plain, dressed, "前提: 着せ替えで絵が変わる");
    assert_ne!(
        dressed, dressed_animated,
        "前提: アニメーションで絵が変わる"
    );

    let alone = |p: &EmoPresenter, id: u32| {
        let c = p
            .compose_alone(SHOWN, id)
            .expect("登録済み")
            .expect("合成は Ok");
        c.bytes().to_vec()
    };
    assert_eq!(alone(&presenter, BASE), plain);
    let t = presenter.targets.get(&SHOWN).expect("登録済み target");
    assert_eq!(
        alone(&presenter, PART),
        native_golden(&t.emo_world, &t.atlas, PART).0
    );

    show(&mut presenter, &mut world, SHOWN, BASE, bound(), animated());
    assert_eq!(
        alone(&presenter, BASE),
        dressed,
        "最後の着せ替えだけが掛かる"
    );
}

/// ⑸ 単体の合成の前後で、今の surface・可視の状態・最後に表示した surface の答えが変わらず、
/// 合成メモにも入らない。
#[test]
fn compose_alone_leaves_display_state_unchanged() {
    let (mut presenter, mut world) = setup();
    show(
        &mut presenter,
        &mut world,
        SHOWN,
        BASE,
        bound(),
        PatternState::default(),
    );

    let snapshot = |p: &EmoPresenter| {
        let (id, picture) = p.last_shown(SHOWN).expect("表示の後");
        (
            p.current_surface_id(SHOWN),
            p.target_visible(SHOWN),
            id,
            picture.map(bytes_of),
        )
    };
    let before = snapshot(&presenter);
    assert_eq!(before.0, Some(BASE), "前提: 表示している");
    assert_eq!(before.1, Some(true), "前提: 可視");

    for id in [PART, BASE, PART] {
        presenter
            .compose_alone(SHOWN, id)
            .expect("登録済み")
            .expect("合成は Ok");
    }

    assert_eq!(snapshot(&presenter), before);
    let cache = &presenter
        .targets
        .get(&SHOWN)
        .expect("登録済み target")
        .cache;
    assert!(
        cache
            .get(PART, &bound(), &PatternState::default())
            .is_none(),
        "単体の合成は合成メモに入れない"
    );
}

/// ⑹ 3 本とも記録を出さない。`has_surface`／`last_shown` は全水準で 0 件。`compose_alone` は
/// 合成の層が正常な経路でも `trace!`／`debug!` を出しうるため ERROR／WARN の 0 件で判定する。
#[test]
fn reads_emit_no_records() {
    let (mut presenter, mut world) = setup();
    show(
        &mut presenter,
        &mut world,
        SHOWN,
        BASE,
        bound(),
        PatternState::default(),
    );

    let ((), lookups) = capture(|| {
        for id in [BASE, PART, ALIAS_ONLY, ABSENT] {
            let _ = presenter.has_surface(SHOWN, id);
        }
        let _ = presenter.has_surface(UNREGISTERED, BASE);
        for target in [SHOWN, NEVER_SHOWN, UNREGISTERED] {
            let _ = presenter.last_shown(target);
        }
    });
    assert!(
        lookups.is_empty(),
        "has_surface／last_shown が記録を出した: {lookups:?}"
    );

    let ((), composes) = capture(|| {
        for (target, id) in [
            (SHOWN, BASE),
            (SHOWN, PART),
            (NEVER_SHOWN, BASE),
            (UNREGISTERED, BASE),
        ] {
            let _ = presenter.compose_alone(target, id);
        }
    });
    let loud: Vec<_> = composes
        .iter()
        .filter(|e| e.level == Level::ERROR || e.level == Level::WARN)
        .collect();
    assert!(
        loud.is_empty(),
        "compose_alone が ERROR／WARN を出した: {loud:?}"
    );
}

/// ⑺ 別名の表と面の ID の集合の写し（spec `areka-P0-mcp-author-tools` タスク 2.2）は、組んだ資産
/// どおりに返り、未登録は無し。写しを取っても表示の状態が変わらず、記録も出さない。
#[test]
fn alias_snapshot_and_surface_ids_copy_the_built_assets() {
    let (mut presenter, mut world) = setup();
    show(
        &mut presenter,
        &mut world,
        SHOWN,
        BASE,
        bound(),
        PatternState::default(),
    );

    let state = |p: &EmoPresenter| {
        let (id, picture) = p.last_shown(SHOWN).expect("表示の後");
        (
            p.current_surface_id(SHOWN),
            p.target_visible(SHOWN),
            id,
            picture.map(bytes_of),
        )
    };
    let before = state(&presenter);
    assert_eq!(before.0, Some(BASE), "前提: 表示している");

    let (copies, records) = capture(|| {
        (
            presenter.alias_snapshot(SHOWN),
            presenter.surface_ids(SHOWN),
            presenter.alias_snapshot(UNREGISTERED),
            presenter.surface_ids(UNREGISTERED),
        )
    });
    let (aliases, ids, no_aliases, no_ids) = copies;

    assert_eq!(
        aliases,
        Some(std::collections::BTreeMap::from([(
            ALIAS_ONLY.to_string(),
            vec![BASE]
        )])),
        "別名の表は組んだ資産どおり"
    );
    assert_eq!(
        ids,
        Some(std::collections::BTreeSet::from([BASE, PART])),
        "面の ID の集合は生の ID だけ（別名の表にだけ在る番号は入らない）"
    );
    assert_eq!(no_aliases, None);
    assert_eq!(no_ids, None);
    assert!(records.is_empty(), "写しが記録を出した: {records:?}");
    assert_eq!(state(&presenter), before, "写しで表示の状態が変わった");
}
