//! 読むだけの口 3 本（`has_surface`／`last_shown`／`compose_alone`）と写しの口 2 本
//! （`alias_snapshot`／`surface_ids`）の決定論テスト。
//!
//! # 合成メモの当たりで「表示」を成立させる方法
//!
//! 同じ入力の計画を先に target の合成メモへ入れ、**本物の `ShowSurface` 指令**を当たりの
//! 経路で通す（節の葉は実 GPU で記録する）。
//! `last_show`・`current_surface_id`・`visible` を書くのは本番の表示成立点そのものである。

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, ComposedSurface, Composer, EmoWorld, PatternState};
use areka_parsers::shell::{AliasKey, DefRef, Shell, SurfaceAlias};
use bevy_ecs::world::World;
use tracing::Level;

use super::super::test_support::{
    bake_world, bind_overlay_surface, capture, elem, hide_ok, invalidate_ok, native_golden,
    native_golden_with, pattern_overlay_at, show_with_ok, spawn_window_with_dpi, surface,
};
use super::super::{EmoPresenter, TargetId};

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
    let surfaces = vec![
        bind_overlay_surface(0, 0),
        surface(PART, vec![elem("q.png", 0, 0)]),
    ];

    let (w, h) = (4u32, 3u32);
    let mut img = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            img.extend_from_slice(&[(x * 40) as u8, (y * 60) as u8, 0x30, 0xFF]);
        }
    }
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
    bake_world(
        shell,
        vec![("p.png", w, h, img), ("q.png", 1, 1, vec![0xFF; 4])],
    )
}

/// `SHOWN` と `NEVER_SHOWN` の 2 target を登録した presenter と World。
fn setup() -> (EmoPresenter, World) {
    let mut world = super::super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    for target in [SHOWN, NEVER_SHOWN] {
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas) = build_assets();
        presenter.attach_target(target, window, emo_world, atlas, 96);
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
    let plan = Composer::new()
        .plan(&t.emo_world, &t.atlas, surface_id, &binds, &pattern)
        .expect("表示の入力の計画は Ok");
    t.cache
        .insert(surface_id, binds.clone(), pattern.clone(), plan);

    show_with_ok(presenter, world, target, surface_id, binds, pattern);
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

/// 一度も表示していない target は「最後に表示した surface」が無く、「surface の有無」は答える。
#[test]
fn never_shown_target_has_no_last_shown_but_answers_has_surface() {
    let (presenter, _world) = setup();

    assert!(presenter.last_shown(NEVER_SHOWN).is_none());
    assert_eq!(presenter.has_surface(NEVER_SHOWN, BASE), Some(true));
    assert_eq!(presenter.has_surface(NEVER_SHOWN, ABSENT), Some(false));
}

/// 表示の後は ID と絵が返り、隠した後も同じ ID と同じ絵が返る。
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

    hide_ok(&mut presenter, &mut world, SHOWN);
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
    assert_eq!(
        presenter.last_binds(SHOWN),
        Some(&bound()),
        "隠した後も最後の着せ替えが返る"
    );
    assert_eq!(presenter.last_binds(NEVER_SHOWN), None, "未表示は無し");
    assert_eq!(presenter.last_binds(UNREGISTERED), None, "未登録は無し");
}

/// 合成メモを捨てた後も、最後に表示した ID は返り、絵だけが無い（`Some((id, None))`）。
#[test]
fn last_shown_after_invalidate_cache_keeps_id_without_picture() {
    let (mut presenter, mut world) = setup();
    show(
        &mut presenter,
        &mut world,
        SHOWN,
        BASE,
        bound(),
        PatternState::default(),
    );
    invalidate_ok(&mut presenter, &mut world, SHOWN);

    let shown = presenter.last_shown(SHOWN);
    assert!(
        matches!(shown, Some((BASE, None))),
        "メモを捨てた後は ID だけが返る: {:?}",
        shown.map(|(id, c)| (id, c.is_some()))
    );
}

/// 合成メモを捨てた後も、最後に表示した入力（着せ替え・アニメーションの段）の絵を合成し直して返す。
/// 未表示・未登録は無し。
#[test]
fn compose_last_shown_after_invalidate_cache_returns_the_last_picture() {
    let (mut presenter, mut world) = setup();
    let expected = golden(&presenter, BASE, &bound(), &animated());
    assert_ne!(
        expected,
        golden(&presenter, BASE, &bound(), &PatternState::default()),
        "前提: アニメーションの段で絵が変わる"
    );
    show(&mut presenter, &mut world, SHOWN, BASE, bound(), animated());
    invalidate_ok(&mut presenter, &mut world, SHOWN);
    assert!(
        matches!(presenter.last_shown(SHOWN), Some((BASE, None))),
        "前提: 絵が合成メモに無い"
    );

    let again = presenter
        .compose_last_shown(SHOWN)
        .map(|r| r.map(|c| bytes_of(&c)).map_err(|e| e.to_string()));
    assert_eq!(again, Some(Ok((expected, 4, 3))));
    assert!(presenter.compose_last_shown(NEVER_SHOWN).is_none());
    assert!(presenter.compose_last_shown(UNREGISTERED).is_none());
}

/// 「surface の有無」は生の ID にだけ真で、別名の表にだけ在る番号と無い番号は偽、未登録は無し。
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

/// 単体の合成は着せ替えなし・アニメーションなしの合成と一致し、表示の後は最後に表示したときの
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

/// 3 本とも記録を出さない。`has_surface`／`last_shown` は全水準で 0 件。`compose_alone` は
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

/// 別名の表と面の ID の集合の写しは、組んだ資産どおりに返り、未登録は無し。記録も出さない
/// （表示の状態を変えないことは `&self` の型が保証する）。
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
}

