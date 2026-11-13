//! 焼きの入口 [`bake`] の檻（結線・失敗の脱落と継続・密な採番・0 寸と全透明の記録・着せ替えの「無し」の絵）。
//!
//! もとは `lib.rs` の中のテスト（spec: audit-emo-atlas 指摘 19 で兄弟のファイルへ移した）。

use super::*;
use crate::decode::MemoryDecoder;
use crate::pack::PackConfig;
use crate::table::SetId;
use crate::test_support::{
    BIND_DEBUG, CLEAR_WARN, ZERO_WARN, dressed_shell, elem, opaque_2x2, register, set, surface,
    transparent_2x2, zero_sized,
};
use areka_parsers::shell::{Interval, Surface};
use std::path::Path;

/// テスト①: end-to-end 結線（複数セット）。shell + balloon の 2 セットを 1 度の bake で
/// 処理し、索引表＋頁バッファが**値で直接**得られる（channel 非依存・R6.5 ＋ 1.x/4.x/5.x/6.x）。
#[test]
fn end_to_end_wiring_multiple_sets() {
    let shell_base = Path::new("shell/master");
    let balloon_base = Path::new("balloon/foo");

    let shell = vec![
        surface(0, vec![elem(0, "surface0.png"), elem(1, "eye0.png")]),
        surface(1, vec![elem(0, "surface1.png")]),
    ];
    let balloon = vec![surface(0, vec![elem(0, "balloon0.png")])];

    let mut dec = MemoryDecoder::new();
    register(&mut dec, shell_base, "surface0.png", opaque_2x2());
    register(&mut dec, shell_base, "eye0.png", opaque_2x2());
    register(&mut dec, shell_base, "surface1.png", opaque_2x2());
    register(&mut dec, balloon_base, "balloon0.png", opaque_2x2());

    let sets = [set(&shell, shell_base), set(&balloon, balloon_base)];

    // BakeResult は素の値。channel/async を一切介さず直接受け取る（R6.5・構造的証明）。
    let result: BakeResult = bake(&sets, &dec, PackConfig::default());

    // 失敗なし。
    assert!(
        result.errors.is_empty(),
        "no decode/normalize failures expected"
    );

    // 各セットの present key が resolve→Some・placement あり（非透明ゆえ焼かれる）。
    for (set, rel) in [
        (SetId(0), "surface0.png"),
        (SetId(0), "eye0.png"),
        (SetId(0), "surface1.png"),
        (SetId(1), "balloon0.png"),
    ] {
        let id = result
            .table
            .resolve(set, rel)
            .unwrap_or_else(|| panic!("{rel} in set {} must resolve", set.0));
        assert!(
            result.table.entry(id).placement.is_some(),
            "{rel} is non-transparent → placement present"
        );
    }

    // 頁バッファが値として得られている（焼付結果・非空）。
    assert!(
        !result.table.pages().is_empty(),
        "pages materialized by value"
    );
    assert_eq!(result.table.len(), 4, "4 survivors densely indexed");
}

/// テスト②: デコード失敗は索引表に載せず・他エントリ継続（R2.2）。
/// 1 つだけ未登録（NotFound）にすると、その key は resolve が None・errors に Decode が
/// 1 件・他の全 key は present（継続の実証）。
#[test]
fn decode_failure_skipped_others_continue() {
    let base = Path::new("shell/master");
    let shell = vec![
        surface(0, vec![elem(0, "ok_a.png"), elem(1, "missing.png")]),
        surface(1, vec![elem(0, "ok_b.png")]),
    ];

    let mut dec = MemoryDecoder::new();
    register(&mut dec, base, "ok_a.png", opaque_2x2());
    register(&mut dec, base, "ok_b.png", opaque_2x2());
    // "missing.png" は登録しない → decode NotFound。

    let sets = [set(&shell, base)];
    let result = bake(&sets, &dec, PackConfig::default());

    // 失敗 key は索引表に不在。
    assert_eq!(
        result.table.resolve(SetId(0), "missing.png"),
        None,
        "failed key absent from table"
    );
    // 生存 key は present。
    assert!(result.table.resolve(SetId(0), "ok_a.png").is_some());
    assert!(result.table.resolve(SetId(0), "ok_b.png").is_some());
    assert_eq!(result.table.len(), 2, "only survivors indexed");

    // errors はちょうど 1 件の Decode（欠落パス保持）。
    assert_eq!(result.errors.len(), 1, "exactly one decode failure");
    match &result.errors[0] {
        BakeError::Decode(crate::decode::DecodeError::NotFound { path }) => {
            assert_eq!(path, &base.join("missing.png") as &std::path::Path);
        }
        other => panic!("expected Decode(NotFound), got {other:?}"),
    }
}

