//! 表が動く絵の子の行を採る檻（spec: areka-P0-animated-image-playback 要件 2.5・4.1・8.1・8.3・8.4）。
//!
//! seriko は読み込みの側（アトラス）に依存しないので、子の定義は分解（`EmoWorld::bind_atlas`）が
//! 作る値と同じ形を手で組み、[`AnimationTable::from_world_and_films`] へ渡す（`from_world` は
//! 同じ関数へ `film_sheets()`・`film_skips()` を渡すだけ）。面の表の element の種類も、分解と同じに
//! `ElementKind::Film` へ替えて組む。

use std::num::{NonZeroU32, NonZeroU64};

use areka_emo_compose::{
    ElementKind, EmoWorld, FilmId, FilmSheet, FilmSkip, FilmSkipReason, PartKey, SurfaceMaster,
};
use log_capture_kit::{LineFormat, capture_lines};

use super::{AnimationTable, LoopTrigger};

const FIXTURE: &str =
    include_str!("../../areka-emo-compose/tests/fixtures/animated-playback/shell/surfaces.txt");

/// 検体の動く絵の親の絵の番号（アトラスの番号の代わり）。
const RGB_APNG: u32 = 3;
const ALPHA_WEBP: u32 = 5;
const RGB_WEBP: u32 = 8;

fn sheet(id: u32, path: &str, delays_ms: &[u32], laps: Option<u32>) -> FilmSheet {
    FilmSheet {
        id: FilmId(id),
        path: path.to_string(),
        frames: (0..delays_ms.len() as u32).map(|i| id + i).collect(),
        delays_ms: delays_ms.to_vec(),
        laps: laps.and_then(NonZeroU32::new),
        original: (2, 2),
        rest: 0,
    }
}

/// 検体の README の値で組んだ子の定義（番号の昇順）。
fn fixture_sheets() -> Vec<FilmSheet> {
    vec![
        sheet(RGB_APNG, "rgb.apng", &[100, 100], None),
        sheet(ALPHA_WEBP, "alpha.webp", &[100, 0, 70], Some(3)),
        sheet(RGB_WEBP, "rgb.webp", &[100, 100], None),
    ]
}

/// 検体の面の表を組み、動く絵の element を分解と同じに子を置く element へ替える。
fn fixture_world() -> EmoWorld {
    let mut world = EmoWorld::build(&areka_parsers::shell::parse(FIXTURE));
    let w = world.world_mut();
    let mut q = w.query::<&mut SurfaceMaster>();
    for mut master in q.iter_mut(w) {
        for e in &mut master.elements {
            let film = match e.path.as_str() {
                "rgb.apng" => RGB_APNG,
                "alpha.webp" => ALPHA_WEBP,
                "rgb.webp" => RGB_WEBP,
                _ => continue,
            };
            e.kind = ElementKind::Film(FilmId(film));
        }
    }
    world
}

fn build(
    world: &EmoWorld,
    sheets: &[FilmSheet],
    skips: &[FilmSkip],
) -> (AnimationTable, Vec<String>) {
    capture_lines(LineFormat::LevelTargetFields, || {
        AnimationTable::from_world_and_films(world, sheets.iter(), skips)
    })
}

fn warns(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("level=WARN")).collect()
}

/// 子 1 つにつき `always` を 1 本（animation の番号 0）。周期はコマの待ち時間の合計、回数はファイルの
/// 値。コマは絵を指し、pattern i の待ちはコマ i−1 の待ち時間（pattern 0 は 0）。
#[test]
fn film_rows_carry_period_laps_and_picture_frames() {
    let world = fixture_world();
    let (table, _) = build(&world, &fixture_sheets(), &[]);

    let rgb = table.part_animations(PartKey::Film(FilmId(RGB_APNG)));
    assert_eq!(rgb.len(), 1);
    assert_eq!(rgb[0].id, 0);
    assert_eq!(
        rgb[0].trigger,
        LoopTrigger::Always {
            period_ms: NonZeroU64::new(200).unwrap(),
            laps: None
        }
    );
    let frames: Vec<(Option<u32>, u32)> = rgb[0]
        .frames
        .iter()
        .map(|f| (f.picture, f.wait_ms))
        .collect();
    assert_eq!(frames, vec![(Some(3), 0), (Some(4), 100)]);

    let alpha = table.part_animations(PartKey::Film(FilmId(ALPHA_WEBP)));
    assert_eq!(alpha.len(), 1);
    assert_eq!(
        alpha[0].trigger,
        LoopTrigger::Always {
            period_ms: NonZeroU64::new(170).unwrap(),
            laps: NonZeroU32::new(3)
        }
    );
    let frames: Vec<(Option<u32>, u32)> = alpha[0]
        .frames
        .iter()
        .map(|f| (f.picture, f.wait_ms))
        .collect();
    assert_eq!(frames, vec![(Some(5), 0), (Some(6), 100), (Some(7), 0)]);
    assert!(
        alpha[0].frames.iter().all(|f| f.surface_id < 0),
        "絵を指すコマはサーフェスの番号を指さない（番号の経路へ渡っても何も描かない）"
    );
}