/// 書き出しと読み戻しは、合成メモの計画を画面の外で描いた絵を返す。
#[test]
fn last_shown_and_read_back_draw_the_cached_plan() {
    let (mut presenter, mut world) = setup();
    let expected = golden(&presenter, BASE, &bound(), &animated());
    let t = presenter.targets.get_mut(&SHOWN).expect("登録済み target");
    let plan = Composer::new()
        .plan(&t.emo_world, &t.atlas, BASE, &bound(), &animated())
        .expect("計画は Ok");
    t.cache.insert(BASE, bound(), animated(), plan);
    show_with_ok(&mut presenter, &mut world, SHOWN, BASE, bound(), animated());

    let (_, picture) = presenter.last_shown(SHOWN).expect("表示の後は Some");
    assert_eq!(
        bytes_of(picture.expect("描ける")),
        (expected.clone(), 4, 3),
        "書き出しは計画を描いた絵"
    );
    assert_eq!(
        presenter.read_back(SHOWN).expect("読み戻せる"),
        expected,
        "読み戻しは書き出しと同じ絵"
    );
}

/// 画面の外で描けなければ単体の合成は描けない失敗を返し、`error!` を残す。
#[test]
fn compose_alone_returns_render_failure_when_drawing_fails() {
    let (presenter, _world) = setup();
    let (result, records) = capture(|| {
        let _armed = crate::display::arm_display_fault(crate::display::DisplayFault::CreateBitmap);
        presenter.compose_alone(NEVER_SHOWN, BASE)
    });
    assert!(
        matches!(
            result,
            Some(Err(areka_emo_compose::ComposeError::Render {
                surface: BASE,
                ..
            }))
        ),
        "描けない失敗: {:?}",
        result.map(|r| r.map(|c| c.width()))
    );
    assert!(
        records.iter().any(|e| e.level == Level::ERROR),
        "error! が残る: {records:?}"
    );
}

/// 書き出しの控えは表示では描かず、初めて読まれたときに 1 度だけ描いて、以後は同じ絵を返す。
#[test]
fn snapshot_is_drawn_on_first_read_only() {
    let (mut presenter, mut world) = setup();
    show(
        &mut presenter,
        &mut world,
        SHOWN,
        BASE,
        bound(),
        PatternState::default(),
    );
    let entry_snapshot = |p: &EmoPresenter| {
        p.targets[&SHOWN]
            .cache
            .get(BASE, &bound(), &PatternState::default())
            .expect("表示の項")
            .snapshot
            .get()
            .and_then(|c| c.as_ref().ok())
            .map(|c| c as *const ComposedSurface)
    };
    assert_eq!(entry_snapshot(&presenter), None, "表示では描かない");

    let first = presenter
        .last_shown(SHOWN)
        .and_then(|(_, c)| c)
        .map(|c| c as *const _);
    assert!(first.is_some(), "初めて読むと描く");
    assert_eq!(entry_snapshot(&presenter), first, "控えに入る");
    let again = presenter
        .last_shown(SHOWN)
        .and_then(|(_, c)| c)
        .map(|c| c as *const _);
    assert_eq!(again, first, "2 度目は同じ控え");
}

/// 画面の外で描けなければ、書き出しは ID だけを返して `error!` を 1 度だけ残す。同じ項を読み直しても
/// 描き直さず、同じ失敗を黙って返す。
#[test]
fn last_shown_without_picture_when_drawing_fails() {
    let (mut presenter, mut world) = setup();
    show(
        &mut presenter,
        &mut world,
        SHOWN,
        BASE,
        bound(),
        PatternState::default(),
    );
    *presenter.targets[&SHOWN].gpu.borrow_mut() = None;

    let (shown, records) = capture(|| {
        let _armed = crate::display::arm_display_fault(crate::display::DisplayFault::CreateBitmap);
        presenter.last_shown(SHOWN).map(|(id, c)| (id, c.is_some()))
    });
    assert_eq!(shown, Some((BASE, false)));
    assert!(
        records.iter().any(|e| e.level == Level::ERROR),
        "error! が残る: {records:?}"
    );
    let (again, records) = capture(|| presenter.last_shown(SHOWN).map(|(id, c)| (id, c.is_some())));
    assert_eq!(again, Some((BASE, false)), "読み直しも同じ失敗");
    assert!(records.is_empty(), "読み直しで描き直した: {records:?}");
    assert!(presenter.read_back(SHOWN).is_err(), "読み戻しも同じ失敗");
}

/// 一度も表示していない target の単体の合成は、初めに作った頁の控えを次の呼び出しでも使う。
#[test]
fn compose_alone_keeps_the_atlas_it_built_for_a_never_shown_target() {
    let (presenter, _world) = setup();
    let device_of = |p: &EmoPresenter| {
        p.targets[&NEVER_SHOWN]
            .gpu
            .borrow()
            .as_ref()
            .map(|g| g.device().clone())
    };
    assert!(device_of(&presenter).is_none(), "前提: 頁の控えが無い");
    let first = presenter
        .compose_alone(NEVER_SHOWN, BASE)
        .expect("登録済み");
    assert!(first.is_ok(), "描ける");
    let device = device_of(&presenter).expect("作った頁の控えを持つ");
    let second = presenter
        .compose_alone(NEVER_SHOWN, BASE)
        .expect("登録済み");
    assert_eq!(
        second.map(|c| bytes_of(&c)),
        first.map(|c| bytes_of(&c)),
        "同じ絵"
    );
    assert!(
        device_of(&presenter) == Some(device),
        "2 度目で頁の控えを作り直した"
    );
}
