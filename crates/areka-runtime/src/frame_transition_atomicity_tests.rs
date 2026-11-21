//! 遷移の原子性と随伴を多フレーム駆動で固定する。
//!
//! # 何を固定するのか
//!
//! 駆動は**経路 (b)＝表更新が先**である。実行時のモニタ表と
//! 窓の拡大率を同じフレームへ流し込み、同期段 → 拡大率の相 → 作業領域再スナップ →
//! 連鎖の解き直しを本番と同じ順で 1 フレームぶん回す。そのうえで次を主張する:
//!
//! | 主張 | 判定量 |
//! |---|---|
//! | 全窓の書込が同一フレーム | 起点フレームより後に書込を持つフレームが無い（[`TRANSITION_FRAME_BOUND`]） |
//! | キャラ窓 1 回・バルーン窓 1 回 | 窓ごとの指令数（`WRITES_PER_WINDOW_MAX`） |
//! | 同期書込（経路 A）0 回 | `origin=dpi-suggested` の件数（[`PATH_A_WRITES_MAX`]） |
//! | 随伴バルーンが同一フレームで窓相対へ | 同一 drain にバルーンの指令が居る＋追従 offset が表示 DPI 比で追随 |
//! | 遷移中の接地点が規約値から外れない | `kind=ground` の `diff`（[`GROUND_DIFF_MAX`]）＋書込 1 本の下端 |
//! | 拡大率 120／192・複数モニタの作業領域 | [`transition_is_atomic_at_120`]／[`transition_is_atomic_at_192`] |
//!
//! # 判定器（`summarize`）をそのまま回さない理由
//!
//! 判定器の `writes_per_window`／`frames_to_last_write` が数える `kind=write` レコードは
//! 一括 flush（実 `SetWindowPos` を呼ぶ経路）が出すもので、決定論テストはそこを通さずキューを
//! 実行せずに取り出す（[`FrameHarness::drain_writes`]）。この 2 量の集計は本ファイルがキューの
//! 中身で行う。ただし固定する値は判定器の `pub const` を引く（回帰テストと実機サインオフが
//! 別々の数字を持つと、片方だけ緩めたときに静かに食い違う）。接地点は判定器の解析器
//! （`parse_transition_line`）を通し、`kind=ground` 行から `diff` を読む。
//!
//! # 零件の主張には陽性の対を置く（同じテスト本体の内側）
//!
//! 零件の主張は駆動が死んでいれば空虚に緑になるので、対を同じ本体へ置く:
//!
//! - 「経路 A の書込 0」→ 末尾で**本番の行組立**（`write_line`）を通した `stage=sync`／
//!   `origin=dpi-suggested` の観測行を 1 本流し、**同じ述語**がそれを拾うことを見る。
//! - 「起点フレームより後は書込 0」→ 起点フレームで 4 本の指令が出ていることが対である。
//! - 「整合待ちが 1 件も付かない」→ 同上。待ちが起きていれば書込は 0 になるので、
//!   「4 本出た」が待ちの不在を陽に裏づける。
//!
//! # 経路 A はキューを通らない——数えるのは観測行の側である
//!
//! 経路 A（`WM_DPICHANGED` 受理時の同期書込）は
//! `crates/wintf/src/ecs/window_proc/window_pos.rs` が `guarded_set_window_pos` を**直接**呼び、
//! `SetWindowPosCommand` のキューを通らない。`origin=dpi-suggested` が現れるのは同じ処理が組む
//! 観測行（`write_line`）**だけ**で、指令のタグにこの語を載せる経路は本番に無い
//! （タグを付けるのは `follow/window_move.rs` の `enqueue_window_set_pos`・wintf の `apply_window_pos_changes`・`pair_fix_command`）。ゆえに
//! **キュー上で `tag.origin` を数える形は、本番のどんな退行でも赤にならない**。本ファイルは
//! 観測行を判定器の `summarize` へ通し、`origin` で数える `path_a_writes` と `stage` で数える
//! `sync_stage_writes` を別々に読む。
//!
//! この 0 が言っているのは「遷移フレームの観測窓に経路 A の行が 1 本も出ていない」までである。
//! 本ハーネスは window_proc を駆動しないので、実機で経路 A が 0 回であること自体は証明できない。
//! 実機側の 0 の持ち主は実機サインオフ（判定器が `Bounds::deterministic` の
//! `PATH_A_WRITES_MAX` で判定する）である。

