//! 合成キャッシュ（`ComposeCache`）＝ surface id → 表示バッファ・当たり判定マスク対の全保持。
//!
//! 上流 `areka-emo-compose` が値返しする [`ComposedSurface`] を surface id 鍵で全保持し、挿入時に
//! [`AlphaMask`] を **1 回だけ**生成して同一エントリ（[`CacheEntry`]）へ束ねる純粋な状態層である。
//! 表示バッファと当たり判定マスクを 1 エントリに封じることで、対の入替がキャッシュ操作 1 回で
//! 原子的に起きる（R2.4）。表示のたびにマスクを再生成しない（R2.1）ことを、マスク生成を挿入
//! API の内側へ隠して**構造で**担保する（呼び手はマスク生成を書けない・忘れられない）。
//!
//! # 責務分界（合成は持たない・純粋状態層）
//!
//! 本キャッシュは合成器（`Composer`）を所有しない。ミス時に合成して [`insert`] を呼ぶのは提示段
//! （後続 `presenter`）の責務であり、本層は「保持・引き当て・全無効化」だけを担う純粋な状態
//! （設計 §State Management）である。UI スレッド専有（`EmoPresenter` が NonSend）ゆえロックを
//! 持たない。全保持 `HashMap`（emo2 規模で妥当・LRU 不採用＝簡素化の設計判断）で、部分無効化は
//! 実需まで凍結し [`invalidate_all`] のみ提供する（R4.3）。
//!
//! [`insert`]: ComposeCache::insert
//! [`invalidate_all`]: ComposeCache::invalidate_all

use std::collections::HashMap;

use areka_emo_compose::ComposedSurface;
use wintf::ecs::widget::bitmap_source::AlphaMask;

/// キャッシュエントリ＝表示バッファと当たり判定マスクの原子対（R2.4 の構造的担保）。
///
/// `composed` は表示（WUC アップロード）の真実源、`mask` はさわり判定の真実源であり、両者は
/// **同一 `composed.bytes()` 由来**である（[`ComposeCache::insert`] が挿入時に一度だけ生成し束ねる）。
/// 1 エントリへ束ねてあるため、surface 切替に伴う対の入替は `HashMap` 操作 1 回で原子的に起きる。
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// premultiplied BGRA・表示の真実源。
    pub composed: ComposedSurface,
    /// `composed.bytes()` から挿入時に 1 回だけ生成した当たり判定マスク・さわり判定の真実源。
    pub mask: AlphaMask,
}

/// surface id → [`CacheEntry`] の全保持キャッシュ（LRU 不採用・全無効化のみ）。
///
/// UI スレッド専有の純粋な状態容器で、内部ロックを持たない。合成結果の保持と引き当て、および
/// アトラス再構築・ghost 再読込時の全破棄（[`invalidate_all`]）だけを担う。合成器は所有せず、
/// ミス時の合成・挿入は提示段の責務である（本モジュール冒頭 §責務分界）。
///
/// [`invalidate_all`]: ComposeCache::invalidate_all
#[derive(Debug, Default)]
pub struct ComposeCache {
    /// surface id → 表示・マスク対。全保持（部分退去なし）。
    entries: HashMap<u32, CacheEntry>,
}

impl ComposeCache {
    /// 空のキャッシュを構築する。
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// 合成結果を surface id 鍵で挿入し、[`AlphaMask`] を**挿入時に 1 回だけ**生成して束ねる。
    ///
    /// 呼び手は合成済み [`ComposedSurface`] を渡すだけでよく、マスク生成
    /// （[`AlphaMask::from_pbgra32`]）は本メソッド内部で `composed.bytes()`／`width`／`height`／
    /// `stride` から一度だけ行う。マスク生成 API を挿入の内側へ隠すことで、「表示のたびに再生成
    /// しない」（R2.1）を呼び手が破れない構造にする。既存 id への挿入は対ごと置換する（R2.4）。
    ///
    /// 挿入したエントリへの共有参照を返す（提示段がそのまま表示・マスク同期へ用いる）。
    pub fn insert(&mut self, surface_id: u32, composed: ComposedSurface) -> &CacheEntry {
        // マスクは挿入時に 1 回だけ生成し（R2.1）、表示バッファと同一 bytes 由来で束ねる（R2.4）。
        let mask = AlphaMask::from_pbgra32(
            composed.bytes(),
            composed.width(),
            composed.height(),
            composed.stride(),
        );
        self.entries.insert(surface_id, CacheEntry { composed, mask });
        // 直前に挿入したエントリは必ず存在する。
        self.entries
            .get(&surface_id)
            .expect("entry was just inserted")
    }

