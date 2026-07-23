//! 合成メモ（`ComposeCache`）＝ **合成入力（surface id ＋ bind 集合）** → 表示バッファ・当たり判定マスク対の
//! 容量 1 メモ化スロット。
//!
//! 上流 `areka-emo-compose` の合成は `(surface_id, BindSet)` の純粋関数であり、**キーが合成入力の
//! 全体を捕捉しない限りキャッシュは正しくない**（surface id のみをキーにすると、同一 surface で
//! bind 集合だけ異なる着せ替え・まばたきが古い合成結果に衝突する）。本スロットは直前の合成入力と
//! その結果を 1 件だけ保持し、**同一入力なら再利用・1 ビットでも異なれば必ずミス（＝再合成）**を
//! 構造で担保する。多エントリ保持は採らない: seriko のアニメ pattern 状態を合成入力へ加えると
//! 状態空間が膨張し、全保持はメモリ堆積（1 エントリ＝原寸ビットマップ）と低ヒット率の二重苦になる
//! ため、「状態が変わらない間だけ前回画像を継続する」直近 1 件こそが正しい戦略である。
//!
//! 挿入時に [`AlphaMask`] を **1 回だけ**生成して同一エントリ（[`CacheEntry`]）へ束ねる純粋な状態層で
//! ある点は従来どおり。表示バッファと当たり判定マスクを 1 エントリに封じることで、対の入替が
//! スロット操作 1 回で原子的に起きる（R2.4）。表示のたびにマスクを再生成しない（R2.1）ことを、
//! マスク生成を挿入 API の内側へ隠して**構造で**担保する（呼び手はマスク生成を書けない・忘れられない）。
//!
//! # 責務分界（合成は持たない・純粋状態層）
//!
//! 本スロットは合成器（`Composer`）を所有しない。ミス時に合成して [`insert`] を呼ぶのは提示段
//! （`presenter`）の責務であり、本層は「保持・引き当て・全無効化」だけを担う純粋な状態
//! （設計 §State Management）である。UI スレッド専有（`EmoPresenter` が NonSend）ゆえロックを
//! 持たない。無効化はアトラス再構築・ghost 再読込用の [`invalidate_all`] のみ提供する（R4.3）。
//!
//! [`insert`]: ComposeCache::insert
//! [`invalidate_all`]: ComposeCache::invalidate_all

use areka_emo_compose::{BindSet, ComposedSurface, PatternState};
use wintf::ecs::widget::bitmap_source::AlphaMask;

/// キャッシュエントリ＝表示バッファと当たり判定マスクの原子対（R2.4 の構造的担保）。
///
/// `composed` は表示（WUC アップロード）の真実源、`mask` はさわり判定の真実源であり、両者は
/// **同一 `composed.bytes()` 由来**である（[`ComposeCache::insert`] が挿入時に一度だけ生成し束ねる）。
/// 1 エントリへ束ねてあるため、surface 切替に伴う対の入替はスロット操作 1 回で原子的に起きる。
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// premultiplied BGRA・表示の真実源。
    pub composed: ComposedSurface,
    /// `composed.bytes()` から挿入時に 1 回だけ生成した当たり判定マスク・さわり判定の真実源。
    pub mask: AlphaMask,
}

/// 合成入力キー＝合成結果を一意に定める入力の全体（surface id ＋ bind 集合 ＋ pattern 状態）。
///
/// `EmoWorld`／`AtlasTable` は target 構築時に固定（変わるときは [`ComposeCache::invalidate_all`] が
/// 走る契約）ゆえキーに含めない。seriko のアニメ pattern 状態（[`PatternState`]）は合成入力の第一級
/// 要素として本キーに含める（R5.2）——1 ビットでも異なれば（surface id・binds・pattern のいずれか）
/// ミスして再合成する、という「キー＝合成入力の全体」不変条件を保つ。`PatternState` の等価は内部
/// `BTreeMap` の正準（昇順）順序で安定する（task 2）ため、挿入順に依存せず決定論的にヒット判定できる。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ComposeKey {
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
}

/// 合成入力 → [`CacheEntry`] の容量 1 メモ化スロット（直前の合成結果のみ保持）。
///
/// UI スレッド専有の純粋な状態容器で、内部ロックを持たない。直前の合成入力と結果の保持・
/// 完全一致引き当て、およびアトラス再構築・ghost 再読込時の破棄（[`invalidate_all`]）だけを担う。
/// 合成器は所有せず、ミス時の合成・挿入は提示段の責務である（本モジュール冒頭 §責務分界）。
///
/// [`invalidate_all`]: ComposeCache::invalidate_all
#[derive(Debug, Default)]
pub struct ComposeCache {
    /// 直前の合成入力と結果の対。`None` は未合成／無効化済み。
    slot: Option<(ComposeKey, CacheEntry)>,
}

