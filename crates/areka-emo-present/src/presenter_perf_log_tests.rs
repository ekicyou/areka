//! 計時ログ較正檻（design.md §Testing Strategy「Integration Tests（presenter 経由）」項目 3）。
//!
//! 本ファイルは **`apply_show` を実際に駆動して出た行そのもの**を検査する。`timing_tests.rs` は
//! `FrameTiming` を注入時刻シームで単体検査するだけで、**配線（mark の設置位置・emit の回数・
//! 成立点との対）を 1 つも固定していない**（tasks.md「1.3 → 1.4 への申し送り（最重要）」——
//! `timing.mark(Stage::Upload)` を削除しても全件が緑のままだったことがレビューの変異検査で
//! 判明している）。本ファイルがその穴を塞ぐ。
//!
//! 固定する契約は 4 つである。
//!
//! 1. **perf サマリ行のスキーマ**（Requirement 1.1/1.3/1.4・design.md §Data Models）:
//!    固定文言 [`PERF_LINE_MESSAGE`]・debug 水準・14 フィールド **ちょうど**
//! 2. **既存の表示成立点 info! 行の不変性**（design.md §Boundary Commitments「Out of Boundary」——
//!    実機サインオフ契約ゆえ文言・水準・フィールドとも不変）: 文言・info 水準・12 フィールド
//!    **ちょうど**。本ファイルはこの契約の防波堤であり、info! を 1 文字でも触ると RED になる
//! 3. **mark の設置位置**（Requirement 1.4）: 段が実行される入力では非 0、実行されない入力では
//!    **厳密に 0**。どの段が 0 になるかは mark をどの分岐へ置いたかの構造的帰結であり、
//!    測定マシンの速度では変わらない
//! 4. **1 適用 1 行・成立点の対**（Requirement 1.1）: 表示が成立した適用は info! 行と perf 行を
//!    **隣接する対**として各 1 本出し、早期復帰した適用は **1 本も出さない**
//!
//! # 実時間を合否条件に使わない（Requirement 6.2）
//!
//! `sleep`・スピン・経過ミリ秒の閾値は 1 つも使わない。非 0 を主張するのは
//! **入力の大きさが決めている段**だけである——合成・リサンプル・マスク生成・供給面転写は
//! いずれも本檻が選んだ 240×180（k 適用後 480×360 ＝ 691,200 バイト）の画素を舐める作業であり、
//! キャッシュ照会段は本檻が渡す 10 万要素の bind 集合の等価比較（400KB の逐語比較）が支配する。
//! いずれも µs 分解能の 1 目盛りに対して 1 桁以上の余裕があり、「速いマシンなら 0 になる」形の
//! 閾値ではない。0 を主張する側（スキップ段）は mark が呼ばれないことの帰結ゆえ厳密である。
//!
//! # 「出ないこと」の主張は同一スコープ内で陽性と対にする
//!
//! `tracing` の callsite interest はプロセス大域にキャッシュされるため、subscriber 未設置の
//! 並列テストが同一 callsite を先に踏むと「1 本も出ていない」が恒真になり得る。本ファイルの
//! 陰性主張はすべて、**同一の `with_default` スコープ内で同一 callsite の陽性 1 本**を先に
//! 観測したうえで成立させる（presenter_refresh_and_log_tests.rs :661-697 の前例）。
//!
//! # 捕捉の流儀
//!
//! ログ捕捉は presenter 系テストの共有ヘルパ（`presenter_test_support.rs` の
//! `CaptureSubscriber`／`CapturedEvent`）を使う。`tracing` 単体・
//! `tracing::subscriber::with_default`（＝スレッドローカル）であり、`set_global_default` は
//! プロセス大域で並列テストと混線するため使わない。
//!
//! # `t_cache_us` の非 0 主張が拠って立つ前提（変更時は再検証が要る）
//!
//! [`cache_hit_apply_reports_exact_zero_for_skipped_stages_and_zero_allocs`] の
//! `assert_ne!(stage_us(hit, "t_cache_us"), 0)` は、`timing.mark(Stage::CacheLookup)` の設置を
//! 固定している**唯一の**主張である。非 0 の余裕は [`BIND_COUNT`] 要素の bind 集合の等価比較が
//! 作っており、`BindSet` の等価比較が要素数に対して線形である間だけ成立する。詳細と、
//! 主張が揺らいだ場合に取るべき対応は [`BIND_COUNT`] の説明を参照すること。

use super::*;

use std::path::Path;
use std::time::Duration;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::Surface;

use super::test_support::{
    CaptureSubscriber, CapturedEvent, build_target_assets, elem, shell_of, spawn_window_with_dpi,
    surface,
};
use super::timing::{PERF_LINE_MESSAGE, compose_key_hash};

// ── 契約面の正本（この 3 つの定数がコード側と乖離したら RED）───────────────────────────

