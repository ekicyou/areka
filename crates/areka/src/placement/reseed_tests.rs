//! シェルの見た目の値の入れ直し（task 9.2・要件 2.7）の決定論テスト。
//!
//! 窓は bare `World` に spawn の本物で組み、偽 HWND の `WindowHandle` で窓書込の口を通す
//! （`SetWindowPosCommand` は enqueue だけで flush しない＝実窓には触れない）。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use areka_sylphya::{Axis, PersistKey};
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::drag::DragConfig;
use wintf::ecs::{DPI, Point, WindowHandle, WindowPos};

use super::{BalloonPlacementInputs, apply_shell_descript};
use crate::placement::config::BalloonXMode;
use crate::placement::follow::{Anchored, BalloonFollow, MonitorSnapshot, OffsetBase};
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::placement::shared_test_support::{
    TempDir, WA, balloon_root, emo2_root, synth_declared_dpi_ghost, with_com_initialized,
};
use crate::placement::source::{DescriptSource, GhostTitles};
use crate::placement::source::{load_balloon_author_dpi, load_descript_source_for_shell};
use crate::placement::spawn::{
    BalloonKeywordBase, CharWindowMarker, GhostWindows, spawn_ghost_windows,
};
use crate::placement::test_support::capture_logs;
use crate::placement::{load_scope_windowpositions, prepare_ghost_windows_with_work_area};

const CHAR0: SizePx = SizePx { w: 434, h: 687 };
const CHAR1: SizePx = SizePx { w: 278, h: 357 };
const BALLOON: SizePx = SizePx { w: 223, h: 158 };

/// 起動時のシェル（Bottom・バルーンは左・ずらし無し）で組んだ 2 スコープの配置。
fn boot_placements() -> Vec<ScopePlacement> {
    let scope = |scope, char_pos: PointPx, char_size, keyword| {
        let offset = PointPx {
            x: -BALLOON.w,
            y: 0,
        };
        ScopePlacement {
            scope,
            char_pos,
            char_size,
            balloon_pos: PointPx {
                x: char_pos.x + offset.x,
                y: char_pos.y,
            },
            balloon_size: BALLOON,
            balloon_offset: offset,
            balloon_offset_base: OffsetBase {
                offset,
                dpi: Some(DPI::from_dpi(96, 96)),
            },
            balloon_limit: true,
            anchor: Anchor::Bottom,
            balloon_keyword_base: keyword,
        }
    };
    vec![
        scope(
            0,
            PointPx { x: 2000, y: 1413 },
            CHAR0,
            Some((BalloonXMode::CenterTop, PointPx { x: 0, y: 0 })),
        ),
        scope(1, PointPx { x: 1500, y: 1743 }, CHAR1, None),
    ]
}

/// 起動の窓を組み、窓書込の口が通るよう偽 HWND と窓の DPI を持たせる。
pub(crate) fn booted_world(dpi: u16) -> (World, GhostWindows) {
    world_from(&boot_placements(), dpi)
}

/// `placements` で窓を組む（作業領域は十分に広い 1 枚・全窓の DPI は `dpi`）。
fn world_from(placements: &[ScopePlacement], dpi: u16) -> (World, GhostWindows) {
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2100,
        }],
    });
    let titles = GhostTitles::from_scope_titles([(0, "s".to_string()), (1, "k".to_string())]);
    let windows = spawn_ghost_windows(&mut world, placements, &titles);
    for (i, scope) in windows.scopes().enumerate() {
        let char_window = windows.char_window(scope).unwrap();
        let balloon_window = windows.balloon_window(scope).unwrap();
        world.entity_mut(char_window).insert(fake_handle(0x100 + i));
        world
            .entity_mut(balloon_window)
            .insert(fake_handle(0x200 + i));
    }
    // `WindowHandle` の付与の hook が偽 HWND の DPI（取れず 96）を遅延で書くので、流してから置く。
    world.flush();
    for scope in windows.scopes() {
        for window in [windows.char_window(scope), windows.balloon_window(scope)] {
            world
                .entity_mut(window.unwrap())
                .insert(DPI::from_dpi(dpi, dpi));
        }
    }
    (world, windows)
}

