//! 配置判断の遷移観測レコード（`snapshot`／`hold`／`ground`／`chain`）に対する決定論テスト。
//!
//! 固定するのは 3 つである。
//!
//! - **語彙**——4 つのレコード種別語・段階語・理由語・フィールド名が字面で固定され、上流 2 crate
//!   （wintf・areka-emo-present）の語彙と交わらないこと。
//! - **行の形**——4 種のレコード純関数が逐語で同じ行を組み、欠損は番兵で埋めてフィールドを
//!   落とさないこと（正例）。語を壊した入力が語彙表を素通りしないこと（負例）。
//! - **World からの転写**——刻印は World 資源（`FrameCount`＋`TickStart`）から組み（D1）、
//!   書込タグは窓の marker から scope と種別を読むこと。
//!
//! 接地点レコードを実際に発行する側（`resize_window_to`）の検証は
//! `follow_transition_diag_tests.rs` が持つ（発行点は `follow` 配下ゆえ）。

use windows::Win32::Foundation::RECT;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::window::transition_diag::{
    self as base, FIELD_ENTITY, FIELD_SCOPE, FIELD_STAGE, FIELD_WIN_KIND, MISSING, Stamp,
};

use super::*;
use crate::placement::chain_finalize::ChainDeferReason;
use crate::placement::diag::{PlacementRoute, WindowKind};
use crate::placement::resolver::{PointPx, RectPx, SizePx};
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

/// 任意の刻印（純関数は時刻を読まないので、テストは好きな値で行を組める）。
fn stamp() -> Stamp {
    Stamp {
        frame: 7,
        t_us: 1234,
    }
}

fn ent(raw: u32) -> Entity {
    Entity::from_raw_u32(raw).expect("テスト用 entity index は有効")
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RectPx {
    RectPx {
        left,
        top,
        right,
        bottom,
    }
}

/// 行を `名前=値` の辞書へ（`judge-perf.py::parse_fields` と同じ規則＝空白区切り・後勝ち）。
fn field_value<'a>(line: &'a str, name: &str) -> &'a str {
    let needle = format!("{name}=");
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(&needle))
        .unwrap_or_else(|| panic!("フィールド `{name}` が行に無い: {line}"))
}

// ---------------------------------------------------------------------------
// 語彙（種別語・段階語・理由語・フィールド名）
// ---------------------------------------------------------------------------

/// 5 つのレコード種別語の字面が固定されている（design Data Models の `kind` 列）。
#[test]
fn placement_record_kinds_are_fixed() {
    assert_eq!(KIND_SNAPSHOT, "snapshot");
    assert_eq!(KIND_HOLD, "hold");
    assert_eq!(KIND_GROUND, "ground");
    assert_eq!(KIND_CHAIN, "chain");
    assert_eq!(KIND_OFFSET, "offset");
    assert_eq!(
        PLACEMENT_KIND_ALL,
        &[
            KIND_SNAPSHOT,
            KIND_HOLD,
            KIND_GROUND,
            KIND_CHAIN,
            KIND_OFFSET
        ]
    );
}

/// areka が足す種別語は wintf・emo-present の語彙と 1 つも交わらない
/// （交わると判定器の起点判定・レコード振り分けが壊れる）。
#[test]
fn placement_record_kinds_do_not_collide_with_the_upstream_vocabularies() {
    for kind in PLACEMENT_KIND_ALL {
        assert!(
            !base::KIND_ALL.contains(kind),
            "wintf の種別語と衝突している: {kind}"
        );
        assert_ne!(
            *kind,
            areka_emo_present::presenter::KIND_SURFACE,
            "emo-present の種別語と衝突している: {kind}"
        );
    }
}