/// 既存の表示成立点 info! の固定文言（show.rs の `info!`・実機サインオフ契約）。
const INFO_LINE_MESSAGE: &str = "apply(ShowSurface): 表示・マスクを更新";

/// 表示成立点 info! のフィールド集合（show.rs の `info!` から逐語で写した**全項目**）。
///
/// design.md §Boundary Commitments「Out of Boundary」が「文言・水準・フィールドとも不変」と
/// 定めた契約の正本である。**ちょうど**で照合するため、削除・改名だけでなく**追加**も RED。
const INFO_LINE_FIELDS: [&str; 13] = [
    "author_dpi",
    "cache_hit",
    "k",
    "k_ratio",
    "message",
    "native_h",
    "native_w",
    "scaled_h",
    "scaled_w",
    "size_changed",
    "surface_id",
    "target_id",
    "window_dpi",
];

/// perf サマリ行のフィールド集合（design.md §Data Models「perf サマリ行スキーマ」の 14 項目
/// ＋ `areka-P0-dpi-transition-atomicity` D12 が末尾へ足した `frame` ＋ `message`）。
/// 判定スクリプト `tools/perf/judge-perf.py` との唯一のデータ契約である。
///
/// 本配列は**昇順の集合**であり行の順序は語らない。`frame` が行の**末尾**に出ること
/// （既存フィールドの順序を崩していないこと）は
/// `presenter/transition_record_tests.rs::the_perf_line_appends_the_frame_field_at_the_very_end`
/// が本文走査で固定している。
const PERF_LINE_FIELDS: [&str; 16] = [
    "alloc_compose_dst",
    "alloc_mask",
    "alloc_resample_dst",
    "alloc_xmap",
    "cache_hit",
    "frame",
    "key_hash",
    "message",
    "surface_id",
    "t_cache_us",
    "t_compose_us",
    "t_mask_us",
    "t_resample_us",
    "t_total_us",
    "t_upload_us",
    "target_id",
];

// ── 捕捉列の読み口 ─────────────────────────────────────────────────────────────────────

/// 捕捉列中の perf サマリ行の位置（固定文言の完全一致）。
fn perf_line_indices(events: &[CapturedEvent]) -> Vec<usize> {
    events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.message() == PERF_LINE_MESSAGE)
        .map(|(i, _)| i)
        .collect()
}

/// 捕捉列中の表示成立点 info! 行の位置（固定文言の完全一致）。
fn info_line_indices(events: &[CapturedEvent]) -> Vec<usize> {
    events
        .iter()
        .enumerate()
        .filter(|(_, e)| e.message() == INFO_LINE_MESSAGE)
        .map(|(i, _)| i)
        .collect()
}

/// フィールドを引く（無ければ全フィールドを添えて落とす＝欠落が判る形）。
fn field(ev: &CapturedEvent, name: &str) -> String {
    ev.fields
        .get(name)
        .unwrap_or_else(|| panic!("行にフィールド `{name}` が無い: {:?}", ev.fields))
        .clone()
}

/// µs フィールドを数値で引く（0／非 0 の構造判定に使う。値の**大きさ**は一切見ない）。
fn stage_us(ev: &CapturedEvent, name: &str) -> u64 {
    field(ev, name)
        .parse::<u64>()
        .unwrap_or_else(|e| panic!("`{name}` が u64 で出ていない（{e}）: {:?}", ev.fields))
}

/// 表示成立点 info! 行と perf 行が **同一適用の隣接する対**として各 1 本ずつ出ていることを
/// 確認し、`(info 行, perf 行)` を返す。
///
/// 隣接（perf の位置 == info の位置 + 1）まで見るのは、design.md §Data Models の
/// 「両者は同一 apply の対として隣接 emit される」が契約だからである。emit を成立点から
/// 引き剥がして別経路（フレーム末尾等）へ移す改変はここで RED になる。
fn expect_single_adjacent_pair(events: &[CapturedEvent]) -> (CapturedEvent, CapturedEvent) {
    let infos = info_line_indices(events);
    let perfs = perf_line_indices(events);
    assert_eq!(
        infos.len(),
        1,
        "表示成立点 info! 行が 1 本でない（{} 本）: {events:?}",
        infos.len()
    );
    assert_eq!(
        perfs.len(),
        1,
        "perf サマリ行が 1 適用 1 行でない（{} 本）: {events:?}",
        perfs.len()
    );
    assert_eq!(
        perfs[0],
        infos[0] + 1,
        "info! 行と perf 行が同一適用の隣接する対になっていない（info={} perf={}）: {events:?}",
        infos[0],
        perfs[0]
    );
    (events[infos[0]].clone(), events[perfs[0]].clone())
}

