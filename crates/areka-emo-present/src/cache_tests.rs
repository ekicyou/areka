use super::*;
use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, ComposeMethod, Composer, EmoWorld, PatternFrame};
use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};
use std::path::Path;

// ── ComposedSurface 生成補助 ──────────────────────────────────────────────
// `ComposedSurface::bytes_mut` は emo-compose の pub(crate) ゆえ本クレートから画素を直接
// 焼けない。よって「全透明」は公開 `new(w,h)` で、「不透明画素を含む結果」は上流公開 API
// （atlas bake → EmoWorld → Composer::compose_into）で本物を合成して得る。後者はマスクが
// composed の bytes 由来であることを実合成経路で保証する（模造バッファでの偽陽性を避ける）。

/// カウント用途で十分な任意サイズの全透明合成結果（内容は不問・件数計上のみに使う）。
fn transparent_surface(w: u32, h: u32) -> ComposedSurface {
    ComposedSurface::new(w, h)
}

/// 原寸の合成面の bytes からマスクを生成する（設計 D4 でマスク生成が挿入の外へ出たため、
/// **呼び手側**の責務になった手順を檻でもそのまま踏む）。
fn mask_of(composed: &ComposedSurface) -> Arc<AlphaMask> {
    Arc::new(AlphaMask::from_pbgra32(
        composed.bytes(),
        composed.width(),
        composed.height(),
        composed.stride(),
    ))
}

/// 「原寸の合成面＋その bytes 由来マスク」の原子対を組んで挿入する（既存檻の署名追随）。
///
/// 設計 D4 で `insert` は生成済みマスクを引数で受ける形になった。檻側でマスクを別出所から
/// 作ると原子対の主張が空虚になるため、生成は必ず `composed` の bytes からのみ行う。
///
/// 表示スケール k は引数に無い——エントリが持つのは native 原寸の面だけで、k はキー要素でも
/// 保持内容でもない（要件 5.1／5.2）。
fn insert_with_mask(
    cache: &mut ComposeCache,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
    composed: ComposedSurface,
) -> &CacheEntry {
    let mask = mask_of(&composed);
    let display = GraphicsCommandList::empty(); // 檻は GPU を持たない（記録は保持されるだけ）
    cache.insert(surface_id, binds, pattern, composed, mask, display)
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

/// 不透明画素と透明画素を**必ず両方**含む本物の合成を、**与えられた出力先へ書き戻す**。
///
/// 3×1 の単一画像（不透明赤 / 透明 / 不透明赤）を 1 element として合成する。両端が不透明の
/// ため α=0 除外トリムでも中央の透明画素が矩形内に残る（＝合成結果に不透明・透明が共存）。
///
/// 出力先を受け取る形なのは、回収したバッファへの書き戻しを**リサンプルに依らず**測るためで
/// ある（要件 3.1）——本仕様で表示スケールは描画経路の変換行列へ移り、合成層の出力は常に
/// native 原寸になった。`Composer::compose_into` は出力先の確保を再利用するので、回収した
/// 容量がそのまま引き継がれているかがここで観測できる。
fn compose_small_into(out: &mut ComposedSurface) {
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
    assert!(
        baked.errors.is_empty(),
        "atlas bake セットアップは失敗しない"
    );

    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));

    let mut composer = Composer::new();
    composer
        .compose_into(
            out,
            &world,
            &baked.table,
            1000,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("静的 element 単体の合成は Ok");
}

