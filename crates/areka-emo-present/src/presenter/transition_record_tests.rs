//! サーフェス記録（`kind=surface`）の檻——語彙の逐語固定・前置ガードの本文走査・実 `apply_show`
//! 経由の配線検査（design.md C3・§Testing Strategy「Integration Tests」項目 8）。
//!
//! 本ファイルが固定する契約は 4 つである。
//!
//! 1. **語彙**（Requirement 2.7・7.4）: `surface_line` の字面。正例の逐語再現と、判定語を壊した
//!    入力が語彙照合で落ちること（負例）の両方を持つ。
//! 2. **前置ガード**（Requirement 10.4）: 発行点はレコードを**組む前**に
//!    [`transition_diag::is_enabled`] で分岐する。発行が `debug!` である以上、ログ濾過テストは
//!    ガードの有無を**原理的に区別できない**（濾過されるかどうかは同じ結果になる）ため、
//!    ここだけは本文走査で固定する。`recompose-budget` が成立させた定常アロケーション 0 を
//!    守っているのはこの走査 1 本である。
//! 3. **刻印の出所**（D1）: 発行点は World 資源から刻印を組む。スレッド局所ミラー
//!    （`transition_diag::stamp()`）は World を借りられない点の専用口であり、`&mut World` を
//!    受け取る本 crate の観測点がこれを読むのは退行である。多スレッド実行器では読めない値に
//!    なるが、**その退行はログ捕捉テストでは緑のまま通る**（本 crate のテストは UI スレッド上で
//!    走るため）ゆえ、これも本文走査で固定する。
//! 4. **配線**（Requirement 2.2・2.8・4.6）: 実 `apply_show`／`refresh_scale` を駆動して、寸が
//!    変わらない定常フレームでは 1 行も出ず、遷移では `upload`／`visualize` が出て、見送りでは
//!    `skipped` が理由つきで出て、いずれも perf 行と同一フレーム番号で突合できること。
//!
//! # 「出ないこと」の主張は同一スコープ内の陽性と対にする
//!
//! 陰性主張（定常フレームで `surface` 行 0 本）は、**同一の `with_default` スコープ内で**
//! perf 行 1 本を観測したうえで成立させる。捕捉が死んでいるだけで緑になる形にしない
//! （`presenter_perf_log_tests.rs` の前例と同じ規律）。

use super::*;

use std::time::{Duration, Instant};

use areka_actor::reply_channel;
use areka_emo_compose::{BindSet, PatternState};

use super::super::EmoPresenter;
use super::super::test_support::{
    CaptureSubscriber, CapturedEvent, build_target_assets, make_world_with_gpu, set_window_dpi,
    spawn_window_with_dpi,
};
use crate::command::{PresentCommand, PresentOutcome};

// ── 語彙の読み口 ───────────────────────────────────────────────────────────────────────

/// 檻が渡す刻印（本番は [`stamp_of`] が `FrameCount`＋`TickStart` から組む）。
fn fixed_stamp(frame: u32, t_us: u64) -> Stamp {
    Stamp { frame, t_us }
}

/// 行のフィールド名を出現順に読む（値に空白を含まない契約に依る）。
fn field_names(line: &str) -> Vec<&str> {
    line.split_whitespace()
        .filter_map(|token| token.split_once('='))
        .map(|(name, _)| name)
        .collect()
}

/// 行から 1 フィールドの値を引く（無ければ落ちる）。
fn field_value<'a>(line: &'a str, name: &str) -> &'a str {
    line.split_whitespace()
        .filter_map(|token| token.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("行にフィールド `{name}` が無い: {line}"))
}

// ── 語彙の逐語固定（Requirement 2.7）────────────────────────────────────────────────────

/// `stage=upload` の行は寸と `resized` を持ち、`reason` は番兵で埋まる。
#[test]
fn the_upload_line_carries_the_size_and_the_resize_verdict() {
    let line = surface_line(&SurfaceRecord {
        stamp: fixed_stamp(41, 1234),
        stage: SurfaceStage::Upload,
        target_id: TargetId(3),
        size: Some((480, 360)),
        resized: Some(true),
        reason: None,
    });

    assert_eq!(
        line,
        "[transition] frame=41 t_us=1234 kind=surface stage=upload target_id=3 \
         w=480 h=360 resized=true reason=-"
    );
}