/// 表示成立点 info! 行の**文言・水準・全フィールド**が不変であることを固定する。
///
/// design.md §Boundary Commitments「Out of Boundary」の実機サインオフ契約の防波堤。
/// `info!` を `debug!` へ落とす／フィールドを 1 つ改名する／1 つ足す、のいずれも RED。
fn assert_info_line_contract(ev: &CapturedEvent) {
    assert_eq!(
        ev.level,
        tracing::Level::INFO,
        "表示成立点ログが info 水準でない（実機サインオフの RUST_LOG grep が既定条件で読めない）"
    );
    assert_eq!(
        ev.field_names(),
        INFO_LINE_FIELDS.to_vec(),
        "表示成立点 info! のフィールド集合が変わった（design.md「Out of Boundary」＝文言・水準・\
         フィールドとも不変の契約違反。実機サインオフの判定材料が静かに壊れる）"
    );
}

/// perf サマリ行の**文言・水準・全フィールド**が契約どおりであることを固定する。
fn assert_perf_line_contract(ev: &CapturedEvent) {
    assert_eq!(
        ev.level,
        tracing::Level::DEBUG,
        "perf サマリ行が debug 水準でない（Requirement 1.2 の常設ログ水準の契約違反）"
    );
    assert_eq!(
        ev.field_names(),
        PERF_LINE_FIELDS.to_vec(),
        "perf サマリ行のフィールド集合が design.md §Data Models のスキーマと違う\
         （judge-perf.py との唯一のデータ契約の破壊）"
    );
}

// ── fixture ────────────────────────────────────────────────────────────────────────────

/// 段の非 0 を入力の大きさで担保するための native 外形（k=2/1 で 480×360＝691,200 バイト）。
///
/// 合成・リサンプル・マスク生成・供給面転写はいずれもこの画素数を舐めるため、µs 分解能の
/// 1 目盛りに対して 1 桁以上の余裕がある（Requirement 6.2: 閾値ではなく入力で決める）。
const NATIVE_W: u32 = 240;
const NATIVE_H: u32 = 180;

/// キャッシュ照会段の非 0 を入力の大きさで担保するための bind 集合の要素数。
///
/// `ComposeCache::get` はヒット時に `key.binds == *binds` を逐語比較する（`Vec<u32>` ＝
/// 400KB）。surface 1000 は `animations` を 1 件も持たないため、この集合は**合成の内容にも
/// 所要にも影響しない**（`build_plan` は surface の animation 側から `binds.contains` を引く
/// だけ）——純粋に「照会段が舐める入力量」だけを増やす仕掛けである。
///
/// # この値が支えている主張と、その前提（`BindSet` を変更する人へ）
///
/// [`cache_hit_apply_reports_exact_zero_for_skipped_stages_and_zero_allocs`] の
/// `assert_ne!(stage_us(hit, "t_cache_us"), 0)` は、`timing.mark(Stage::CacheLookup)` が
/// 設置されていることを固定している**唯一の**主張である。ほかにこの mark を押さえている
/// テストは無いため、この 1 行が失われると mark を削除しても全件が緑のまま通る。
///
/// 実測（release ビルド）では、ヒット経路の `t_cache_us` はおよそ 30〜45µs、比較を伴わない
/// ミス経路の同じ段は 1〜15µs である。つまり非 0 の余裕は**ほぼ全てが上記の逐語比較**から
/// 来ており、この主張は
/// **`BindSet` の等価比較コストが要素数に対して線形である間だけ**成立する。
/// 等価比較にハッシュのキャッシュ・要素数や指紋による短絡・ポインタ同一性判定などの
/// O(1) 経路を入れると、`t_cache_us` は µs 分解能の切り捨て圏（0 になり得る領域）へ落ちる。
///
/// **この主張が間欠的に落ちるようになった場合、正しい対応は主張の削除ではない。**
/// 削除すると `timing.mark(Stage::CacheLookup)` を消しても緑のままになり、本ファイルが
/// 塞いだ穴がそのまま開く。時間に依存しない形（照会段が実行されたことを構造で示す固定点）に
/// 置き換えること。
const BIND_COUNT: u32 = 100_000;

/// 照会段を非 0 にするための大きな bind 集合（合成結果は既定集合と同一）。
fn heavy_binds() -> BindSet {
    BindSet::from_ids(1..=BIND_COUNT)
}

/// 有効 surface 1000（`w×h` 全不透明）＋ 定義層皆無で外形 0×0 に退化する surface 7000 を
/// 同一 target へ載せる `(EmoWorld, AtlasTable)`。
///
/// surface 7000 は `Composer::compose` が `Err(ComposeError::EmptyComposition(7000))` を返す
/// （presenter_display_tests.rs の同型 fixture と同じ構成）。show.rs の早期復帰 8 経路のうち
/// EmptyComposition 縮退（warn! ＋ Hide 縮退）へ到達する唯一の手段である。
fn build_assets_with_empty_surface(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![
        surface(1000, vec![elem("p.png", 0, 0)]),
        Surface {
            id: 7000,
            targets: vec![areka_parsers::shell::AppendTarget::Single(7000)],
            elements: Vec::new(),
            collisions: Vec::new(),
            animations: Vec::new(),
        },
    ];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            img.extend_from_slice(&[b, g, r, 0xFF]);
        }
    }
    dec.insert(base.join("p.png"), w, h, stride, img, true);

    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// `ShowSurface` を 1 回適用し、reply の結果を返す（成功・失敗のどちらも呼び手が判定する）。
