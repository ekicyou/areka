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

mod animated;
pub mod bake;
pub mod decode;
pub mod error;
mod limits;
pub mod manifest;
pub mod normalize;
pub mod pack;
pub mod table;
pub mod trim;

/// `bake` の成果物（索引表＋失敗集合・channel 非依存・R6.5）。
///
/// `table` は成功・空エントリの密 `AtlasTable`（頁バッファ内包）、`errors` は
/// デコードで脱落したエントリの診断可能な集合。値・共有参照として直接返す
/// （通信機構を介さない・R6.5）。
pub struct BakeResult {
    /// 生存エントリ（Placed / Empty）の密索引表＋頁バッファ。
    pub table: AtlasTable,
    /// 脱落エントリの診断可能なエラー集合（R2.2）。空でも成功扱い。
    pub errors: Vec<crate::error::BakeError>,
    /// 焼いた絵のうち、同じ名前の `.pna` が添えてあったものの数（`.pna` は使っていない）。
    pub ignored_pna: usize,
}

/// 素材基盤層の公開入口（emo 三段直列 1/3・契約正本）。
///
/// マニフェスト導出 → デコード → 正規化 → トリム → packing → 焼付の各段を単一の
/// 入口関数として結線し、複数の入力集合（shell 用・balloon 用など）をまとめて処理する。
/// デコードに失敗したエントリは索引表に載せず、失敗内容を [`BakeResult::errors`]
/// へ集約しつつ他エントリの処理を継続する（R2.2）。索引表と頁バッファは通信機構を
/// 介さず値・共有参照として直接返す（R6.5）。
///
/// # 決定性
/// マニフェスト採番（`SetId` 昇順・rel_path 昇順）→ 同一失敗集合 → 同一生存集合 →
/// 同一密再採番、と各段が決定的ゆえ、同一入力（＋同一失敗）で同一 `AtlasTable` を返す
/// （golden 安定・D7）。
///
/// 動く絵の上限は環境変数の既定の取り方（[`AnimationLimits::from_env`]）で決める。
pub fn bake(sets: &[SurfaceSet<'_>], decoder: &impl ElementDecoder, cfg: PackConfig) -> BakeResult {
    bake_with_limits(sets, decoder, cfg, AnimationLimits::from_env())
}

/// 動く絵の上限を外から渡す入口（spec: areka-P0-animated-image-decode 要件 6.9）。
///
/// 動く絵（コマが 2 枚以上の APNG・WebP）は全コマを載せる: 0 番のコマは今までの鍵の
/// エントリ、2 枚目以降は鍵のエントリが全部並んだ後ろへ（親の番号・コマの番号の昇順で）
/// 足し、[`AtlasTable::animation`] で引ける。動く絵が無ければ結果は今までと同じ。
pub fn bake_with_limits(
    sets: &[SurfaceSet<'_>],
    decoder: &impl ElementDecoder,
    cfg: PackConfig,
    limits: AnimationLimits,
) -> BakeResult {
    bake_core(sets, decoder, cfg, limits, Records::Emit)
}

/// 記録を出さない焼きの入口（spec: emo-bake-log-overlap 要件 1.1〜1.3・1.6）。
///
/// [`bake`] と同じ中身で焼き、戻り値も等しい。絵ごとの記録（抜き色の `debug!`・元画像が
/// 0 寸の `warn!`・着せ替えの「無し」の絵の `debug!`・全透明の `warn!`）だけを出さない。
/// 動く絵の上限は [`bake`] と同じく [`AnimationLimits::from_env`] で決める。
///
/// 動く絵の 3 本の `warn!` はこの入口でも出る。採寸の道で出ないのは採寸の読み手が動く絵の
/// 道へ入らせないため。前提の崩れの `error!`（`bake.rs`・`pack.rs`）もいつも出る
/// （同 要件 1.7）。
pub fn bake_quiet(
    sets: &[SurfaceSet<'_>],
    decoder: &impl ElementDecoder,
    cfg: PackConfig,
) -> BakeResult {
    bake_core(
        sets,
        decoder,
        cfg,
        AnimationLimits::from_env(),
        Records::Quiet,
    )
}

/// 記録を出すかどうか（中身は同じ 1 本）。クレートの中だけで使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Records {
    Emit,
    Quiet,
}

impl Records {
    pub(crate) fn emits(self) -> bool {
        self == Records::Emit
    }
}

/// 焼きの中身（入口 3 つが共有する 1 本）。絵ごとの記録を出すのは `records` が出すときだけ。
fn bake_core(
    sets: &[SurfaceSet<'_>],
    decoder: &impl ElementDecoder,
    cfg: PackConfig,
    limits: AnimationLimits,
    records: Records,
) -> BakeResult {
    // 1. マニフェスト導出（列挙・重複排除・決定的採番）。
    let manifest = ManifestDeriver.derive(sets);
    // 着せ替え（bind）のアニメーションからだけ参照される面の絵（全透明でも警告しない側・
    // real-run-log-noise 要件 3.7）。入口で 1 回だけ求める。
    let bind_only = manifest::bind_only_paths(sets);

    let mut errors: Vec<crate::error::BakeError> = Vec::new();
    // この呼び出しで全コマを載せた動く絵の画素の合計（上限 ㋒・呼び出しごとに 0 から）。
    let mut used_pixels = 0u64;
    // 同じ名前の `.pna` が添えてあった絵の数（`.pna` は使わない）。
    let mut ignored_pna = 0usize;
    // 2 枚目以降のコマ（親の番号の昇順）。全部の鍵を回った後に番号を振る。
    let mut pending_frames: Vec<animated::PendingFrames> = Vec::new();

    // 生存エントリ（デコードを通過）を manifest 順で密に積む。
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

        // デコード（失敗＝索引表に載せず・エラー集約・継続・R2.2）。動く絵は 0 番のコマが
        // 静止画と同じ道を通り、残りのコマ・待ち時間・繰り返し回数は脇へ取っておく。
        let (decoded, animation, frames_pixels) =
            match animated::load(decoder, &path, key, limits, cfg, used_pixels) {
                animated::Loaded::Still(d) => (d, None, 0),
                animated::Loaded::Frames(anim, pixels) => {
                    let mut frames = anim.frames.into_iter();
                    let first = frames.next().expect("load checks >= 2 frames");
                    (
                        first.image,
                        Some((first.delay_ms, frames.collect(), anim.loop_count)),
                        pixels,
                    )
                }
                animated::Loaded::Failed(e) => {
                    errors.push(crate::error::BakeError::Decode(e));
                    continue;
                }
            };

        // `.pna` は使わず、添えてあった絵を数えるだけ（spec: areka-P0-self-alpha-declaration 要件 5.7）。
        if decoder.probe_pna(&path) {
            ignored_pna += 1;
        }

        // 画素の扱いは絵ごとに 1 枚目で 1 回だけ決め、動く絵なら残りのコマにも同じ扱いを当てる
        // （同 要件 9.4）。正規化は失敗しないので、読めた動く絵は必ず合計に足す
        // （spec: areka-P0-animated-image-decode 要件 6.8）。
        let rule = Normalizer::plan(&decoded, set.alpha_params);
        let normalized = normalize::apply_to(decoded, rule);
        used_pixels += frames_pixels;

        // 抜き色で扱った絵は、抜いた色を記録する（spec: areka-P0-shell-implicit-surface 要件 6.3）。
        if let Some([b, g, r, a]) = rule.key
            && records.emits()
        {
            tracing::debug!(
                target: "areka_emo_atlas",
                set = key.set.0,
                rel_path = key.rel_path.as_str(),
                b = b,
                g = g,
                r = r,
                a = a,
                "bake: element を抜き色（左上の 1 画素と同じ色）で透過しました"
            );
        }

        // トリム（全透明→空エントリ／それ以外→配置エントリ）。生存確定＝密採番へ加える。
        let trim = Trimmer.trim(&normalized);
        let final_id = ElementId(survivor_keys.len() as u32);

        // 動く絵の残りのコマに 0 番と同じ扱いを当てて切り詰める（要件 3.1〜3.3）。
        let frames = animation.map(|(first_delay_ms, rest, loop_count)| {
            animated::PendingFrames::new(
                final_id,
                key.clone(),
                first_delay_ms,
                rest,
                loop_count,
                rule,
            )
        });

        // 0 寸/全透明 element の早期警告（ゴースト制作者ミスの可能性が高いため・設計ディスカッション #1）。
        // **ログのみの増分**: bake の生成結果（索引表・placement・trim 矩形）は一切変更しない。
        // (a) 元画像 0 寸（0×0）／(b) α=0 トリム後 0 寸（全透明→placement None）を判別してログ。
        if records.emits() && (trim.original.w == 0 || trim.original.h == 0) {
            tracing::warn!(
                target: "areka_emo_atlas",
                set = key.set.0,
                rel_path = key.rel_path.as_str(),
                original_w = trim.original.w,
                original_h = trim.original.h,
                "bake: element の元画像が 0 寸です（ゴースト制作者ミスの可能性）"
            );
        } else if records.emits()
            && trim.placement.is_none()
            && frames.as_ref().is_none_or(|f| f.all_transparent())
        {
            // 動く絵の透明なコマは正当な 1 コマ。全部のコマが透明な動く絵にだけ出す（要件 3.5）。
            if bind_only.contains(&(key.set.0, key.rel_path.clone())) {
                // 着せ替えの「無し」の絵（作者の意図した空の絵）は警告せず記録だけ残す
                // （real-run-log-noise 要件 3.4）。文言は数え直しで警告と混ざらないよう別にする。
                tracing::debug!(
                    target: "areka_emo_atlas",
                    set = key.set.0,
                    rel_path = key.rel_path.as_str(),
                    original_w = trim.original.w,
                    original_h = trim.original.h,
                    "bake: element が全透明（α=0）で、着せ替え（bind）のアニメーションからだけ参照される面の絵です——着せ替えの「無し」の絵とみなし、警告しない"
                );
            } else {
                tracing::warn!(
                    target: "areka_emo_atlas",
                    set = key.set.0,
                    rel_path = key.rel_path.as_str(),
                    original_w = trim.original.w,
                    original_h = trim.original.h,
                    "bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性）"
                );
            }
        }

        survivor_keys.push(key.clone());
        survivor_originals.push(trim.original);
        if let Some(trimmed) = trim.placement {
            placed_items.push((final_id, trimmed));
        }
        pending_frames.extend(frames);
    }

    // 2 枚目以降のコマを鍵のエントリの後ろへ（親の番号・コマの番号の昇順・要件 4.7）。
    let animations = animated::append_frames(
        pending_frames,
        &mut survivor_keys,
        &mut survivor_originals,
        &mut placed_items,
    );

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
    let table = AtlasTable::with_frames(survivor_keys, entries, pages_vec, animations);

    // 8. 値で直接返す（channel 非依存・R6.5）。
    BakeResult {
        table,
        errors,
        ignored_pna,
    }
}

// 成果物契約の正本型（D3・R6）。下流 emo-compose はこれらを import する。
pub use table::{
    Animation, AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, LoopCount, Placement, Point,
    Rect, SetId, Size,
};

// デコードポート（D4・R2.3）。既定手段（WIC）を露出せず、trait とデータ型のみ公開。
pub use decode::{
    AnimatedImage, AnimationFrame, AnimationInfo, DecodeError, DecodedImage, ElementDecoder,
    MemoryDecoder,
};

// 動く絵の 3 つの上限（spec: areka-P0-animated-image-decode 要件 6.9）。
pub use limits::AnimationLimits;

// 既定デコード腕（WIC 経由・COM 隔離・D4）。上位は trait 越しに用いる。
pub use decode::wic_arm::WicDecoderArm;

// マニフェスト導出（列挙層・R1.1–1.6/5.6・D6）。
pub use manifest::{Manifest, ManifestDeriver, SurfaceSet};

// 共有透過パラメータ型（normalize の設計本拠・SurfaceSet が運ぶ・3.6）。
pub use normalize::{AlphaParams, UseSelfAlpha};

// 正規化層（use_self_alpha 解釈・premultiplied BGRA 統一・R3・D5/D8）。
pub use normalize::{NormalizedImage, Normalizer};

// トリム層（α>0 タイト bbox・オフセット記録・全透明→空・R4・D8）。
pub use trim::{TrimResult, Trimmed, Trimmer};

// packing 層（決定的・複数頁・padding・座標のみ・R5・D7）。
pub use pack::{PackConfig, PackOutput, PackedEntry, Packer};

// 焼付層（頁確保・premultiplied 素通し blit・座標不変・R4.3/R6.3・D8・Critical Issue 3）。
pub use bake::Baker;

// 診断可能なエラー型（bake パイプラインの脱落集約・R2.2）。
pub use error::BakeError;

// 検体を窓口から取得しプロセス寿命で保持する共有の受け口（spec: areka-P0-nar-install 要件 1.6）。
#[cfg(test)]
mod sample_test_support;

// クレートのテストが共有する部品の置き場（COM の手伝い・組み立て・読み出し・spec: audit-emo-atlas 指摘 4・10）。
#[cfg(test)]
pub(crate) mod test_support;

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

// image-webp を固定コミットから取り込んでいることの検査（spec: areka-P0-animated-image-decode 要件 7.6）。
#[cfg(test)]
mod webp_pin_tests;

// 記録を出さない焼きの入口の檻（spec: emo-bake-log-overlap 要件 1.1〜1.3・1.6・3.2・3.4）。
#[cfg(test)]
#[path = "bake_quiet_tests.rs"]
mod bake_quiet_tests;

// 本物の読み手で動く絵の検体のフォルダを焼く結合テスト（spec: areka-P0-animated-image-decode 要件 8.3〜8.6）。
#[cfg(test)]
mod samples_e2e_tests;

// 焼きの入口 `bake` の檻（spec: audit-emo-atlas 指摘 19 で兄弟のファイルへ移した）。
#[cfg(test)]
mod bake_entry_tests;