/// 整合待ちの判定語・観測点語・連鎖の段階語が字面で固定され、それぞれ一意である。
#[test]
fn hold_and_chain_words_are_fixed_and_distinct() {
    assert_eq!(HOLD_DECISION_PROCEED, "proceed");
    assert_eq!(HOLD_DECISION_HOLD, "hold");
    assert_eq!(HOLD_DECISION_PROCEED_AFTER_TIMEOUT, "proceed-after-timeout");
    assert_eq!(HOLD_SITE_DPI, "dpi");
    assert_eq!(HOLD_SITE_RECONCILE, "reconcile");
    assert_eq!(HOLD_SITE_RESNAP, "resnap");
    assert_eq!(HOLD_SITE_WORK_AREA_RESNAP, "work-area-resnap");
    assert_eq!(CHAIN_STAGE_ARMED, "armed");
    assert_eq!(CHAIN_STAGE_REALIGNED, "realigned");
    assert_eq!(CHAIN_STAGE_DEFERRED, "deferred");

    for words in [
        HOLD_DECISION_ALL,
        HOLD_SITE_ALL,
        CHAIN_STAGE_ALL,
        OFFSET_VERDICT_ALL,
    ] {
        let mut sorted = words.to_vec();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(before, sorted.len(), "語が重複している: {words:?}");
        for word in words {
            assert_ne!(*word, MISSING, "番兵を語彙に混ぜてはならない");
            assert!(
                !word.contains(char::is_whitespace),
                "値に空白があると `名前=値` の切り出しが壊れる: {word}"
            );
        }
    }
}

/// 1 行に同じフィールド名を 2 度出さない（`judge-perf.py::parse_fields` は後勝ち＝先の値が消える）。
#[test]
fn no_placement_line_repeats_a_field_name() {
    let lines = [
        snapshot_line(&SnapshotRecord {
            stamp: stamp(),
            monitors: &[
                MonitorEntry {
                    dpi: 96,
                    work_area: rect(0, 0, 3200, 1752),
                },
                MonitorEntry {
                    dpi: 192,
                    work_area: rect(3200, 0, 6400, 1800),
                },
            ],
        }),
        hold_line(&HoldRecord {
            stamp: stamp(),
            entity: ent(3),
            scope: Some(0),
            win_kind: WindowKind::Char.as_str(),
            window_dpi: 192,
            table_dpi: Some(96),
            since_frame: 5,
            decision: HOLD_DECISION_HOLD,
            site: HOLD_SITE_DPI,
        }),
        ground_line(&GroundRecord {
            stamp: stamp(),
            scope: Some(0),
            ground_y: 1704,
            wa_bottom: Some(1752),
            route: PlacementRoute::DpiReproject,
        }),
        chain_line(&ChainRecord {
            stamp: stamp(),
            stage: CHAIN_STAGE_REALIGNED,
            scopes: 2,
            moved: 1,
            reason: None,
        }),
        offset_line(&OffsetRecord {
            stamp: stamp(),
            scope: Some(0),
            base_dpi: Some(96),
            new_dpi: 192,
            base_offset: PointPx { x: 10, y: 20 },
            old_offset: PointPx { x: 10, y: 20 },
            new_offset: PointPx { x: 20, y: 40 },
            verdict: OFFSET_VERDICT_RESCALED,
        }),
    ];

    for line in lines {
        let mut names: Vec<&str> = line
            .split_whitespace()
            .filter_map(|token| token.split_once('=').map(|(name, _)| name))
            .collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(before, names.len(), "同名フィールドが 2 度出ている: {line}");
    }
}

// ---------------------------------------------------------------------------
// 行の形（正例）
// ---------------------------------------------------------------------------

/// 作業領域源の同期レコード——台数と、モニタごとの `拡大率:左,上,右,下`。
#[test]
fn snapshot_line_lists_the_count_and_every_monitor() {
    assert_eq!(
        snapshot_line(&SnapshotRecord {
            stamp: stamp(),
            monitors: &[
                MonitorEntry {
                    dpi: 96,
                    work_area: rect(0, 0, 3200, 1752),
                },
                MonitorEntry {
                    dpi: 192,
                    work_area: rect(3200, 0, 6400, 1800),
                },
            ],
        }),
        "[transition] frame=7 t_us=1234 kind=snapshot monitors=2 \
         m0=96:0,0,3200,1752 m1=192:3200,0,6400,1800"
    );
}

/// モニタ 0 台は**レコードの不在**ではなく `monitors=0` として観測できる
/// （不在は「観測が無効だった」と区別が付かない・要件 5.5）。
#[test]
fn snapshot_line_reports_zero_monitors_as_a_count_not_as_a_missing_record() {
    assert_eq!(
        snapshot_line(&SnapshotRecord {
            stamp: stamp(),
            monitors: &[],
        }),
        "[transition] frame=7 t_us=1234 kind=snapshot monitors=0"
    );
}

