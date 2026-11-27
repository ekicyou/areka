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
//! パイプラインの各段と、外へ出す型（段のモジュールはクレートの中だけ）:
//! - マニフェスト導出（全部の面の絵を直に数える・重複排除）— 入力は [`SurfaceSet`]
//! - デコードポート（trait）＋既定 WIC 腕 — [`ElementDecoder`]・[`WicDecoderArm`]・[`MemoryDecoder`]
//! - 透過正規化（premultiplied 統一）— [`Normalizer`]・[`AlphaParams`]
//! - α トリミング（外へ出す型は無い）
//! - packing 座標算出 — [`PackConfig`]
//! - 頁バッファへの焼付 — 入口は [`bake`]
//! - 成果物契約型（正本）— [`AtlasTable`] ほか
//! - 診断可能なエラー型 — [`BakeError`]・[`DecodeError`]

mod animated;
mod asis;
mod bake;
mod decode;
mod error;
mod limits;
mod manifest;
mod normalize;
mod pack;
mod table;
mod trim;

/// `bake` の成果物（索引表＋失敗集合）。
///
/// `table` は成功・空エントリの密 `AtlasTable`（頁バッファ内包）、`errors` は
/// デコードで脱落したエントリの集合。
pub struct BakeResult {
    /// 生存エントリ（Placed / Empty）の密索引表＋頁バッファ。
    pub table: AtlasTable,
    /// 脱落エントリのエラー集合。空でも成功扱い。
    pub errors: Vec<crate::error::BakeError>,
    /// 焼いた絵のうち、同じ名前の `.pna` が添えてあったものの数（`.pna` は使っていない）。
    pub ignored_pna: usize,
}