impl ComposeCache {
    /// 空のスロットを構築する。
    pub fn new() -> Self {
        Self { slot: None }
    }

    /// 合成結果を合成入力（surface id ＋ bind 集合 ＋ pattern 状態）鍵で挿入し、[`AlphaMask`] を
    /// **挿入時に 1 回だけ**生成して束ねる。既存スロットは対ごと置換する（直前 1 件のみ保持・R2.4）。
    ///
    /// 呼び手は合成済み [`ComposedSurface`] を渡すだけでよく、マスク生成
    /// （[`AlphaMask::from_pbgra32`]）は本メソッド内部で `composed.bytes()`／`width`／`height`／
    /// `stride` から一度だけ行う。マスク生成 API を挿入の内側へ隠すことで、「表示のたびに再生成
    /// しない」（R2.1）を呼び手が破れない構造にする。
    ///
    /// `pattern` は seriko のアニメ pattern 状態（[`PatternState`]）で、`binds` と同格の合成入力
    /// キー要素である（R5.2）。挿入したエントリへの共有参照を返す（提示段がそのまま表示・マスク
    /// 同期へ用いる）。
    pub fn insert(
        &mut self,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        composed: ComposedSurface,
    ) -> &CacheEntry {
        // マスクは挿入時に 1 回だけ生成し（R2.1）、表示バッファと同一 bytes 由来で束ねる（R2.4）。
        let mask = AlphaMask::from_pbgra32(
            composed.bytes(),
            composed.width(),
            composed.height(),
            composed.stride(),
        );
        let key = ComposeKey {
            surface_id,
            binds,
            pattern,
        };
        self.slot = Some((key, CacheEntry { composed, mask }));
        // 直前に挿入したスロットは必ず存在する。
        &self.slot.as_ref().expect("slot was just inserted").1
    }

    /// 合成入力（surface id ＋ bind 集合 ＋ pattern 状態）が直前の合成と**完全一致**するときのみ
    /// エントリを返す。
    ///
    /// surface id・bind 集合・pattern 状態のいずれかが 1 ビットでも異なればミス（＝呼び手は再合成
    /// する）。これが「同一 surface の着せ替え切替・アニメ pattern 進行で古い絵を返さない」ことの
    /// 構造的担保である（R5.2）。`pattern` 等価は [`PatternState`] の `Eq`（正準順序で安定・task 2）に従う。
    pub fn get(
        &self,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
    ) -> Option<&CacheEntry> {
        match &self.slot {
            Some((key, entry))
                if key.surface_id == surface_id
                    && key.binds == *binds
                    && key.pattern == *pattern =>
            {
                Some(entry)
            }
            _ => None,
        }
    }

