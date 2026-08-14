//! 起動時関門 [`apply_balloon_limit`] の決定論檻
//! （areka-P0-windowposition-limit task 2.4・design「Testing Strategy / Unit Tests 5」・
//! 要件 2.2／4.7／4.9／5.5／6.1）。
//!
//! クランプ**式**そのものの行列（4 辺 × k=1/k≠1 × 逆転区間 × 冪等）は
//! `balloon_limit_tests.rs`（task 1.3）が持つ。本ファイルが固定するのはその 1 段上——
//! **関門の判断分岐**である:
//!
//! - `balloon_limit` の有効判定（要件 2.7 の関門側の腕）
//! - 基準モニタが**キャラ窓**の帰属モニタであること（要件 5.5。バルーン窓ではない）
//! - 補正が `balloon_pos` だけに作用し `balloon_offset` を 1 ビットも動かさないこと（DD6）
//! - 作業領域が決まらない scope の警告付き素通し（design Error Handling）
//! - 補正したときだけ `[balloon-limit] Clamp` を出すこと（要件 6.1）
//!
//! # ログ檻の非空虚性
//!
//! 捕捉ハーネス（`test_support::capture_logs`）は較正済みだが、「◯◯のログが無い」は
//! 捕捉ゼロでも真になってしまう。本ファイルの「無い」側の主張は**必ず同じ捕捉集合から
//! [`expect_one`] で 1 件取り出した後**に行う（`expect_one` は 0 件で panic するので、
//! 捕捉が生きていることがその時点で証明されている）。

use super::super::config::{BalloonXMode, PlacementConfig, ScopeConfig};
use super::super::resolver::{Anchor, ScopeInput, resolve_placement};
use super::super::test_support::{LogEvent, capture_logs, expect_one};
use super::*;

// ---------------------------------------------------------------------------
// 檻の道具
// ---------------------------------------------------------------------------

/// 単一モニタの作業領域（左上原点＝下の複数モニタ表と対比させるため素直な値）。
const WA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 1920,
    bottom: 1080,
};

/// キャラ窓寸（帰属判定に効く＝中心が動く寸法）。
const CSZ: SizePx = SizePx { w: 400, h: 600 };
/// バルーン窓寸。
const BSZ: SizePx = SizePx { w: 200, h: 300 };

fn snapshot(areas: &[RectPx]) -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: areas.to_vec(),
    }
}

fn point(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

/// resolver 出力相当の placement を組む。
///
/// `balloon_pos` は檻側で `char_pos + balloon_offset` から作る——resolver 出力時点の
/// 事後条件を満たした値だけを関門へ渡し、「関門通過**後**に恒等式が崩れる」ことを
/// 各テストが観測できるようにする（DD6 の再スコープ）。
fn placement(
    scope: usize,
    char_pos: PointPx,
    balloon_offset: PointPx,
    balloon_limit: bool,
) -> ScopePlacement {
    ScopePlacement {
        scope,
        char_pos,
        char_size: CSZ,
        balloon_pos: point(char_pos.x + balloon_offset.x, char_pos.y + balloon_offset.y),
        balloon_size: BSZ,
        balloon_offset,
        balloon_limit,
        anchor: Anchor::Free,
    }
}

/// 本文に `needle` を含むイベントが 1 件も無いことを主張する。
///
/// 呼び手は**同じ `events` から [`expect_one`] で 1 件取り出した後**に呼ぶこと
/// （捕捉が生きていることが確認済みの集合に対してのみ「無い」を主張する）。
fn assert_no_event(events: &[LogEvent], needle: &str) {
    let hits: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.message().contains(needle))
        .collect();
    assert!(hits.is_empty(), "`{needle}` が出てはならない: {hits:?}");
}