/// 上の合成をまっさらな器へ書いて値で返す（出力先を持たない檻用の便宜）。
fn composed_with_opaque_and_transparent() -> ComposedSurface {
    let mut out = ComposedSurface::new(0, 0);
    compose_small_into(&mut out);
    out
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

// ── 容量政策の檻用の最小補助 ─────────────────────────────────────────────
//
// 既定キー（`BindSet::default()`／`PatternState::default()`）で **surface id だけ**を変える。
// 容量・置換方式は「キーが違う」ことだけで駆動されるため、弁別に要る差はこれで足りる。

/// 既定キーの surface id `id` で挿入する。
fn insert_id(cache: &mut ComposeCache, id: u32) {
    insert_with_mask(
        cache,
        id,
        BindSet::default(),
        PatternState::default(),
        transparent_surface(4, 4),
    );
}

/// 既定キーの surface id `id` が保持されているか。
///
/// [`ComposeCache::get`] は**最近使用順を動かさない**（[`ComposeCache::touch`] だけが動かす）ため、
/// この観測は LRU の状態を乱さない——檻が自分の観測で置換順を書き換えてしまう罠を構造で避ける。
fn holds(cache: &ComposeCache, id: u32) -> bool {
    cache
        .get(id, &BindSet::default(), &PatternState::default())
        .is_some()
}

/// 既定キーの surface id `id` を引き当てて**最近使用へ引き上げる**（本番のヒット経路と同じ口）。
fn touch_id(cache: &mut ComposeCache, id: u32) -> bool {
    cache.touch(id, &BindSet::default(), &PatternState::default())
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
        insert_with_mask(
            &mut cache,
            id,
            binds.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
    }
    assert_eq!(compose_calls, 1, "first access must compose exactly once");

    // 2 回目: 同一合成入力＝ヒット → 合成しない（カウンタ据え置き）。
    if cache.get(id, &binds, &PatternState::default()).is_none() {
        compose_calls += 1;
        insert_with_mask(
            &mut cache,
            id,
            binds.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
    }
    assert_eq!(
        compose_calls, 1,
        "second access is a hit; must not recompute"
    );
    assert!(
        cache.get(id, &binds, &PatternState::default()).is_some(),
        "entry must be retained after hit"
    );
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

    insert_with_mask(
        &mut cache,
        id,
        eyes_open.clone(),
        PatternState::default(),
        transparent_surface(4, 4),
    );
    assert!(
        cache
            .get(id, &eyes_open, &PatternState::default())
            .is_some(),
        "同一入力はヒットする"
    );
    assert!(
        cache
            .get(id, &eyes_closed, &PatternState::default())
            .is_none(),
        "同一 surface でも bind 集合が異なればミスしなければならない（着せ替えバグの回帰檻）"
    );

    // 異なる binds を挿入すると**別エントリ**として共存する（容量 3・要件 7.1）。まばたきの
    // 開閉のように行き来する入力で再合成を省けることが、容量 3 の裁定の狙いそのものである。
    insert_with_mask(
        &mut cache,
        id,
        eyes_closed.clone(),
        PatternState::default(),
        transparent_surface(4, 4),
    );
    assert!(
        cache
            .get(id, &eyes_closed, &PatternState::default())
            .is_some(),
        "挿入した新入力がヒットする"
    );
    assert!(
        cache
            .get(id, &eyes_open, &PatternState::default())
            .is_some(),
        "容量 3: 別キーの挿入は既存キーを追い出さない（まばたきの開閉が共存する）"
    );

    // **無限堆積はしない**: 異なるキーを入れ続ければ最も古い引き当てから落ちる。
    for extra in [1401_u32, 1402, 1403] {
        insert_with_mask(
            &mut cache,
            id,
            BindSet::from_ids([1101, 1302, extra]),
            PatternState::default(),
            transparent_surface(4, 4),
        );
    }
    assert!(
        cache
            .get(id, &eyes_open, &PatternState::default())
            .is_none(),
        "容量 3 の上限: 異なるキーを入れ続ければ最も古い引き当てから落ちる（無限堆積しない）"
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

    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern_a.clone(),
        transparent_surface(4, 4),
    );

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

    // 2 つの pattern は**別エントリ**として共存し、それぞれ**自分の絵**を返す（古い絵を返さない）。
    // 外形を変えてあるので、キーだけ通ってエントリが取り違わる形はここで赤になる。
    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern_b.clone(),
        transparent_surface(5, 5),
    );
    assert_eq!(
        cache
            .get(id, &binds, &pattern_b)
            .map(|e| (e.composed.width(), e.composed.height())),
        Some((5, 5)),
        "pattern_b の引き当ては pattern_b の絵を返す"
    );
    assert_eq!(
        cache
            .get(id, &binds, &pattern_a)
            .map(|e| (e.composed.width(), e.composed.height())),
        Some((4, 4)),
        "容量 3: pattern_a のエントリは残り、しかも**自分の絵**を返す（取り違えない）"
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
    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        PatternState::default(),
        transparent_surface(4, 4),
    );
    assert!(
        cache.get(id, &binds, &PatternState::default()).is_some(),
        "空 pattern で挿入 → 空 pattern の get はヒット（拡張前と観測等価・R5.4）"
    );
    assert!(
        cache.get(id, &binds, &pat).is_none(),
        "空 pattern で挿入 → 非空 pattern の get はミス（空と非空は別キー）"
    );

    // 逆向き: 非空 pattern **だけ**を入れた表では、同値の非空 pattern はヒットし空 pattern は
    // ミスする。**新しい表で測る**——同じ表へ足すと容量 3 では両方が共存してしまい、「空と非空が
    // 別キーである」という主張が観測できなくなる（測っているのは弁別であって置換ではない）。
    let mut cache = ComposeCache::new();
    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pat.clone(),
        transparent_surface(4, 4),
    );
    assert!(
        cache.get(id, &binds, &pat).is_some(),
        "非空 pattern で挿入 → 同値 pattern の get はヒット"
    );
    assert!(
        cache.get(id, &binds, &PatternState::default()).is_none(),
        "非空 pattern で挿入 → 空 pattern の get はミス（空と非空は別キー）"
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

    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pat.clone(),
        transparent_surface(4, 4),
    );
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
    if cache
        .get(id, &dressed_a, &PatternState::default())
        .is_none()
    {
        compose_calls += 1;
        insert_with_mask(
            &mut cache,
            id,
            dressed_a.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
    }
    assert_eq!(compose_calls, 1, "初回 Show は 1 回だけ合成する");

    // 2 回目の Show（同一 surface・異なる BindSet B）: ミス → 再合成（カウンタ +1）。
    if cache
        .get(id, &dressed_b, &PatternState::default())
        .is_none()
    {
        compose_calls += 1;
        insert_with_mask(
            &mut cache,
            id,
            dressed_b.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
    }
    assert_eq!(
        compose_calls, 2,
        "着せ替え集合が変われば同一 surface でも再合成が走らねばならない（R6.1）"
    );

    // 再合成が走ったこと（`compose_calls == 2`）が本檻の主張である。容量 3 では両方の着せ替えが
    // 別エントリとして共存し、着せ替えを戻したときは再合成が省ける。
    assert!(
        cache
            .get(id, &dressed_b, &PatternState::default())
            .is_some(),
        "再合成後の新 binds はヒットする"
    );
    assert!(
        cache
            .get(id, &dressed_a, &PatternState::default())
            .is_some(),
        "容量 3: 旧 binds も保持される（着せ替えを戻す再発行は再合成を要さない）"
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
        insert_with_mask(
            &mut cache,
            id,
            dressed.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
    }
    assert_eq!(compose_calls, 1, "初回 Show は 1 回だけ合成する");

    // 2 回目の Show（同一 surface・同一 BindSet）: ヒット → 再合成しない（カウンタ据え置き）。
    let hit = cache.get(id, &dressed, &PatternState::default());
    assert!(
        hit.is_some(),
        "同一着せ替え集合の再発行はヒットする（R6.2）"
    );
    if hit.is_none() {
        compose_calls += 1;
        insert_with_mask(
            &mut cache,
            id,
            dressed.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
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

/// 容量 3 メモ: 異なる surface id は別エントリとして共存し、上限を超えた挿入で最も古い引き当てが落ちる。
#[test]
fn different_surface_ids_coexist_until_the_capacity_is_exceeded() {
    let mut cache = ComposeCache::new();
    let binds = BindSet::default();
    let insert = |cache: &mut ComposeCache, id: u32| {
        insert_with_mask(
            cache,
            id,
            binds.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
    };
    let held =
        |cache: &ComposeCache, id: u32| cache.get(id, &binds, &PatternState::default()).is_some();

    insert(&mut cache, 0);
    insert(&mut cache, 1000);
    assert!(held(&cache, 1000), "直近挿入の id はヒットする");
    assert!(
        held(&cache, 0),
        "容量 3: 旧 id も保持される（別エントリとして共存する）"
    );

    // 上限を超える 2 件を足すと、最も古い引き当て（id=0）から落ちる。
    insert(&mut cache, 2000);
    insert(&mut cache, 3000);
    assert!(!held(&cache, 0), "上限超過で最も古い引き当ての id が落ちる");
    assert!(held(&cache, 1000) && held(&cache, 2000) && held(&cache, 3000));
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
        insert_with_mask(
            &mut cache,
            id,
            binds.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
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
        insert_with_mask(
            &mut cache,
            id,
            binds.clone(),
            PatternState::default(),
            transparent_surface(4, 4),
        );
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
    let entry = insert_with_mask(
        &mut cache,
        1000,
        binds.clone(),
        PatternState::default(),
        composed,
    );

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
    let got = cache
        .get(1000, &binds, &PatternState::default())
        .expect("entry retained");
    assert_eq!(got.composed.width(), w);
    assert_eq!(got.composed.height(), h);
    assert!(got.mask.is_hit(ox, oy));
    assert!(!got.mask.is_hit(tx, ty));
}

// ── キー要素は合成入力の 3 つだけ（要件 5.2・設計 cache::ComposeCache） ──────

/// キー比較の非退行: 合成入力（surface id・binds・pattern）のいずれかが異なれば必ずミスする。
///
/// 表示スケールがキーから外れた（要件 5.2）ときに、残る 3 要素の比較まで一緒に落ちていないか
/// を見る檻である。1 要素ずつ違えた 3 通りと、陰性対照の完全一致を並べる。
#[test]
fn other_key_elements_still_miss_on_the_same_compose_inputs() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1101, 1302]);
    let pattern = pattern_of(2000, 1001);

    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern.clone(),
        transparent_surface(4, 4),
    );

    assert!(
        cache.get(id, &binds, &pattern).is_some(),
        "陰性対照: 全要素一致はヒットする"
    );
    assert!(
        cache.get(1001, &binds, &pattern).is_none(),
        "surface id が異なればミスする"
    );
    assert!(
        cache
            .get(id, &BindSet::from_ids([1101]), &pattern)
            .is_none(),
        "bind 集合が異なればミスする"
    );
    assert!(
        cache.get(id, &binds, &pattern_of(2000, 1002)).is_none(),
        "pattern が異なればミスする"
    );
    // 2 要素が同時に異なる場合も当然ミスする。
    assert!(
        cache.get(1001, &binds, &pattern_of(2000, 1002)).is_none(),
        "複数のキー要素が異なればミスする"
    );
}

/// R4.3 変わらず: `invalidate_all` は**保持中の全エントリ**を破棄する（挙動不変）。
///
/// 上限いっぱいの 3 キーを入れてから無効化し、どれ 1 つ残らないことを見る。1 件だけの表で
/// 測ると「先頭 1 件しか消さない」実装を通してしまうため、満杯の表で測る。
#[test]
fn invalidate_all_clears_every_entry() {
    let mut cache = ComposeCache::new();
    let binds = BindSet::from_ids([1100]);
    let pattern = pattern_of(2000, 1001);

    insert_with_mask(
        &mut cache,
        7,
        binds.clone(),
        pattern.clone(),
        transparent_surface(5, 5),
    );
    insert_id(&mut cache, 8001);
    insert_id(&mut cache, 8002);
    assert!(
        cache.get(7, &binds, &pattern).is_some() && holds(&cache, 8001) && holds(&cache, 8002),
        "前提: 3 キーとも保持されている"
    );

    cache.invalidate_all();
    assert!(
        cache.get(7, &binds, &pattern).is_none(),
        "invalidate_all は保持中のエントリを破棄する（R4.3 挙動不変）"
    );
    assert!(
        !holds(&cache, 8001) && !holds(&cache, 8002),
        "invalidate_all は 1 件だけでなく**全て**を破棄する"
    );
}

// ── 容量回収（`take_recycled`・要件 3.1・設計 D2⑵） ─────────────────────────

/// 要件 3.1・設計 D2⑵: **容量回収を挟んでも承認済み意味論は不変**である。
///
/// 承認済み意味論（`completed/areka-P0-emo-present` R4.1・2026-08-15 の裁定で容量のみ 1 → 3 へ
/// 改訂）は ⑴上限 3 件のメモ化表 ⑵キー完全一致のみヒット ⑶表示バッファとマスクの原子対、の
/// 3 点である。`take_recycled` はバッファの容量を回収するだけの口であって政策を変えないため、
/// 回収 → 挿入を経ても 3 点ともそのまま成立しなければならない。上限が緩む・キー比較が落ちる・
/// 対が崩れる、のいずれもここで赤になる。
#[test]
fn insert_after_take_recycled_preserves_approved_semantics() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1101, 1302]);
    let pattern = pattern_of(2000, 1001);

    // 回収は満杯のときだけ成立する。追跡対象（このキー）が最も古い引き当てになるよう先に入れる。
    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern.clone(),
        transparent_surface(4, 4),
    );
    insert_id(&mut cache, 8001);
    insert_id(&mut cache, 8002);

    // 回収: エントリは**対のまま**出てきて、その席だけが空く。
    let recycled = cache.take_recycled().expect("満杯なら 1 本追い出される");
    assert_eq!(
        (recycled.composed.width(), recycled.composed.height()),
        (4, 4),
        "回収したエントリは表示バッファを保持している"
    );
    assert_eq!(
        (recycled.mask.width(), recycled.mask.height()),
        (4, 4),
        "回収したエントリはマスクも対で保持している（原子対のまま出る）"
    );
    assert!(
        cache.get(id, &binds, &pattern).is_none(),
        "回収されたキーは以後ミスする"
    );
    assert!(
        holds(&cache, 8001) && holds(&cache, 8002),
        "回収は 1 本だけ——残りのエントリは剥がれない"
    );

    // 回収後の挿入。原子対の主張を空虚にしないため、不透明・透明を両方含む本物の合成結果を使う。
    let composed = composed_with_opaque_and_transparent();
    let ((ox, oy), (tx, ty)) = find_opaque_and_transparent(&composed);
    let (w, h) = (composed.width(), composed.height());
    insert_with_mask(&mut cache, id, binds.clone(), pattern.clone(), composed);

    // ⑵ キー完全一致のみヒットし、⑶ 対で引ける。
    let entry = cache
        .get(id, &binds, &pattern)
        .expect("回収後の挿入でも完全一致キーはヒットする");
    assert_eq!((entry.composed.width(), entry.composed.height()), (w, h));
    assert!(
        entry.mask.is_hit(ox, oy),
        "不透明画素 ({ox},{oy}) は同一エントリのマスクでヒットする（原子対）"
    );
    assert!(
        !entry.mask.is_hit(tx, ty),
        "透明画素 ({tx},{ty}) は同一エントリのマスクでヒットしない（原子対）"
    );

    // ⑵ キー 3 成分のいずれか 1 つでも異なればミスする（回収を挟んでも比較は落ちない）。
    assert!(
        cache.get(1001, &binds, &pattern).is_none(),
        "surface id が異なればミスする"
    );
    assert!(
        cache
            .get(id, &BindSet::from_ids([1101]), &pattern)
            .is_none(),
        "bind 集合が異なればミスする"
    );
    assert!(
        cache.get(id, &binds, &pattern_of(2000, 1002)).is_none(),
        "pattern が異なればミスする"
    );

    // ⑴ 上限 3 件: 保持件数は上限を超えず、超える挿入は最も古い引き当てから落とす
    //    （回収を挟んでも容量政策は動かない・要件 7.1）。
    assert_eq!(cache.len(), 3, "回収 → 挿入で保持件数が上限を超えている");
    insert_id(&mut cache, 8003);
    assert_eq!(cache.len(), 3, "上限を超える挿入の後も保持件数は上限のまま");
    assert!(
        !holds(&cache, 8001),
        "上限超過で落ちるのは最も古い引き当て（回収を挟んでも LRU のまま・要件 7.1）"
    );
    assert!(
        cache.get(id, &binds, &pattern).is_some(),
        "直近に入れたキーは残る"
    );
}