fn show(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    surface_id: u32,
    binds: BindSet,
) -> PresentOutcome {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id,
            binds,
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("ShowSurface の reply を受信できない")
}

/// GPU World ＋ DPI 付き窓 ＋ 装着済み target を 1 組作る（`author_dpi` は 96 固定）。
fn attach(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    window_dpi: u16,
    salt: u8,
) {
    let window = spawn_window_with_dpi(world, window_dpi);
    let (emo_world, atlas, _golden) = build_target_assets(NATIVE_W, NATIVE_H, salt);
    presenter
        .attach_target(world, target, window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
}

// ── 檻 ────────────────────────────────────────────────────────────────────────────────

/// Requirement 1.1／1.3／1.4 観測完了（**配線の檻・ミス経路**）: 実際の `apply_show` が
/// 表示成立点 info! 行と perf 行を**隣接する対**として各 1 本出し、perf 行が固定文言・
/// debug 水準・14 フィールドちょうどを備え、5 段すべてに mark が効いていて、`alloc_*` が
/// 実 `FrameBudget` の値で埋まる。
///
/// k=2/1・引き当てミスの初回適用は **5 段すべてが実行される唯一の入力**である。ゆえに
/// どれか 1 つの `timing.mark(..)` を削ると、その段が 0 になって RED になる。同様に
/// `note_alloc` を 1 つ削ると該当 `alloc_*` が 0 になって RED になる。
///
/// # 殺す誤実装
///
/// - `timing.mark(Stage::X)` の削除・分岐外への移動 → 該当段が 0 で RED
/// - `target.budget.note_alloc(AllocSite::X)` の削除 → 該当 `alloc_*` が 0 で RED
/// - `timing.emit` を成立点から引き剥がす → 隣接判定で RED
/// - info! の文言・水準・フィールドの変更 → [`assert_info_line_contract`] で RED
#[test]
fn miss_apply_emits_adjacent_info_and_perf_pair_with_every_stage_and_alloc_wired() {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, TargetId(0), 192, 0xA1);

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        assert!(
            show(&mut presenter, &mut world, TargetId(0), 1000, BindSet::default()).is_ok(),
            "前提: k=2/1 の初回表示は成立する"
        );
    });

    let events = cap.events();
    let (info, perf) = expect_single_adjacent_pair(&events);
    assert_info_line_contract(&info);
    assert_perf_line_contract(&perf);

    // 表示成立点 info! の値（実機サインオフが読む判定材料そのもの）。
    assert_eq!(field(&info, "target_id"), "TargetId(0)");
    assert_eq!(field(&info, "surface_id"), "1000");
    assert_eq!(field(&info, "cache_hit"), "false");
    assert_eq!(field(&info, "k"), "2.0");
    assert_eq!(field(&info, "author_dpi"), "96");
    assert_eq!(field(&info, "window_dpi"), "Some((192, 192))");
    assert_eq!(field(&info, "native_w"), NATIVE_W.to_string());
    assert_eq!(field(&info, "native_h"), NATIVE_H.to_string());
    assert_eq!(field(&info, "scaled_w"), (NATIVE_W * 2).to_string());
    assert_eq!(field(&info, "scaled_h"), (NATIVE_H * 2).to_string());
    assert_eq!(field(&info, "size_changed"), "true");
    let k_ratio = field(&info, "k_ratio");
    assert!(
        k_ratio.contains("num: 2") && k_ratio.contains("den: 1"),
        "k_ratio に既約 num/den が出ていない: {k_ratio}"
    );

    // perf 行の同定情報は info! と同一適用を指す（対であることの内容側の担保）。
    assert_eq!(field(&perf, "target_id"), field(&info, "target_id"));
    assert_eq!(field(&perf, "surface_id"), field(&info, "surface_id"));
    assert_eq!(field(&perf, "cache_hit"), field(&info, "cache_hit"));

    // key_hash は檻が独立に再計算した値と一致する（Requirement 7.2 の裁定材料の帰属可能性）。
    let expected_hash = compose_key_hash(
        1000,
        &BindSet::default(),
        &PatternState::default(),
        ScaleRatio::new(2, 1).expect("2/1 は構築できる"),
    );
    assert_eq!(
        field(&perf, "key_hash"),
        expected_hash.to_string(),
        "key_hash が合成キー（surface_id・binds・pattern・k）から導かれていない"
    );

    // (i) mark の設置位置: k≠1 のミスは 5 段すべてが実行される唯一の入力である。
    //
    // 照会段（`t_cache_us`）だけは本檻で非 0 を主張しない——ミス経路の照会は空スロットの
    // 判定で終わり、舐める入力量が無いためである（照会段の mark は
    // [`cache_hit_apply_reports_exact_zero_for_skipped_stages_and_zero_allocs`] が
    // 10 万要素の bind 集合の逐語比較で固定する）。フィールドの存在自体は
    // [`assert_perf_line_contract`] が集合ちょうどで押さえている。
    for name in [
        "t_compose_us",
        "t_resample_us",
        "t_mask_us",
        "t_upload_us",
        "t_total_us",
    ] {
        assert_ne!(
            stage_us(&perf, name),
            0,
            "段 `{name}` が 0（k≠1 のミス経路では 5 段すべてが実行される＝mark が\
             削除／分岐外へ移動している）: {:?}",
            perf.fields
        );
    }

    // (iv) alloc_* は実 FrameBudget 由来。ミス＋k≠1 の初回適用は 4 発生点すべてを 1 回ずつ踏む。
    assert_eq!(field(&perf, "alloc_compose_dst"), "1", "合成先の確保が計数されていない");
    assert_eq!(
        field(&perf, "alloc_resample_dst"),
        "1",
        "表示バッファ（リサンプル先）の確保が計数されていない"
    );
    assert_eq!(field(&perf, "alloc_xmap"), "1", "リサンプル作業領域の確保が計数されていない");
    assert_eq!(field(&perf, "alloc_mask"), "1", "当たり判定マスクの確保が計数されていない");
}