/// 子の行と作者のサーフェスの行は鍵の種類が違うので混ざらない（要件 1.12）。
#[test]
fn film_rows_do_not_collide_with_surface_numbers() {
    let world = fixture_world();
    let sheets = vec![sheet(0, "rgb.apng", &[100, 100], None)];
    let (table, _) = build(&world, &sheets, &[]);
    let surface0 = table.part_animations(PartKey::Surface(0));
    let film0 = table.part_animations(PartKey::Film(FilmId(0)));
    assert_eq!(surface0, table.animations(0), "作者のサーフェスは今の列");
    assert_eq!(surface0[0].frames[0].picture, None);
    assert_eq!(film0[0].frames[0].picture, Some(0));
    assert!(table.part_animations(PartKey::Film(FilmId(99))).is_empty());
}

/// 完了の姿: 検体の表に手書きの `always` と子の行が載り、子を置いたサーフェスが在るので
/// 「動く部品が在る」も門も真。記録は合計 0 の手書きの `always` の 1 件だけ。
#[test]
fn fixture_table_carries_film_rows_and_counts_film_parts() {
    let world = fixture_world();
    let (table, logs) = build(&world, &fixture_sheets(), &[]);
    for id in [RGB_APNG, ALPHA_WEBP, RGB_WEBP] {
        assert_eq!(
            table.part_animations(PartKey::Film(FilmId(id))).len(),
            1,
            "子 {id}"
        );
    }
    assert_eq!(table.animations(0).len(), 1, "surface0 の手書きの always");
    assert!(
        table.has_animated_parts(),
        "動く絵の子を置いたサーフェスが在る"
    );
    assert!(table.is_continuous());
    assert_eq!(warns(&logs).len(), 1, "{logs:?}");
}

/// 子だけ（手書きの `always` も `random` も無い）でも門は真。
#[test]
fn film_only_table_is_continuous() {
    let mut world = EmoWorld::build(&areka_parsers::shell::parse(
        "surface0\n{\nelement0,overlay,rgb.apng,0,0\n}\n",
    ));
    let w = world.world_mut();
    let mut q = w.query::<&mut SurfaceMaster>();
    for mut master in q.iter_mut(w) {
        master.elements[0].kind = ElementKind::Film(FilmId(RGB_APNG));
    }
    let (table, logs) = build(
        &world,
        &[sheet(RGB_APNG, "rgb.apng", &[100, 100], None)],
        &[],
    );
    assert!(table.is_empty(), "作者のサーフェスの行は無い");
    assert!(table.has_animated_parts());
    assert!(table.is_continuous());
    assert!(warns(&logs).is_empty(), "{logs:?}");
}

/// 分解しなかった絵 1 件ごとに、相対パスと理由を `warn!` でちょうど 1 回出す（要件 2.5・8.4）。
#[test]
fn film_skips_warn_once_per_cause() {
    let world = EmoWorld::build(&areka_parsers::shell::parse("surface0\n{\n}\n"));
    let skips = [
        FilmSkip {
            path: "broken.apng".into(),
            reason: FilmSkipReason::BrokenFrames,
        },
        FilmSkip {
            path: "zero.webp".into(),
            reason: FilmSkipReason::ZeroTotalDelay,
        },
    ];
    let (table, logs) = build(&world, &[], &skips);
    assert!(!table.is_continuous(), "分解しなかった絵は行を作らない");
    let w = warns(&logs);
    assert_eq!(w.len(), 2, "1 件ごとに 1 回: {logs:?}");
    assert!(
        w[0].contains("broken.apng") && w[0].contains("BrokenFrames"),
        "{}",
        w[0]
    );
    assert!(
        w[1].contains("zero.webp") && w[1].contains("ZeroTotalDelay"),
        "{}",
        w[1]
    );
}

/// `from_world` は面の表の `film_sheets()`・`film_skips()` を渡すだけ: アトラスを束縛していない
/// 面の表では子の行も記録も無い。
#[test]
fn from_world_without_atlas_has_no_film_rows() {
    let world = fixture_world();
    let (a, _) = build(&world, &[], &[]);
    let b = AnimationTable::from_world(&world);
    assert!(
        b.part_animations(PartKey::Film(FilmId(RGB_APNG)))
            .is_empty()
    );
    assert_eq!(a.animations(0), b.animations(0));
    assert_eq!(a.is_continuous(), b.is_continuous());
}
