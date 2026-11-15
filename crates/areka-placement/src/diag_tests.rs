use bevy_ecs::entity::Entity;
use tracing::Level;

use super::*;
use log_capture_kit::{capture, capture_under_filter};

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

/// 経路ごとの、[`PlacementRoute::ALL`] での番号・表示語・[`PlacementRoute::is_system_reanchor`]
/// の答え（Req 2.4 の結論語彙・D9／D13／D16）。
///
/// 網羅の `match` ゆえ、経路を 1 つ足すとここが組めなくなる。番号を振る人は `ALL` へも同じ
/// 位置に足す（番号が列の位置と合わなければ下のテストが赤になる）。
fn expected(route: PlacementRoute) -> (usize, &'static str, bool) {
    match route {
        PlacementRoute::SpawnInitial => (0, "SpawnInitial", false),
        PlacementRoute::Restore => (1, "Restore", false),
        PlacementRoute::AnchorChange => (2, "AnchorChange", true),
        PlacementRoute::Resnap => (3, "Resnap", true),
        PlacementRoute::DpiReproject => (4, "DpiReproject", true),
        PlacementRoute::KeepPositionResize => (5, "KeepPositionResize", false),
        PlacementRoute::BalloonFollow => (6, "BalloonFollow", false),
        PlacementRoute::ReportedSizeReconcile => (7, "ReportedSizeReconcile", true),
        PlacementRoute::MoveCue => (8, "MoveCue", false),
        PlacementRoute::BalloonLimitRelease => (9, "BalloonLimitRelease", false),
        PlacementRoute::WorkAreaResnap => (10, "WorkAreaResnap", true),
        PlacementRoute::ChainRealign => (11, "ChainRealign", true),
        PlacementRoute::PatternMove => (12, "PatternMove", true),
    }
}

/// `ALL` の各要素の番号が 0〜N−1 をちょうど 1 度ずつ埋める（列の位置＝表の番号）。
///
/// 表の番号が重なる・`ALL` に同じ経路が 2 度ある・並びが表と違う、のどれでも赤になる。
#[test]
fn placement_route_all_lists_every_route_once_in_order() {
    for (i, route) in PlacementRoute::ALL.into_iter().enumerate() {
        let (n, _, _) = expected(route);
        assert_eq!(n, i, "ALL の {i} 番目の {route} は表では {n} 番");
    }
}

/// 各経路の表示語と再据え付けの判定が表の答えと等しく、表示語が重ならない。
#[test]
fn placement_route_answers_match_the_table() {
    for route in PlacementRoute::ALL {
        let (_, word, reanchor) = expected(route);
        assert_eq!(route.as_str(), word, "{route:?} の表示語が表と違う");
        assert_eq!(
            route.is_system_reanchor(),
            reanchor,
            "{route:?} の再据え付けの判定が表と違う"
        );
    }
    let mut names: Vec<&str> = PlacementRoute::ALL.iter().map(|r| r.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(
        names.len(),
        PlacementRoute::ALL.len(),
        "表示語が重複している: {names:?}"
    );
}

/// 経路語は**行の値として**成立していなければならない（要件 2.7・判定側の辞書化規則）。
///
/// tools/perf/ の Python の道具の `parse_fields` と同形の切り出しは `名前=値` を空白で割る
/// ため、値に空白が混じると `origin=` の値が途中で切れて経路の名指しが壊れる。番兵（`-`）と同じ字面も
/// 「経路が判らなかった書込」と衝突するので禁じる。
#[test]
fn placement_route_words_are_single_tokens_and_never_the_sentinel() {
    for route in PlacementRoute::ALL {
        let word = route.as_str();
        assert!(
            !word.is_empty() && !word.contains(char::is_whitespace),
            "経路語に空白が混じっている（`名前=値` の切り出しが壊れる）: {word:?}"
        );
        assert_ne!(
            word,
            wintf::ecs::window::transition_diag::MISSING,
            "経路語が番兵と同じ字面（タグ付け忘れと区別が付かない）"
        );
    }
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
    let (_, events) = capture(|| log_zorder_pair_strategy(strategy));
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
    let (_, events) = capture(|| log_zorder_pair_declared(2, char_entity, balloon_entity));
    assert_eq!(events.len(), 1, "1 ペア 1 レコード: {events:?}");
    assert_eq!(
        events[0].message(),
        zorder_pair_declared_line(2, char_entity, balloon_entity)
    );
    assert_eq!(events[0].level, Level::DEBUG);
}

// ------------------------------------------------------------------
// 出力（debug! 水準・専用 target）
// ------------------------------------------------------------------

/// `log_monitor_snapshot` は見出し 1 行＋モニタ 1 台 1 行を `debug!` で出し、
/// 本文は純関数の組立結果と**一致**する（ログと純関数の二重実装を許さない）。
#[test]
fn log_monitor_snapshot_emits_header_and_one_line_per_monitor_at_debug() {
    let monitors = [rec(1, true), rec(2, false)];
    let (_, events) = capture(|| log_monitor_snapshot(&monitors, "prepare_ghost_windows"));

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
    let (_, events) = capture(|| log_monitor_snapshot(&[], "monitor_snapshot"));
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
    let (_, events) = capture(|| log_window_move(&record));
    assert_eq!(events.len(), 1, "1 移動 1 レコード: {events:?}");
    assert_eq!(events[0].message(), window_move_record_line(&record));
    assert_eq!(events[0].level, Level::DEBUG);
}

// ------------------------------------------------------------------
// Req 1.7: 既定では 1 行も出ない・手順書の RUST_LOG でのみ点灯
// ------------------------------------------------------------------

/// 与えた `RUST_LOG` 相当の directive で本モジュールの全出力を実際に濾して集める
/// （捕捉の窓が生きていなければ `capture_under_filter` が panic する＝「出ない」が空振りしない）。
fn emit_all_under_filter(directives: &str) -> String {
    capture_under_filter(directives, || {
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
    })
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
        emit_all_under_filter("info,areka_placement::diag=info"),
        "",
        "debug! ではない水準の観測点が混ざっている（Req 1.7・水準割当表）"
    );
}

/// 手順書の directive（`areka_placement::diag=debug`）で点灯する
/// ＝target のリテラルが手順書と 1:1 で結ばれていることの機械的証明（Req 1.5/1.7）。
#[test]
fn diag_records_light_up_under_the_procedure_directive() {
    let out = emit_all_under_filter("info,areka_placement::diag=debug");
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
