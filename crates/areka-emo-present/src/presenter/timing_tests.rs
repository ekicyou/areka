//! `FrameTiming`（timing.rs）の単体檻。
//!
//! 本ファイルが固定するのは **perf サマリ行のスキーマそのもの**（design.md §Data Models
//! 「perf サマリ行スキーマ（コード⇔判定スクリプト間の唯一のデータ契約）」）である。判定
//! スクリプト `tools/perf/judge-perf.py` はこの固定文言とフィールド名だけを頼りに行を機械
//! 抽出するため、文言・水準・フィールド集合は観測状態と同格の契約であり、1 つでも欠ければ
//! 下流の集計が黙って部分集計になる（Requirement 2.5 の行単位欠落検出が成立しなくなる）。
//!
//! # 実時間を合否条件に使わない（Requirement 6.2）
//!
//! 段の所要は私有シーム `start_at` / `mark_at` / `emit_at` へ**任意の [`Instant`] を注入**して
//! 与える。単調時計の差分だけで値が決まるため、測定マシンの速度・スケジューリングに一切
//! 依存しない（`sleep` もスピンも使わない）。公開シーム `start` / `mark` / `emit`（実時計）
//! は別テストでスキーマの成立だけを確認する。
//!
//! # 捕捉の流儀
//!
//! emo-present 既存の自前 `CaptureSubscriber`（presenter_refresh_and_log_tests.rs）と同型の
//! 最小 subscriber を用いる（`tracing::subscriber::with_default` ＝ スレッドローカル既定
//! subscriber。`set_global_default` はプロセス大域で並列テストと混線するため使わない）。
//! 本ファイルは「ログが**出ない**こと」を主張しない——callsite interest キャッシュ由来の
//! 恒真化の罠は陰性主張にのみ効く。既存 info! 行の不変性と陰陽対の観測は task 1.4 の
//! `presenter_perf_log_tests` が担う。

use super::*;

use areka_emo_compose::{ComposeMethod, PatternFrame};

// ── 捕捉 subscriber（tracing 単体・新規依存なし） ─────────────────────────────────────

/// 捕捉した 1 イベント（level ＋ フィールド名 → Debug 表現）。
#[derive(Debug, Clone)]
struct CapturedEvent {
    level: tracing::Level,
    fields: std::collections::HashMap<String, String>,
}

/// 全フィールドを Debug 表現で拾う visitor。
///
/// [`tracing::field::Visit`] の `record_u64`／`record_bool` 等は既定実装が `record_debug` へ
/// 転送するため、`record_debug` 1 本で型を問わず全フィールドを捕捉できる。
struct FieldGrab(std::collections::HashMap<String, String>);

impl tracing::field::Visit for FieldGrab {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_string(), format!("{value:?}"));
    }
}

/// イベントを溜めるだけの最小 subscriber（span は使わないので new_span は固定 id を返す）。
#[derive(Clone, Default)]
struct CaptureSubscriber(std::sync::Arc<std::sync::Mutex<Vec<CapturedEvent>>>);