/// テスト④: 失敗後の密再採番・resolve/entry/key 往復整合（欠番/panic なし）。
/// manifest 上は 3 key（a,b,c 昇順）だが b を失敗させ、生存 a,c が密に 0..2 で番号付く。
#[test]
fn dense_renumbering_and_roundtrip_after_failure() {
    let base = Path::new("shell/master");
    // rel_path 昇順で a.png, b.png, c.png（manifest 採番順）。
    let shell = vec![surface(
        0,
        vec![elem(0, "a.png"), elem(1, "b.png"), elem(2, "c.png")],
    )];

    let mut dec = MemoryDecoder::new();
    register(&mut dec, base, "a.png", opaque_2x2());
    // b.png は登録しない → 脱落。
    register(&mut dec, base, "c.png", opaque_2x2());

    let sets = [set(&shell, base)];
    let result = bake(&sets, &dec, PackConfig::default());

    assert_eq!(result.errors.len(), 1, "b.png failed");
    assert_eq!(result.table.len(), 2, "two survivors densely numbered");

    // 生存 a,c は密 index 0..2 で resolve→entry/key 往復整合。
    for rel in ["a.png", "c.png"] {
        let id = result
            .table
            .resolve(SetId(0), rel)
            .expect("survivor resolves");
        assert!(
            (id.0 as usize) < result.table.len(),
            "id within dense range"
        );
        // key(id) 逆引きが同じ rel_path を返す（往復）。
        assert_eq!(result.table.key(id).rel_path, rel);
        assert_eq!(result.table.key(id).set, SetId(0));
    }
    // 失敗 key は不在。
    assert_eq!(result.table.resolve(SetId(0), "b.png"), None);
}

/// テスト⑥（task 2.4・設計ディスカッション #1 増分）: 全透明（α=0 トリム後 0 寸）element を
/// 含む bake で `warn!` が発火する（tracing capture で決定論 assert・当該 element を名指し）。
/// **bake 結果自体は既存挙動不変**（全透明はエラーでなく空エントリ・placement None）。
#[test]
fn warn_fires_on_all_transparent_element() {
    let base = Path::new("shell/master");
    let shell = vec![surface(0, vec![elem(0, "solid.png"), elem(1, "clear.png")])];
    let mut dec = MemoryDecoder::new();
    register(&mut dec, base, "solid.png", opaque_2x2());
    register(&mut dec, base, "clear.png", transparent_2x2());
    let sets = [set(&shell, base)];

    let mut captured: Option<BakeResult> = None;
    let out = crate::log_capture::capture_logs(|| {
        captured = Some(bake(&sets, &dec, PackConfig::default()));
    });

    // warn が発火し、target・当該 element（rel_path）を名指ししている。
    assert!(
        out.contains("level=WARN"),
        "全透明 element で warn 発火: {out}"
    );
    assert!(
        out.contains("target=areka_emo_atlas"),
        "target 正しい: {out}"
    );
    assert!(out.contains("clear.png"), "問題の element を名指し: {out}");

    // bake 結果は既存挙動不変: 全透明はエラーでなく空エントリ（placement None・原寸記録）。
    let result = captured.unwrap();
    assert!(result.errors.is_empty(), "全透明は error ではない（不変）");
    let clear_id = result
        .table
        .resolve(SetId(0), "clear.png")
        .expect("空エントリも resolve");
    assert!(
        result.table.entry(clear_id).placement.is_none(),
        "全透明 → 空エントリ（placement None・不変）"
    );
    assert_eq!(
        result.table.entry(clear_id).original,
        crate::table::Size { w: 2, h: 2 },
        "原寸は記録される（不変）"
    );
    // 対照: solid.png は placement あり（焼付対象・不変）。
    let solid_id = result.table.resolve(SetId(0), "solid.png").unwrap();
    assert!(result.table.entry(solid_id).placement.is_some());
}