/// Requirement 1.1／1.3／1.4 観測完了（**配線の檻・ヒット経路**）: 引き当てが成立した適用では
/// 合成・リサンプル・マスク生成の 3 段が**厳密に 0**で出て、確保計数も全て 0 になる。
/// 照会段と転写段は実行されるので非 0 のままである。
///
/// 「どの段が 0 になるか」は mark をどの分岐へ置いたかの構造的帰結であり、測定マシンの速度では
/// 変わらない。`timing.mark(Stage::Compose)` をミス分岐の外（無条件）へ移す改変・`note_alloc` を
/// ヒット経路でも踏む位置へ移す改変は、いずれもここで RED になる。
///
/// 照会段の非 0 は 10 万要素の bind 集合の等価比較（400KB の逐語比較）が担保する
/// （[`BIND_COUNT`] の説明を参照——入力量で決めており、実時間の閾値ではない）。
#[test]
fn cache_hit_apply_reports_exact_zero_for_skipped_stages_and_zero_allocs() {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, TargetId(0), 192, 0xA2);

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        // 1 回目＝ミス（スロットを埋める）、2 回目＝完全一致ヒット。
        assert!(show(&mut presenter, &mut world, TargetId(0), 1000, heavy_binds()).is_ok());
        assert!(show(&mut presenter, &mut world, TargetId(0), 1000, heavy_binds()).is_ok());
    });

    let events = cap.events();
    let infos = info_line_indices(&events);
    let perfs = perf_line_indices(&events);
    assert_eq!(infos.len(), 2, "2 適用で info! 行が 2 本でない: {events:?}");
    assert_eq!(
        perfs.len(),
        2,
        "2 適用で perf 行が 2 本でない（1 適用 1 行の契約違反）: {events:?}"
    );
    // (ii) emit-once: 2 適用がそれぞれ独立した隣接対を成す。
    assert_eq!(perfs[0], infos[0] + 1, "1 適用目の対が隣接していない: {events:?}");
    assert_eq!(perfs[1], infos[1] + 1, "2 適用目の対が隣接していない: {events:?}");

    for i in &infos {
        assert_info_line_contract(&events[*i]);
    }
    for i in &perfs {
        assert_perf_line_contract(&events[*i]);
    }

    let hit = &events[perfs[1]];
    assert_eq!(
        field(hit, "cache_hit"),
        "true",
        "前提: 2 回目の同一入力適用は引き当てで済むこと"
    );
    assert_eq!(field(&events[infos[1]], "cache_hit"), "true", "info! 側も引き当てを報告する");

    // 実行されなかった段は**厳密に 0**（mark を無条件へ移すと非 0 になり RED）。
    for name in ["t_compose_us", "t_resample_us", "t_mask_us"] {
        assert_eq!(
            stage_us(hit, name),
            0,
            "引き当て経路で段 `{name}` が 0 でない（mark がミス分岐の外へ出ている）: {:?}",
            hit.fields
        );
    }
    // 実行される段は非 0（照会段・転写段の mark を削ると RED）。
    for name in ["t_cache_us", "t_upload_us", "t_total_us"] {
        assert_ne!(
            stage_us(hit, name),
            0,
            "引き当て経路でも実行される段 `{name}` が 0（mark が削除されている）: {:?}",
            hit.fields
        );
    }
    // 確保計数も全 0（`note_alloc` をヒット経路でも踏む位置へ移すと RED）。
    for name in [
        "alloc_compose_dst",
        "alloc_resample_dst",
        "alloc_xmap",
        "alloc_mask",
    ] {
        assert_eq!(
            field(hit, name),
            "0",
            "引き当て経路で確保計数 `{name}` が増えている（確保しない経路で数えている）: {:?}",
            hit.fields
        );
    }
}