/// `stage=visualize` の行は寸だけを持ち、`resized`／`reason` は番兵で埋まる。
///
/// `resized` を `false` へ潰さないのは、「寸は変わらなかった」と「この段階では測っていない」を
/// 同じ字面にしないためである。
#[test]
fn the_visualize_line_leaves_the_resize_verdict_as_a_sentinel() {
    let line = surface_line(&SurfaceRecord {
        stamp: fixed_stamp(41, 2001),
        stage: SurfaceStage::Visualize,
        target_id: TargetId(3),
        size: Some((480, 360)),
        resized: None,
        reason: None,
    });

    assert_eq!(
        line,
        "[transition] frame=41 t_us=2001 kind=surface stage=visualize target_id=3 \
         w=480 h=360 resized=- reason=-"
    );
}

/// `stage=skipped` の行は理由だけを持ち、寸の 2 フィールドは番兵で残る（落とさない）。
#[test]
fn the_skipped_line_carries_the_reason_and_keeps_the_size_fields_as_sentinels() {
    let line = surface_line(&SurfaceRecord {
        stamp: fixed_stamp(7, 5),
        stage: SurfaceStage::Skipped,
        target_id: TargetId(5),
        size: None,
        resized: None,
        reason: Some(SURFACE_REASON_K_UNCHANGED),
    });

    assert_eq!(
        line,
        "[transition] frame=7 t_us=5 kind=surface stage=skipped target_id=5 \
         w=- h=- resized=- reason=k-unchanged"
    );
}

/// `target_id` は `Debug` 表現（`TargetId(3)`）ではなく**内側の u32** で出る。
///
/// 判定側は `target_id` から scope を規則「shell = 2·scope ／ balloon = 2·scope+1」で写像する
/// ため、値が算術で読めなければ判定が組み立たない。加えて `TargetId(3)` の字面は
/// `judge-perf.py` と同じ辞書化規則で 1 トークンに切れない形（括弧）を持ち込む。
#[test]
fn the_target_id_is_written_as_a_bare_number() {
    let line = surface_line(&SurfaceRecord {
        stamp: fixed_stamp(1, 1),
        stage: SurfaceStage::Upload,
        target_id: TargetId(7),
        size: Some((1, 1)),
        resized: Some(false),
        reason: None,
    });

    assert_eq!(field_value(&line, SURFACE_FIELD_TARGET_ID), "7");
    assert!(
        !line.contains("TargetId"),
        "`TargetId` の Debug 表現が行へ漏れている: {line}"
    );
}

/// 3 段階のどの行も**同じフィールド名を同じ順序で 1 度ずつ**持つ。
///
/// 辞書化は後勝ちなので、1 行に同じ名前が 2 度出ると先の値が黙って消える
/// （`tools/perf/judge-perf.py:562,588-596`）。段階でフィールドを落とさないことと併せて、
/// 「記録が出ていない」と「その段階にはその値が無い」の区別を字面の上で保つ。
#[test]
fn every_surface_line_repeats_no_field_name_and_drops_none() {
    let expected: Vec<&str> = [
        transition_diag::FIELD_FRAME,
        transition_diag::FIELD_T_US,
        transition_diag::FIELD_KIND,
    ]
    .into_iter()
    .chain(SURFACE_FIELDS.iter().copied())
    .collect();

    for (stage, size, resized, reason) in [
        (SurfaceStage::Upload, Some((2, 3)), Some(true), None),
        (SurfaceStage::Visualize, Some((2, 3)), None, None),
        (
            SurfaceStage::Skipped,
            None,
            None,
            Some(SURFACE_REASON_INVISIBLE),
        ),
    ] {
        let line = surface_line(&SurfaceRecord {
            stamp: fixed_stamp(9, 9),
            stage,
            target_id: TargetId(0),
            size,
            resized,
            reason,
        });
        let names = field_names(&line);
        assert_eq!(
            names,
            expected,
            "`stage={}` の行のフィールド名列が語彙と違う: {line}",
            stage.as_str()
        );
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            names.len(),
            "同じフィールド名が 2 度出ている（辞書化で先の値が消える）: {line}"
        );
    }
}

