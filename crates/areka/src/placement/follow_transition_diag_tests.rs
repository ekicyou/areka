//! 単一の窓書込口（`enqueue_window_set_pos`）とアンカー保存リサイズ（`resize_window_to`）が
//! 遷移観測へ足す 2 つのもの——**書込指令の要求語彙タグ**と**接地点レコード**——の決定論テスト。
//!
//! # ここが押さえる「現行の欠陥」
//!
//! [`ground_record_shows_the_current_48px_float_when_the_scale_is_lowered`] は
//! **是正ではなく欠陥そのもの**を固定する。作業領域源 [`MonitorSnapshot`] は起動時に 1 度だけ
//! 構築され以後更新されない（`main.rs` の起動シーム・`follow/work_area.rs` の「セッション内固定」）
//! ため、拡大率を下げるとタスクバーが低くなって実際の作業領域下端は**下がる**のに、接地点は
//! 起動時の下端に留まる——実機で 3/3 再現した −48px の浮きである（`reobservation-2026-08-15.md` §6）。
//!
//! **この差を 0 にするのは task 5.1（作業領域源の実行時同期）であって本テストではない。**
//! 是正が入った時点で本テストは赤へ倒れる。そのとき期待値を 0 へ書き換えるのが正しい対応で
//! あり、要件 7.3 が求める「是正前に失敗し是正後に通過する」対テストの**是正前の側**が
//! ここに在る。

use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::RECT;
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::window::transition_diag::{MISSING, TRANSITION_TARGET};

use super::test_support::{fake_handle, rect, window_pos_sized};
use super::{Anchored, MonitorSnapshot, PlacementRoute, resize_window_to};
use crate::placement::diag::WindowKind;
use crate::placement::resolver::{Anchor, SizePx};
use crate::placement::spawn::CharWindowMarker;
use crate::placement::test_support::{capture_logs, ensure_interest_probes};

/// 実機の 200% 側のキャラ窓寸（`reobservation-2026-08-15.md` §3.1）。
const WIDE: SizePx = SizePx { w: 764, h: 1094 };
/// 実機の 100% 側のキャラ窓寸（同上）。
const NARROW: SizePx = SizePx { w: 382, h: 547 };

/// 起動時（200%）に採った作業領域下端。
const BOOT_WA_BOTTOM: i32 = 1704;
/// 100% へ落とした後の**実際の**作業領域下端（タスクバーが低くなる）。
const LIVE_WA_BOTTOM_AT_100: i32 = 1752;

/// 実行時のモニタ表 1 台（作業領域の下端だけが関心）。
fn monitor_with_work_area_bottom(bottom: i32) -> Monitor {
    Monitor {
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
            bottom,
        },
        dpi: 96,
        is_primary: true,
    }
}

/// 起動時 snapshot（`BOOT_WA_BOTTOM` で固定）＋実行時モニタ表＋下端吸着のキャラ窓 1 枚。
///
/// キャラ窓は 200% 時に正しく接地していた状態（下端＝`BOOT_WA_BOTTOM`）から始める。
fn world_with_stale_snapshot(live_bottom: i32) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![rect(0, 0, 3200, BOOT_WA_BOTTOM)],
    });
    world.spawn(monitor_with_work_area_bottom(live_bottom));
    let char_window = world
        .spawn((
            fake_handle(0x2100),
            window_pos_sized(1000, BOOT_WA_BOTTOM - WIDE.h, WIDE.w, WIDE.h),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 0 },
        ))
        .id();
    (world, char_window)
}

/// 捕捉したイベントから接地点レコードの本文を取り出す（ちょうど 1 件を要求）。
fn ground_line_of(events: &[crate::placement::test_support::LogEvent]) -> String {
    let hits: Vec<&str> = events
        .iter()
        .map(|e| e.message())
        .filter(|m| m.contains("kind=ground"))
        .collect();
    assert_eq!(hits.len(), 1, "接地点レコードが 1 行ではない: {events:?}");
    hits[0].to_string()
}

// ---------------------------------------------------------------------------
// 接地点レコード（要件 5.3）
// ---------------------------------------------------------------------------