/// Requirement 1.1／1.3 観測完了（**恒等 k のミス経路**）: k=1/1 ではリサンプル段が実行されず、
/// `t_resample_us` が**厳密に 0**、`alloc_resample_dst`／`alloc_xmap` も 0 で出る。合成段・
/// マスク生成段・転写段は実行されるので非 0、`alloc_compose_dst`／`alloc_mask` は 1 である。
///
/// `timing.mark(Stage::Resample)` と 2 つの `note_alloc` が**恒等 k の分岐の内側**に置かれて
/// いることの檻である。これらを分岐の外（無条件）へ出す改変はここで RED になる。
#[test]
fn identity_scale_miss_reports_exact_zero_resample_stage_and_no_resample_allocs() {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    // 窓 DPI ＝ author_dpi ＝ 96 ゆえ k=1/1（resample を呼ばない経路）。
    attach(&mut presenter, &mut world, TargetId(0), 96, 0xA3);

    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        assert!(show(&mut presenter, &mut world, TargetId(0), 1000, BindSet::default()).is_ok());
    });

    let events = cap.events();
    let (info, perf) = expect_single_adjacent_pair(&events);
    assert_info_line_contract(&info);
    assert_perf_line_contract(&perf);
    assert_eq!(field(&info, "k"), "1.0", "前提: 96/96 で恒等 k");
    assert_eq!(
        field(&info, "scaled_w"),
        field(&info, "native_w"),
        "前提: 恒等 k では native と scaled が同寸"
    );

    assert_eq!(
        stage_us(&perf, "t_resample_us"),
        0,
        "恒等 k でリサンプル段が 0 でない（mark が分岐の外に出ている）: {:?}",
        perf.fields
    );
    assert_eq!(
        field(&perf, "alloc_resample_dst"),
        "0",
        "恒等 k で表示バッファの確保が計数されている（分岐の外で数えている）"
    );
    assert_eq!(
        field(&perf, "alloc_xmap"),
        "0",
        "恒等 k でリサンプル作業領域の確保が計数されている（分岐の外で数えている）"
    );
    assert_eq!(field(&perf, "alloc_compose_dst"), "1");
    assert_eq!(field(&perf, "alloc_mask"), "1");
    // 照会段は本檻でも非 0 を主張しない（理由は
    // [`miss_apply_emits_adjacent_info_and_perf_pair_with_every_stage_and_alloc_wired`] と同じ）。
    for name in ["t_compose_us", "t_mask_us", "t_upload_us", "t_total_us"] {
        assert_ne!(
            stage_us(&perf, name),
            0,
            "恒等 k でも実行される段 `{name}` が 0（mark が削除されている）: {:?}",
            perf.fields
        );
    }
}