    /// surface id に対応するエントリを引き当てる（ヒット時は再合成不要の証拠源）。
    pub fn get(&self, surface_id: u32) -> Option<&CacheEntry> {
        self.entries.get(&surface_id)
    }

    /// 全エントリを破棄する（アトラス再構築・ghost 再読込時の唯一の無効化口・R4.3）。
    ///
    /// 以後すべての id がミスし、提示段が再合成して再挿入する。部分無効化は提供しない（簡素化）。
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_atlas::{
        AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
    };
    use areka_emo_compose::{BindSet, Composer, EmoWorld};
    use areka_parsers::shell::{
        AppendTarget, DefRef, Element, ElementPath, Shell, Surface,
    };
    use std::path::Path;

    // ── ComposedSurface 生成補助 ──────────────────────────────────────────────
    // `ComposedSurface::bytes_mut` は emo-compose の pub(crate) ゆえ本クレートから画素を直接
    // 焼けない。よって「全透明」は公開 `new(w,h)` で、「不透明画素を含む結果」は上流公開 API
    // （atlas bake → EmoWorld → Composer::compose）で本物を合成して得る。後者はマスクが
    // composed の bytes 由来であることを実合成経路で保証する（模造バッファでの偽陽性を避ける）。

    /// カウント用途で十分な任意サイズの全透明合成結果（内容は不問・件数計上のみに使う）。
    fn transparent_surface(w: u32, h: u32) -> ComposedSurface {
        ComposedSurface::new(w, h)
    }

    fn elem(path: &str, x: i64, y: i64) -> Element {
        Element {
            layer: 0,
            path: ElementPath::new(path.to_string()),
            x,
            y,
        }
    }