/// 要件 3.1・設計 D2⑵ の**本題**: 回収したバッファの容量が次の表示バッファへ引き継がれる。
///
/// 「容量回収」を名乗る以上、返るのは**挿入したその確保**でなければならない——複製や新規確保を
/// 返す実装は、キーの意味論だけを見る檻ではすべて緑のまま素通りする（4.1／4.2 で実測した構図）。
/// ゆえに ⑴挿入したバッファの先頭ポインタが回収したエントリにそのまま現れること ⑵回収バッファを
/// **より小さい出力**の書き戻し先に使っても再確保が起きないこと（＝同寸反復だけを見る檻が
/// 見逃す新規確保形を捕まえる）⑶その確保のまま再挿入できること、の 3 点で固定する。
///
/// ⑵ の「より小さい出力」は**小さな原寸合成**（3×1）で作る。本仕様で表示スケールは描画経路の
/// 変換行列へ移り、この層に縮小出力を作る口は無くなったため、書き戻しは合成そのもの
/// （`Composer::compose_into`）で測る——本番の書き戻し経路と同じ口である。
#[test]
fn take_recycled_carries_over_buffer_capacity() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1100]);
    let pattern = PatternState::default();

    // 大きい合成面（64×64＝16,384 バイト）。以後この 1 本の確保だけを追う。
    let big = transparent_surface(64, 64);
    let big_ptr = big.bytes().as_ptr();
    let big_len = big.bytes().len();
    assert_eq!(big_len, 64 * 4 * 64, "前提: 追跡対象は 16,384 バイトの確保");
    insert_with_mask(&mut cache, id, binds.clone(), pattern.clone(), big);
    // 回収は満杯のときだけ成立する（容量 3・要件 7.1）。追跡対象が最も古い引き当てのまま
    // 残るよう、別キーで表を埋める。
    insert_id(&mut cache, 9001);
    insert_id(&mut cache, 9002);

    // ⑴ 回収されるのは**挿入したその確保**である（複製・新規確保では容量を回収したことにならない）。
    let recycled = cache.take_recycled().expect("満杯なら 1 本追い出される");
    assert_eq!(
        recycled.composed.bytes().as_ptr(),
        big_ptr,
        "回収したのは挿入したそのバッファでなければならない（複製・新規確保は容量回収ではない）"
    );
    assert_eq!(recycled.composed.bytes().len(), big_len);

    // ⑵ 回収バッファを次の合成面として使い回す。**より小さい出力**（3×1）を書き戻しても
    // 再確保が起きない＝回収した容量（16,384）がそのまま引き継がれている。
    // 同寸の反復だけを見る檻は、毎回ちょうど同じ大きさを確保し直す実装を捕まえられない。
    let mut reused = recycled.composed;
    compose_small_into(&mut reused);
    assert!(
        reused.bytes().len() < big_len,
        "前提: 書き戻し先は回収時より小さい外形になる（{}×{}）",
        reused.width(),
        reused.height()
    );
    assert_eq!(
        reused.bytes().as_ptr(),
        big_ptr,
        "回収した容量が引き継がれていれば、より小さい出力への書き戻しで再確保は起きない"
    );

    // ⑶ 引き継いだ確保のまま再挿入でき、エントリはその確保を保持する。
    let mask = mask_of(&reused);
    let entry = cache.insert(
        id,
        binds.clone(),
        pattern.clone(),
        reused,
        mask,
        GraphicsCommandList::empty(),
    );
    assert_eq!(
        entry.composed.bytes().as_ptr(),
        big_ptr,
        "回収 → 使い回し → 再挿入で確保は 1 本のまま（新規確保が挟まっていない）"
    );
}

