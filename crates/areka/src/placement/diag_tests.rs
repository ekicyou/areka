use std::sync::{Arc, Mutex};

use bevy_ecs::entity::Entity;
use tracing::Level;
use tracing_subscriber::EnvFilter;

use super::*;
use crate::placement::test_support::{capture_logs, ensure_interest_probes};

/// 合成モニタレコード（実 wintf `Monitor` 不要＝純データ）。
fn rec(handle: isize, is_primary: bool) -> MonitorRecord {
    MonitorRecord {
        handle,
        bounds: (0, 0, 1920, 1080),
        work_area: (0, 0, 1920, 1040),
        dpi: 120,
        is_primary,
    }
}

fn ent(raw: u32) -> Entity {
    Entity::from_raw_u32(raw).expect("テスト用 entity index は有効")
}

// ------------------------------------------------------------------
// 語彙の固定（Req 1.2/2.4・診断手順書の grep 判定語と 1:1）
// ------------------------------------------------------------------

/// 専用 target のリテラル固定（Req 1.7・手順書の `RUST_LOG` 記述と 1:1）。
#[test]
fn diag_target_literal_is_fixed() {
    assert_eq!(DIAG_TARGET, "areka::placement::diag");
}

/// レコード種別タグのリテラル固定（grep 判定語の錨）。
#[test]
fn record_tags_are_fixed() {
    assert_eq!(MONITOR_SNAPSHOT_TAG, "[diag.monitor_snapshot]");
    assert_eq!(MONITOR_RECORD_TAG, "[diag.monitor]");
    assert_eq!(WINDOW_MOVE_RECORD_TAG, "[diag.window_move]");
    assert_eq!(ZORDER_PAIR_DECLARED_TAG, "[zorder-pair] declared");
    assert_eq!(ZORDER_PAIR_STRATEGY_TAG, "[zorder-pair] strategy-selected");
}

/// 経路語彙 9 種の表示語がリテラル固定されている（Req 2.4 の結論語彙・D13）。
#[test]
fn placement_route_vocabulary_is_fixed() {
    assert_eq!(PlacementRoute::SpawnInitial.as_str(), "SpawnInitial");
    assert_eq!(PlacementRoute::Restore.as_str(), "Restore");
    assert_eq!(PlacementRoute::AnchorChange.as_str(), "AnchorChange");
    assert_eq!(PlacementRoute::Resnap.as_str(), "Resnap");
    assert_eq!(PlacementRoute::DpiReproject.as_str(), "DpiReproject");
    assert_eq!(
        PlacementRoute::KeepPositionResize.as_str(),
        "KeepPositionResize"
    );
    assert_eq!(PlacementRoute::BalloonFollow.as_str(), "BalloonFollow");
    assert_eq!(
        PlacementRoute::ReportedSizeReconcile.as_str(),
        "ReportedSizeReconcile"
    );
    assert_eq!(PlacementRoute::MoveCue.as_str(), "MoveCue");
}

/// `ALL` は 9 バリアント全部・重複なし（語彙が 1 つでも欠けたら落ちる・D13）。
#[test]
fn placement_route_all_covers_nine_distinct_variants() {
    let all = PlacementRoute::ALL;
    assert_eq!(
        all.len(),
        9,
        "経路語彙は 9 種（design Service Interface・D13）"
    );
    for route in [
        PlacementRoute::SpawnInitial,
        PlacementRoute::Restore,
        PlacementRoute::AnchorChange,
        PlacementRoute::Resnap,
        PlacementRoute::DpiReproject,
        PlacementRoute::KeepPositionResize,
        PlacementRoute::BalloonFollow,
        PlacementRoute::ReportedSizeReconcile,
        PlacementRoute::MoveCue,
    ] {
        assert!(all.contains(&route), "ALL に {route} が無い");
    }
    let mut names: Vec<&str> = all.iter().map(|r| r.as_str()).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len(), "表示語が重複している: {names:?}");
}