/// 段階語・理由語は互いに異なり、番兵でなく、空白を含まない。加えて wintf 側の語と交わらない。
///
/// 交われば、1 本の時系列を `stage=` で切り分ける判定側が別 crate の行を取り違える。
#[test]
fn the_stage_and_reason_words_are_distinct_and_disjoint_from_the_wintf_vocabulary() {
    let mut all: Vec<&str> = SURFACE_STAGE_ALL.to_vec();
    all.extend_from_slice(SURFACE_REASON_ALL);
    all.push(KIND_SURFACE);

    for word in &all {
        assert_ne!(*word, transition_diag::MISSING, "語が番兵と同じ: {word}");
        assert!(!word.is_empty(), "語が空");
        assert!(
            !word.contains(char::is_whitespace),
            "語に空白が含まれる（`名前=値` の 1 トークンに切れない）: {word}"
        );
    }

    let mut sorted = all.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), all.len(), "語が重複している: {all:?}");

    for stage in SURFACE_STAGE_ALL {
        assert!(
            !transition_diag::STAGE_ALL.contains(stage),
            "段階語 `{stage}` が wintf の段階語と衝突している"
        );
    }
    assert!(
        !transition_diag::KIND_ALL.contains(&KIND_SURFACE),
        "レコード種別 `{KIND_SURFACE}` が wintf の種別語と衝突している"
    );
}

/// 負例（Requirement 7.4）: 判定語を 1 文字壊した行は語彙照合で落ちる。
///
/// 正例だけの照合は「語彙表が空でも緑」になり得る。壊した側が確かに赤くなることまで見る。
#[test]
fn a_tampered_stage_word_fails_the_vocabulary_check() {
    let good = surface_line(&SurfaceRecord {
        stamp: fixed_stamp(1, 1),
        stage: SurfaceStage::Upload,
        target_id: TargetId(0),
        size: Some((1, 1)),
        resized: Some(false),
        reason: None,
    });
    assert!(
        SURFACE_STAGE_ALL.contains(&field_value(&good, FIELD_STAGE)),
        "正例が語彙表に無い: {good}"
    );

    let broken = good.replace("stage=upload", "stage=uploaded");
    assert!(
        !SURFACE_STAGE_ALL.contains(&field_value(&broken, FIELD_STAGE)),
        "壊した語が語彙表を素通りした: {broken}"
    );
}

// ── 本文走査（ログ濾過テストでは原理的に見えない退行）─────────────────────────────────