/// 空きのある表からの回収は `None` を返し、キャッシュは以後も従来どおり使える。
///
/// 回収は「満杯なら最も古い引き当てを 1 本外す」だけの操作であり、空きがあるときは何も起こらない
/// （べき等）。未挿入・回収直後・無効化後のいずれから呼んでも `None` で、その後の挿入・引き当ては
/// 正常に動く。
#[test]
fn take_recycled_without_a_full_table_returns_none_and_leaves_cache_usable() {
    let mut cache = ComposeCache::new();
    let id = 7;
    let binds = BindSet::from_ids([1100]);
    let pattern = pattern_of(2000, 1001);

    assert!(
        cache.take_recycled().is_none(),
        "未挿入の表からは何も回収できない"
    );

    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern.clone(),
        transparent_surface(4, 4),
    );
    assert!(
        cache.get(id, &binds, &pattern).is_some(),
        "陰性対照: 挿入直後は完全一致キーがヒットする"
    );
    assert!(
        cache.take_recycled().is_none(),
        "1 件しか入っていない表は満杯でない＝生きているエントリを剥がさない"
    );
    assert!(
        cache.get(id, &binds, &pattern).is_some(),
        "回収不成立でエントリが消えていない"
    );

    // 満杯にすれば 1 本だけ回収でき、そこでまた空きができるので次は不成立に戻る。
    insert_id(&mut cache, 7001);
    insert_id(&mut cache, 7002);
    assert!(cache.take_recycled().is_some(), "満杯なら回収できる");
    assert!(
        cache.take_recycled().is_none(),
        "回収で 1 本空いた直後は不成立（連続で剥がし続けない）"
    );
    assert!(
        cache.get(id, &binds, &pattern).is_none(),
        "回収されたのは最も古い引き当て（このキー）である"
    );

    // 回収の後も表は従来どおり使える（回収がキャッシュを壊さない）。
    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern.clone(),
        transparent_surface(4, 4),
    );
    assert!(
        cache.get(id, &binds, &pattern).is_some(),
        "回収後に挿入し直せば再びヒットする"
    );

    // 無効化と回収が二重に効いても壊れない。
    cache.invalidate_all();
    assert!(
        cache.take_recycled().is_none(),
        "invalidate_all 後の表からも何も回収できない"
    );
    assert!(
        cache.get(id, &binds, &pattern).is_none(),
        "無効化後はミスしたままである"
    );
}