/// バルーン側の `windowposition` を持たない走っているバルーン（作者の DPI は 96）。
fn no_balloon() -> BalloonPlacementInputs {
    BalloonPlacementInputs {
        author_dpi: 96,
        windowpositions: Default::default(),
    }
}

fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// 新しいシェル: 揃え方は free・sakura のバルーンは右で (30,40)・kero は (-7,9)。
/// `char2.` のキーで設定にだけスコープ 2 が現れる（走っている窓には無い）。
pub(crate) fn new_shell(extra: &[(&str, &str)]) -> DescriptSource {
    let kv = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    };
    let mut shell_kv = kv(&[
        ("seriko.alignmenttodesktop", "free"),
        ("sakura.balloon.alignment", "right"),
        ("sakura.balloon.offsetx", "30"),
        ("sakura.balloon.offsety", "40"),
        ("kero.balloon.offsetx", "-7"),
        ("kero.balloon.offsety", "9"),
        ("char2.balloon.offsetx", "5"),
    ]);
    shell_kv.extend(kv(extra));
    DescriptSource {
        ghost_kv: kv(&[("kero.name", "k")]),
        shell_kv,
        shell_dir: PathBuf::from("shell/second"),
        titles: GhostTitles::from_scope_titles([(0, "s".to_string()), (1, "k".to_string())]),
    }
}

fn follow_of(world: &World, windows: &GhostWindows, scope: usize) -> BalloonFollow {
    *world
        .get::<BalloonFollow>(windows.char_window(scope).unwrap())
        .unwrap()
}

fn position_of(world: &World, entity: Entity) -> Point {
    world.get::<WindowPos>(entity).unwrap().position.unwrap()
}

pub(crate) fn char_positions(world: &World, windows: &GhostWindows) -> Vec<Point> {
    windows
        .scopes()
        .map(|s| position_of(world, windows.char_window(s).unwrap()))
        .collect()
}

fn char_window_count(world: &mut World) -> usize {
    world.query::<&CharWindowMarker>().iter(world).count()
}

fn saved_balloon_offset(scope: u32, x: i32, y: i32) -> Vec<(PersistKey, String)> {
    vec![
        (
            PersistKey::BalloonOffset {
                scope,
                axis: Axis::X,
            },
            x.to_string(),
        ),
        (
            PersistKey::BalloonOffset {
                scope,
                axis: Axis::Y,
            },
            y.to_string(),
        ),
    ]
}