use areka_emo_compose::ScaleRatio;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER};

use wintf::ecs::window::transition_diag::{
    FIELD_SCOPE, ORIGIN_DPI_SUGGESTED, STAGE_SYNC, Stamp, WriteRecord, WriteStage, WriteTag,
    emit_line, write_line,
};

use crate::placement_tests::transition_judge::{
    GROUND_DIFF_MAX, PATH_A_WRITES_MAX, TRANSITION_FRAME_BOUND, TransitionRecord,
    parse_transition_line, summarize,
};
use areka_placement::chain_finalize::ChainFinalized;
use areka_placement::chain_realign::ChainRealignPending;
use areka_placement::diag::WindowKind;
use areka_placement::dpi_sync::DpiSyncHold;
use areka_placement::follow::BalloonFollow;
use areka_placement::resolver::PointPx;
use areka_placement::transition_diag::{FIELD_DIFF, KIND_GROUND};
use log_capture_kit::{CapturedEvent, capture};

use super::shared_test_support::{BALLOON_SPAWN_SIZE, HIGH_DPI, LOW_DPI, SCALE_120};
use super::test_support::{
    FakeReports, FrameHarness, PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, pos_of,
    s2_assert_work_area_bottom_moves, s2_work_area_for_dpi, settled_sizes,
};
use super::transition_test_support::{
    assert_writes_per_window_within_bound, scaled, settle_at, writes_for,
};
use super::{balloon_target, shell_target};

/// 起点フレームの後に「もう書かない」を見るために回す定常フレーム数。
const STEADY_FRAMES: u32 = 4;

/// 経路 A の陽性の対で使う偽 HWND（ゴースト窓のどの handle とも重ならない値）。
const PATH_A_PROBE_HWND: usize = 0x9_9990;

// ---------------------------------------------------------------------------
// 合成寸（拡大率の関数として組む・絶対 px を判定へ直書きしない）
// ---------------------------------------------------------------------------

/// 当該水準の scope0／scope1 のキャラ窓物理寸。
fn char_sizes(dpi: u16) -> [(u32, u32); 2] {
    [scaled(SPAWN_SIZE_0, dpi), scaled(SPAWN_SIZE_1, dpi)]
}

/// 当該水準の連鎖再解決が読む実表示寸（キャラ窓のみ）。
fn chain_sizes(dpi: u16) -> PerTargetSizes {
    let [size0, size1] = char_sizes(dpi);
    PerTargetSizes::new([(0, Some(size0)), (1, Some(size1))])
}

// ---------------------------------------------------------------------------
// 指令の選り分け
// ---------------------------------------------------------------------------

/// 捕捉行を判定器の `summarize` へ通し、経路 A（`WM_DPICHANGED` 受理時の同期書込）の書込行の件数を
/// `(path_a_writes, sync_stage_writes)`＝`(origin=dpi-suggested の件数, stage=sync の件数)` で返す。
/// 片方へ均すと食い違いが見えなくなるので、別々に返す。キューではなく観測行を見る理由は
/// モジュール doc の「経路 A はキューを通らない」を参照。
fn path_a_write_lines(events: &[CapturedEvent]) -> (u32, u32) {
    let records: Vec<TransitionRecord> = events
        .iter()
        .filter_map(|event| parse_transition_line(event.message()))
        .collect();
    let summary = summarize(&records.iter().collect::<Vec<_>>());
    (summary.path_a_writes, summary.sync_stage_writes)
}