/// Requirement 1.1／1.4 観測完了（**早期復帰では 1 行も出ない・陽性と陰性の対観測**）:
/// 合成前後で早期復帰する 3 経路（未装着 target・合成失敗・全透明退化）は info! 行も perf 行も
/// 1 本も出さない。
///
/// 陰性主張（「出ないこと」）は、**同一の `with_default` スコープ内**で先に成立適用 1 本を
/// 観測してから行う。捕捉が生きていること・両 callsite が有効であることが同じ捕捉列で
/// 証明されているため、陰性主張は恒真になり得ない。
///
/// さらに各早期復帰が**実際に実行された**ことを、その経路固有のログ（error!／warn!）を
/// 陽性として観測することで示す——これが無いと「そもそも経路へ到達していないから行が
/// 増えなかった」だけの空虚な緑になる。
///
/// # カバーする早期復帰（show.rs の 8 経路のうち 3 つ）
///
/// - 未装着 target（`TargetNotAttached`）
/// - 合成失敗（`SurfaceNotFound`）
/// - 全透明退化（`EmptyComposition` → Hide 縮退・reply Ok）
#[test]
fn early_returns_around_compose_emit_neither_info_nor_perf_line() {
    let mut world = super::test_support::make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas) = build_assets_with_empty_surface(NATIVE_W, NATIVE_H, 0xA4);
    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let cap = CaptureSubscriber::default();
    let (ok, not_attached, compose_err, empty) =
        tracing::subscriber::with_default(cap.clone(), || {
            // 陽性: 表示が成立する適用 1 本（両 callsite が有効であることの証明）。
            let ok = show(&mut presenter, &mut world, TargetId(0), 1000, BindSet::default());
            // 陰性 1: 未装着 target（show.rs 冒頭の早期復帰）。
            let a = show(&mut presenter, &mut world, TargetId(9), 1000, BindSet::default());
            // 陰性 2: 解決不能 id（合成失敗の早期復帰）。
            let b = show(&mut presenter, &mut world, TargetId(0), 9999, BindSet::default());
            // 陰性 3: 定義層皆無で外形 0×0（全透明退化 → Hide 縮退の早期復帰）。
            let c = show(&mut presenter, &mut world, TargetId(0), 7000, BindSet::default());
            (ok, a, b, c)
        });

    // 前提: 狙った 3 経路へ本当に落ちたこと（reply の形で確認する）。
    assert!(ok.is_ok(), "前提: 陽性の適用は成立する");
    assert!(
        matches!(not_attached, Err(PresentError::TargetNotAttached(TargetId(9)))),
        "前提: 未装着 target 経路へ落ちること: {not_attached:?}"
    );
    assert!(
        matches!(
            compose_err,
            Err(PresentError::Compose(ComposeError::SurfaceNotFound(9999)))
        ),
        "前提: 合成失敗経路へ落ちること: {compose_err:?}"
    );
    assert!(
        empty.is_ok(),
        "前提: 全透明退化は Hide 縮退＋reply Ok（早期復帰だが Err ではない）: {empty:?}"
    );

    let events = cap.events();
    // 陽性: 成立した 1 適用の対だけが出ている（陰性 3 本は 1 行も足していない）。
    let (info, perf) = expect_single_adjacent_pair(&events);
    assert_info_line_contract(&info);
    assert_perf_line_contract(&perf);
    assert_eq!(field(&info, "surface_id"), "1000", "陽性は surface 1000 の適用");
    assert_eq!(field(&perf, "surface_id"), "1000", "陰性の適用が perf 行を出している");

    // 各早期復帰が実際に走ったことを、その経路固有のログで陽性観測する（空虚な緑の排除）。
    let has = |level: tracing::Level, needle: &str| {
        events
            .iter()
            .any(|e| e.level == level && e.message().contains(needle))
    };
    assert!(
        has(tracing::Level::ERROR, "apply(ShowSurface): 未装着ターゲット"),
        "未装着 target 経路を通っていない（陰性が空虚）: {events:?}"
    );
    assert!(
        has(tracing::Level::ERROR, "apply(ShowSurface): 合成失敗"),
        "合成失敗経路を通っていない（陰性が空虚）: {events:?}"
    );
    assert!(
        has(tracing::Level::WARN, "apply(ShowSurface): 全透明退化"),
        "全透明退化経路を通っていない（陰性が空虚）: {events:?}"
    );
}

/// Requirement 1.1／1.4 観測完了（**供給面生成前の早期復帰でも 1 行も出ない**）: 供給面の
/// 遅延生成に必要な資源が World に無い 2 経路（Compositor 不在・GraphicsCore 不在）でも
/// info! 行・perf 行はいずれも出ない。
///
/// これらの経路は**合成とキャッシュ挿入を済ませた後**に落ちる（＝確保計数は増えているのに
/// 行が出ない）ため、「emit を成立点より手前へ動かす」改変を単独で殺す。`FrameTiming` が
/// `emit` で `self` を消費する構造ゆえ黙って drop される、という設計の主張の実測である。
///
/// 陰性主張は陽性（資源が揃った target での成立適用 1 本）と同一スコープで対にする。
#[test]
fn early_returns_before_swapchain_creation_emit_neither_info_nor_perf_line() {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    // 小さめの外形で 3 target（本檻は行の有無だけを見るので画素量は要らない）。
    for (i, salt) in [(0u32, 0xB1u8), (1, 0xB2), (2, 0xB3)] {
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, salt);
        presenter
            .attach_target(&mut world, TargetId(i), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
    }

    let cap = CaptureSubscriber::default();
    let (ok, no_gfx, no_compositor) = tracing::subscriber::with_default(cap.clone(), || {
        // 陽性: 資源が揃った target の成立適用（両 callsite の有効性の証明）。
        let ok = show(&mut presenter, &mut world, TargetId(0), 1000, BindSet::default());

        // 陰性 1: GraphicsCore 不在（Compositor は在る）。
        let gfx = world
            .remove_resource::<GraphicsCore>()
            .expect("前提: GraphicsCore が載っている");
        let a = show(&mut presenter, &mut world, TargetId(1), 1000, BindSet::default());
        world.insert_resource(gfx);

        // 陰性 2: WucGraphicsResource 不在（＝Compositor を取り出せない）。
        let wuc = world
            .remove_resource::<WucGraphicsResource>()
            .expect("前提: WucGraphicsResource が載っている");
        let b = show(&mut presenter, &mut world, TargetId(2), 1000, BindSet::default());
        world.insert_resource(wuc);

        (ok, a, b)
    });

    assert!(ok.is_ok(), "前提: 陽性の適用は成立する");
    assert!(
        matches!(
            no_gfx,
            Err(PresentError::Device {
                context: "GraphicsCore resource",
                ..
            })
        ),
        "前提: GraphicsCore 不在経路へ落ちること: {no_gfx:?}"
    );
    assert!(
        matches!(
            no_compositor,
            Err(PresentError::Device {
                context: "WucGraphicsResource::compositor",
                ..
            })
        ),
        "前提: Compositor 不在経路へ落ちること: {no_compositor:?}"
    );

    let events = cap.events();
    let (info, perf) = expect_single_adjacent_pair(&events);
    assert_info_line_contract(&info);
    assert_perf_line_contract(&perf);
    assert_eq!(
        field(&info, "target_id"),
        "TargetId(0)",
        "陽性は資源の揃った target の適用でなければならない"
    );
    assert_eq!(
        field(&perf, "target_id"),
        "TargetId(0)",
        "早期復帰した target が perf 行を出している"
    );

    // 各早期復帰が実際に走ったことの陽性観測（空虚な緑の排除）。
    let has_error = |needle: &str| {
        events
            .iter()
            .any(|e| e.level == tracing::Level::ERROR && e.message().contains(needle))
    };
    assert!(
        has_error("GraphicsCore 不在"),
        "GraphicsCore 不在経路を通っていない（陰性が空虚）: {events:?}"
    );
    assert!(
        has_error("WucGraphicsResource/Compositor 不在"),
        "Compositor 不在経路を通っていない（陰性が空虚）: {events:?}"
    );
}