/// 新しいシェルのずらしと揃え方がキャラ窓の部品に入り、窓の位置とスコープの集合は保たれ、
/// バルーン窓は新しいずらしで 1 度置き直される。設定にだけ在るスコープは読み飛ばす。
#[test]
fn t_rs01_new_offset_and_alignment_reach_the_char_window_parts() {
    let (mut world, windows) = booted_world(96);
    let before = char_positions(&world, &windows);

    let ((), events) = capture_logs(|| {
        apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[])
    });

    let c0 = windows.char_window(0).unwrap();
    let pinned96 = Some(DPI::from_dpi(96, 96));
    // sakura: 右（キャラ窓の幅）＋ (30,40)。
    let want0 = PointPx {
        x: CHAR0.w + 30,
        y: 40,
    };
    assert_eq!(follow_of(&world, &windows, 0).offset(), want0);
    assert_eq!(follow_of(&world, &windows, 0).base().dpi, pinned96);
    // kero: 左（バルーンの幅）＋ (-7,9)。
    assert_eq!(
        follow_of(&world, &windows, 1).offset(),
        PointPx {
            x: -BALLOON.w - 7,
            y: 9
        }
    );
    for scope in [0, 1] {
        let c = windows.char_window(scope).unwrap();
        assert_eq!(world.get::<Anchored>(c), Some(&Anchored(Anchor::Free)));
        assert!(
            world.get::<DragConfig>(c).unwrap().move_window,
            "free の窓のドラッグが揃え方に合っていない"
        );
    }
    // キーワードの素材は新しい値に無いので外れる。
    assert!(world.get::<BalloonKeywordBase>(c0).is_none());

    // 窓の位置とスコープの集合は不変・窓は増えない。
    assert_eq!(char_positions(&world, &windows), before);
    assert_eq!(windows.scopes().collect::<Vec<_>>(), vec![0, 1]);
    assert_eq!(
        world
            .resource::<GhostWindows>()
            .scopes()
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(char_window_count(&mut world), 2);

    // バルーン窓は新しいずらしで置き直された。
    let p0 = before[0];
    assert_eq!(
        position_of(&world, windows.balloon_window(0).unwrap()),
        Point {
            x: p0.x + want0.x,
            y: p0.y + want0.y
        }
    );

    let skipped: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reseed_scope_not_running"))
        .collect();
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, tracing::Level::DEBUG);
    assert_eq!(skipped[0].field("scope"), Some("2"));
}

/// 保存済みのバルーンのずらしは新しいシェルの値より優先して保たれる（起動と同じ規則）。
#[test]
fn t_rs02_saved_balloon_offset_survives_the_reseed() {
    let (mut world, windows) = booted_world(96);

    apply_shell_descript(
        &mut world,
        &windows,
        &new_shell(&[]),
        &no_balloon(),
        &saved_balloon_offset(0, -123, -45),
    );

    let f0 = follow_of(&world, &windows, 0);
    assert_eq!(f0.offset(), PointPx { x: -123, y: -45 });
    assert_eq!(f0.base().dpi, None, "保存値は起動と同じく未係留で運ぶ");
    // 保存の無い scope は新しいシェルの値。
    assert_eq!(
        follow_of(&world, &windows, 1).offset(),
        PointPx {
            x: -BALLOON.w - 7,
            y: 9
        }
    );
}

/// 作者の空間のずらしは、起動と同じく窓の DPI ÷ 作者の DPI で物理 px へ換算する。
#[test]
fn t_rs03_author_offset_is_scaled_by_the_window_dpi() {
    let (mut world, windows) = booted_world(192);

    apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[]);

    let f0 = follow_of(&world, &windows, 0);
    assert_eq!(
        f0.offset(),
        PointPx {
            x: CHAR0.w + 60,
            y: 80
        }
    );
    assert_eq!(f0.base().dpi, Some(DPI::from_dpi(192, 192)));

    // 作者の DPI が窓と同じなら換算しない。
    let (mut world, windows) = booted_world(192);
    apply_shell_descript(
        &mut world,
        &windows,
        &new_shell(&[("seriko.dpi", "192")]),
        &no_balloon(),
        &[],
    );
    assert_eq!(
        follow_of(&world, &windows, 0).offset(),
        PointPx {
            x: CHAR0.w + 30,
            y: 40
        }
    );
}

/// 窓が無いスコープは `warn!` を 1 件残して読み飛ばし、残りのスコープは続ける。
#[test]
fn t_rs04_a_missing_window_is_skipped_with_a_warning_and_the_rest_continue() {
    let (mut world, windows) = booted_world(96);
    world.despawn(windows.char_window(1).unwrap());

    let ((), events) = capture_logs(|| {
        apply_shell_descript(&mut world, &windows, &new_shell(&[]), &no_balloon(), &[])
    });

    let skipped: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reseed_skipped"))
        .collect();
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, tracing::Level::WARN);
    assert_eq!(skipped[0].field("scope"), Some("1"));
    assert_eq!(
        follow_of(&world, &windows, 0).offset(),
        PointPx {
            x: CHAR0.w + 30,
            y: 40
        }
    );
}