/// `Display` は `as_str` と同値（ログ組立の 2 経路が食い違わない）。
#[test]
fn placement_route_display_matches_as_str() {
    for route in PlacementRoute::ALL {
        assert_eq!(route.to_string(), route.as_str());
    }
}

/// 窓種別の語彙固定（手順書④の 2 段 grep が char/balloon で選別する）。
#[test]
fn window_kind_vocabulary_is_fixed() {
    assert_eq!(WindowKind::Char.as_str(), "char");
    assert_eq!(WindowKind::Balloon.as_str(), "balloon");
    assert_eq!(WindowKind::Char.to_string(), "char");
    assert_eq!(WindowKind::Balloon.to_string(), "balloon");
}

// ------------------------------------------------------------------
// レコード組立（純関数・全フィールド固定）
// ------------------------------------------------------------------

/// モニタスナップショット見出し行の厳密固定。
#[test]
fn monitor_snapshot_header_line_is_fixed() {
    assert_eq!(
        monitor_snapshot_header_line("prepare_ghost_windows", 2),
        "[diag.monitor_snapshot] context=prepare_ghost_windows count=2"
    );
}

/// モニタレコード行が識別子・bounds・work_area・DPI・primary の**全フィールド**を持つ
/// （1 つでも欠けたらテストが落ちる・Req 1.1）。
#[test]
fn monitor_record_line_carries_every_field() {
    let line = monitor_record_line(&rec(65537, true), 0);
    assert_eq!(
        line,
        "[diag.monitor] index=0 handle=65537 bounds=0,0,1920,1080 \
         work_area=0,0,1920,1040 dpi=120 primary=true"
    );
    for key in [
        "index=",
        "handle=",
        "bounds=",
        "work_area=",
        "dpi=",
        "primary=",
    ] {
        assert!(line.contains(key), "フィールド `{key}` が欠落: {line}");
    }
}

/// 負座標・非プライマリ・3200 超座標（混在 DPI マルチモニタ実機相当）も忠実転写。
#[test]
fn monitor_record_line_transcribes_negative_and_wide_coordinates() {
    let record = MonitorRecord {
        handle: -3,
        bounds: (-1920, -40, 0, 1040),
        work_area: (-1920, -40, 0, 1000),
        dpi: 192,
        is_primary: false,
    };
    assert_eq!(
        monitor_record_line(&record, 3),
        "[diag.monitor] index=3 handle=-3 bounds=-1920,-40,0,1040 \
         work_area=-1920,-40,0,1000 dpi=192 primary=false"
    );
}

/// 窓移動レコードが route・entity・種別・scope・位置・寸・DPI の**全フィールド**を持つ
/// （Req 1.2・design「窓移動レコード」）。
#[test]
fn window_move_record_line_carries_every_field() {
    let entity = ent(12);
    let record = WindowMoveRecord {
        route: PlacementRoute::DpiReproject,
        entity,
        kind: WindowKind::Char,
        scope: 1,
        x: -1500,
        y: 640,
        size: Some((336, 400)),
        dpi: Some(192),
    };
    let line = window_move_record_line(&record);
    assert_eq!(
        line,
        format!(
            "[diag.window_move] route=DpiReproject entity={entity:?} kind=char scope=1 \
             x=-1500 y=640 w=336 h=400 dpi=192"
        )
    );
    for key in [
        "route=", "entity=", "kind=", "scope=", "x=", "y=", "w=", "h=", "dpi=",
    ] {
        assert!(line.contains(key), "フィールド `{key}` が欠落: {line}");
    }
}