impl tracing::Subscriber for CaptureSubscriber {
    fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut grab = FieldGrab(std::collections::HashMap::new());
        event.record(&mut grab);
        self.0
            .lock()
            .expect("捕捉バッファの毒化なし")
            .push(CapturedEvent {
                level: *event.metadata().level(),
                fields: grab.0,
            });
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// 捕捉列から perf サマリ行だけを抜く（固定文言 [`PERF_LINE_MESSAGE`] の完全一致）。
fn perf_lines(cap: &CaptureSubscriber) -> Vec<CapturedEvent> {
    cap.0
        .lock()
        .expect("捕捉バッファ")
        .iter()
        .filter(|e| e.fields.get("message").is_some_and(|m| m == PERF_LINE_MESSAGE))
        .cloned()
        .collect()
}

/// フィールドを引く（無ければ全フィールドを添えて落とす＝欠落が判る形）。
fn field(ev: &CapturedEvent, name: &str) -> String {
    ev.fields
        .get(name)
        .unwrap_or_else(|| {
            panic!("perf サマリ行にフィールド `{name}` が無い: {:?}", ev.fields)
        })
        .clone()
}

/// perf サマリ行スキーマの全段フィールド名（design.md §Data Models・`judge-perf.py` の契約面）。
const STAGE_FIELDS: [&str; 6] = [
    "t_cache_us",
    "t_compose_us",
    "t_resample_us",
    "t_mask_us",
    "t_upload_us",
    "t_total_us",
];

/// perf サマリ行スキーマの確保計数フィールド名。
const ALLOC_FIELDS: [&str; 4] = [
    "alloc_compose_dst",
    "alloc_resample_dst",
    "alloc_xmap",
    "alloc_mask",
];

fn ctx() -> EmitContext {
    EmitContext {
        target_id: TargetId(7),
        surface_id: 1410,
        cache_hit: false,
        key_hash: 0xdead_beef,
    }
}

fn allocs() -> BudgetDelta {
    BudgetDelta {
        alloc_compose_dst: 1,
        alloc_resample_dst: 2,
        alloc_xmap: 3,
        alloc_mask: 4,
    }
}

// ── 檻 ────────────────────────────────────────────────────────────────────────────────

/// Requirement 1.1／1.2 観測完了（design.md §FrameTiming・§Data Models）: 1 適用 = 1 行で、
/// 固定文言・debug 水準・**全段フィールド常時出現**・実行されなかった段は 0、が成立する。
///
/// 注入した [`Instant`] は互いに弁別可能な区間（1ms / 7ms / 2ms・合計 12ms）で組むため、
/// 段の取り違え（cache と compose の入替・total と upload の混同）はすべて RED になる。
/// Resample と MaskGen は **`mark` を呼ばない**——これが「実行されなかった段」であり、
/// フィールドが消えても 0 以外が出ても RED になる。
///
/// # 殺す誤実装
///
/// - スキップ段のフィールドを出さない（`Option` を条件付き emit する）→ フィールド欠落で RED
/// - 段の区間を「起点からの累積」で取る → t_compose_us が 8000 になり RED
/// - `t_total_us` を段の総和で埋める → 10000 になり RED（合計は起点〜emit の全区間）
/// - `debug!` を `trace!`／`info!` へ動かす → level assert で RED
/// - 固定文言を変える → 行が 1 本も見つからず RED
#[test]
fn emits_single_debug_line_with_all_stage_fields_and_zero_for_skipped_stages() {
    let t0 = Instant::now();
    let mut timing = FrameTiming::start_at(t0);
    timing.mark_at(Stage::CacheLookup, t0 + Duration::from_millis(1));
    timing.mark_at(Stage::Compose, t0 + Duration::from_millis(8));
    // Resample・MaskGen は実行されなかった段（mark を呼ばない）。
    timing.mark_at(Stage::Upload, t0 + Duration::from_millis(10));

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        timing.emit_at(&ctx(), allocs(), t0 + Duration::from_millis(12));
    });

    let lines = perf_lines(&cap);
    assert_eq!(
        lines.len(),
        1,
        "1 適用 1 行の契約が壊れている（捕捉列: {:?}）",
        cap.0.lock().expect("捕捉バッファ")
    );
    let ev = &lines[0];

    assert_eq!(
        ev.level,
        tracing::Level::DEBUG,
        "perf サマリ行は開発者向け詳細水準（debug）の常設ログであること（Requirement 1.2）"
    );

    // 全段フィールドが常に出る（Requirement 2.5 の行単位欠落検出の前提）。
    for name in STAGE_FIELDS {
        assert!(
            ev.fields.contains_key(name),
            "全段フィールドが常時出現していない: `{name}` が無い（{:?}）",
            ev.fields
        );
    }
    assert_eq!(field(ev, "t_cache_us"), "1000", "キャッシュ照会段の区間");
    assert_eq!(field(ev, "t_compose_us"), "7000", "合成段の区間（累積でない）");
    assert_eq!(
        field(ev, "t_resample_us"),
        "0",
        "実行されなかった段は 0 で出ること"
    );
    assert_eq!(
        field(ev, "t_mask_us"),
        "0",
        "実行されなかった段は 0 で出ること"
    );
    assert_eq!(field(ev, "t_upload_us"), "2000", "供給面転写段の区間");
    assert_eq!(
        field(ev, "t_total_us"),
        "12000",
        "合計は起点〜emit の全区間（段の総和ではない）"
    );

    // 適用対象の識別子・引き当ての可否・合成キーのハッシュ（Requirement 1.1／7.2）。
    assert_eq!(field(ev, "target_id"), "TargetId(7)");
    assert_eq!(field(ev, "surface_id"), "1410");
    assert_eq!(field(ev, "cache_hit"), "false");
    assert_eq!(field(ev, "key_hash"), format!("{}", 0xdead_beef_u64));

    // 確保計数（Requirement 1.3 の供給先・値は呼び手から素通し）。
    for name in ALLOC_FIELDS {
        assert!(
            ev.fields.contains_key(name),
            "確保計数フィールドが無い: `{name}`（{:?}）",
            ev.fields
        );
    }
    assert_eq!(field(ev, "alloc_compose_dst"), "1");
    assert_eq!(field(ev, "alloc_resample_dst"), "2");
    assert_eq!(field(ev, "alloc_xmap"), "3");
    assert_eq!(field(ev, "alloc_mask"), "4");
}