/// 起動の準備と同じ入力（同じゴースト・同じバルーン・同じ窓寸・同じ DPI）なら、入れ直した
/// 部品は起動の配置と一致する。バルーンの `windowposition` の数値（emo2）・キーワードと y の
/// 調整（合成バルーンの `center`／`-20`）が、シェルの差し替えをまたいで残ることの錨。
///
/// 最後の行は連鎖の錨: `sakura.defaultx,1000` で本体を左へ寄せ、`kero.balloon.alignment,none`
/// の相方の側を「連鎖した仮の X の中心」で決めさせる（連鎖なら中央より左＝右側、スコープ単独で
/// 解けば右端＝左側になって起動と割れる）。作業領域は起動と同じ `WA`。
#[test]
fn t_rs05_reseed_matches_boot_for_the_same_inputs() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (kw_ghost, kw_balloon) = synth_declared_dpi_ghost(&root, "96", "96", None);
        fs::write(
            kw_balloon.join("descript.txt"),
            "charset,UTF-8\ndpi,96\nwindowposition.x,center\nwindowposition.y,-20\n",
        )
        .expect("キーワードのバルーンを書けるはず");
        let chain_root = TempDir::new();
        let (chain_ghost, chain_balloon) = synth_declared_dpi_ghost(&chain_root, "96", "96", None);
        fs::write(
            chain_ghost.join("shell").join("master").join("descript.txt"),
            "charset,UTF-8\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,1000\nkero.defaultx,0\nsakura.balloon.alignment,left\nkero.balloon.alignment,none\n",
        )
        .expect("連鎖の錨のシェルを書けるはず");
        for (ghost, balloon, dpi) in [
            (emo2_root(), balloon_root(), 96u16),
            (emo2_root(), balloon_root(), 192),
            (kw_ghost.clone(), kw_balloon.clone(), 96),
            (chain_ghost.clone(), chain_balloon.clone(), 96),
        ] {
            let boot =
                prepare_ghost_windows_with_work_area(&ghost, &balloon, WA, Some(u32::from(dpi)))
                    .expect("起動の配置の準備は成功する")
                    .placements;
            // 走っている窓は起動の窓寸で、部品だけを崩しておく（入れ直しが空振りなら一致しない）。
            let skewed: Vec<ScopePlacement> = boot
                .iter()
                .map(|p| ScopePlacement {
                    anchor: Anchor::Free,
                    balloon_offset_base: OffsetBase::unpinned(PointPx { x: 1, y: 1 }),
                    balloon_keyword_base: None,
                    ..*p
                })
                .collect();
            let (mut world, windows) = world_from(&skewed, dpi);
            world.insert_resource(MonitorSnapshot {
                work_areas: vec![WA],
            });
            let src = load_descript_source_for_shell(&ghost, None).expect("ゴーストの descript");
            let inputs = BalloonPlacementInputs {
                author_dpi: load_balloon_author_dpi(&balloon),
                windowpositions: load_scope_windowpositions(&balloon, &[0, 1]),
            };

            apply_shell_descript(&mut world, &windows, &src, &inputs, &[]);

            for p in &boot {
                let c = windows.char_window(p.scope).unwrap();
                let case = format!("{} dpi={dpi} scope={}", ghost.display(), p.scope);
                assert_eq!(
                    world.get::<Anchored>(c),
                    Some(&Anchored(p.anchor)),
                    "{case}"
                );
                assert_eq!(
                    world.get::<BalloonFollow>(c).unwrap().base(),
                    p.balloon_offset_base,
                    "{case}"
                );
                assert_eq!(
                    world
                        .get::<BalloonKeywordBase>(c)
                        .map(|k| (k.mode, k.adjust)),
                    p.balloon_keyword_base,
                    "{case}"
                );
            }
        }
    });
}