/// **現行の欠陥を固定する**（task 5.1 の是正前の赤）: 拡大率を下げると接地点が起動時の
/// 作業領域下端に留まり、実際の下端との差が −48px になる。
#[test]
fn ground_record_shows_the_current_48px_float_when_the_scale_is_lowered() {
    let _residue = drain_window_pos_commands();
    let (mut world, char_window) = world_with_stale_snapshot(LIVE_WA_BOTTOM_AT_100);

    let (written, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            char_window,
            NARROW,
            PlacementRoute::DpiReproject,
        )
    });
    assert!(written, "書込が成立していない（前提が崩れている）");

    let line = ground_line_of(&events);
    assert!(
        line.contains(&format!(
            "scope=0 ground_y={BOOT_WA_BOTTOM} wa_bottom={LIVE_WA_BOTTOM_AT_100} diff=-48"
        )),
        "現行の −48px の浮きが接地点レコードに現れていない: {line}"
    );
    assert!(
        line.contains("route=DpiReproject"),
        "接地点を動かした経路が載っていない: {line}"
    );

    let _drained = drain_window_pos_commands();
}

/// 対照（差の値が固定値に焼かれていないこと）: 起動時 snapshot と実行時の作業領域が
/// 一致していれば差は 0——実機で 96→192 の向きが 0 だったのと同じ形。
#[test]
fn ground_record_reports_zero_difference_when_the_snapshot_matches_the_live_work_area() {
    let _residue = drain_window_pos_commands();
    let (mut world, char_window) = world_with_stale_snapshot(BOOT_WA_BOTTOM);

    let (written, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            char_window,
            NARROW,
            PlacementRoute::DpiReproject,
        )
    });
    assert!(written);

    let line = ground_line_of(&events);
    assert!(
        line.contains(&format!(
            "ground_y={BOOT_WA_BOTTOM} wa_bottom={BOOT_WA_BOTTOM} diff=0"
        )),
        "作業領域源が実行時と一致していれば差は 0 のはず: {line}"
    );

    let _drained = drain_window_pos_commands();
}

/// 実行時のモニタ表が空なら、下端も差も番兵——架空の作業領域を発明しない（要件 5.5 と同方針）。
#[test]
fn ground_record_uses_sentinels_when_the_running_monitor_table_is_empty() {
    let _residue = drain_window_pos_commands();
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![rect(0, 0, 3200, BOOT_WA_BOTTOM)],
    });
    let char_window = world
        .spawn((
            fake_handle(0x2101),
            window_pos_sized(1000, BOOT_WA_BOTTOM - WIDE.h, WIDE.w, WIDE.h),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 0 },
        ))
        .id();

    let (_, events) =
        capture_logs(|| resize_window_to(&mut world, char_window, NARROW, PlacementRoute::Resnap));

    let line = ground_line_of(&events);
    assert!(
        line.contains(&format!("ground_y={BOOT_WA_BOTTOM} wa_bottom=- diff=-")),
        "モニタ表を引けないのに下端・差が埋まっている: {line}"
    );

    let _drained = drain_window_pos_commands();
}

/// 接地点は**下端吸着**の規約であり、他アンカーの窓には無い——レコードも出さない。
#[test]
fn no_ground_record_is_emitted_for_non_bottom_anchors() {
    let _residue = drain_window_pos_commands();
    let (mut world, _) = world_with_stale_snapshot(LIVE_WA_BOTTOM_AT_100);
    let free_window = world
        .spawn((
            fake_handle(0x2102),
            window_pos_sized(100, 100, WIDE.w, WIDE.h),
            Anchored(Anchor::Free),
            CharWindowMarker { scope: 1 },
        ))
        .id();

    let (written, events) =
        capture_logs(|| resize_window_to(&mut world, free_window, NARROW, PlacementRoute::Resnap));
    assert!(written, "Free アンカーでも寸の書込自体は起きる");
    // 否定を主張する前に捕捉窓が生きていることを示す（空の窓に対して `all(..)` は恒真）。
    assert!(
        !events.is_empty(),
        "捕捉窓が空＝否定の主張が空振り（捕捉が死んでいても緑になる形になっている）"
    );
    assert!(
        events.iter().all(|e| !e.message().contains("kind=ground")),
        "下端吸着でない窓に接地点レコードが出ている: {events:?}"
    );

    let _drained = drain_window_pos_commands();
}