/// entity は wintf 側ログ（`entity = ?e`＝`Debug` 表現）との**結合キー**ゆえ、
/// 同一の `Debug` 表現で出す（手順書④の 2 段 grep が成立する条件）。
#[test]
fn window_move_record_entity_uses_debug_rendering_of_wintf_logs() {
    let entity = ent(4242);
    let record = WindowMoveRecord {
        route: PlacementRoute::Resnap,
        entity,
        kind: WindowKind::Balloon,
        scope: 0,
        x: 0,
        y: 0,
        size: None,
        dpi: None,
    };
    let line = window_move_record_line(&record);
    assert!(
        line.contains(&format!("entity={:?}", entity)),
        "wintf の `entity = ?e` と同じ Debug 表現ではない: {line}"
    );
}

/// 寸・DPI が取れない経路（移動専用・component 未付与）は欠落させず番兵で出す
/// （フィールド語彙は全経路で不変＝grep が経路ごとに揺れない）。
#[test]
fn window_move_record_line_uses_sentinel_for_unknown_size_and_dpi() {
    let entity = ent(7);
    let record = WindowMoveRecord {
        route: PlacementRoute::BalloonFollow,
        entity,
        kind: WindowKind::Balloon,
        scope: 1,
        x: 100,
        y: 200,
        size: None,
        dpi: None,
    };
    assert_eq!(
        window_move_record_line(&record),
        format!(
            "[diag.window_move] route=BalloonFollow entity={entity:?} kind=balloon scope=1 \
             x=100 y=200 w=- h=- dpi=-"
        )
    );
}

// ------------------------------------------------------------------
// ペア宣言レコード（areka-P0-ghost-window-zorder 要件 6.1）
// ------------------------------------------------------------------

/// ペア宣言レコードが scope・キャラ窓・バルーン窓の**全フィールド**を持つ
/// （design「診断ログ語彙（要件 6）」表の `declared` 行）。
#[test]
fn zorder_pair_declared_line_carries_every_field() {
    let char_entity = ent(3);
    let balloon_entity = ent(4);
    let line = zorder_pair_declared_line(1, char_entity, balloon_entity);
    assert_eq!(
        line,
        format!(
            "[zorder-pair] declared scope=1 char_entity={char_entity:?} \
             balloon_entity={balloon_entity:?}"
        )
    );
    for key in ["scope=", "char_entity=", "balloon_entity="] {
        assert!(line.contains(key), "フィールド `{key}` が欠落: {line}");
    }
}

/// 2 つの entity フィールドは wintf 側レコード（`entity=…`／`peer=…`）と**同一の
/// `Debug` 表現**で出る——scope を持たない wintf 記録との 2 段 grep はこの一致だけが
/// 結合条件である（要件 6.1）。役割の取り違え（char と balloon の入れ替え）も同時に固定する。
#[test]
fn zorder_pair_declared_line_entities_use_debug_rendering_and_keep_their_roles() {
    let char_entity = ent(11);
    let balloon_entity = ent(22);
    let line = zorder_pair_declared_line(0, char_entity, balloon_entity);
    assert!(
        line.contains(&format!("char_entity={char_entity:?}"))
            && line.contains(&format!("balloon_entity={balloon_entity:?}")),
        "wintf の `entity = ?e` と同じ Debug 表現で結合できない: {line}"
    );
    assert_ne!(
        line,
        zorder_pair_declared_line(0, balloon_entity, char_entity),
        "キャラ窓とバルーン窓を入れ替えても同じ行になる（役割が読めない）"
    );
}

// ------------------------------------------------------------------
// 実行時ストラテジ選択レコード（要件 5.6・ゲート判定表の結論を起動時ログへ残す）
// ------------------------------------------------------------------