// ── 容量 3・LRU 置換（要件 7.1・2026-08-15 開発者裁定） ──────────────────────

/// 要件 7.1 観測完了（**承認済み容量そのもの**）: 異なる 3 キーが**同時に**保持される。
///
/// 2026-08-15 の開発者裁定で容量は 1 → 3 へ改訂された（実測の根拠は
/// `remeasure-2026-08-15.md` §4）。ここは改訂後の容量の下限側——3 本目を入れても
/// 1 本目・2 本目が生き残ることを固定する。容量 1・2 のいずれへ戻しても赤になる。
#[test]
fn three_distinct_keys_are_retained_at_the_same_time() {
    let mut cache = ComposeCache::new();
    for id in [1000, 1001, 1002] {
        insert_id(&mut cache, id);
    }
    for id in [1000, 1001, 1002] {
        assert!(
            holds(&cache, id),
            "容量 3: 異なる 3 キーは同時に保持される（surface {id} が落ちている）"
        );
    }
}

/// 要件 7.1 観測完了（**容量の上限そのもの**）: 4 本目の挿入は**ちょうど 1 本**を追い出す。
///
/// 「3 本入る」だけでは容量 3 と容量 4 以上を区別できない。**追い出しが起きること**と
/// **落ちるのが 1 本だけであること**を対で固定して初めて容量が 3 に決まる。
#[test]
fn a_fourth_distinct_key_evicts_exactly_one_entry() {
    let mut cache = ComposeCache::new();
    for id in [1000, 1001, 1002, 1003] {
        insert_id(&mut cache, id);
    }

    let retained: Vec<u32> = [1000, 1001, 1002, 1003]
        .into_iter()
        .filter(|id| holds(&cache, *id))
        .collect();
    assert_eq!(
        retained,
        vec![1001, 1002, 1003],
        "容量 3: 4 本目の挿入は最も古い引き当ての 1 本だけを追い出す（保持されているのは {retained:?}）"
    );
    // 表そのものの件数でも押さえる（引き当てが壊れていても「3 件しか持たない」は言える）。
    assert_eq!(cache.len(), 3, "保持件数が容量を超えている");
}