/// テスト⑦（task 2.4）: 元画像 0 寸（0×0）element を含む bake でも `warn!` が発火する。
#[test]
fn warn_fires_on_zero_sized_source_element() {
    let base = Path::new("shell/master");
    let shell = vec![surface(0, vec![elem(0, "zero.png")])];
    let mut dec = MemoryDecoder::new();
    register(&mut dec, base, "zero.png", zero_sized());
    let sets = [set(&shell, base)];

    let out = crate::log_capture::capture_logs(|| {
        let _ = bake(&sets, &dec, PackConfig::default());
    });
    assert!(
        out.contains("level=WARN"),
        "0 寸 element で warn 発火: {out}"
    );
    assert!(
        out.contains("target=areka_emo_atlas"),
        "target 正しい: {out}"
    );
    assert!(out.contains("zero.png"), "問題の element を名指し: {out}");
}

/// テスト⑧（task 2.4・不変性の陰性確認）: 正常（不透明・非零）element の bake では warn は
/// 発火せず、既知の bake 出力（placement 有り・非零 uv・原寸）が変わらない。
#[test]
fn no_warn_and_output_unchanged_for_normal_bake() {
    let base = Path::new("shell/master");
    let shell = vec![surface(0, vec![elem(0, "solid.png")])];
    let mut dec = MemoryDecoder::new();
    register(&mut dec, base, "solid.png", opaque_2x2());
    let sets = [set(&shell, base)];

    let mut captured: Option<BakeResult> = None;
    let out = crate::log_capture::capture_logs(|| {
        captured = Some(bake(&sets, &dec, PackConfig::default()));
    });

    // 正常 element では warn 経路に入らない。
    assert!(
        !out.contains("level=WARN"),
        "正常 element では warn 皆無: {out}"
    );

    // 既存の bake 出力は不変（placement 有り・非零 uv・原寸 2×2）。
    let result = captured.unwrap();
    assert!(result.errors.is_empty());
    let id = result.table.resolve(SetId(0), "solid.png").unwrap();
    let entry = result.table.entry(id);
    assert_eq!(
        entry.original,
        crate::table::Size { w: 2, h: 2 },
        "原寸不変"
    );
    let placement = entry
        .placement
        .as_ref()
        .expect("非零 → placement 有り（不変）");
    assert_eq!(placement.uv_rect.w, 2, "trim 幅不変");
    assert_eq!(placement.uv_rect.h, 2, "trim 高不変");
}

// ---- 檻 T9: 着せ替えの「無し」の絵の全透明は debug・それ以外は今の warn（real-run-log-noise 要件 3.2〜3.6・5.2）----

/// 面 0（不透明の絵）の `interval` のアニメーションが面 3000 を参照し、面 3000 が `rel` だけを持つシェル。
fn shell_owned_by(interval: Interval, rel: &str) -> Vec<Surface> {
    dressed_shell(surface(0, vec![elem(0, "solid.png")]), interval, rel)
}