/// `balloon_pos` 以外の全フィールドが入力と同一であることを主張する（DD6 の中核）。
fn assert_only_balloon_pos_may_differ(before: &ScopePlacement, after: &ScopePlacement) {
    assert_eq!(after.scope, before.scope, "scope は不変");
    assert_eq!(
        after.char_pos, before.char_pos,
        "キャラ窓位置は不変（要件 2.8）"
    );
    assert_eq!(after.char_size, before.char_size, "キャラ窓寸は不変");
    assert_eq!(
        after.balloon_size, before.balloon_size,
        "バルーン窓寸は不変"
    );
    assert_eq!(
        after.balloon_offset, before.balloon_offset,
        "論理相対位置は生値のまま＝補正を焼き付けない（DD6・要件 3.1(d)）"
    );
    assert_eq!(
        after.balloon_limit, before.balloon_limit,
        "limit 解決値は不変"
    );
    assert_eq!(after.anchor, before.anchor, "アンカーは不変");
}

// ---------------------------------------------------------------------------
// 補正の本体（要件 2.2・6.1・DD6）
// ---------------------------------------------------------------------------

/// 保存位置が画面外相当のとき、表示位置は補正済み・相対位置は生値のまま（要件 2.2・DD6）。
///
/// 併せて補正ログ（要件 6.1: scope・補正前後・契機）を固定する。
#[test]
fn boot_gate_clamps_display_position_and_keeps_offset_raw() {
    // キャラ窓は作業領域の左寄り（中心は域内）。バルーンは左辺を 50px はみ出す。
    let before = placement(0, point(30, 300), point(-50, 10), true);
    assert_eq!(
        before.balloon_pos,
        point(-20, 310),
        "前提: 左辺から 20px はみ出す"
    );

    let (out, events) = capture_logs(|| apply_balloon_limit(vec![before], &snapshot(&[WA])));

    assert_eq!(out.len(), 1, "出力長＝入力長");
    assert_eq!(
        out[0].balloon_pos,
        point(0, 310),
        "表示位置は作業領域の左辺へ補正される（要件 2.2）"
    );
    assert_only_balloon_pos_may_differ(&before, &out[0]);
    // 恒等式は関門通過後に崩れる——それが正しい（DD6 の再スコープ）。
    assert_ne!(
        point(
            out[0].balloon_pos.x - out[0].char_pos.x,
            out[0].balloon_pos.y - out[0].char_pos.y
        ),
        out[0].balloon_offset,
        "補正後は balloon_offset ≡ balloon_pos − char_pos が成立しない（DD6）"
    );

    let hit = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(hit.level, tracing::Level::INFO, "補正の記録は info（DD7）");
    assert_eq!(
        hit.field("scope"),
        "0",
        "補正した scope を記録する（要件 6.1）"
    );
    assert_eq!(
        hit.field("from"),
        format!("{:?}", point(-20, 310)),
        "補正前の位置を記録する（要件 6.1）"
    );
    assert_eq!(
        hit.field("to"),
        format!("{:?}", point(0, 310)),
        "補正後の位置を記録する（要件 6.1）"
    );
    assert!(
        hit.field("context").contains(BALLOON_LIMIT_BOOT_CONTEXT),
        "契機（どの関門か）を記録する（要件 6.1）: {}",
        hit.field("context")
    );
}

/// 作業領域内に収まっている scope は補正もログも起こさない（要件 6.1「実際に動かしたとき」）。
///
/// 空虚化を避けるため、**同じ呼び出しの中に補正される scope 1 を混ぜる**——捕捉が
/// 生きていること（Clamp が 1 件出ること）を先に証明したうえで、その 1 件が scope 1 の
/// ものだけであることを示す。
#[test]
fn boot_gate_logs_only_for_scopes_it_actually_moved() {
    let inside = placement(0, point(700, 300), point(-50, 10), true);
    let outside = placement(1, point(30, 300), point(-50, 10), true);
    assert_eq!(inside.balloon_pos, point(650, 310), "前提: scope 0 は域内");

    let (out, events) =
        capture_logs(|| apply_balloon_limit(vec![inside, outside], &snapshot(&[WA])));

    assert_eq!(out[0], inside, "域内の scope は入力に完全恒等（補正なし）");
    assert_eq!(
        out[1].balloon_pos,
        point(0, 310),
        "域外の scope だけが補正される"
    );

    let hit = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(
        hit.field("scope"),
        "1",
        "補正ログは実際に動いた scope の分だけ（域内 scope の分は出ない）"
    );
}