/// 要件 3.1／7.1 観測完了: [`ComposeCache::take_recycled`] は**満杯のときだけ** 1 本を返す。
///
/// 容量 1 の頃は「唯一のスロットを取り出して空にする」口だった。容量 3 では
/// **追い出しが起きる回にだけ**回収が成立し、空きがある間は `None` を返して残りに触れない
/// ——ここが緩むと、暖機中に生きているエントリを剥がして命中率が落ちる（裁定の前提が壊れる）。
#[test]
fn take_recycled_yields_one_entry_only_when_the_cache_is_full() {
    let mut cache = ComposeCache::new();

    for (n, id) in [1000_u32, 1001, 1002].into_iter().enumerate() {
        insert_id(&mut cache, id);
        if n + 1 < 3 {
            assert!(
                cache.take_recycled().is_none(),
                "空きがある間は回収しない（{}/3 本目で追い出しが起きた）",
                n + 1
            );
        }
    }

    // 満杯（1000・1001・1002）: 回収は最も古い引き当ての 1 本だけで、残り 2 本は据え置き。
    let recycled = cache.take_recycled().expect("満杯なら 1 本追い出される");
    assert_eq!(
        (recycled.composed.width(), recycled.composed.height()),
        (4, 4),
        "回収したエントリは表示バッファを対のまま保持している"
    );
    assert!(
        !holds(&cache, 1000),
        "追い出されたのは最も古い引き当ての 1 本"
    );
    assert!(holds(&cache, 1001), "残りのエントリは回収で剥がれない");
    assert!(holds(&cache, 1002), "残りのエントリは回収で剥がれない");

    // 追い出しで 1 本空いたので、次の回収はまた `None`（連続で剥がし続けない）。
    assert!(
        cache.take_recycled().is_none(),
        "1 本空いた直後の回収は不成立（満杯でない限り剥がさない）"
    );
}