    fn surface(id: u32, elements: Vec<Element>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations: Vec::new(),
        }
    }

    fn shell_of(surfaces: Vec<Surface>) -> Shell {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions,
        }
    }

    /// 不透明画素と透明画素を**必ず両方**含む本物の合成結果を上流公開 API で作る。
    ///
    /// 3×1 の単一画像（不透明赤 / 透明 / 不透明赤）を 1 element として合成する。両端が不透明の
    /// ため α=0 除外トリムでも中央の透明画素が矩形内に残る（＝合成結果に不透明・透明が共存）。
    fn composed_with_opaque_and_transparent() -> ComposedSurface {
        let base = Path::new("shell/master");
        let surfaces = vec![surface(1000, vec![elem("otr.png", 0, 0)])];

        let mut dec = MemoryDecoder::new();
        // 3×1 premultiplied BGRA: (0,0)不透明赤・(1,0)透明・(2,0)不透明赤。stride = 3*4 = 12。
        let img: Vec<u8> = vec![
            0, 0, 200, 255, // (0,0) 不透明赤
            0, 0, 0, 0, // (1,0) 全透明
            0, 0, 200, 255, // (2,0) 不透明赤
        ];
        dec.insert(base.join("otr.png"), 3, 1, 12, img, true);

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

        let mut composer = Composer::new();
        composer
            .compose(&world, &baked.table, 1000, &BindSet::default())
            .expect("静的 element 単体の合成は Ok")
    }

    /// 合成結果 bytes から最初の不透明（α≧128）・最初の透明（α<128）画素座標を探す。
    fn find_opaque_and_transparent(cs: &ComposedSurface) -> ((u32, u32), (u32, u32)) {
        let stride = cs.stride();
        let bytes = cs.bytes();
        let mut opaque = None;
        let mut transparent = None;
        for y in 0..cs.height() {
            for x in 0..cs.width() {
                let alpha = bytes[(y * stride + x * 4 + 3) as usize];
                if alpha >= 128 && opaque.is_none() {
                    opaque = Some((x, y));
                }
                if alpha < 128 && transparent.is_none() {
                    transparent = Some((x, y));
                }
            }
        }
        (
            opaque.expect("fixture は不透明画素を含む"),
            transparent.expect("fixture は透明画素を含む"),
        )
    }

    /// R4.1/R4.2: ミス→1 回だけ計算、ヒット→再計算しない（Composer 不呼出の檻）。
    ///
    /// get-or-insert フローを同一 id で 2 回回し、2 回目がヒット（合成カウンタ据え置き）で
    /// あることを固定する。カウンタ増分＝キャッシュミス時のみ合成する契約の回帰檻。
    #[test]
    fn miss_computes_once_hit_does_not_recompute() {
        let mut cache = ComposeCache::new();
        let mut compose_calls = 0u32;
        let id = 42;

        // 1 回目: ミス → 合成（カウンタ +1）→ 挿入。
        if cache.get(id).is_none() {
            compose_calls += 1;
            cache.insert(id, transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 1, "first access must compose exactly once");

        // 2 回目: ヒット → 合成しない（カウンタ据え置き）。
        if cache.get(id).is_none() {
            compose_calls += 1;
            cache.insert(id, transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 1, "second access is a hit; must not recompute");
        assert!(cache.get(id).is_some(), "entry must be retained after hit");
    }

    /// R4.3: `invalidate_all` 後は同一 id がミスし、再合成される。
    #[test]
    fn invalidate_all_forces_recompute() {
        let mut cache = ComposeCache::new();
        let mut compose_calls = 0u32;
        let id = 7;

        if cache.get(id).is_none() {
            compose_calls += 1;
            cache.insert(id, transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 1);

        cache.invalidate_all();
        assert!(cache.get(id).is_none(), "id must miss after invalidate_all");

        // 無効化後の再アクセスはミス → 再合成（カウンタ +1）。
        if cache.get(id).is_none() {
            compose_calls += 1;
            cache.insert(id, transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 2, "invalidate_all must force a recompute");
    }

    /// R2.1/R2.4: マスクは挿入時に composed の bytes から 1 回生成され、正しく引ける。
    ///
    /// 不透明・透明を両方含む本物の合成結果を挿入し、その bytes から見つけた不透明/透明座標で
    /// エントリの `mask` がヒット/非ヒットを返すことを固定する（マスクが同一 bytes 由来で、
    /// 表示バッファと対で保持される証拠）。
    #[test]
    fn mask_generated_once_from_composed_bytes_and_correct() {
        let composed = composed_with_opaque_and_transparent();
        let ((ox, oy), (tx, ty)) = find_opaque_and_transparent(&composed);
        let (w, h) = (composed.width(), composed.height());

        let mut cache = ComposeCache::new();
        let entry = cache.insert(1000, composed);

        // 同一エントリに束ねたマスクが composed の α を反映する。
        assert!(
            entry.mask.is_hit(ox, oy),
            "opaque pixel ({ox},{oy}) must be a hit in the entry mask"
        );
        assert!(
            !entry.mask.is_hit(tx, ty),
            "transparent pixel ({tx},{ty}) must not be a hit"
        );

        // エントリは composed とマスクを対で保持する（表示の真実源も残る）。
        let got = cache.get(1000).expect("entry retained");
        assert_eq!(got.composed.width(), w);
        assert_eq!(got.composed.height(), h);
        assert!(got.mask.is_hit(ox, oy));
        assert!(!got.mask.is_hit(tx, ty));
    }
}