/// 整合待ちのレコード——窓・表・判定・観測点の全フィールド。
#[test]
fn hold_line_carries_every_field() {
    let entity = ent(3);
    assert_eq!(
        hold_line(&HoldRecord {
            stamp: stamp(),
            entity,
            scope: Some(1),
            win_kind: WindowKind::Balloon.as_str(),
            window_dpi: 192,
            table_dpi: Some(96),
            since_frame: 5,
            decision: HOLD_DECISION_HOLD,
            site: HOLD_SITE_RESNAP,
        }),
        format!(
            "[transition] frame=7 t_us=1234 kind=hold entity={entity:?} scope=1 win_kind=balloon \
             window_dpi=192 table_dpi=96 since_frame=5 decision=hold site=resnap"
        )
    );
}

/// 表を引けない窓（帰属モニタなし・表そのものが無い）は番兵で埋め、フィールドを落とさない。
#[test]
fn hold_line_uses_sentinels_for_an_unresolvable_table_and_scope() {
    let entity = ent(9);
    assert_eq!(
        hold_line(&HoldRecord {
            stamp: stamp(),
            entity,
            scope: None,
            win_kind: MISSING,
            window_dpi: 120,
            table_dpi: None,
            since_frame: 0,
            decision: HOLD_DECISION_PROCEED,
            site: HOLD_SITE_RECONCILE,
        }),
        format!(
            "[transition] frame=7 t_us=1234 kind=hold entity={entity:?} scope=- win_kind=- \
             window_dpi=120 table_dpi=- since_frame=0 decision=proceed site=reconcile"
        )
    );
}

/// 接地点レコード——接地点・作業領域下端・その差・経路。**差は本レコードが計算する**
/// （2 箇所で引き算すると観測と判定が静かに食い違う）。
#[test]
fn ground_line_carries_the_ground_point_the_work_area_bottom_and_their_difference() {
    assert_eq!(
        ground_line(&GroundRecord {
            stamp: stamp(),
            scope: Some(0),
            ground_y: 1704,
            wa_bottom: Some(1752),
            route: PlacementRoute::DpiReproject,
        }),
        "[transition] frame=7 t_us=1234 kind=ground scope=0 ground_y=1704 wa_bottom=1752 \
         diff=-48 route=DpiReproject"
    );
}

/// 作業領域を解決できない窓は下端も差も番兵（差を 0 へ潰さない＝「差が無い」と
/// 「測れなかった」を同じ字面にしない）。
#[test]
fn ground_line_uses_sentinels_when_the_work_area_cannot_be_resolved() {
    assert_eq!(
        ground_line(&GroundRecord {
            stamp: stamp(),
            scope: None,
            ground_y: 900,
            wa_bottom: None,
            route: PlacementRoute::WorkAreaResnap,
        }),
        "[transition] frame=7 t_us=1234 kind=ground scope=- ground_y=900 wa_bottom=- \
         diff=- route=WorkAreaResnap"
    );
}

/// 連鎖再解決のレコード——段階・対象スコープ数・動かした数・見送り理由。
#[test]
fn chain_line_carries_the_stage_counts_and_reason() {
    assert_eq!(
        chain_line(&ChainRecord {
            stamp: stamp(),
            stage: CHAIN_STAGE_REALIGNED,
            scopes: 2,
            moved: 1,
            reason: None,
        }),
        "[transition] frame=7 t_us=1234 kind=chain stage=realigned scopes=2 moved=1 reason=-"
    );
    assert_eq!(
        chain_line(&ChainRecord {
            stamp: stamp(),
            stage: CHAIN_STAGE_DEFERRED,
            scopes: 2,
            moved: 0,
            reason: Some(HOLD_SITE_DPI),
        }),
        "[transition] frame=7 t_us=1234 kind=chain stage=deferred scopes=2 moved=0 reason=dpi"
    );
}

// ---------------------------------------------------------------------------
// 行の形（負例）——語を壊した入力が語彙表を素通りしない
// ---------------------------------------------------------------------------