/// `shell_owned_by` のシェルを、`rel` に `spec` の絵を当てて焼き、捕捉した記録と結果を返す。
fn bake_owned(
    interval: Interval,
    rel: &str,
    spec: (u32, u32, u32, Vec<u8>, bool),
) -> (String, BakeResult) {
    let base = Path::new("shell/master");
    let shell = shell_owned_by(interval, rel);
    let mut dec = MemoryDecoder::new();
    register(&mut dec, base, "solid.png", opaque_2x2());
    register(&mut dec, base, rel, spec);
    let sets = [set(&shell, base)];
    let mut captured: Option<BakeResult> = None;
    let out = crate::log_capture::capture_logs(|| {
        captured = Some(bake(&sets, &dec, PackConfig::default()));
    });
    (out, captured.unwrap())
}

/// T9 ⑴: `bind` だけから参照される面の全透明の絵は `warn` 0 件・`debug` 1 件（欄は今の `warn` と同じ）。
/// 索引表は見分けの無い場合（同じ形を `runonce` で参照）と同じ。
#[test]
fn bind_only_all_transparent_logs_debug_not_warn() {
    let (out, result) = bake_owned(Interval::Bind, "null.png", transparent_2x2());
    assert!(
        !out.contains("level=WARN"),
        "bind だけの全透明は warn 0 件: {out}"
    );
    let debug: Vec<&str> = out.lines().filter(|l| l.contains(BIND_DEBUG)).collect();
    assert_eq!(debug.len(), 1, "見分けた debug は 1 件: {out}");
    let line = debug[0];
    for want in [
        "level=DEBUG",
        "target=areka_emo_atlas",
        "set=0",
        "rel_path=\"null.png\"",
        "original_w=2",
        "original_h=2",
    ] {
        assert!(line.contains(want), "{want} を含む: {line}");
    }
    assert!(!line.contains(CLEAR_WARN), "警告の文言と別: {line}");

    let (_, plain) = bake_owned(Interval::Runonce, "null.png", transparent_2x2());
    assert_eq!(result.errors.len(), plain.errors.len(), "失敗の集計は同じ");
    assert_eq!(
        result.table.len(),
        plain.table.len(),
        "索引表の大きさは同じ"
    );
    for rel in ["null.png", "solid.png"] {
        let id = result.table.resolve(SetId(0), rel);
        assert_eq!(id, plain.table.resolve(SetId(0), rel), "{rel} の番号は同じ");
        let (got, want) = (
            result.table.entry(id.unwrap()),
            plain.table.entry(id.unwrap()),
        );
        assert_eq!(got.original, want.original, "{rel} の原寸は同じ");
        assert_eq!(
            format!("{:?}", got.placement),
            format!("{:?}", want.placement),
            "{rel} の配置・トリムの矩形は同じ"
        );
    }
}

/// T9 ⑵: `runonce` から参照される面の全透明の絵は今の `warn` 1 件（見分けない・debug 無し）。
#[test]
fn runonce_owned_all_transparent_keeps_warn() {
    let (out, _) = bake_owned(Interval::Runonce, "null.png", transparent_2x2());
    let warns: Vec<&str> = out.lines().filter(|l| l.contains("level=WARN")).collect();
    assert_eq!(warns.len(), 1, "今の warn 1 件: {out}");
    assert!(
        warns[0].contains(CLEAR_WARN) && warns[0].contains("null.png"),
        "{out}"
    );
    assert!(!out.contains(BIND_DEBUG), "見分けの debug は出ない: {out}");
}

/// T9 ⑶: `bind` だけから参照される面の 0 寸の絵は、見分けの前の 0 寸の `warn` 1 件。
#[test]
fn bind_only_zero_sized_keeps_zero_size_warn() {
    let (out, _) = bake_owned(Interval::Bind, "zero.png", zero_sized());
    let warns: Vec<&str> = out.lines().filter(|l| l.contains("level=WARN")).collect();
    assert_eq!(warns.len(), 1, "0 寸の warn 1 件: {out}");
    assert!(
        warns[0].contains(ZERO_WARN) && warns[0].contains("zero.png"),
        "{out}"
    );
    assert!(!out.contains(BIND_DEBUG), "0 寸は見分けに入らない: {out}");
}
