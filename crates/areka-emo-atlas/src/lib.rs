//! # areka-emo-atlas
//!
//! emo レンダリングエンジンの**素材基盤層**（pure）。emo の三段直列
//! （`emo-atlas` → `emo-compose` → `emo-present`）のうち **1/3** を担う。
//!
//! 責務は bake パイプライン
//! （マニフェスト導出 → デコード → 正規化 → トリム → packing → 焼付）で、
//! shell/balloon の surface 表現から element 画像を列挙し、premultiplied BGRA の
//! アトラス頁へ正規化・トリム・配置し、成果物契約（`AtlasTable` ほか）を正本として
//! 定義する。COM/WIC 依存はデコードポートの WIC 腕にのみ隔離し、正規化以降の
//! 純粋コアは wintf 本体（ECS/D2D/GraphicsCore）へ依存しない。
//!
//! パイプライン各段はモジュールへ分割される（本タスクでは雛形のみ）:
//! - [`manifest`] — マニフェスト導出（列挙・間接参照解決・重複排除）
//! - [`decode`] — デコードポート（trait）＋既定 WIC 腕
//! - [`normalize`] — 透過正規化（premultiplied 統一）
//! - [`trim`] — α トリミング
//! - [`pack`] — packing 座標算出
//! - [`bake`] — 頁バッファへの焼付
//! - [`table`] — 成果物契約型（正本）
//! - [`error`] — 診断可能なエラー型

pub mod bake;
pub mod decode;
pub mod error;
pub mod manifest;
pub mod normalize;
pub mod pack;
pub mod table;
pub mod trim;

/// `bake` の成果物（索引表＋失敗集合・channel 非依存・R6.5）。
///
/// `table` は成功・空エントリの密 `AtlasTable`（頁バッファ内包）、`errors` は
/// デコード/正規化で脱落したエントリの診断可能な集合。値・共有参照として直接返す
/// （通信機構を介さない・R6.5）。
pub struct BakeResult {
    /// 生存エントリ（Placed / Empty）の密索引表＋頁バッファ。
    pub table: AtlasTable,
    /// 脱落エントリの診断可能なエラー集合（R2.2）。空でも成功扱い。
    pub errors: Vec<crate::error::BakeError>,
}