/// `limit=0` の scope は画面外でも補正しない（要件 2.7 の関門側の腕）。
///
/// 捕捉が生きていることは同じ呼び出しの scope 1（limit 有効・同じはみ出し量）の
/// Clamp 1 件で証明する。
#[test]
fn boot_gate_skips_scopes_with_limit_disabled() {
    let disabled = placement(0, point(30, 300), point(-50, 10), false);
    let enabled = placement(1, point(30, 300), point(-50, 10), true);

    let (out, events) =
        capture_logs(|| apply_balloon_limit(vec![disabled, enabled], &snapshot(&[WA])));

    assert_eq!(
        out[0], disabled,
        "limit=0 の scope は画面外のままで入力に完全恒等（要件 2.7）"
    );
    assert_eq!(
        out[1].balloon_pos,
        point(0, 310),
        "limit=1 の scope は補正される"
    );

    let hit = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
    assert_eq!(
        hit.field("scope"),
        "1",
        "limit=0 の scope の補正ログは出ない"
    );
    // limit=0 は縮退ではない（作者の宣言どおりの挙動）ので警告も出さない。
    assert_no_event(&events, BALLOON_LIMIT_UNRESOLVED_TAG);
}

// ---------------------------------------------------------------------------
// 縮退（design Error Handling「起動時関門で work area 解決不能（snapshot 空）」）
// ---------------------------------------------------------------------------

/// モニタ情報が空のときは警告のうえ素通しし、書き込みを阻害しない。
#[test]
fn boot_gate_warns_and_passes_through_when_snapshot_is_empty() {
    let before = placement(0, point(30, 300), point(-50, 10), true);

    let (out, events) = capture_logs(|| apply_balloon_limit(vec![before], &snapshot(&[])));

    assert_eq!(
        out,
        vec![before],
        "作業領域が決まらない scope は入力へ完全恒等＝素通し（架空の領域を発明しない）"
    );

    let hit = expect_one(&events, BALLOON_LIMIT_UNRESOLVED_TAG);
    assert_eq!(hit.level, tracing::Level::WARN, "縮退は warn（log-first）");
    assert_eq!(
        hit.field("scope"),
        "0",
        "どの scope が素通ししたかを記録する"
    );
    assert!(
        hit.field("context").contains(BALLOON_LIMIT_BOOT_CONTEXT),
        "どの関門の縮退かを記録する"
    );
    // 捕捉は生きている（直前の expect_one が 1 件取り出した）＝この主張は空虚でない。
    assert_no_event(&events, BALLOON_LIMIT_CLAMP_TAG);
}

// ---------------------------------------------------------------------------
// 基準モニタ（要件 5.5）
// ---------------------------------------------------------------------------

/// 複数モニタでは**キャラ窓**が属する側の作業領域が基準になる（要件 5.5）。
///
/// 入力は「バルーン窓の矩形は左モニタに完全に収まるが、キャラ窓は右モニタに属する」
/// という配置——基準をバルーン窓（あるいは先頭モニタ）から引く実装なら補正が起きず
/// `None` のまま素通しになるので、この 1 本で取り違えが赤になる。
#[test]
fn boot_gate_uses_work_area_of_monitor_owning_char_window() {
    let left_monitor = WA;
    let right_monitor = RectPx {
        left: 1920,
        top: 0,
        right: 3840,
        bottom: 1080,
    };
    // キャラ窓 (1950,300)-(2350,900)・中心 (2150,600) → 右モニタへ帰属。
    // バルーン (1700,310)-(1900,610) は左モニタに完全に収まる。
    let before = placement(0, point(1950, 300), point(-250, 10), true);
    assert_eq!(before.balloon_pos, point(1700, 310));
    assert_eq!(
        clamp_rect_to_work_area(before.balloon_pos, BSZ, left_monitor),
        before.balloon_pos,
        "前提: 左モニタ基準なら無補正（＝取り違えたら結果が変わる入力になっている）"
    );

    let out = apply_balloon_limit(vec![before], &snapshot(&[left_monitor, right_monitor]));

    assert_eq!(
        out[0].balloon_pos,
        point(1920, 310),
        "キャラ窓が属する右モニタの左辺へ補正される（要件 5.5）"
    );
    assert_only_balloon_pos_may_differ(&before, &out[0]);
}