#[test]
fn a_broken_stage_word_does_not_pass_the_chain_vocabulary() {
    let good = chain_line(&ChainRecord {
        stamp: stamp(),
        stage: CHAIN_STAGE_REALIGNED,
        scopes: 1,
        moved: 1,
        reason: None,
    });
    assert!(CHAIN_STAGE_ALL.contains(&field_value(&good, FIELD_STAGE)));

    let broken = good.replace("stage=realigned", "stage=realign");
    assert!(
        !CHAIN_STAGE_ALL.contains(&field_value(&broken, FIELD_STAGE)),
        "壊した語が語彙表を素通りした: {broken}"
    );
}

#[test]
fn a_broken_decision_word_does_not_pass_the_hold_vocabulary() {
    let good = hold_line(&HoldRecord {
        stamp: stamp(),
        entity: ent(1),
        scope: Some(0),
        win_kind: WindowKind::Char.as_str(),
        window_dpi: 96,
        table_dpi: Some(192),
        since_frame: 2,
        decision: HOLD_DECISION_PROCEED_AFTER_TIMEOUT,
        site: HOLD_SITE_DPI,
    });
    assert!(HOLD_DECISION_ALL.contains(&field_value(&good, FIELD_DECISION)));

    let broken = good.replace("decision=proceed-after-timeout", "decision=timeout");
    assert!(
        !HOLD_DECISION_ALL.contains(&field_value(&broken, FIELD_DECISION)),
        "壊した語が語彙表を素通りした: {broken}"
    );
}

/// 必須フィールド列は実際の行と過不足なく一致する（判定器が参照する単一の定義元）。
#[test]
fn the_declared_field_lists_match_the_lines_that_are_actually_built() {
    let ground = ground_line(&GroundRecord {
        stamp: stamp(),
        scope: Some(0),
        ground_y: 1,
        wa_bottom: Some(2),
        route: PlacementRoute::Resnap,
    });
    for name in GROUND_FIELDS {
        assert!(
            ground.contains(&format!("{name}=")),
            "接地点レコードに必須フィールド `{name}` が無い: {ground}"
        );
    }
    assert_eq!(
        GROUND_FIELDS,
        &[
            FIELD_SCOPE,
            FIELD_GROUND_Y,
            FIELD_WA_BOTTOM,
            FIELD_DIFF,
            FIELD_ROUTE
        ]
    );

    let hold = hold_line(&HoldRecord {
        stamp: stamp(),
        entity: ent(1),
        scope: None,
        win_kind: MISSING,
        window_dpi: 96,
        table_dpi: None,
        since_frame: 0,
        decision: HOLD_DECISION_HOLD,
        site: HOLD_SITE_DPI,
    });
    for name in HOLD_FIELDS {
        assert!(
            hold.contains(&format!("{name}=")),
            "整合待ちレコードに必須フィールド `{name}` が無い: {hold}"
        );
    }
    assert_eq!(HOLD_FIELDS[0], FIELD_ENTITY);
    assert_eq!(HOLD_FIELDS[2], FIELD_WIN_KIND);
    assert_eq!(CHAIN_FIELDS[0], FIELD_STAGE);
    assert_eq!(SNAPSHOT_FIELDS, &[FIELD_MONITORS]);

    let offset = offset_line(&OffsetRecord {
        stamp: stamp(),
        scope: None,
        base_dpi: None,
        new_dpi: 96,
        base_offset: PointPx { x: 0, y: 0 },
        old_offset: PointPx { x: 0, y: 0 },
        new_offset: PointPx { x: 0, y: 0 },
        verdict: OFFSET_VERDICT_ANCHORED,
    });
    for name in OFFSET_FIELDS {
        assert!(
            offset.contains(&format!("{name}=")),
            "追随レコードに必須フィールド `{name}` が無い: {offset}"
        );
    }
    assert_eq!(
        OFFSET_FIELDS,
        &[
            FIELD_SCOPE,
            FIELD_BASE_DPI,
            base::FIELD_NEW_DPI,
            FIELD_BASE_OFFSET,
            FIELD_OLD_OFFSET,
            FIELD_NEW_OFFSET,
            FIELD_VERDICT
        ]
    );
}

// ---------------------------------------------------------------------------
// World からの転写（刻印・書込タグ）
// ---------------------------------------------------------------------------