/// 行コメントを落とした本文（説明文の中の字面を数えないため）。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 空行を落とした本文の行列（前後関係を見るため）。
fn code_lines(code: &str) -> Vec<&str> {
    code.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

/// Requirement 10.4: `show.rs` の 2 つの発行点は「寸が変わったとき」かつ「観測が有効なとき」に
/// **限って**レコードを組む。
///
/// # 殺す誤実装
///
/// - 前置ガードを外して常時組む → 既定運転で `String` の確保が毎フレーム走る（定常アロケー
///   ション 0 の破壊）。`debug!` は濾過されるので**ログ出力は変わらず**、濾過テストは緑のまま
/// - `size_changed || resized` の条件を落とす → 定常フレームでも 2 行ずつ組む
#[test]
fn the_show_path_builds_the_surface_records_behind_the_front_guard() {
    let code = code_only(include_str!("show.rs"));

    assert!(
        code.contains(
            "let observe_surface = (size_changed || resized) && transition_diag::is_enabled();"
        ),
        "サーフェス記録の前置ガードが失われている——既定運転でも観測の費用を払う形になる"
    );
    assert_eq!(
        code.matches("transition_diag::is_enabled()").count(),
        1,
        "本文に `is_enabled()` が 2 度以上ある（ガードされていない組立が混ざり得る）"
    );
    assert_eq!(
        code.matches("surface_line(&SurfaceRecord {").count(),
        2,
        "サーフェス記録の組立が 2 箇所（upload・visualize）でない"
    );
    assert_eq!(
        code.matches("if observe_surface {").count(),
        2,
        "組立がガードの内側に 2 箇所そろっていない"
    );

    // 走査が中身を見ている対照——落とし過ぎていないこと。
    assert!(
        code.contains("pub(super) fn apply_show("),
        "説明文を落とす処理が本文まで落としている"
    );
}

/// design D4: `resized` は upload の**直前**に読んだ寸との比較で得る。
///
/// 途中に別の文が挟まると「upload 直前の寸」ではなくなる。`chain.upload` の戻り値型を変えずに
/// `resized` を得る唯一の形がこの前後比較であり（旧案 `UploadOutcome` は撤回済み）、
/// エラー分岐の字面は `test-cage-determinism` ④の観測点ゆえ動かさない。
#[test]
fn the_previous_size_is_read_immediately_before_the_upload_and_the_error_branch_is_unmoved() {
    let code = code_only(include_str!("show.rs"));
    let lines = code_lines(&code);

    let at = lines
        .iter()
        .position(|line| *line == "let prev_size = chain.size();")
        .expect("upload 直前の寸取得が無い（`resized` の出所が失われている）");
    assert_eq!(
        lines[at + 1],
        "if let Err(e) = chain.upload(&entry.composed) {",
        "寸取得と upload のあいだに文が挟まっている（「直前の寸」でなくなる）"
    );
    assert_eq!(
        code.matches("chain.size()").count(),
        2,
        "`chain.size()` の呼出が upload の前後 2 回でない"
    );
}

/// Requirement 4.6: `refresh_scale` の 2 つの見送りは理由つきで記録され、いずれもガードの内側で
/// 組まれる。見送りの判定そのものは変えない。
#[test]
fn the_refresh_path_records_both_skips_behind_the_front_guard() {
    let code = code_only(include_str!("refresh.rs"));

    assert_eq!(
        code.matches("if transition_diag::is_enabled() {").count(),
        2,
        "見送り 2 経路の前置ガードがそろっていない"
    );
    assert_eq!(
        code.matches("surface_line(&SurfaceRecord {").count(),
        2,
        "見送りの記録が 2 箇所（拡大率不変・不可視）でない"
    );
    assert!(
        code.contains("if previous == Some(scale) {") && code.contains("if !visible {"),
        "見送りの判定式が変わっている（本 spec は判定を変えない）"
    );
    assert!(
        code.contains("pub fn refresh_scale("),
        "説明文を落とす処理が本文まで落としている"
    );
}

/// D12: perf 行の `frame=` は**末尾**に足す（既存フィールドの順序・文言は不変）。
#[test]
fn the_perf_line_appends_the_frame_field_at_the_very_end() {
    let code = code_only(include_str!("timing.rs"));
    let lines = code_lines(&code);

    let at = lines
        .iter()
        .position(|line| *line == "\"perf(apply_show): 段階別計時\"")
        .expect("perf 行の固定文言が本文に無い");
    assert_eq!(
        lines[at - 1],
        "frame,",
        "`frame` が perf 行の末尾フィールドでない（既存フィールドの順序を崩している）"
    );
    assert_eq!(
        lines[at - 2],
        "key_hash,",
        "`frame` の直前が従来の末尾フィールド `key_hash` でない（既存順序が動いている）"
    );
    // 「末尾に在る」だけでは、**先頭にもう 1 つ足した**形を捕まえられない。1 行に同じ名前が
    // 2 度出ると `judge-perf.py::parse_fields` の後勝ちで先の値が消え、しかも捕捉列の
    // フィールド集合検査（`presenter_perf_log_tests`）は辞書へ畳むので気付けない。
    //
    // 数える範囲は `tracing::debug!(` の内側だけである（`EmitContext` の分解にも同じ字面の
    // `frame,` が在り、そちらは行のフィールドではない）。
    let emit_at = lines
        .iter()
        .position(|line| *line == "tracing::debug!(")
        .expect("perf 行の発行が本文に無い");
    assert_eq!(
        lines[emit_at..at]
            .iter()
            .filter(|line| **line == "frame,")
            .count(),
        1,
        "perf 行に `frame` が 2 度出ている（辞書化は後勝ち・先の値が黙って消える）"
    );
}

/// D1: World を借りられる観測点はスレッド局所ミラーを読まない。
///
/// ミラーを読む形（`transition_diag::stamp()`）は、本 crate のテストが UI スレッド上で走るため
/// **ログ捕捉では一度も赤にならない**。本文走査だけがこの退行を捕まえる。
#[test]
fn the_world_holding_observation_points_never_read_the_thread_local_mirror() {
    for (name, source) in [
        ("show.rs", code_only(include_str!("show.rs"))),
        ("refresh.rs", code_only(include_str!("refresh.rs"))),
        (
            "transition_record.rs",
            code_only(include_str!("transition_record.rs")),
        ),
    ] {
        assert!(
            !source.contains("transition_diag::stamp()"),
            "{name} が World 外専用のスレッド局所ミラーを読んでいる（D1 違反）"
        );
        assert!(
            !source.contains("current_frame()"),
            "{name} が World 外専用のフレーム番号を読んでいる（D1 違反）"
        );
    }

    let record = code_only(include_str!("transition_record.rs"));
    assert!(
        record.contains("transition_diag::stamp_from_world(frame, tick_start)"),
        "刻印が World 資源から組まれていない（D1 違反）"
    );
}

// ── 配線（実 `apply_show`／`refresh_scale` を駆動する）──────────────────────────────────

/// 段が非 0 になる大きさは要らない（本ファイルは所要を一切見ない）。小さく取って GPU 資源の
/// 生成コストだけに寄せる。
const NATIVE_W: u32 = 24;
const NATIVE_H: u32 = 18;

/// 檻が World へ据えるフレーム番号（本番は `try_tick_world` が進める）。
const TEST_FRAME: u32 = 4_242;

/// 刻印資源を載せた GPU World（本番の `EcsWorld::new` と同じ 2 資源）。
fn gpu_world_with_frame() -> World {
    let mut world = make_world_with_gpu();
    world.insert_resource(FrameCount(TEST_FRAME));
    world.insert_resource(TickStart(Instant::now()));
    world
}

/// GPU World ＋ DPI 付き窓 ＋ 装着済み target を 1 組作る（`author_dpi` は 96 固定）。
fn attach(presenter: &mut EmoPresenter, world: &mut World, target: TargetId, window_dpi: u16) {
    let window = spawn_window_with_dpi(world, window_dpi);
    let (emo_world, atlas, _golden) = build_target_assets(NATIVE_W, NATIVE_H, 0xC3);
    presenter
        .attach_target(world, target, window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
}

/// `ShowSurface` を 1 回適用する。
fn show(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) -> PresentOutcome {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("ShowSurface の reply を受信できない")
}

/// `Hide` を 1 回適用する。
fn hide(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::Hide {
            target,
            reply: Some(tx),
        },
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("Hide の reply を受信できない")
        .expect("Hide は成功する");
}

/// 捕捉列から `kind=surface` の行だけを発火順に取り出す。
fn surface_lines(events: &[CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with(transition_diag::RECORD_PREFIX_TAG))
        .filter(|m| field_value(m, transition_diag::FIELD_KIND) == KIND_SURFACE)
        .collect()
}

/// 捕捉列の perf サマリ行（`timing.rs` の固定文言）。
fn perf_line(events: &[CapturedEvent]) -> CapturedEvent {
    let mut found = events
        .iter()
        .filter(|e| e.message() == super::super::timing::PERF_LINE_MESSAGE);
    let one = found
        .next()
        .expect("perf サマリ行が 1 本も出ていない（捕捉が死んでいる）");
    assert!(found.next().is_none(), "perf サマリ行が 2 本以上出ている");
    one.clone()
}

/// Requirement 2.2／2.8: 遷移（初回表示）では `upload`→`visualize` の 2 行が出て、perf 行と
/// **同一のフレーム番号**で突合できる。
#[test]
fn a_transition_records_upload_then_visualize_on_the_same_frame_as_the_perf_line() {
    let mut world = gpu_world_with_frame();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, TargetId(3), 96);

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        assert!(
            show(&mut presenter, &mut world, TargetId(3)).is_ok(),
            "前提: 初回表示は成立する"
        );
    });

    let events = cap.events();
    let lines = surface_lines(&events);
    assert_eq!(lines.len(), 2, "遷移で 2 行でない: {lines:?}");

    assert_eq!(
        field_value(&lines[0], FIELD_STAGE),
        SURFACE_STAGE_UPLOAD,
        "1 行目が upload でない: {lines:?}"
    );
    assert_eq!(
        field_value(&lines[1], FIELD_STAGE),
        SURFACE_STAGE_VISUALIZE,
        "2 行目が visualize でない: {lines:?}"
    );

    for line in &lines {
        assert_eq!(field_value(line, SURFACE_FIELD_TARGET_ID), "3");
        assert_eq!(field_value(line, SURFACE_FIELD_W), NATIVE_W.to_string());
        assert_eq!(field_value(line, SURFACE_FIELD_H), NATIVE_H.to_string());
        assert_eq!(
            field_value(line, transition_diag::FIELD_FRAME),
            TEST_FRAME.to_string(),
            "刻印が World 資源のフレーム番号でない: {line}"
        );
    }

    // 供給面は原寸で遅延生成されるため、初回の upload は**バッファ寸を変えない**。
    // 記録が出ているのは `size_changed`（前回適用寸なし）の側が真だからである。
    assert_eq!(
        field_value(&lines[0], SURFACE_FIELD_RESIZED),
        "false",
        "初回表示で `resized` が真になっている（前後比較が効いていない）: {lines:?}"
    );

    let perf = perf_line(&events);
    assert_eq!(
        perf.fields.get("frame").map(String::as_str),
        Some(TEST_FRAME.to_string().as_str()),
        "perf 行のフレーム番号が突合しない: {:?}",
        perf.fields
    );
}