// ---------------------------------------------------------------------------
// キーワード由来の初期位置（要件 4.9）
// ---------------------------------------------------------------------------

/// `windowposition.x` キーワード由来の初期既定位置にも同一規則が掛かる（要件 4.9）。
///
/// 檻の入力を手で組まず **resolver P5 の実出力**（`CenterTop`）を関門へ流すことで、
/// 「キーワード経路も同じ合流点を通る」ことを構造として固定する。`CenterTop` は
/// バルーン下端をシェル画像上端へ接させるため、背の高いシェルでは基本位置が
/// 作業領域の上辺を突き抜ける——これは数値指定では作れない、キーワード固有の
/// はみ出しである。
#[test]
fn boot_gate_clamps_keyword_derived_initial_position() {
    let mut cfg = PlacementConfig {
        scopes: Default::default(),
        zorder_raw: None,
        sticky_window_raw: None,
        shell_dpi_raw: None,
    };
    cfg.scopes.insert(
        0,
        ScopeConfig {
            balloon_x_mode: BalloonXMode::CenterTop,
            ..ScopeConfig::default()
        },
    );
    let placements = resolve_placement(
        &cfg,
        WA,
        &[ScopeInput {
            scope: 0,
            // 作業領域の高さ 1080 のほとんどを占める背の高いシェル。
            char_size: SizePx { w: 400, h: 1000 },
            balloon_size: BSZ,
        }],
    );
    let before = placements[0];
    assert_eq!(before.char_pos, point(1520, 80), "前提: P1/P2 の既定位置");
    assert_eq!(
        before.balloon_pos,
        point(1620, -220),
        "前提: 中央上の基本位置は作業領域の上辺を突き抜ける（resolver はクランプしない）"
    );

    let out = apply_balloon_limit(placements, &snapshot(&[WA]));

    assert_eq!(
        out[0].balloon_pos,
        point(1620, 0),
        "キーワード由来の初期位置にも同一のクランプ規則が掛かる（要件 4.9）"
    );
    assert_only_balloon_pos_may_differ(&before, &out[0]);
    assert_eq!(
        out[0].balloon_offset,
        point(100, -300),
        "キーワードが定めた論理相対位置は生値のまま（DD6）"
    );
}

// ---------------------------------------------------------------------------
// 4 辺 × 4 隅で論理相対位置が 1 ビットも動かない（DD6）
// ---------------------------------------------------------------------------

/// はみ出しの辺・隅の組み合わせによらず、補正は `balloon_pos` だけに作用する（DD6・要件 2.3）。
#[test]
fn boot_gate_never_touches_balloon_offset_on_any_edge() {
    // キャラ窓は常に作業領域の中央付近（中心 (900,600) → 帰属は WA 固定）。
    let char_pos = point(700, 300);
    // (相対位置, 期待表示位置, 説明)
    let table = [
        (point(-750, 0), point(0, 300), "左辺のみ"),
        (point(1100, 0), point(1720, 300), "右辺のみ"),
        (point(0, -320), point(700, 0), "上辺のみ"),
        (point(0, 600), point(700, 780), "下辺のみ"),
        (point(-750, -320), point(0, 0), "左上"),
        (point(-750, 600), point(0, 780), "左下"),
        (point(1100, -320), point(1720, 0), "右上"),
        (point(1100, 600), point(1720, 780), "右下"),
    ];

    for (offset, expected, what) in table {
        let before = placement(0, char_pos, offset, true);
        let out = apply_balloon_limit(vec![before], &snapshot(&[WA]));

        assert_eq!(
            out[0].balloon_pos, expected,
            "{what}: 表示位置は作業領域内へ補正される"
        );
        assert_eq!(
            out[0].balloon_offset, offset,
            "{what}: 論理相対位置は 1 ビットも変わらない（DD6）"
        );
        assert_only_balloon_pos_may_differ(&before, &out[0]);
    }
}