/// 刻印は World 資源から組む（D1）。資源の無い World は値を捏造せず 0 を返す。
#[test]
fn the_stamp_comes_from_the_world_resources_and_degrades_to_zero_without_them() {
    let mut world = World::new();
    assert_eq!(stamp_of(&world), Stamp { frame: 0, t_us: 0 });

    world.insert_resource(wintf::ecs::FrameCount(41));
    world.insert_resource(base::TickStart(std::time::Instant::now()));
    assert_eq!(
        stamp_of(&world).frame,
        41,
        "資源が在るのにフレーム番号が拾えていない"
    );
}

/// 書込タグは経路語・scope・窓種別を運ぶ（要件 2.1）。
#[test]
fn the_write_tag_carries_the_route_scope_and_window_kind() {
    let mut world = World::new();
    let char_window = world.spawn(CharWindowMarker { scope: 0 }).id();
    let balloon = world.spawn(BalloonWindowMarker { scope: 1 }).id();

    let char_tag = write_tag(&world, char_window, Some(PlacementRoute::DpiReproject));
    assert_eq!(char_tag.origin, PlacementRoute::DpiReproject.as_str());
    assert_eq!(char_tag.scope, Some(0));
    assert_eq!(char_tag.kind, WindowKind::Char.as_str());

    let balloon_tag = write_tag(&world, balloon, Some(PlacementRoute::BalloonFollow));
    assert_eq!(balloon_tag.origin, PlacementRoute::BalloonFollow.as_str());
    assert_eq!(balloon_tag.scope, Some(1));
    assert_eq!(balloon_tag.kind, WindowKind::Balloon.as_str());
}

/// 経路語彙を持たない書込（ドラッグ）と marker を持たない窓は番兵になる
/// ——「付け忘れた経路」が行の上で見分けられることが番兵の存在理由（wintf `UNTAGGED` と同じ）。
#[test]
fn the_write_tag_falls_back_to_sentinels_for_untracked_routes_and_unmarked_windows() {
    let mut world = World::new();
    let plain = world.spawn_empty().id();

    let tag = write_tag(&world, plain, None);
    assert_eq!(tag.origin, MISSING);
    assert_eq!(tag.scope, None);
    assert_eq!(tag.kind, MISSING);

    let marked = world.spawn(CharWindowMarker { scope: 2 }).id();
    let dragged = write_tag(&world, marked, None);
    assert_eq!(dragged.origin, MISSING, "ドラッグ経路は経路語を持たない");
    assert_eq!(dragged.scope, Some(2), "窓の素性は route に依らず載る");
    assert_eq!(dragged.kind, WindowKind::Char.as_str());
}

// ---------------------------------------------------------------------------
// 実行時のモニタ表からの作業領域下端（接地点レコードの相方）
// ---------------------------------------------------------------------------

/// 実行時のモニタ表から、窓中心が属するモニタの作業領域下端を引く。
/// 帰属規則は `work_area_for_window` そのまま（別規則を発明しない）。
#[test]
fn the_live_work_area_bottom_uses_the_running_monitor_table() {
    let mut world = World::new();
    world.spawn(Monitor {
        handle: 1,
        bounds: RECT {
            left: 0,
            top: 0,
            right: 3200,
            bottom: 1800,
        },
        work_area: RECT {
            left: 0,
            top: 0,
            right: 3200,
            bottom: 1752,
        },
        dpi: 96,
        is_primary: true,
    });

    assert_eq!(
        live_work_area_bottom(&mut world, rect(1000, 1157, 1382, 1704)),
        Some(1752)
    );
}

/// モニタ表が空（0 台・未挿入）なら架空の矩形を発明せず `None`（レコードは番兵になる）。
#[test]
fn the_live_work_area_bottom_is_none_without_any_monitor() {
    let mut world = World::new();
    assert_eq!(
        live_work_area_bottom(&mut world, rect(0, 0, 100, 100)),
        None
    );
}