/// 素材基盤層の公開入口（emo 三段直列 1/3・契約正本）。
///
/// 複数の入力集合（shell 用・balloon 用など）をまとめて処理する。
/// デコードに失敗したエントリは索引表に載せず、[`BakeResult::errors`] へ集めて他エントリの処理を継続する。
///
/// # 決定性
/// マニフェスト採番（`SetId` 昇順・rel_path 昇順）から先の各段が決定的なので、
/// 同一入力（＋同一失敗）で同一 `AtlasTable` を返す（golden 安定）。
///
/// 動く絵の上限は環境変数の既定の取り方（[`AnimationLimits::from_env`]）で決める。
pub fn bake(sets: &[SurfaceSet<'_>], decoder: &impl ElementDecoder, cfg: PackConfig) -> BakeResult {
    bake_with_limits(sets, decoder, cfg, AnimationLimits::from_env())
}

/// 動く絵の上限を外から渡す入口。
///
/// 動く絵（コマが 2 枚以上の APNG・WebP）は全コマを載せる: 0 番のコマは今までの鍵の
/// エントリ、2 枚目以降は鍵のエントリが全部並んだ後ろへ（親の番号・コマの番号の昇順で）
/// 足し、[`AtlasTable::animation`] で引ける。動く絵が無ければ結果は今までと同じ。
///
/// `asis` で参照される絵は、乗算せず・透過色を抜かず・縁を切らず・α＝255 の原寸の絵を通常の絵の
/// 後ろに足し、[`AtlasTable::resolve_asis`] で引ける。[`AtlasTable::resolve`] の答えは変わらない。
pub fn bake_with_limits(
    sets: &[SurfaceSet<'_>],
    decoder: &impl ElementDecoder,
    cfg: PackConfig,
    limits: AnimationLimits,
) -> BakeResult {
    bake_core(sets, decoder, cfg, limits, Records::Emit)
}

/// 記録を出さない焼きの入口。
///
/// [`bake`] と同じ中身で焼き、戻り値も等しい。絵ごとの記録（抜き色の `debug!`・元画像が
/// 0 寸の `warn!`・着せ替えの「無し」の絵の `debug!`・全透明の `warn!`）だけを出さない。
/// 動く絵の上限は [`bake`] と同じく [`AnimationLimits::from_env`] で決める。
///
/// 動く絵の 3 本の `warn!` はこの入口でも出る。採寸の道で出ないのは採寸の読み手が動く絵の
/// 道へ入らせないため。前提の崩れの `error!`（`bake.rs`・`pack.rs`）もいつも出る。
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
    let manifest = ManifestDeriver.derive(sets);
    // 着せ替え（bind）のアニメーションからだけ参照される面の絵（全透明でも警告しない側）。
    let bind_only = manifest::bind_only_paths(sets);

    let mut errors: Vec<crate::error::BakeError> = Vec::new();
    // この呼び出しで全コマを載せた動く絵の画素の合計（上限の判定に使う・呼び出しごとに 0 から）。
    let mut used_pixels = 0u64;
    let mut ignored_pna = 0usize;
    let mut pending_frames: Vec<animated::PendingFrames> = Vec::new();

    // 生存エントリ（デコードを通過）を manifest 順で密に積む。survivor_keys[final] が最終 ElementId(final)。
    // original は Empty も含め全生存で持ち、placed_items には Placed だけ入れる。
    let mut survivor_keys: Vec<AtlasKey> = Vec::new();
    let mut survivor_originals: Vec<crate::table::Size> = Vec::new();
    let mut placed_items: Vec<(ElementId, Trimmed)> = Vec::new();

    for key in &manifest.keys {
        let set = &sets[key.set.0 as usize];

        let path = set.base_dir.join(&key.rel_path);

        if key.kind == PictureKind::Asis {
            let picture = match asis::load(decoder, &path) {
                Ok(picture) => picture,
                Err(e) => {
                    errors.push(crate::error::BakeError::Decode(e));
                    continue;
                }
            };
            let final_id = ElementId(survivor_keys.len() as u32);
            survivor_keys.push(key.clone());
            survivor_originals.push(picture.original);
            if let Some(placed) = picture.placement {
                placed_items.push((final_id, placed));
            }
            continue;
        }

        // 動く絵は 0 番のコマが静止画と同じ道を通り、残りのコマ・待ち時間・繰り返し回数は脇へ取っておく。
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

        if decoder.probe_pna(&path) {
            ignored_pna += 1;
        }

        // 画素の扱いは絵ごとに 1 枚目で 1 回だけ決め、動く絵なら残りのコマにも同じ扱いを当てる。
        // 正規化は失敗しないので、読めた動く絵は必ず合計に足す。
        let rule = Normalizer::plan(&decoded, set.alpha_params);
        let normalized = normalize::apply_to(decoded, rule);
        used_pixels += frames_pixels;

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

        let trim = Trimmer.trim(&normalized);
        let final_id = ElementId(survivor_keys.len() as u32);

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

        // 0 寸・全透明の element はゴースト制作者のミスの可能性が高いので警告する（ログだけで、生成結果は変えない）。
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
            // 動く絵の透明なコマは正当な 1 コマ。全部のコマが透明な動く絵にだけ出す。
            if bind_only.contains(&(key.set.0, key.rel_path.clone())) {
                // 着せ替えの「無し」の絵（作者の意図した空の絵）は警告せず記録だけ残す。
                // 文言は数え直しで警告と混ざらないよう別にする。
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

    let animations = animated::append_frames(
        pending_frames,
        &mut survivor_keys,
        &mut survivor_originals,
        &mut placed_items,
    );

    let pack = Packer.pack(&placed_items, cfg);

    let (pages, placements) = Baker.bake_pages(&placed_items, &pack);
    let placement_of: std::collections::HashMap<ElementId, Placement> =
        placements.into_iter().collect();

    let entries: Vec<AtlasEntry> = survivor_originals
        .into_iter()
        .enumerate()
        .map(|(final_idx, original)| AtlasEntry {
            original,
            placement: placement_of.get(&ElementId(final_idx as u32)).cloned(),
        })
        .collect();

    let table = AtlasTable::with_frames(survivor_keys, entries, pages, animations);

    BakeResult {
        table,
        errors,
        ignored_pna,
    }
}

// 成果物契約の正本型。下流 emo-compose はこれらを import する。
pub use table::{
    Animation, AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, LoopCount, PictureKind,
    Placement, Point, Rect, SetId, Size,
};

// デコードポート。既定手段（WIC）を露出せず、trait とデータ型のみ公開。
pub use decode::{
    AnimatedImage, AnimationFrame, AnimationInfo, DecodeError, DecodedImage, ElementDecoder,
    MemoryDecoder,
};

pub use limits::AnimationLimits;

// 既定デコード腕（WIC 経由・COM 隔離）。上位は trait 越しに用いる。
pub use decode::wic_arm::WicDecoderArm;

pub use manifest::SurfaceSet;

// 共有透過パラメータ型（SurfaceSet が運ぶ）。
pub use normalize::{AlphaParams, UseSelfAlpha};

pub use normalize::{NormalizedImage, Normalizer};

pub use pack::PackConfig;

pub use error::BakeError;

// 焼きの中の段（マニフェスト・トリム・packing・焼付）。クレートの外には出さない。
pub(crate) use bake::Baker;
pub(crate) use manifest::ManifestDeriver;
pub(crate) use pack::Packer;
pub(crate) use trim::{Trimmed, Trimmer};

// 検体を窓口から取得しプロセス寿命で保持する共有の受け口。
#[cfg(test)]
mod sample_test_support;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod emo2_e2e;

#[cfg(test)]
mod emo2_golden;

// tracing ログ捕捉ハーネス（0 寸/全透明 warn の決定論 assert 用）。
#[cfg(test)]
mod log_capture;

// image-webp を固定コミットから取り込んでいることの検査。
#[cfg(test)]
mod webp_pin_tests;

#[cfg(test)]
mod bake_quiet_tests;

#[cfg(test)]
mod samples_e2e_tests;

#[cfg(test)]
mod bake_entry_tests;

// 信用できない読み手の答えを焼きの段で断るテスト。
#[cfg(test)]
mod bake_guard_tests;