/// 3 通りのストラテジがそれぞれ**別の・読んで分かる**行になる（要件 5.6）。
///
/// 将来どれかへ切り替わっても記録が嘘をつかないためには、選ばれ得る全ての値が
/// 今この場で読める字面に固定されていなければならない。
#[test]
fn zorder_pair_strategy_line_renders_every_strategy() {
    assert_eq!(
        zorder_pair_strategy_line(ZOrderPairStrategy::OwnerLink {
            raise_assist: false
        }),
        "[zorder-pair] strategy-selected plan=A mechanism=owner-link raise_assist=false"
    );
    assert_eq!(
        zorder_pair_strategy_line(ZOrderPairStrategy::OwnerLink { raise_assist: true }),
        "[zorder-pair] strategy-selected plan=A mechanism=owner-link raise_assist=true"
    );
    assert_eq!(
        zorder_pair_strategy_line(ZOrderPairStrategy::ExplicitMaintenance),
        "[zorder-pair] strategy-selected plan=B mechanism=explicit-maintenance raise_assist=-"
    );
}

/// `log_zorder_pair_strategy` は純関数の組立結果を `debug!` 水準で 1 行出す。
#[test]
fn log_zorder_pair_strategy_emits_the_assembled_record_at_debug() {
    let strategy = ZOrderPairStrategy::OwnerLink {
        raise_assist: false,
    };
    let (_, events) = capture_logs(|| log_zorder_pair_strategy(strategy));
    assert_eq!(events.len(), 1, "1 運転 1 レコード: {events:?}");
    assert_eq!(events[0].message(), zorder_pair_strategy_line(strategy));
    assert_eq!(events[0].level, Level::DEBUG);
}

/// `log_zorder_pair_declared` は純関数の組立結果を `debug!` 水準で 1 行出す
/// （組立の二重実装を許さない＝テストが本番の書式を固定する）。
#[test]
fn log_zorder_pair_declared_emits_the_assembled_record_at_debug() {
    let char_entity = ent(5);
    let balloon_entity = ent(6);
    let (_, events) = capture_logs(|| log_zorder_pair_declared(2, char_entity, balloon_entity));
    assert_eq!(events.len(), 1, "1 ペア 1 レコード: {events:?}");
    assert_eq!(
        events[0].message(),
        zorder_pair_declared_line(2, char_entity, balloon_entity)
    );
    assert_eq!(events[0].level, Level::DEBUG);
}

/// 全 9 経路が `route=<語>` として組み上がる（配管先が増えても語彙が固定される）。
#[test]
fn window_move_record_line_renders_every_route() {
    for route in PlacementRoute::ALL {
        let record = WindowMoveRecord {
            route,
            entity: ent(1),
            kind: WindowKind::Char,
            scope: 0,
            x: 0,
            y: 0,
            size: Some((1, 1)),
            dpi: Some(96),
        };
        assert!(
            window_move_record_line(&record).contains(&format!("route={}", route.as_str())),
            "route={route} がレコードに現れない"
        );
    }
}

// ------------------------------------------------------------------
// 出力（debug! 水準・専用 target）
// ------------------------------------------------------------------

/// `log_monitor_snapshot` は見出し 1 行＋モニタ 1 台 1 行を `debug!` で出し、
/// 本文は純関数の組立結果と**一致**する（ログと純関数の二重実装を許さない）。
#[test]
fn log_monitor_snapshot_emits_header_and_one_line_per_monitor_at_debug() {
    let monitors = [rec(1, true), rec(2, false)];
    let (_, events) = capture_logs(|| log_monitor_snapshot(&monitors, "prepare_ghost_windows"));

    let messages: Vec<&str> = events.iter().map(|e| e.message()).collect();
    assert_eq!(
        messages,
        vec![
            monitor_snapshot_header_line("prepare_ghost_windows", 2).as_str(),
            monitor_record_line(&monitors[0], 0).as_str(),
            monitor_record_line(&monitors[1], 1).as_str(),
        ]
    );
    for e in &events {
        assert_eq!(e.level, Level::DEBUG, "恒久観測は debug! 水準（Req 1.7）");
    }
}