/// 要件 7.1 観測完了（**置換方式そのもの**）: 追い出されるのは**最近最も使われていない**もので
/// あって、挿入順の最古（FIFO）ではない。
///
/// # LRU と FIFO が食い違う形を作る（さもなくば檻は両者を区別できない）
///
/// 挿入だけを並べた台本では LRU と FIFO の追い出し先が一致するため、どちらの実装でも緑になる。
/// ゆえに**挿入順と最近使用順が食い違う状態**を作る: 1000・1001・1002 を入れてから 1000 を
/// 引き当てると、挿入順の最古は 1000・最近使用の最古は 1001 になる。ここで 1003 を入れたとき
///
/// - **LRU（正）**: 1001 が落ち、1000 は残る
/// - **FIFO（誤）**: 1000 が落ち、1001 が残る
///
/// と結果が反転するので、1 本の檻で置換方式が決まる。
///
/// # なぜ置換方式が load-bearing なのか
///
/// 容量 3 という裁定は、実走行の適用列を **LRU で再生**した命中率（キャラ面 56.2%）を根拠に
/// 下りている（`remeasure-2026-08-15.md` §4）。FIFO へ退化すると、まばたきのように「直前に
/// 使った面へ戻る」列で命中しなくなり、数字の出所が実装と対応しなくなる。**表示バイトも確保
/// 計数も 1 つも変わらない**ため、この檻だけが退化を捕まえる。
#[test]
fn eviction_picks_the_least_recently_used_not_the_oldest_inserted() {
    let mut cache = ComposeCache::new();
    for id in [1000, 1001, 1002] {
        insert_id(&mut cache, id);
    }

    // 挿入順の最古（1000）を引き当てて最近使用へ引き上げる＝2 つの順序が食い違う状態を作る。
    assert!(touch_id(&mut cache, 1000), "前提: 1000 は保持されている");

    insert_id(&mut cache, 1003);

    assert!(
        holds(&cache, 1000),
        "LRU なら直前に引き当てた 1000 は残る（挿入順で追い出す FIFO 実装ではここが落ちる）"
    );
    assert!(
        !holds(&cache, 1001),
        "LRU なら最も古い引き当ての 1001 が落ちる（FIFO 実装ではこれが残る）"
    );
    assert!(holds(&cache, 1002), "1002 は落ちない");
    assert!(holds(&cache, 1003), "いま入れたキーは保持される");
}

/// 要件 7.1 観測完了（**回収も LRU に従う**）: [`ComposeCache::take_recycled`] が返すのも
/// 最近最も使われていない 1 本である。
///
/// 上の檻と同じ「挿入順と最近使用順が食い違う」状態を作り、回収された実体を**外形で**弁別する
/// （どのエントリが出てきたかを、残ったキーだけでなく回収物そのものでも言う）。
#[test]
fn take_recycled_picks_the_least_recently_used_entry() {
    let mut cache = ComposeCache::new();
    // 外形を変えて「どのエントリが回収されたか」を回収物から直接読めるようにする。
    for (id, side) in [(1000_u32, 4_u32), (1001, 5), (1002, 6)] {
        insert_with_mask(
            &mut cache,
            id,
            BindSet::default(),
            PatternState::default(),
            transparent_surface(side, side),
        );
    }
    assert!(touch_id(&mut cache, 1000), "前提: 1000 は保持されている");

    let recycled = cache.take_recycled().expect("満杯なら 1 本追い出される");
    assert_eq!(
        (recycled.composed.width(), recycled.composed.height()),
        (5, 5),
        "回収されたのは最も古い引き当て（1001・5×5）でなければならない（FIFO なら 1000 の 4×4）"
    );
    assert!(holds(&cache, 1000), "直前に引き当てたエントリは剥がれない");
    assert!(!holds(&cache, 1001), "剥がれたのは最も古い引き当て");
    assert!(holds(&cache, 1002), "残りのエントリは剥がれない");
}