/// Requirement 1.5 観測完了（design.md D5・**表示結果はログ設定に依らない**）: 捕捉 subscriber の
/// 下で適用した target と、subscriber を一切設置せずに適用した target が、**同一の表示バイト・
/// 同一の実適用 k・同一の窓寸 reconcile 要求**を与える。
///
/// 計測（`FrameTiming::start`／`mark`）はログ設定を参照せず無条件に走り、フィルタに委ねるのは
/// emit だけである、という構造の実測である。`if tracing::enabled!(..)` のような前置ガードを
/// 足して表示経路をログ設定で分岐させる改変は——分岐の結果が表示に及べば——ここで RED になる。
#[test]
fn display_result_is_independent_of_log_configuration() {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    // 同一 salt ＝ 同一の絵。窓 DPI も同一ゆえ、両 target は完全に同じ入力である。
    attach(&mut presenter, &mut world, TargetId(0), 192, 0xC7);
    attach(&mut presenter, &mut world, TargetId(1), 192, 0xC7);

    // (a) 捕捉 subscriber あり。
    let cap = CaptureSubscriber::default();
    tracing::subscriber::with_default(cap.clone(), || {
        assert!(show(&mut presenter, &mut world, TargetId(0), 1000, BindSet::default()).is_ok());
    });
    // (b) subscriber なし（既定の NoSubscriber ＝ perf 行も info! 行も一切出ない）。
    assert!(show(&mut presenter, &mut world, TargetId(1), 1000, BindSet::default()).is_ok());

    // 前提: (a) では行が出ている（(b) との差がログ設定だけであることの担保）。
    let events = cap.events();
    assert_eq!(
        perf_line_indices(&events).len(),
        1,
        "前提: 捕捉ありの側では perf 行が出ていること: {events:?}"
    );

    let bytes_a = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    let bytes_b = presenter.read_back(TargetId(1)).expect("read_back 失敗");
    // 非空性ガード（design.md Integration Test 1 と同じ規律）: 両方とも真っ黒／空だと
    // 「一致した」が空虚に成立してしまうため、実際に絵が載っていることを先に確かめる。
    assert!(
        !bytes_a.is_empty() && bytes_a.iter().any(|&b| b != 0),
        "捕捉ありの側の readback が空（不透明画素 0）＝比較が空虚"
    );
    assert!(
        !bytes_b.is_empty() && bytes_b.iter().any(|&b| b != 0),
        "捕捉なしの側の readback が空（不透明画素 0）＝比較が空虚"
    );
    assert_eq!(
        bytes_a, bytes_b,
        "ログ設定の有無で表示バイトが変わっている（Requirement 1.5 違反）"
    );
    assert_eq!(
        presenter.applied_scale(TargetId(0)),
        presenter.applied_scale(TargetId(1)),
        "ログ設定の有無で実適用 k が変わっている（Requirement 1.5 違反）"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        presenter.take_pending_resize(TargetId(1)),
        "ログ設定の有無で窓寸 reconcile 要求が変わっている（Requirement 1.5 違反）"
    );
    assert_eq!(
        presenter.current_surface_id(TargetId(0)),
        presenter.current_surface_id(TargetId(1)),
        "ログ設定の有無で現サーフェスが変わっている（Requirement 1.5 違反）"
    );
}