/// 実行時の表を読んだ接地点レコードが 1 行だけ出る（発行の口）。
#[test]
fn log_char_ground_emits_exactly_one_record_from_the_live_table() {
    let mut world = World::new();
    world.spawn(Monitor {
        handle: 1,
        bounds: RECT {
            left: 0,
            top: 0,
            right: 3200,
            bottom: 1800,
        },
        work_area: RECT {
            left: 0,
            top: 0,
            right: 3200,
            bottom: 1752,
        },
        dpi: 96,
        is_primary: true,
    });
    let char_window = world.spawn(CharWindowMarker { scope: 0 }).id();

    let (_, events) = crate::placement::test_support::capture_logs(|| {
        log_char_ground(
            &mut world,
            char_window,
            PointPx { x: 1000, y: 1157 },
            SizePx { w: 382, h: 547 },
            PlacementRoute::DpiReproject,
        );
    });

    let hits: Vec<&str> = events
        .iter()
        .map(|e| e.message())
        .filter(|m| m.contains("kind=ground"))
        .collect();
    assert_eq!(hits.len(), 1, "接地点レコードが 1 行でない: {events:?}");
    assert!(
        hits[0].contains("scope=0 ground_y=1704 wa_bottom=1752 diff=-48 route=DpiReproject"),
        "接地点レコードの中身が違う: {}",
        hits[0]
    );
}

/// 連鎖再解決の記録が 1 段階につき 1 行だけ出る（発行の口・task 5.6）。
///
/// 見送りの行は理由語を載せ、見送り以外は番兵で埋める——落とすと「記録が出ていない」と
/// 「その段階にはその値が無い」の区別が事後に付かない。
#[test]
fn log_chain_emits_exactly_one_record_per_stage() {
    let world = World::new();

    let (_, events) = crate::placement::test_support::capture_logs(|| {
        log_chain(&world, CHAIN_STAGE_REALIGNED, 2, 1, None);
        log_chain(
            &world,
            CHAIN_STAGE_DEFERRED,
            2,
            0,
            Some(ChainDeferReason::DpiSyncHeld { scope: 1 }.as_str()),
        );
    });

    let hits: Vec<&str> = events
        .iter()
        .map(|e| e.message())
        .filter(|m| m.contains("kind=chain"))
        .collect();
    assert_eq!(hits.len(), 2, "連鎖レコードが 2 行でない: {events:?}");
    assert!(
        hits[0].contains("stage=realigned scopes=2 moved=1 reason=-"),
        "解き直しの行の中身が違う: {}",
        hits[0]
    );
    assert!(
        hits[1].contains("stage=deferred scopes=2 moved=0 reason=dpi-sync-held"),
        "見送りの行の中身が違う: {}",
        hits[1]
    );
}

// ---------------------------------------------------------------------------
// 追随レコード（`kind=offset`・要件 3.7／8.3・design D10）
// ---------------------------------------------------------------------------

/// 種別語と判定語 6 値が字面で固定され、種別が全数表に載っている。
///
/// 全数表への追加を怠ると、判定側は行を `UnknownKind` として落とすだけで**赤にならない**
/// ——種別の取りこぼしを構造で捕まえる唯一の口が全数表である。
#[test]
fn the_offset_kind_and_its_six_verdict_words_are_fixed() {
    assert!(
        PLACEMENT_KIND_ALL.contains(&KIND_OFFSET),
        "全数表が追随の種別を載せていない"
    );

    assert_eq!(OFFSET_VERDICT_RESCALED, "rescaled");
    assert_eq!(OFFSET_VERDICT_ANCHORED, "anchored");
    assert_eq!(OFFSET_VERDICT_UNCHANGED, "unchanged");
    assert_eq!(OFFSET_VERDICT_KEYWORD_PENDING, "keyword-pending");
    assert_eq!(OFFSET_VERDICT_UNRESOLVED, "unresolved");
    assert_eq!(OFFSET_VERDICT_SATURATED, "saturated");
    assert_eq!(
        OFFSET_VERDICT_ALL,
        &[
            OFFSET_VERDICT_RESCALED,
            OFFSET_VERDICT_ANCHORED,
            OFFSET_VERDICT_UNCHANGED,
            OFFSET_VERDICT_KEYWORD_PENDING,
            OFFSET_VERDICT_UNRESOLVED,
            OFFSET_VERDICT_SATURATED,
        ]
    );
}