// ---------------------------------------------------------------------------
// limit {0,1} × はみ出し 4 辺（＋4 隅）× k=1／k≠1 の全交差（要件 7.1・task 4.3）
// ---------------------------------------------------------------------------

/// DPI パラメタ水準。96＝k=1 相当、120/144/192＝k≠1 相当。
const GATE_DPIS: [i32; 4] = [96, 120, 144, 192];

/// 論理基準値 → 各 DPI の物理 px（`balloon_limit_tests.rs` と同一の道具・厳密整除を強制）。
///
/// 関門は物理 px しか見ない。同じ論理形状を各水準の物理値で与えたとき期待値も同じ
/// 物理式から出ることを固定する（隠れた `/96` 変換があれば 96 以外の水準で崩れる）。
fn px(logical: i32, dpi: i32) -> i32 {
    assert_eq!(
        (logical * dpi) % 96,
        0,
        "テスト入力は厳密整除になる論理値（4 の倍数）で構築する"
    );
    logical * dpi / 96
}

/// 矩形 (pos, size) が area へ 4 辺内包されているか（檻側の独立実装）。
fn contained(pos: PointPx, size: SizePx, area: RectPx) -> bool {
    pos.x >= area.left
        && pos.y >= area.top
        && pos.x + size.w <= area.right
        && pos.y + size.h <= area.bottom
}

/// 寸法を明示して placement を組む（[`placement`] の DPI 可変版）。
fn placement_sized(
    scope: usize,
    char_pos: PointPx,
    balloon_offset: PointPx,
    balloon_limit: bool,
    char_size: SizePx,
    balloon_size: SizePx,
) -> ScopePlacement {
    ScopePlacement {
        scope,
        char_pos,
        char_size,
        balloon_pos: point(char_pos.x + balloon_offset.x, char_pos.y + balloon_offset.y),
        balloon_size,
        balloon_offset,
        balloon_limit,
        anchor: Anchor::Free,
    }
}