/// Requirement 1.1 観測完了: **1 段も記録されなかった適用**でも 5 段すべてのフィールドが 0 で
/// 出る（早期の成立・全段スキップという極端形でも行の形が崩れない）。
///
/// これは「フィールドを記録済みの段だけ出す」実装を構造的に殺す檻である。
#[test]
fn all_stage_fields_appear_as_zero_when_nothing_was_marked() {
    let t0 = Instant::now();
    let timing = FrameTiming::start_at(t0);

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        timing.emit_at(&ctx(), BudgetDelta::default(), t0 + Duration::from_micros(42));
    });

    let lines = perf_lines(&cap);
    assert_eq!(lines.len(), 1, "全段スキップでも 1 行は出ること");
    let ev = &lines[0];

    for name in ["t_cache_us", "t_compose_us", "t_resample_us", "t_mask_us", "t_upload_us"] {
        assert_eq!(
            field(ev, name),
            "0",
            "実行されなかった段 `{name}` は 0 で出ること"
        );
    }
    assert_eq!(field(ev, "t_total_us"), "42", "合計は起点〜emit の全区間");
    for name in ALLOC_FIELDS {
        assert_eq!(field(ev, name), "0", "既定の確保計数は 0");
    }
}

/// Requirement 1.5／design.md D5 観測完了: 公開シーム（実時計）でも同一スキーマの行が 1 本出る。
///
/// 計測は無条件に実行され、フィルタに委ねるのは emit だけである——`start`／`mark` は
/// subscriber 未設置でも呼べて panic しないこと（＝表示経路がログ設定で分岐しないこと）を、
/// **subscriber スコープの外**で計測してから中で emit する形で示す。実時間の値には一切
/// assert しない（Requirement 6.2）。
#[test]
fn public_clock_seam_emits_the_same_schema() {
    // subscriber 未設置のまま計測する（ログ設定に依存せず計測が走ることの実演）。
    let mut timing = FrameTiming::start();
    timing.mark(Stage::CacheLookup);
    timing.mark(Stage::Compose);
    timing.mark(Stage::Upload);

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        timing.emit(&ctx(), allocs());
    });

    let lines = perf_lines(&cap);
    assert_eq!(lines.len(), 1, "公開シームでも 1 適用 1 行");
    let ev = &lines[0];
    for name in STAGE_FIELDS.iter().chain(ALLOC_FIELDS.iter()) {
        assert!(
            ev.fields.contains_key(*name),
            "公開シーム経由でもフィールド `{name}` が出ること（{:?}）",
            ev.fields
        );
    }
    // 実行されなかった段は実時計でも 0（値が時間依存にならない唯一の段）。
    assert_eq!(field(ev, "t_resample_us"), "0");
    assert_eq!(field(ev, "t_mask_us"), "0");
}