/// 追随レコードは 7 欄すべてを載せ、点は `x,y`（空白なし）で 1 フィールドに畳む。
#[test]
fn offset_line_carries_the_base_pair_the_two_offsets_and_the_verdict() {
    assert_eq!(
        offset_line(&OffsetRecord {
            stamp: stamp(),
            scope: Some(0),
            base_dpi: Some(96),
            new_dpi: 192,
            base_offset: PointPx { x: 10, y: -20 },
            old_offset: PointPx { x: 10, y: -20 },
            new_offset: PointPx { x: 20, y: -40 },
            verdict: OFFSET_VERDICT_RESCALED,
        }),
        "[transition] frame=7 t_us=1234 kind=offset scope=0 base_dpi=96 new_dpi=192 \
         base_offset=10,-20 old_offset=10,-20 new_offset=20,-40 verdict=rescaled"
    );
}

/// 未係留の基準（永続値の腕・要件 5.2）と marker を持たない窓は番兵で埋める
/// ——欄を落とすと「記録が出ていない」と「その腕にはその値が無い」の区別が事後に付かない。
#[test]
fn offset_line_uses_a_sentinel_for_an_unpinned_base_and_an_unmarked_scope() {
    assert_eq!(
        offset_line(&OffsetRecord {
            stamp: stamp(),
            scope: None,
            base_dpi: None,
            new_dpi: 120,
            base_offset: PointPx { x: 3, y: 4 },
            old_offset: PointPx { x: 3, y: 4 },
            new_offset: PointPx { x: 3, y: 4 },
            verdict: OFFSET_VERDICT_ANCHORED,
        }),
        "[transition] frame=7 t_us=1234 kind=offset scope=- base_dpi=- new_dpi=120 \
         base_offset=3,4 old_offset=3,4 new_offset=3,4 verdict=anchored"
    );
}

/// 発行は 1 呼出 1 行（1 遷移 1 スコープにつき高々 1 行という契約の下半分）。
#[test]
fn log_offset_rescale_emits_exactly_one_record_per_call() {
    let world = World::new();

    let (_, events) = crate::placement::test_support::capture_logs(|| {
        log_offset_rescale(
            &world,
            Some(1),
            Some(96),
            192,
            PointPx { x: 10, y: 20 },
            PointPx { x: 10, y: 20 },
            PointPx { x: 20, y: 40 },
            OFFSET_VERDICT_RESCALED,
        );
    });

    let hits: Vec<&str> = events
        .iter()
        .map(|e| e.message())
        .filter(|m| m.contains("kind=offset"))
        .collect();
    assert_eq!(hits.len(), 1, "追随レコードが 1 行でない: {events:?}");
    assert!(
        hits[0].contains(
            "scope=1 base_dpi=96 new_dpi=192 base_offset=10,20 old_offset=10,20 \
             new_offset=20,40 verdict=rescaled"
        ),
        "追随レコードの中身が違う: {}",
        hits[0]
    );
}

/// 要件 3.7／8.3・design D10: 発行口は前置ガードを**自分で**持ち、ガードは行の組立より
/// 前にある。`debug!` は既定運転で濾過されるので、ガードを失っても**出力は変わらない**
/// ——濾過テストでは検出できない退行であり、固定できるのは本文走査だけである。
#[test]
fn the_offset_record_is_built_behind_the_front_guard() {
    let code = code_only_lines(include_str!("transition_diag.rs"));
    let body = code
        .split("pub fn log_offset_rescale(")
        .nth(1)
        .expect("発行口 `log_offset_rescale` が無い");
    let guard = body
        .find("if !is_enabled() {")
        .expect("発行口に前置ガードが無い——既定運転でも観測の費用を払う形になる");
    let build = body
        .find("OffsetRecord {")
        .expect("発行口がレコードを組んでいない");
    assert!(
        guard < build,
        "前置ガードがレコードの組立より後ろにある（既定運転で確保が起きる）"
    );
    assert!(
        body[..guard].find("format!").is_none() && body[..guard].find("to_string()").is_none(),
        "ガードより前に文字列の確保がある"
    );

    // 走査が中身を見ている対照——説明文を落とす処理が本文まで落としていないこと。
    assert!(
        code.contains("pub fn offset_line("),
        "説明文を落とす処理が本文まで落としている"
    );
}

/// 説明行（`//` 始まり）を落とした本文。doc の字面を主張と取り違えないための前処理。
fn code_only_lines(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}