/// 捕捉行のうち `kind=ground` を判定器の解析器へ通し、`(scope, diff, frame)` を取り出す。
///
/// 解析器を通すのは、行の語彙が判定器の期待と食い違ったら**ここで落ちる**ようにするため
/// である（`is_well_formed` が必須フィールドの欠落を拾う）。
fn ground_records(events: &[CapturedEvent]) -> Vec<(u32, i32, u32)> {
    events
        .iter()
        .filter_map(|event| parse_transition_line(event.message()))
        .filter(|record| record.kind == KIND_GROUND)
        .map(|record| {
            assert!(
                record.is_well_formed(),
                "接地点レコードが判定器の語彙規約を破っている: {:?}",
                record.defects
            );
            assert!(
                record.has_frame_stamp(),
                "接地点レコードの frame が読めない（縮退値 0 と区別が付かない）: {record:?}"
            );
            (
                record
                    .int_field::<u32>(FIELD_SCOPE)
                    .expect("接地点レコードは scope を持つ"),
                record
                    .int_field::<i32>(FIELD_DIFF)
                    .expect("接地点レコードは diff を持つ"),
                record.stamp.frame,
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 駆動の型（起動 → 遷移 → 定常）
// ---------------------------------------------------------------------------

/// 等倍で整地し、起動時の連鎖確定まで済ませる。窓書込のキューと witness は末尾でもう一度
/// 掃除するので、以後の主張は**遷移が起こした書込だけ**を見る。
fn boot_at_base(harness: &mut FrameHarness, source: &mut FakeReports) {
    settle_at(harness, source, LOW_DPI);
    harness.run_chain_finalize(&settled_sizes());
    assert!(
        harness.world.get_resource::<ChainFinalized>().is_some(),
        "前提が崩れている: 起動時の連鎖確定が駆動していない（遷移後の解き直しが武装しない）"
    );
    let _priming = harness.drain_writes();
    harness.reset_write_witness();
}

/// OS 設定の拡大率変更を**表更新が先**の順序で 1 フレームぶん流し込む（経路 (b)）。
///
/// 実行時のモニタ表（`Monitor` entity 群）と窓の拡大率を置いてからフレームを進める。作業
/// 領域源には触らない——**同期段が表から作り直す**のが本番の順序で、それを飛ばすと「表更新が先」を
/// 検証したことにならない。戻り値は（同期段が作業領域源を差し替えたか, 捕捉した観測行）。
fn apply_scale_change(
    harness: &mut FrameHarness,
    source: &mut FakeReports,
    dpi: u16,
) -> (bool, Vec<CapturedEvent>) {
    let [size0, size1] = char_sizes(dpi);
    harness.set_monitor_table_for_dpi(dpi);
    harness.set_window_dpi(dpi);
    // 4 窓すべてに再表示の報告を与える——バルーン窓へ**自分の**寸法変更が来る状態でこそ、
    // 同一窓へ 2 つの指令が積まれ、合流（`coalesce_geometry`）が効いているかが観測できる。
    source.refresh.insert(shell_target(0).0, size0);
    source.refresh.insert(shell_target(1).0, size1);
    source
        .refresh
        .insert(balloon_target(0).0, scaled(BALLOON_SPAWN_SIZE, dpi));
    source
        .refresh
        .insert(balloon_target(1).0, scaled(BALLOON_SPAWN_SIZE, dpi));
    let sizes = chain_sizes(dpi);
    capture(|| {
        harness.advance_frame();
        let change = harness.run_placement_phases(source);
        harness.run_chain_realign(&sizes);
        change.is_some()
    })
}

// ---------------------------------------------------------------------------
// 本体
// ---------------------------------------------------------------------------

/// 1 水準ぶんの原子性検査。
///
/// 呼び出し側を水準ごとの `#[test]` に分けてあるのは、落ちたときに**どちらの水準か**が
/// テスト名で判るようにするためである（1 本のループにすると先に落ちた側で止まり、もう
/// 一方が検査されたかどうかが結果から読めない）。
fn transition_is_atomic_at(dpi: u16) {
    // 探針の非退化: この 2 水準で作業領域下端が実際に動く（動かなければ接地点の主張は空虚）。
    s2_assert_work_area_bottom_moves(LOW_DPI, dpi);
    let [size0, size1] = char_sizes(dpi);
    assert_ne!(
        (size0, size1),
        (SPAWN_SIZE_0, SPAWN_SIZE_1),
        "探針が退化している: dpi={dpi} で窓寸が等倍と同じ（遷移が起きない）"
    );

    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_at_base(&mut harness, &mut source);

    let scopes = harness.scopes().to_vec();
    let char0 = harness.char_window(0);
    let balloon_offsets_before: Vec<(usize, PointPx)> = scopes
        .iter()
        .map(|&scope| {
            let offset = harness
                .world
                .get::<BalloonFollow>(harness.char_window(scope))
                .expect("char 窓は BalloonFollow を持つ")
                .offset();
            (scope, offset)
        })
        .collect();
    let balloon_pos_before: Vec<(usize, i32)> = scopes
        .iter()
        .map(|&scope| {
            (
                scope,
                pos_of(&harness.world, harness.balloon_window(scope))
                    .expect("balloon 位置がある")
                    .y,
            )
        })
        .collect();
    let ground_x_before = harness.ground_point(0).0;

    // ── 起点フレーム（表更新が先） ────────────────────────────────────────
    let (sync_rebuilt, events) = apply_scale_change(&mut harness, &mut source, dpi);
    let origin_frame = harness.frame();
    let new_bottom = s2_work_area_for_dpi(dpi).bottom;

    assert!(
        sync_rebuilt,
        "同期段が作業領域源を作り直していない（表更新が先の順序になっていない）"
    );
    assert_eq!(
        harness
            .work_area_source()
            .expect("作業領域源がある")
            .work_areas[0]
            .bottom,
        new_bottom,
        "作業領域源が新しい水準の下端になっていない（拡大率の相が旧下端を読む）"
    );

    // ── 窓ごとの書込回数 ───────────────────────────────────────
    let writes = harness.drain_writes();
    assert_eq!(
        writes.len(),
        scopes.len() * 2,
        "遷移フレームの窓書込が 4 本（2 スコープ × キャラ／バルーン）ではない: {writes:?}"
    );
    for &scope in &scopes {
        let scope = scope as u32;
        for kind in [WindowKind::Char, WindowKind::Balloon] {
            assert_writes_per_window_within_bound(
                &writes,
                scope,
                kind,
                &format!("dpi={dpi} scope={scope} {kind}"),
            );
        }
    }

    // ── 同期書込（経路 A）0 回 ─────────────────────────────────
    // 数えるのは**観測行**である（経路 A はキューを通らない）。2 量とも判定器と同じ述語。
    let (by_origin, by_stage) = path_a_write_lines(&events);
    // 上限は u32 の 0 なので `<=` は clippy の absurd_extreme_comparisons（deny）に掛かる。
    assert!(
        by_origin.cmp(&PATH_A_WRITES_MAX).is_le() && by_stage.cmp(&PATH_A_WRITES_MAX).is_le(),
        "OS 提案位置の同期書込（origin={ORIGIN_DPI_SUGGESTED}／stage={STAGE_SYNC}）の観測行が遷移フレームに出ている（origin={by_origin}・stage={by_stage}）"
    );

    // ── 中間矩形を出さない（1 本の書込が新しい規約値の下端に載る） ──
    for &scope in &scopes {
        let scope = scope as u32;
        let char_write = writes_for(&writes, scope, WindowKind::Char)
            .pop()
            .expect("キャラ窓の書込がある");
        assert_eq!(
            char_write.y + char_write.height,
            new_bottom,
            "dpi={dpi} scope={scope}: キャラ窓の書込が新しい作業領域下端に載っていない（遷移前後のどちらでもない矩形）"
        );
        assert_eq!(
            harness.ground_point(scope as usize).1,
            new_bottom,
            "dpi={dpi} scope={scope}: 遷移後の接地点が規約値（作業領域下端）から外れている"
        );
    }
    assert_eq!(
        harness.ground_point(0).0,
        ground_x_before,
        "dpi={dpi}: 連鎖の起点スコープの接地点 X が動いた（下端中央が保たれていない）"
    );

    // ── 接地点レコードの diff（判定器が読む行の上での 0） ────────────
    let grounds = ground_records(&events);
    assert_eq!(
        grounds.len(),
        scopes.len(),
        "接地点レコードがスコープ数ぶん出ていない（観測が死んでいる／窓が書かれていない）: {grounds:?}"
    );
    for (scope, diff, frame) in &grounds {
        assert!(
            diff.abs() <= GROUND_DIFF_MAX,
            "dpi={dpi} scope={scope}: 接地点と作業領域下端の差が {GROUND_DIFF_MAX} を超えている（diff={diff}）"
        );
        assert_eq!(
            *frame, origin_frame,
            "dpi={dpi} scope={scope}: 接地点レコードが起点フレームのものでない（frame={frame}）"
        );
    }

    // ── 随伴バルーンが同一フレームで窓相対へ移る ─────────────────────
    //
    // 拡大率遷移では追従オフセットが表示 DPI 比で追随する。作業領域の再スナップ単独では
    // 追従オフセットは変わらず、それは `frame_transition_branch_tests.rs` の
    // `a_work_area_only_change_writes_once` が固定する。追従の基準は窓（char 左上）相対である。
    // `balloon_offsets_before` は書込の**前**に読むので恒真ではなく、追随が抜ければ赤になる。
    let follow_ratio = ScaleRatio::new(dpi.into(), LOW_DPI.into()).expect("非ゼロ比");
    for (scope, before) in &balloon_offsets_before {
        let char_window = harness.char_window(*scope);
        let balloon = harness.balloon_window(*scope);
        // 前読み値から**独立に**導いた期待値（丸めは既存の単一権威 `scale_len` へ委譲）。
        let axis = |v: i32| {
            let magnitude = follow_ratio.scale_len(v.unsigned_abs()) as i32;
            if v < 0 { -magnitude } else { magnitude }
        };
        let expected = (axis(before.x), axis(before.y));
        // 非空虚性: 期待値が前読み値と異なる（＝「追随した」が「据え置き」と区別できる）。
        assert_ne!(
            expected,
            (before.x, before.y),
            "dpi={dpi} scope={scope}: 期待値が前読み値と同じ（追随の主張が空虚）"
        );
        let char_pos = pos_of(&harness.world, char_window).expect("char 位置がある");
        let balloon_pos = pos_of(&harness.world, balloon).expect("balloon 位置がある");
        assert_eq!(
            (balloon_pos.x - char_pos.x, balloon_pos.y - char_pos.y),
            expected,
            "dpi={dpi} scope={scope}: 随伴恒等式 balloon − char ≡ 追随後の BalloonFollow.offset が崩れている"
        );
        let after = harness
            .world
            .get::<BalloonFollow>(char_window)
            .expect("char 窓は BalloonFollow を持つ")
            .offset();
        assert_eq!(
            (after.x, after.y),
            expected,
            "dpi={dpi} scope={scope}: 追従オフセットが表示 DPI 比で追随していない\
             （拡大率遷移では追従オフセットも表示 DPI 比で伸びる）"
        );
    }
    // 「相対不変」が「何も動かなかった」の言い換えに退化していないこと。
    for (scope, before_y) in &balloon_pos_before {
        assert_ne!(
            pos_of(&harness.world, harness.balloon_window(*scope))
                .expect("balloon 位置がある")
                .y,
            *before_y,
            "dpi={dpi} scope={scope}: バルーンの絶対位置が動いていない（恒等式が空虚に成立している）"
        );
    }

    // ── 合流の非空虚性: 畳まれた 1 本が最終ジオメトリを持つ ────────────
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_none(),
        "dpi={dpi}: 連鎖の解き直しが同一フレームで解決していない（武装が残っている）"
    );
    let scope0_x = pos_of(&harness.world, char0).expect("char0 位置がある").x;
    let scope1_write = writes_for(&writes, 1, WindowKind::Char)
        .pop()
        .expect("scope1 のキャラ窓の書込がある");
    assert_eq!(
        (scope1_write.x, scope1_write.width),
        (scope0_x - size1.0 as i32, size1.0 as i32),
        "dpi={dpi}: 畳まれた 1 本が最終ジオメトリ（解き直し後の位置＋遷移後の寸）を持っていない: {writes:?}"
    );

    // ── この順序では待たない（対は「4 本の書込が出たこと」） ───
    for &scope in &scopes {
        for window in [harness.char_window(scope), harness.balloon_window(scope)] {
            assert!(
                harness.world.get::<DpiSyncHold>(window).is_none(),
                "dpi={dpi} scope={scope}: 表更新が先の順序なのに整合待ちの札が付いた"
            );
        }
    }

    // ── 有界のフレーム数（起点フレームで終わる） ─────────────────────
    let sizes = chain_sizes(dpi);
    let mut frames_to_last_write = 0u32;
    for offset in 1..=STEADY_FRAMES {
        harness.advance_frame();
        harness.run_placement_phases(&mut source);
        harness.run_chain_realign(&sizes);
        if !harness.drain_writes().is_empty() {
            frames_to_last_write = offset;
        }
    }
    // 上限は u32 の 0 なので `<=` は clippy の absurd_extreme_comparisons（deny）に掛かる。
    assert!(
        frames_to_last_write.cmp(&TRANSITION_FRAME_BOUND).is_le(),
        "dpi={dpi}: 遷移が起点フレームで終わっていない（最後の書込が {frames_to_last_write} フレーム後・上限 {TRANSITION_FRAME_BOUND}）"
    );

    // ── 経路 A 0 件の陽性の対（同じ述語・同じ本体） ───────────────────────
    //
    // 行は**本番の組立関数**（`write_line`）へ通す——テストが字面を手で組むと、発行側が
    // 書式を変えても対照だけが生き残って「上の 0 件は空虚」を隠してしまう。
    // scope と win_kind は番兵である。
    let (_probe, probe_events) = capture(|| {
        emit_line(&write_line(&WriteRecord {
            stamp: transition_diag_stamp(origin_frame),
            stage: WriteStage::Sync,
            seq: 0,
            hwnd: HWND(PATH_A_PROBE_HWND as *mut _),
            tag: WriteTag {
                origin: ORIGIN_DPI_SUGGESTED,
                ..WriteTag::UNTAGGED
            },
            x: 0,
            y: 0,
            cx: 0,
            cy: 0,
            flags: (SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE).0,
            after: None,
            call_us: 0,
            ok: true,
            in_batch: false,
        }));
    });
    assert_eq!(
        path_a_write_lines(&probe_events),
        (1, 1),
        "経路 A の述語が壊れている（本番の `write_line` が組んだ行を拾えない）——上の 0 件は空虚"
    );
}

/// 陽性の対で使う刻印（起点フレームと同じ番号・経過は 0）。
///
/// 判定器は差分でしか読まないので経過の値は判定へ入らない。
fn transition_diag_stamp(frame: u32) -> Stamp {
    Stamp { frame, t_us: 0 }
}

/// 拡大率 120 での遷移が原子的である。
#[test]
fn transition_is_atomic_at_120() {
    transition_is_atomic_at(SCALE_120);
}

/// 拡大率 192 での遷移が原子的である。
#[test]
fn transition_is_atomic_at_192() {
    transition_is_atomic_at(HIGH_DPI);
}