/// 同一段の二重記録は `debug_assert` で拒否する（design.md §FrameTiming Service Interface）。
///
/// 常時テストは debug_assertions 有効で走るため檻として成立する。release ビルドでは
/// `debug_assert` が消えるので、このテストは `debug_assertions` 下でのみ有効にする。
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "同一段の二重記録")]
fn marking_the_same_stage_twice_is_rejected() {
    let t0 = Instant::now();
    let mut timing = FrameTiming::start_at(t0);
    timing.mark_at(Stage::Compose, t0 + Duration::from_millis(1));
    timing.mark_at(Stage::Compose, t0 + Duration::from_millis(2));
}

/// Requirement 7.2 観測完了: `key_hash` は **run 内で安定**（同一キー → 同一値）かつ、
/// キー要素（surface_id・binds・pattern・k）の**いずれか 1 つでも違えば別値**になる。
///
/// 判定スクリプトはこの値の異なり数を「必要スロット数の実測根拠」として数えるため、
/// ⑴同一キーで揺れる（プロセス毎ランダム化ハッシュ）→ 異なり数が過大になり裁定材料が壊れる
/// ⑵キー要素を取りこぼす（例: pattern を混ぜない）→ 異なり数が過少になり同じく壊れる。
/// 本檻は両方向を殺す。
#[test]
fn compose_key_hash_is_stable_and_separates_every_key_element() {
    let binds = BindSet::from_ids([1, 2, 3]);
    let pattern = {
        let mut p = PatternState::default();
        p.set(
            5,
            PatternFrame {
                surface_id: 20,
                method: ComposeMethod::Overlay,
                x: 3,
                y: -4,
            },
        );
        p
    };
    let k = ScaleRatio::new(5, 4).expect("5/4 は構築できる");

    let base = compose_key_hash(1000, &binds, &pattern, k);

    // ⑴ 同一キーは同一値（同一 run 内での安定性）。
    assert_eq!(
        base,
        compose_key_hash(1000, &binds, &pattern, k),
        "同一キーで key_hash が揺れている（非ランダム化ハッシュであること）"
    );
    // 別インスタンスから組み直した等価キーでも同一値（値等価に従う）。
    assert_eq!(
        base,
        compose_key_hash(1000, &BindSet::from_ids([3, 2, 1]), &pattern.clone(), k),
        "値等価なキーは同一 key_hash になること（BindSet は昇順正準）"
    );

    // ⑵ キー要素ごとの弁別。
    assert_ne!(
        base,
        compose_key_hash(1001, &binds, &pattern, k),
        "surface_id 差が key_hash に現れていない"
    );
    assert_ne!(
        base,
        compose_key_hash(1000, &BindSet::from_ids([1, 2]), &pattern, k),
        "binds 差が key_hash に現れていない"
    );
    let other_pattern = {
        let mut p = PatternState::default();
        p.set(
            5,
            PatternFrame {
                surface_id: 21,
                method: ComposeMethod::Overlay,
                x: 3,
                y: -4,
            },
        );
        p
    };
    assert_ne!(
        base,
        compose_key_hash(1000, &binds, &other_pattern, k),
        "pattern 差（コマの surface_id）が key_hash に現れていない"
    );
    let shifted_pattern = {
        let mut p = PatternState::default();
        p.set(
            5,
            PatternFrame {
                surface_id: 20,
                method: ComposeMethod::Overlay,
                x: 3,
                y: -5,
            },
        );
        p
    };
    assert_ne!(
        base,
        compose_key_hash(1000, &binds, &shifted_pattern, k),
        "pattern 差（コマのオフセット）が key_hash に現れていない"
    );
    assert_ne!(
        base,
        compose_key_hash(1000, &binds, &PatternState::default(), k),
        "pattern 不在との差が key_hash に現れていない"
    );
    assert_ne!(
        base,
        compose_key_hash(1000, &binds, &pattern, ScaleRatio::ONE),
        "表示スケール k 差が key_hash に現れていない"
    );
}