// ---------------------------------------------------------------------------
// 書込指令の要求語彙タグ（要件 2.1）
// ---------------------------------------------------------------------------

/// 単一の窓書込口が積む指令に、経路語・scope・窓種別の 3 フィールドが載る。
#[test]
fn the_single_writer_tags_the_command_with_the_route_scope_and_window_kind() {
    let _residue = drain_window_pos_commands();
    let (mut world, char_window) = world_with_stale_snapshot(LIVE_WA_BOTTOM_AT_100);

    assert!(resize_window_to(
        &mut world,
        char_window,
        NARROW,
        PlacementRoute::DpiReproject
    ));

    let drained = drain_window_pos_commands();
    assert_eq!(drained.len(), 1, "アンカー保存リサイズは 1 指令だけ積む");
    assert_eq!(drained[0].tag.origin, PlacementRoute::DpiReproject.as_str());
    assert_eq!(drained[0].tag.scope, Some(0));
    assert_eq!(drained[0].tag.kind, WindowKind::Char.as_str());
}

/// 経路語彙を持たない窓（marker 無し）は番兵で積まれる——「付け忘れた経路」が
/// 行の上で見分けられる（wintf の `UNTAGGED` と同じ理由）。
#[test]
fn an_unmarked_window_is_enqueued_with_sentinel_tag_fields() {
    let _residue = drain_window_pos_commands();
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![rect(0, 0, 3200, BOOT_WA_BOTTOM)],
    });
    let window = world
        .spawn((
            fake_handle(0x2103),
            window_pos_sized(100, 100, WIDE.w, WIDE.h),
            Anchored(Anchor::Free),
        ))
        .id();

    assert!(resize_window_to(
        &mut world,
        window,
        NARROW,
        PlacementRoute::Resnap
    ));

    let drained = drain_window_pos_commands();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].tag.origin, PlacementRoute::Resnap.as_str());
    assert_eq!(drained[0].tag.scope, None);
    assert_eq!(drained[0].tag.kind, MISSING);
}

// ---------------------------------------------------------------------------
// 実濾過（既定 OFF・専用 target で点灯）
// ---------------------------------------------------------------------------

/// 捕捉が生きている証拠として同じ窓の中で必ず出す対照行。
///
/// `info` 水準・観測 target で出すので、既定 directive（`info`）でもサインオフ directive
/// （`info,wintf::transition=debug`）でも**必ず**通る——「観測行が出ていないこと」を主張する
/// テストが、実は捕捉そのものが死んでいて緑になっている、という空振りを塞ぐ唯一の手段である
/// （tasks.md Implementation Notes・task 1.2 が wintf 側で確立した形）。
const CONTROL_LINE: &str = "[transition-probe] alive";

/// 与えた `RUST_LOG` 相当の directive で、遷移を 1 回起こしたときの出力を実際に濾して集める。
///
/// 戻り値には必ず [`CONTROL_LINE`] が含まれる。含まれない戻り値は「濾過の結果 0 行だった」
/// ではなく**捕捉装置の故障**であり、呼び手はそれを先に主張すること。
fn resize_under_filter(directives: &str) -> String {
    ensure_interest_probes();

    #[derive(Clone)]
    struct VecWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for VecWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("捕捉バッファのロック取得に失敗")
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(directives))
        .with_ansi(false)
        .with_writer(move || VecWriter(sink.clone()))
        .finish();

    let _residue = drain_window_pos_commands();
    let (mut world, char_window) = world_with_stale_snapshot(LIVE_WA_BOTTOM_AT_100);
    tracing::subscriber::with_default(subscriber, || {
        tracing::callsite::rebuild_interest_cache();
        // 対照行は観測対象の**内側**で出す（同じ捕捉窓・同じ target・同じ directive を通る）。
        tracing::info!(target: TRANSITION_TARGET, "{CONTROL_LINE}");
        resize_window_to(
            &mut world,
            char_window,
            NARROW,
            PlacementRoute::DpiReproject,
        );
    });
    let _drained = drain_window_pos_commands();

    String::from_utf8(buf.lock().expect("捕捉バッファのロック取得に失敗").clone()).expect("UTF-8")
}