/// 素材基盤層の公開入口（emo 三段直列 1/3・契約正本）。
///
/// マニフェスト導出 → デコード → 正規化 → トリム → packing → 焼付の各段を単一の
/// 入口関数として結線し、複数の入力集合（shell 用・balloon 用など）をまとめて処理する。
/// デコード/正規化に失敗したエントリは索引表に載せず、失敗内容を [`BakeResult::errors`]
/// へ集約しつつ他エントリの処理を継続する（R2.2）。索引表と頁バッファは通信機構を
/// 介さず値・共有参照として直接返す（R6.5）。
///
/// # 決定性
/// マニフェスト採番（`SetId` 昇順・rel_path 昇順）→ 同一失敗集合 → 同一生存集合 →
/// 同一密再採番、と各段が決定的ゆえ、同一入力（＋同一失敗）で同一 `AtlasTable` を返す
/// （golden 安定・D7）。
pub fn bake(sets: &[SurfaceSet<'_>], decoder: &impl ElementDecoder, cfg: PackConfig) -> BakeResult {
    // 1. マニフェスト導出（列挙・重複排除・決定的採番）。
    let manifest = ManifestDeriver.derive(sets);

    let mut errors: Vec<crate::error::BakeError> = Vec::new();

    // 生存エントリ（デコード・正規化を通過）を manifest 順で密に積む。
    //   survivor_keys[final] ↔ 最終 ElementId(final)
    //   original は全生存で保持（Empty も原寸記録・R4.2/4.5）
    //   Placed のみ placed_items へ（packing/焼付対象）。
    let mut survivor_keys: Vec<AtlasKey> = Vec::new();
    let mut survivor_originals: Vec<crate::table::Size> = Vec::new();
    let mut placed_items: Vec<(ElementId, Trimmed)> = Vec::new();

    for key in &manifest.keys {
        let set = &sets[key.set.0 as usize];

        // 唯一の実パス化（Path::join・一度きり・design 3.6）。
        let path = set.base_dir.join(&key.rel_path);

        // デコード（失敗＝索引表に載せず・エラー集約・継続・R2.2）。
        let decoded = match decoder.decode(&path) {
            Ok(d) => d,
            Err(e) => {
                errors.push(crate::error::BakeError::Decode(e));
                continue;
            }
        };

        let has_pna = decoder.probe_pna(&path);

        // 正規化（emo2 経路では成功・シーム到達は診断可能に集約・継続・3.5）。
        let normalized = match Normalizer.normalize(decoded, set.alpha_params, has_pna) {
            Ok(n) => n,
            Err(source) => {
                errors.push(crate::error::BakeError::Normalize {
                    key: key.clone(),
                    source,
                });
                continue;
            }
        };

        // トリム（全透明→空エントリ／それ以外→配置エントリ）。生存確定＝密採番へ加える。
        let trim = Trimmer.trim(&normalized);

        // 0 寸/全透明 element の早期警告（ゴースト制作者ミスの可能性が高いため・設計ディスカッション #1）。
        // **ログのみの増分**: bake の生成結果（索引表・placement・trim 矩形）は一切変更しない。
        // (a) 元画像 0 寸（0×0）／(b) α=0 トリム後 0 寸（全透明→placement None）を判別してログ。
        // 0 寸/全透明 element の早期警告（ゴースト制作者ミスの可能性が高いため・設計ディスカッション #1）。
        // **ログのみの増分**: bake の生成結果（索引表・placement・trim 矩形）は一切変更しない。
        // (a) 元画像 0 寸（0×0）／(b) α=0 トリム後 0 寸（全透明→placement None）を判別してログ。
        if trim.original.w == 0 || trim.original.h == 0 {
            tracing::warn!(
                target: "areka_emo_atlas",
                set = key.set.0,
                rel_path = key.rel_path.as_str(),
                original_w = trim.original.w,
                original_h = trim.original.h,
                "bake: element の元画像が 0 寸です（ゴースト制作者ミスの可能性）"
            );
        } else if trim.placement.is_none() {
            tracing::warn!(
                target: "areka_emo_atlas",
                set = key.set.0,
                rel_path = key.rel_path.as_str(),
                original_w = trim.original.w,
                original_h = trim.original.h,
                "bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性）"
            );
        }

        let final_id = ElementId(survivor_keys.len() as u32);
        survivor_keys.push(key.clone());
        survivor_originals.push(trim.original);
        if let Some(trimmed) = trim.placement {
            placed_items.push((final_id, trimmed));
        }
    }

    // 5. packing（座標のみ・最終 ElementId で採番済み）。
    let pack = Packer.pack(&placed_items, cfg);

    // 6. 焼付（頁バッファ確保＋blit）。Placement は最終 ElementId キー。
    let (pages, placements) = Baker.bake_pages(&placed_items, &pack, cfg.page_size);
    let placement_of: std::collections::HashMap<ElementId, Placement> =
        placements.into_iter().collect();

    // 7. 生存集合上に密な索引表を構築（final ElementId 順）。
    let entries: Vec<AtlasEntry> = survivor_keys
        .iter()
        .enumerate()
        .map(|(final_idx, _)| AtlasEntry {
            original: survivor_originals[final_idx],
            // Placed は Baker 由来 Placement・Empty（全透明）は None（転写スキップ）。
            placement: placement_of.get(&ElementId(final_idx as u32)).cloned(),
        })
        .collect();

    let pages_vec: Vec<AtlasPage> = pages;
    let table = AtlasTable::new(survivor_keys, entries, pages_vec);

    // 8. 値で直接返す（channel 非依存・R6.5）。
    BakeResult { table, errors }
}

// 成果物契約の正本型（D3・R6）。下流 emo-compose はこれらを import する。
pub use table::{
    AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, Placement, Point, Rect, SetId, Size,
};

// デコードポート（D4・R2.3）。既定手段（WIC）を露出せず、trait とデータ型のみ公開。
pub use decode::{DecodeError, DecodedImage, ElementDecoder, MemoryDecoder};

// 既定デコード腕（WIC 経由・COM 隔離・D4）。上位は trait 越しに用いる。
pub use decode::wic_arm::WicDecoderArm;

// マニフェスト導出（列挙層・R1.1–1.6/5.6・D6）。
pub use manifest::{Manifest, ManifestDeriver, SurfaceSet};

// 共有透過パラメータ型（normalize の設計本拠・SurfaceSet が運ぶ・3.6）。
pub use normalize::{AlphaParams, UseSelfAlpha};

// 正規化層（use_self_alpha 解釈・premultiplied BGRA 統一・R3・D5/D8）。
pub use normalize::{AlphaSource, NormalizeError, NormalizedImage, Normalizer};

// トリム層（α>0 タイト bbox・オフセット記録・全透明→空・R4・D8）。
pub use trim::{TrimResult, Trimmed, Trimmer};

// packing 層（決定的・複数頁・padding・座標のみ・R5・D7）。
pub use pack::{PackConfig, PackOutput, PackedEntry, Packer};

// 焼付層（頁確保・premultiplied 素通し blit・座標不変・R4.3/R6.3・D8・Critical Issue 3）。
pub use bake::Baker;

// 診断可能なエラー型（bake パイプラインの脱落集約・R2.2）。
pub use error::BakeError;

// emo2 fixture を用いた統合テスト（shell・balloon 横断・task 3.2）。
#[cfg(test)]
mod emo2_e2e;

// emo2 fixture の bake 出力に対する決定性（golden）テスト（task 4.1・5.5）。
#[cfg(test)]
mod emo2_golden;

// テスト専用の tracing ログ捕捉ハーネス（0 寸/全透明 warn の決定論 assert 用・task 2.4）。
// `#[cfg(test)]` 限定ゆえ本番 bake 経路に現れない。
#[cfg(test)]
mod log_capture;

#[cfg(test)]
mod bake_entry_tests {
    use super::*;
    use crate::decode::MemoryDecoder;
    use crate::manifest::SurfaceSet;
    use crate::normalize::{AlphaParams, UseSelfAlpha};
    use crate::pack::PackConfig;
    use crate::table::SetId;
    use areka_parsers::shell::{AppendTarget, Element, ElementPath, Surface};
    use std::path::Path;

    // ---- 合成モデルビルダ（emo2 相当構造・実 fixture パースは統合 task 3.2 送り）----

    fn elem(layer: u32, path: &str) -> Element {
        Element {
            layer,
            path: ElementPath::new(path.to_string()),
            x: 0,
            y: 0,
        }
    }

    /// element のみを持つ最小 surface（collision/animation 空）。
    fn surface(id: u32, elements: Vec<Element>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations: Vec::new(),
        }
    }

    fn params() -> AlphaParams {
        AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        }
    }

    fn set<'a>(surfaces: &'a [Surface], base: &'a Path) -> SurfaceSet<'a> {
        SurfaceSet {
            surfaces,
            base_dir: base,
            alpha_params: params(),
        }
    }

    /// tightly-packed 2×2 の premultiplied BGRA・不透明（α=255）画像スペックを返す。
    fn opaque_2x2() -> (u32, u32, u32, Vec<u8>, bool) {
        let (w, h) = (2u32, 2u32);
        let stride = w * 4;
        // 全画素 α=255・色は適当（premultiplied 適合: B,G,R <= A）。
        let bgra = vec![
            10, 20, 30, 255, //
            40, 50, 60, 255, //
            70, 80, 90, 255, //
            100, 110, 120, 255,
        ];
        (w, h, stride, bgra, true)
    }

    /// tightly-packed 2×2 の完全透明（全 α=0）画像スペック（has_alpha=true で normalize 通過）。
    fn transparent_2x2() -> (u32, u32, u32, Vec<u8>, bool) {
        let (w, h) = (2u32, 2u32);
        let stride = w * 4;
        let bgra = vec![0u8; (stride * h) as usize];
        (w, h, stride, bgra, true)
    }

    /// 0×0（元画像 0 寸）の画像スペック（has_alpha=true で normalize 通過・trim は空エントリ）。
    fn zero_sized() -> (u32, u32, u32, Vec<u8>, bool) {
        (0, 0, 0, Vec::new(), true)
    }

    /// MemoryDecoder へ base_dir.join(rel_path) の実パスで画像を登録する。
    fn register(
        dec: &mut MemoryDecoder,
        base: &Path,
        rel: &str,
        spec: (u32, u32, u32, Vec<u8>, bool),
    ) {
        let (w, h, stride, bgra, has_alpha) = spec;
        dec.insert(base.join(rel), w, h, stride, bgra, has_alpha);
    }

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

    /// テスト③: 全透明 → 空エントリ（placement None）であって**エラーではない**（4.4/6.2）。
    #[test]
    fn all_transparent_is_empty_entry_not_error() {
        let base = Path::new("shell/master");
        let shell = vec![surface(0, vec![elem(0, "solid.png"), elem(1, "clear.png")])];

        let mut dec = MemoryDecoder::new();
        register(&mut dec, base, "solid.png", opaque_2x2());
        register(&mut dec, base, "clear.png", transparent_2x2());

        let sets = [set(&shell, base)];
        let result = bake(&sets, &dec, PackConfig::default());

        // 全透明はエラーに載らない。
        assert!(result.errors.is_empty(), "all-transparent is NOT an error");

        // clear.png は resolve→Some だが placement None（空エントリ・転写スキップ）。
        let clear_id = result
            .table
            .resolve(SetId(0), "clear.png")
            .expect("empty entry still resolves");
        assert!(
            result.table.entry(clear_id).placement.is_none(),
            "all-transparent → empty entry (placement None)"
        );
        // 原寸は記録される（4.2/4.5）。
        assert_eq!(
            result.table.entry(clear_id).original,
            crate::table::Size { w: 2, h: 2 }
        );

        // solid.png は placement あり（対照）。
        let solid_id = result.table.resolve(SetId(0), "solid.png").unwrap();
        assert!(result.table.entry(solid_id).placement.is_some());
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
            // entry(id) は panic せず引ける。
            let _ = result.table.entry(id);
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

    /// テスト⑤（構造的）: channel 非依存（R6.5）。BakeResult は素の値であり、通信機構/async を
    /// 一切介さずその場で分解・利用できる。本テスト全体が channel を使わない実証。
    #[test]
    fn bake_result_is_plain_value_channel_independent() {
        let base = Path::new("shell/master");
        let shell = vec![surface(0, vec![elem(0, "only.png")])];
        let mut dec = MemoryDecoder::new();
        register(&mut dec, base, "only.png", opaque_2x2());
        let sets = [set(&shell, base)];

        // 値としてそのまま move・分解（channel も Future も経由しない）。
        let BakeResult { table, errors } = bake(&sets, &dec, PackConfig::default());
        assert!(errors.is_empty());
        assert_eq!(table.len(), 1);
        assert!(table.resolve(SetId(0), "only.png").is_some());
    }
}