/// モニタ 0 台でも見出し 1 行は出る（count=0 が観測できる・panic しない）。
#[test]
fn log_monitor_snapshot_with_no_monitors_still_emits_header() {
    let (_, events) = capture_logs(|| log_monitor_snapshot(&[], "monitor_snapshot"));
    let messages: Vec<&str> = events.iter().map(|e| e.message()).collect();
    assert_eq!(
        messages,
        vec!["[diag.monitor_snapshot] context=monitor_snapshot count=0"]
    );
}

/// `log_window_move` は純関数の組立結果を `debug!` 水準で 1 行出す。
#[test]
fn log_window_move_emits_the_assembled_record_at_debug() {
    let record = WindowMoveRecord {
        route: PlacementRoute::AnchorChange,
        entity: ent(9),
        kind: WindowKind::Char,
        scope: 0,
        x: 10,
        y: 20,
        size: Some((30, 40)),
        dpi: Some(120),
    };
    let (_, events) = capture_logs(|| log_window_move(&record));
    assert_eq!(events.len(), 1, "1 移動 1 レコード: {events:?}");
    assert_eq!(events[0].message(), window_move_record_line(&record));
    assert_eq!(events[0].level, Level::DEBUG);
}

// ------------------------------------------------------------------
// Req 1.7: 既定では 1 行も出ない・手順書の RUST_LOG でのみ点灯
// ------------------------------------------------------------------

/// 与えた `RUST_LOG` 相当の directive で本モジュールの全出力を実際に濾して集める。
fn emit_all_under_filter(directives: &str) -> String {
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

    tracing::subscriber::with_default(subscriber, || {
        tracing::callsite::rebuild_interest_cache();
        log_monitor_snapshot(&[rec(1, true)], "prepare_ghost_windows");
        log_window_move(&WindowMoveRecord {
            route: PlacementRoute::SpawnInitial,
            entity: ent(1),
            kind: WindowKind::Char,
            scope: 0,
            x: 0,
            y: 0,
            size: Some((1, 1)),
            dpi: Some(96),
        });
        log_zorder_pair_declared(0, ent(1), ent(2));
        log_zorder_pair_strategy(ZOrderPairStrategy::OwnerLink {
            raise_assist: false,
        });
    });

    String::from_utf8(buf.lock().expect("捕捉バッファのロック取得に失敗").clone()).expect("UTF-8")
}

/// 既定水準（`RUST_LOG=info`＝`main.rs` のフォールバック）では **1 行も出ない**（Req 1.7）。
#[test]
fn diag_records_are_silent_under_default_info_filter() {
    assert_eq!(
        emit_all_under_filter("info"),
        "",
        "既定 RUST_LOG=info で診断出力が漏れている（Req 1.7 違反）"
    );
}

/// 専用 target を `info` で開けても出ない＝観測点は `debug!` 水準に置かれている。
#[test]
fn diag_records_are_silent_when_target_opened_only_to_info() {
    assert_eq!(
        emit_all_under_filter("info,areka::placement::diag=info"),
        "",
        "debug! ではない水準の観測点が混ざっている（Req 1.7・水準割当表）"
    );
}

/// 手順書の directive（`areka::placement::diag=debug`）で点灯する
/// ＝target のリテラルが手順書と 1:1 で結ばれていることの機械的証明（Req 1.5/1.7）。
#[test]
fn diag_records_light_up_under_the_procedure_directive() {
    let out = emit_all_under_filter("info,areka::placement::diag=debug");
    assert!(
        out.contains(MONITOR_SNAPSHOT_TAG)
            && out.contains(MONITOR_RECORD_TAG)
            && out.contains(WINDOW_MOVE_RECORD_TAG)
            && out.contains(ZORDER_PAIR_DECLARED_TAG)
            && out.contains(ZORDER_PAIR_STRATEGY_TAG),
        "手順書の RUST_LOG で診断レコードが点灯しない: {out}"
    );
    assert!(
        out.contains(DIAG_TARGET),
        "出力に専用 target が現れない（target 指定漏れ）: {out}"
    );
}