/// [`ComposeCache::get`] は最近使用順を**動かさない**（LRU を打ち直すのは [`ComposeCache::touch`] だけ）。
///
/// # なぜこの区別が要るのか
///
/// `show.rs` は 1 適用のあいだに `get` を複数回呼ぶ（供給面の遅延生成・アップロード直前・原寸の
/// 写し取り）。読み取りが順序を動かす形だと「1 適用で最近使用を何度も打ち直す」ことになり、
/// 檻もまた自分の観測で LRU の状態を書き換えてしまう（本ファイルの [`holds`] がそれである）。
///
/// 逆向きの退化——`touch` が順序を動かさない——は上の 2 本の檻が捕まえる。本檻はその対で、
/// 「両方が同じことをする」形（`get` も動かす／どちらも動かさない）を残さない。
#[test]
fn get_does_not_disturb_the_recency_order() {
    let mut cache = ComposeCache::new();
    for id in [1000, 1001, 1002] {
        insert_id(&mut cache, id);
    }

    // 読み取りだけ（`get`）を最古のキーへ何度当てても、順序は動かない。
    for _ in 0..4 {
        assert!(holds(&cache, 1000), "前提: 1000 は保持されている");
    }

    insert_id(&mut cache, 1003);
    assert!(
        !holds(&cache, 1000),
        "`get` が最近使用順を動かしている（読み取りが置換順を書き換えている）"
    );
    assert!(holds(&cache, 1001), "順序が動いていなければ 1001 は残る");
}

/// 引き当ての判定規則は [`ComposeCache::touch`] と [`ComposeCache::get`] で 1 ビットも違わない。
///
/// `touch` が「ヒットしたか」を `bool` で返す別経路である以上、判定が `get` と食い違うと
/// `show.rs` は「ミスだと思って合成したのに `get` では引ける」あるいはその逆に落ちる。キー 3 成分の
/// それぞれについて、両者の答えが一致することを見る。
#[test]
fn touch_and_get_agree_on_every_key_component() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1101, 1302]);
    let pattern = pattern_of(2000, 1001);

    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern.clone(),
        transparent_surface(5, 5),
    );

    let probes: [(&str, u32, BindSet, PatternState); 4] = [
        ("完全一致", id, binds.clone(), pattern.clone()),
        ("surface id 相違", 1001, binds.clone(), pattern.clone()),
        ("binds 相違", id, BindSet::from_ids([1101]), pattern.clone()),
        ("pattern 相違", id, binds.clone(), pattern_of(2000, 1002)),
    ];
    for (what, pid, pbinds, ppattern) in probes {
        let by_get = cache.get(pid, &pbinds, &ppattern).is_some();
        let by_touch = cache.touch(pid, &pbinds, &ppattern);
        assert_eq!(
            by_touch, by_get,
            "{what}: touch と get の引き当て判定が食い違う（get={by_get} touch={by_touch}）"
        );
    }
}

/// 要件 7.1 観測完了（**上限が実効的であること**）: 同一キーの再挿入は重複エントリを作らず、
/// 後から入れた対で置き換える。
///
/// # なぜ専用の檻が要るのか（変異検査が見つけた穴）
///
/// `insert` が重複を許すと、表の席が同じキーで埋まって**実効容量が縮む**（3 席のうち 2 席が
/// 同じキーになれば、実際に保持できる異なりキーは 2 個である）。しかも引き当ては先に見つかった
/// 側を返すので、**古い絵が返る**経路までできる。容量 3 の裁定はどちらも前提にしていない。
///
/// 本番経路では `insert` はミス時にしか呼ばれないため同一キーの再挿入は起きないが、`insert` は
/// 公開 API であり、この不変条件は呼び手の規律ではなく本層の構造で守るべきものである。実際、
/// 重複を作る変異（同一キーの席を外す枝を殺す）を当てたところ**クレート全体が緑のまま**だった。
#[test]
fn reinserting_the_same_key_replaces_the_entry_without_duplicating_it() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1100]);
    let pattern = PatternState::default();

    // 同一キーで 2 度挿入する。外形を変えて「どちらが返るか」を弁別できるようにする。
    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern.clone(),
        transparent_surface(4, 4),
    );
    insert_with_mask(
        &mut cache,
        id,
        binds.clone(),
        pattern.clone(),
        transparent_surface(5, 5),
    );

    assert_eq!(
        cache.len(),
        1,
        "同一キーの再挿入で重複エントリができている（実効容量が縮む）"
    );
    assert_eq!(
        cache
            .get(id, &binds, &pattern)
            .map(|e| (e.composed.width(), e.composed.height())),
        Some((5, 5)),
        "再挿入は後から入れた対を返す（重複した古い側を返している）"
    );

    // 重複が席を食っていないこと: 別キーを 2 本足しても上限に達しないので、どれも落ちない。
    insert_id(&mut cache, 8001);
    insert_id(&mut cache, 8002);
    assert!(
        cache.get(id, &binds, &pattern).is_some() && holds(&cache, 8001) && holds(&cache, 8002),
        "重複エントリが席を食って異なり 3 キーを保持できていない"
    );
}