/// 既定水準（`RUST_LOG=info`）では接地点レコードが 1 行も出ない（要件 2.5）。
///
/// **対照を先に主張する**——空の出力に対して `!out.contains(..)` は恒真であり、捕捉が
/// 死んでいるだけの緑と、本当に無音である緑を区別できない（要件 8.5 の「消灯の観測点を
/// 0 回の根拠にしない」を、テスト自身に対して適用した形）。
#[test]
fn the_ground_record_is_silent_under_the_default_directives() {
    let out = resize_under_filter("info");
    assert!(
        out.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる（このテストの否定は空振りになる）: {out}"
    );
    assert!(
        !out.contains("kind=ground"),
        "既定 RUST_LOG=info で遷移観測が漏れている: {out}"
    );
}

/// サインオフ手順書の directive で点灯する＝target のリテラルが手順書と 1:1 で結ばれている。
#[test]
fn the_ground_record_lights_up_under_the_signoff_directive() {
    let out = resize_under_filter("info,wintf::transition=debug");
    assert!(
        out.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {out}"
    );
    assert!(
        out.contains("kind=ground") && out.contains("diff=-48"),
        "手順書の RUST_LOG で接地点レコードが点灯しない: {out}"
    );
}

// ---------------------------------------------------------------------------
// 本文走査（ログ濾過テストでは原理的に見えない退行）
// ---------------------------------------------------------------------------

/// 行コメントを落とした本文（説明文の中の字面を数えないため）。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 要件 10.6／10.4: 接地点レコードは「下端吸着のとき」かつ「観測が有効なとき」に**限って**
/// 組む。前置ガードを外すと、既定運転でも毎回モニタ表を舐めて `String` を確保する形になる
/// ——`debug!` は濾過されるので**出力は変わらず**、濾過テストは緑のまま通ってしまう。
#[test]
fn the_ground_record_is_built_behind_the_front_guard() {
    let code = code_only(include_str!("follow/window_move.rs"));

    assert!(
        code.contains("if matches!(anchor, Anchor::Bottom) && transition_diag::is_enabled() {"),
        "接地点レコードの前置ガードが失われている——既定運転でも観測の費用を払う形になる"
    );
    assert_eq!(
        code.matches("transition_diag::is_enabled()").count(),
        1,
        "本文に `is_enabled()` が 2 度以上ある（ガードされていない組立が混ざり得る）"
    );
    assert_eq!(
        code.matches("transition_diag::log_char_ground(").count(),
        1,
        "接地点レコードの発行が 1 箇所でない"
    );

    // 走査が中身を見ている対照——落とし過ぎていないこと。
    assert!(
        code.contains("pub fn resize_window_to("),
        "説明文を落とす処理が本文まで落としている"
    );
}

/// design D3／要件 10.5: 本タスクは観測を足すだけで、既存の 3 つに 1 bit も触らない
/// ——`[diag.window_move]` チャネル・`windowposition.limit` の関門・手順 5a の一度きり再導出。
#[test]
fn the_observation_addition_leaves_the_existing_channels_and_gates_untouched() {
    let code = code_only(include_str!("follow/window_move.rs"));

    assert_eq!(
        code.matches("diag::log_window_move(&WindowMoveRecord {")
            .count(),
        1,
        "`[diag.window_move]` レコードの組立点が動いている（design D3 は不動と定めている）"
    );
    assert!(
        code.contains(
            "let corrected = apply_balloon_limit_gate(world, window, PointPx { x, y }, size, route);"
        ),
        "windowposition-limit の関門呼出が変わっている（要件 10.5）"
    );
    assert_eq!(
        code.matches("rederive_keyword_balloon_offset(").count(),
        1,
        "キーワード基本位置の一度きり再導出（手順 5a）の呼出数が変わっている（要件 10.5）"
    );
}