/// Requirement 10.4: 寸が変わらない定常フレームでは `surface` 行が 1 本も増えない。
///
/// 陰性主張は同一捕捉スコープ内の perf 行 1 本（陽性）と対にする。
#[test]
fn a_steady_reapply_with_an_unchanged_size_emits_no_surface_line() {
    let mut world = gpu_world_with_frame();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, TargetId(0), 96);
    assert!(
        show(&mut presenter, &mut world, TargetId(0)).is_ok(),
        "前提: 初回表示は成立する（ここまでは捕捉の外）"
    );

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        assert!(
            show(&mut presenter, &mut world, TargetId(0)).is_ok(),
            "前提: 同一入力の再適用も成立する"
        );
    });

    let events = cap.events();
    let _positive_control = perf_line(&events);
    assert_eq!(
        surface_lines(&events),
        Vec::<String>::new(),
        "寸が変わらない定常フレームで `surface` 行が出ている"
    );
}

/// Requirement 2.2: 拡大率が変わって供給面のバッファ寸が変わった回は `resized=true` で出る。
#[test]
fn a_scale_change_records_the_buffer_resize() {
    let mut world = gpu_world_with_frame();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(NATIVE_W, NATIVE_H, 0xC3);
    presenter
        .attach_target(&mut world, TargetId(1), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    assert!(
        show(&mut presenter, &mut world, TargetId(1)).is_ok(),
        "前提: k=1/1 の初回表示は成立する"
    );

    set_window_dpi(&mut world, window, 192);
    let cap = CaptureSubscriber::default();
    let reported = tracing::subscriber::with_default(cap.clone(), || {
        presenter.refresh_scale(&mut world, TargetId(1))
    });
    assert_eq!(
        reported,
        Some((NATIVE_W * 2, NATIVE_H * 2)),
        "前提: k=2/1 への再表示が成立し新物理寸が報告される"
    );

    let lines = surface_lines(&cap.events());
    assert_eq!(lines.len(), 2, "遷移で 2 行でない: {lines:?}");
    assert_eq!(
        field_value(&lines[0], SURFACE_FIELD_RESIZED),
        "true",
        "バッファ寸が変わったのに `resized` が偽: {lines:?}"
    );
    for line in &lines {
        assert_eq!(
            field_value(line, SURFACE_FIELD_W),
            (NATIVE_W * 2).to_string()
        );
        assert_eq!(
            field_value(line, SURFACE_FIELD_H),
            (NATIVE_H * 2).to_string()
        );
    }
}

/// Requirement 4.6: 再表示の見送りは理由つきで記録される（判定側が合否から除外できる）。
#[test]
fn the_skipped_reapply_is_recorded_with_the_reason_that_gated_it() {
    let mut world = gpu_world_with_frame();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(NATIVE_W, NATIVE_H, 0xC3);
    presenter
        .attach_target(&mut world, TargetId(2), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    assert!(
        show(&mut presenter, &mut world, TargetId(2)).is_ok(),
        "前提: 初回表示は成立する"
    );

    // ⑴ 拡大率不変（窓 DPI を変えない）。
    let cap = CaptureSubscriber::default();
    let unchanged = tracing::subscriber::with_default(cap.clone(), || {
        presenter.refresh_scale(&mut world, TargetId(2))
    });
    assert_eq!(unchanged, None, "前提: k 不変では再表示しない");
    let lines = surface_lines(&cap.events());
    assert_eq!(lines.len(), 1, "見送りの記録が 1 行でない: {lines:?}");
    assert_eq!(field_value(&lines[0], FIELD_STAGE), SURFACE_STAGE_SKIPPED);
    assert_eq!(
        field_value(&lines[0], SURFACE_FIELD_REASON),
        SURFACE_REASON_K_UNCHANGED
    );
    assert_eq!(
        field_value(&lines[0], SURFACE_FIELD_W),
        transition_diag::MISSING
    );
    assert_eq!(field_value(&lines[0], SURFACE_FIELD_TARGET_ID), "2");

    // ⑵ 不可視（k は変えたうえで隠す＝先のゲートを抜けて不可視ゲートへ届かせる）。
    hide(&mut presenter, &mut world, TargetId(2));
    set_window_dpi(&mut world, window, 192);
    let cap = CaptureSubscriber::default();
    let invisible = tracing::subscriber::with_default(cap.clone(), || {
        presenter.refresh_scale(&mut world, TargetId(2))
    });
    assert_eq!(invisible, None, "前提: 不可視では再表示しない");
    let lines = surface_lines(&cap.events());
    assert_eq!(lines.len(), 1, "見送りの記録が 1 行でない: {lines:?}");
    assert_eq!(
        field_value(&lines[0], SURFACE_FIELD_REASON),
        SURFACE_REASON_INVISIBLE
    );
}

/// 刻印資源を持たない World（`EcsWorld::new` 以外で組まれた World）でもフィールドは落ちない。
#[test]
fn a_world_without_the_stamp_resources_still_produces_a_complete_line() {
    let world = World::new();
    let stamp = stamp_of(&world);
    assert_eq!(stamp.frame, 0, "資源不在で値を捏造しない（0 を返す）");
    assert_eq!(stamp.t_us, 0);
    assert_eq!(frame_of(&world), 0);

    let mut with_frame = World::new();
    with_frame.insert_resource(FrameCount(11));
    with_frame.insert_resource(TickStart(Instant::now()));
    assert_eq!(frame_of(&with_frame), 11);
    assert_eq!(stamp_of(&with_frame).frame, 11);
}