/// 要件 7.1 の行列を**関門の層で**閉じる: `limit {0,1}` × はみ出し方向 4 辺（単独）と
/// 4 隅（複合）× k=1（dpi=96）および k≠1（120/144/192）の全交差。
///
/// クランプ**式**の 4 辺 × k 行列は `balloon_limit_tests.rs` が持つが、そこには
/// `limit` の分岐が無い（design C5「limit=0 は呼び出し側で skip」＝有効判定は関門の
/// 責務）。一方これまでの関門檻は `limit=0` を**1 辺・1 水準ずつ**しか通していなかった
/// （[`boot_gate_skips_scopes_with_limit_disabled`] は左辺・96 のみ）。ゆえに
/// 「limit=0 × 上辺」「limit=0 × k≠1」といった交差セルがどの檻にも無かった。本檻が
/// その交差を 1 本で埋める。
///
/// 各セルは **limit=0 と limit=1 を同じ呼び出しへ同時に**入れる——同一の入力に対して
/// 一方は完全恒等・他方は補正、という対比が同じ捕捉集合の中で成立することが、
/// 「素通しは限界式の失敗ではなく limit の判定によるものだ」ことの証明になる
/// （limit=0 だけを見る檻は、式が壊れていても緑になりうる）。
#[test]
fn boot_gate_limit_arms_cover_every_edge_at_every_dpi_level() {
    // (説明, 相対位置の論理値, 期待表示位置の論理値＝作業領域左上からの変位)
    let table = [
        ("左辺のみ", (-800, 0), (0, 240)),
        ("右辺のみ", (1000, 0), (1720, 240)),
        ("上辺のみ", (0, -280), (760, 0)),
        ("下辺のみ", (0, 580), (760, 780)),
        ("左上", (-800, -280), (0, 0)),
        ("右上", (1000, -280), (1720, 0)),
        ("左下", (-800, 580), (0, 780)),
        ("右下", (1000, 580), (1720, 780)),
    ];

    for dpi in GATE_DPIS {
        // 左上が原点**でない**作業領域（境界の left/top 依存を暴く）。
        let area = RectPx {
            left: px(64, dpi),
            top: px(40, dpi),
            right: px(64 + 1920, dpi),
            bottom: px(40 + 1080, dpi),
        };
        let csz = SizePx {
            w: px(400, dpi),
            h: px(600, dpi),
        };
        let bsz = SizePx {
            w: px(200, dpi),
            h: px(300, dpi),
        };
        // キャラ窓中心が作業領域の中心＝帰属が最近傍フォールバックへ落ちない正常腕。
        let char_pos = point(area.left + px(760, dpi), area.top + px(240, dpi));
        assert!(
            contained(char_pos, csz, area),
            "dpi={dpi}: 前提＝キャラ窓は作業領域に内包（帰属の解決が正常腕で決まる）"
        );

        for (what, (ox, oy), (ex, ey)) in table {
            let offset = point(px(ox, dpi), px(oy, dpi));
            let expected = point(area.left + px(ex, dpi), area.top + px(ey, dpi));

            let disabled = placement_sized(0, char_pos, offset, false, csz, bsz);
            let enabled = placement_sized(1, char_pos, offset, true, csz, bsz);
            // 探針: 入力が本当にはみ出していること（はみ出していなければ limit=1 側も
            // `None` を返し、limit=0 の「動かない」が何も意味しなくなる）。
            assert!(
                !contained(disabled.balloon_pos, bsz, area),
                "dpi={dpi} {what}: 探針の提案位置がはみ出していない（檻が空虚）"
            );

            let (out, events) =
                capture_logs(|| apply_balloon_limit(vec![disabled, enabled], &snapshot(&[area])));

            // limit=0 の腕: 辺・隅・DPI 水準によらず入力へ完全恒等（要件 2.7・5.2）。
            assert_eq!(
                out[0], disabled,
                "dpi={dpi} {what}: limit=0 の scope が補正された（現行挙動が変わっている）"
            );
            // limit=1 の腕: 同じ入力が作業領域内へ補正される（要件 2.3）。
            assert_eq!(
                out[1].balloon_pos, expected,
                "dpi={dpi} {what}: limit=1 の表示位置が期待の補正値でない"
            );
            assert!(
                contained(expected, bsz, area),
                "dpi={dpi} {what}: 期待値そのものが作業領域外（期待値の算出が誤り）"
            );
            assert_eq!(
                out[1].balloon_offset, offset,
                "dpi={dpi} {what}: 論理相対位置は生値のまま（DD6）"
            );

            // 補正ログは実際に動いた scope の分だけ＝limit=0 の腕は 1 行も出さない。
            // `expect_one` が 1 件取り出せている時点で捕捉は生きている（空虚でない）。
            let hit = expect_one(&events, BALLOON_LIMIT_CLAMP_TAG);
            assert_eq!(
                hit.field("scope"),
                "1",
                "dpi={dpi} {what}: limit=0 の scope の補正ログが出ている"
            );
            assert_no_event(&events, BALLOON_LIMIT_UNRESOLVED_TAG);
        }
    }
}

/// 入力の順序と長さは保たれる（下流は index で spawn するため順序が契約）。
#[test]
fn boot_gate_preserves_order_and_length() {
    let input = vec![
        placement(0, point(30, 300), point(-50, 10), true),
        placement(1, point(700, 300), point(-50, 10), false),
        placement(2, point(700, 300), point(0, 600), true),
    ];

    let out = apply_balloon_limit(input.clone(), &snapshot(&[WA]));

    assert_eq!(out.len(), input.len(), "出力長＝入力長");
    assert_eq!(
        out.iter().map(|p| p.scope).collect::<Vec<_>>(),
        vec![0, 1, 2],
        "scope の順序は入力のまま"
    );
}