    /// スロットを破棄する（アトラス再構築・ghost 再読込時の唯一の無効化口・R4.3）。
    ///
    /// 以後あらゆる合成入力がミスし、提示段が再合成して再挿入する。
    pub fn invalidate_all(&mut self) {
        self.slot = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_atlas::{
        AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
    };
    use areka_emo_compose::{BindSet, ComposeMethod, Composer, EmoWorld, PatternFrame};
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

    /// 非空の `PatternState`（animation `anim_id` に surface `surf` の `Overlay` コマ 1 枚）を作る。
    /// `PatternState::default()` と等価でないことを保証するキー要素の実体（pattern 差分の檻用）。
    fn pattern_of(anim_id: u32, surf: u32) -> PatternState {
        let mut p = PatternState::default();
        p.set(
            anim_id,
            PatternFrame {
                surface_id: surf,
                method: ComposeMethod::Overlay,
                x: 0,
                y: 0,
            },
        );
        p
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
            .compose(&world, &baked.table, 1000, &BindSet::default(), &PatternState::default())
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

    /// R4.1/R4.2: ミス→1 回だけ計算、同一合成入力のヒット→再計算しない（Composer 不呼出の檻）。
    ///
    /// get-or-insert フローを同一 (id, binds) で 2 回回し、2 回目がヒット（合成カウンタ据え置き）で
    /// あることを固定する。カウンタ増分＝キャッシュミス時のみ合成する契約の回帰檻。
    #[test]
    fn miss_computes_once_hit_does_not_recompute() {
        let mut cache = ComposeCache::new();
        let mut compose_calls = 0u32;
        let id = 42;
        let binds = BindSet::default();

        // 1 回目: ミス → 合成（カウンタ +1）→ 挿入。
        if cache.get(id, &binds, &PatternState::default()).is_none() {
            compose_calls += 1;
            cache.insert(id, binds.clone(), PatternState::default(), transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 1, "first access must compose exactly once");

        // 2 回目: 同一合成入力＝ヒット → 合成しない（カウンタ据え置き）。
        if cache.get(id, &binds, &PatternState::default()).is_none() {
            compose_calls += 1;
            cache.insert(id, binds.clone(), PatternState::default(), transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 1, "second access is a hit; must not recompute");
        assert!(cache.get(id, &binds, &PatternState::default()).is_some(), "entry must be retained after hit");
    }

    /// 回帰檻（キャッシュ仕様バグ）: **同一 surface id でも bind 集合が異なればミス**する。
    ///
    /// surface id のみをキーにした旧設計では、同一 surface の着せ替え・まばたき（bind 差分）が
    /// 古い合成結果にヒットし表示が更新されなかった。合成入力（id＋binds）の完全一致のみを
    /// ヒットとすることを固定する。
    #[test]
    fn different_binds_on_same_surface_must_miss() {
        let mut cache = ComposeCache::new();
        let id = 1000;
        let eyes_open = BindSet::from_ids([1101, 1302]);
        let eyes_closed = BindSet::from_ids([1101, 1302, 1400]);

        cache.insert(id, eyes_open.clone(), PatternState::default(), transparent_surface(4, 4));
        assert!(cache.get(id, &eyes_open, &PatternState::default()).is_some(), "同一入力はヒットする");
        assert!(
            cache.get(id, &eyes_closed, &PatternState::default()).is_none(),
            "同一 surface でも bind 集合が異なればミスしなければならない（着せ替えバグの回帰檻）"
        );

        // 異なる binds を挿入すると slot は置換され、以後は新入力のみヒットする（直前 1 件保持）。
        cache.insert(id, eyes_closed.clone(), PatternState::default(), transparent_surface(4, 4));
        assert!(cache.get(id, &eyes_closed, &PatternState::default()).is_some(), "置換後は新入力がヒットする");
        assert!(
            cache.get(id, &eyes_open, &PatternState::default()).is_none(),
            "容量 1 メモ: 置換後の旧入力はミスする（無限堆積しない）"
        );
    }

    /// R5.2 回帰檻（pattern がキー要素）: **surface id ＋ bind 集合が完全同一でも pattern が異なれば
    /// ミス**する。pattern を合成入力キーへ加えた（task 8.1）ことの load-bearing な証拠——この 1 点が
    /// 欠けると seriko のアニメ pattern 進行が古い合成結果に衝突し表示が更新されない。
    ///
    /// 同値 pattern ではヒット、pattern を 1 コマ変えるとミス、を同一 (id, binds) で固定する。
    #[test]
    fn different_pattern_on_same_surface_and_binds_must_miss() {
        let mut cache = ComposeCache::new();
        let id = 1000;
        let binds = BindSet::from_ids([1101, 1302]);
        let pattern_a = pattern_of(2000, 1001);
        let pattern_b = pattern_of(2000, 1002);
        assert_ne!(pattern_a, pattern_b, "前提: 2 つの pattern 状態は異なる");

        cache.insert(id, binds.clone(), pattern_a.clone(), transparent_surface(4, 4));

        // (1) 同一 (id, binds, pattern) → ヒット。
        assert!(
            cache.get(id, &binds, &pattern_a).is_some(),
            "surface id・binds・pattern が完全一致すればヒットする"
        );
        // (2) surface id・binds は同一だが pattern が異なる → ミス（新キー要素が load-bearing）。
        assert!(
            cache.get(id, &binds, &pattern_b).is_none(),
            "surface id・binds 同一でも pattern が異なればミスしなければならない（R5.2・pattern がキー要素）"
        );

        // 置換後は新 pattern のみヒット・旧 pattern はミス（容量 1 メモ・古い絵を返さない）。
        cache.insert(id, binds.clone(), pattern_b.clone(), transparent_surface(4, 4));
        assert!(
            cache.get(id, &binds, &pattern_b).is_some(),
            "置換後は新 pattern がヒットする"
        );
        assert!(
            cache.get(id, &binds, &pattern_a).is_none(),
            "容量 1 メモ: 置換後の旧 pattern はミスする"
        );
    }

    /// R5.4 の逆側檻（空 pattern はキーへ寄与しない＝拡張前と観測等価）と、非空 pattern の同値
    /// ヒットを固定する。空 pattern で挿入したエントリは空 pattern の get にヒットし、非空 pattern の
    /// get にはミスする（＝空と非空が別キー）。
    #[test]
    fn empty_vs_nonempty_pattern_are_distinct_keys() {
        let mut cache = ComposeCache::new();
        let id = 42;
        let binds = BindSet::default();
        let pat = pattern_of(3000, 5000);

        // 空 pattern で挿入 → 空 pattern はヒット・非空 pattern はミス。
        cache.insert(id, binds.clone(), PatternState::default(), transparent_surface(4, 4));
        assert!(
            cache.get(id, &binds, &PatternState::default()).is_some(),
            "空 pattern で挿入 → 空 pattern の get はヒット（拡張前と観測等価・R5.4）"
        );
        assert!(
            cache.get(id, &binds, &pat).is_none(),
            "空 pattern で挿入 → 非空 pattern の get はミス（空と非空は別キー）"
        );

        // 非空 pattern で挿入 → 同値の非空 pattern はヒット・空 pattern はミス。
        cache.insert(id, binds.clone(), pat.clone(), transparent_surface(4, 4));
        assert!(
            cache.get(id, &binds, &pat).is_some(),
            "非空 pattern で挿入 → 同値 pattern の get はヒット"
        );
        assert!(
            cache.get(id, &binds, &PatternState::default()).is_none(),
            "非空 pattern で挿入 → 空 pattern の get はミス"
        );
    }

    /// R4.3 変わらず: `invalidate_all` は pattern に依らずスロットを破棄する（挙動不変）。
    /// 非空 pattern で挿入したエントリも `invalidate_all` 後は同一 pattern でミスする。
    #[test]
    fn invalidate_all_clears_regardless_of_pattern() {
        let mut cache = ComposeCache::new();
        let id = 7;
        let binds = BindSet::from_ids([1100]);
        let pat = pattern_of(2000, 1001);

        cache.insert(id, binds.clone(), pat.clone(), transparent_surface(4, 4));
        assert!(
            cache.get(id, &binds, &pat).is_some(),
            "挿入直後は同一 (id, binds, pattern) がヒットする"
        );

        cache.invalidate_all();
        assert!(
            cache.get(id, &binds, &pat).is_none(),
            "invalidate_all は pattern に依らずスロットを破棄する（R4.3 挙動不変）"
        );
    }

    /// R6.1: 動的 Show 再発行で **直前と異なる着せ替え集合**を載せると、合成キャッシュがミスし
    /// 再合成が走る（mayuna 動的 bind 文脈の回帰檻）。
    ///
    /// seriko が per-scope の着せ替え状態を積み替えて新 `BindSet` を載せた `Show` を再発行する
    /// 経路を、提示段の get-or-insert フローで模す。同一 surface id でも bind 集合が 1 要素でも
    /// 異なれば `get` がミスし、提示段が再合成する（合成カウンタ +1）ことを固定する。
    /// `different_binds_on_same_surface_must_miss` がキー完全一致の構造を固定するのに対し、
    /// 本檻は「Show 再発行 → ミス → 再合成」という動的発行フローでの再合成駆動を固定する。
    #[test]
    fn dynamic_show_reissue_different_binds_recomposes() {
        let mut cache = ComposeCache::new();
        let mut compose_calls = 0u32;
        let id = 1000;
        let dressed_a = BindSet::from_ids([1100]);
        let dressed_b = BindSet::from_ids([1100, 1207]);
        assert_ne!(dressed_a, dressed_b, "前提: 2 つの着せ替え集合は異なる");

        // 1 回目の Show（BindSet A）: ミス → 再合成（カウンタ +1）→ 挿入。
        if cache.get(id, &dressed_a, &PatternState::default()).is_none() {
            compose_calls += 1;
            cache.insert(id, dressed_a.clone(), PatternState::default(), transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 1, "初回 Show は 1 回だけ合成する");

        // 2 回目の Show（同一 surface・異なる BindSet B）: ミス → 再合成（カウンタ +1）。
        if cache.get(id, &dressed_b, &PatternState::default()).is_none() {
            compose_calls += 1;
            cache.insert(id, dressed_b.clone(), PatternState::default(), transparent_surface(4, 4));
        }
        assert_eq!(
            compose_calls, 2,
            "着せ替え集合が変われば同一 surface でも再合成が走らねばならない（R6.1）"
        );

        // 置換後は新入力のみヒット・旧入力はミス（容量 1 メモ・古い絵を返さない）。
        assert!(cache.get(id, &dressed_b, &PatternState::default()).is_some(), "再合成後の新 binds はヒットする");
        assert!(
            cache.get(id, &dressed_a, &PatternState::default()).is_none(),
            "容量 1 メモ: 置換後の旧 binds はミスする"
        );
    }

    /// R6.2: 同一の着せ替え集合で表示を再発行すると、既存キャッシュから **再合成なしで復帰**する
    /// （既存キャッシュ挙動の維持）。
    ///
    /// 同一 (surface id, BindSet) の Show を 2 回発行しても 2 回目はヒットし、提示段は再合成せず
    /// キャッシュ済みサーフェスをそのまま返す（合成カウンタ据え置き）ことを固定する。
    #[test]
    fn dynamic_show_reissue_same_binds_returns_cached() {
        let mut cache = ComposeCache::new();
        let mut compose_calls = 0u32;
        let id = 1000;
        let dressed = BindSet::from_ids([1100, 1207]);

        // 1 回目の Show: ミス → 再合成（カウンタ +1）→ 挿入。
        if cache.get(id, &dressed, &PatternState::default()).is_none() {
            compose_calls += 1;
            cache.insert(id, dressed.clone(), PatternState::default(), transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 1, "初回 Show は 1 回だけ合成する");

        // 2 回目の Show（同一 surface・同一 BindSet）: ヒット → 再合成しない（カウンタ据え置き）。
        let hit = cache.get(id, &dressed, &PatternState::default());
        assert!(hit.is_some(), "同一着せ替え集合の再発行はヒットする（R6.2）");
        if hit.is_none() {
            compose_calls += 1;
            cache.insert(id, dressed.clone(), PatternState::default(), transparent_surface(4, 4));
        }
        assert_eq!(
            compose_calls, 1,
            "同一 binds の再発行は再合成なしで復帰しなければならない（R6.2）"
        );
        assert!(
            cache.get(id, &dressed, &PatternState::default()).is_some(),
            "ヒット後もキャッシュ済みサーフェスは保持される"
        );
    }

    /// 容量 1 メモ: 異なる surface id への挿入はスロットを置換し、旧 id はミスする。
    #[test]
    fn different_surface_id_replaces_slot() {
        let mut cache = ComposeCache::new();
        let binds = BindSet::default();

        cache.insert(0, binds.clone(), PatternState::default(), transparent_surface(4, 4));
        cache.insert(1000, binds.clone(), PatternState::default(), transparent_surface(4, 4));
        assert!(
            cache.get(1000, &binds, &PatternState::default()).is_some(),
            "直近挿入の id はヒットする"
        );
        assert!(
            cache.get(0, &binds, &PatternState::default()).is_none(),
            "容量 1 メモ: 旧 id はミスする（多エントリ保持はしない）"
        );
    }

    /// R4.3: `invalidate_all` 後は同一合成入力がミスし、再合成される。
    #[test]
    fn invalidate_all_forces_recompute() {
        let mut cache = ComposeCache::new();
        let mut compose_calls = 0u32;
        let id = 7;
        let binds = BindSet::default();

        if cache.get(id, &binds, &PatternState::default()).is_none() {
            compose_calls += 1;
            cache.insert(id, binds.clone(), PatternState::default(), transparent_surface(4, 4));
        }
        assert_eq!(compose_calls, 1);

        cache.invalidate_all();
        assert!(
            cache.get(id, &binds, &PatternState::default()).is_none(),
            "id must miss after invalidate_all"
        );

        // 無効化後の再アクセスはミス → 再合成（カウンタ +1）。
        if cache.get(id, &binds, &PatternState::default()).is_none() {
            compose_calls += 1;
            cache.insert(id, binds.clone(), PatternState::default(), transparent_surface(4, 4));
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
        let binds = BindSet::default();

        let mut cache = ComposeCache::new();
        let entry = cache.insert(1000, binds.clone(), PatternState::default(), composed);

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
        let got = cache.get(1000, &binds, &PatternState::default()).expect("entry retained");
        assert_eq!(got.composed.width(), w);
        assert_eq!(got.composed.height(), h);
        assert!(got.mask.is_hit(ox, oy));
        assert!(!got.mask.is_hit(tx, ty));
    }
}
