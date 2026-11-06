//! キャンバス外形（[`Extent`]）の表の算出（束縛のときに 1 度だけ・`plan.rs` の下の module）。

use areka_emo_atlas::AtlasTable;
use areka_parsers::shell::{Animation, Interval};

use super::{Extent, is_bind_interval, surface_and_binding};
use crate::nesting::{ElementKind, is_always_interval};
use crate::outline_walk::{Memo, Outline, Scratch, Step, walk};
use crate::pattern_word::{PatternWord, pattern_word};
use crate::world::{EmoWorld, SurfaceExtents};

#[cfg(test)]
use super::*;

/// 外形の一辺の上限（画素）。合成の絵は表示の側が原寸のまま GPU の面へ上げるので、Direct3D 11 の
/// 機能水準 11 の 2D テクスチャの一辺の最大（`D3D11_REQ_TEXTURE2D_U_OR_V_DIMENSION`）を写す。
pub(crate) const MAX_EXTENT: u32 = 16_384;

/// 外形の面積の上限（画素・指摘 23）。合成のたびに幅×高さ×4 バイトを 0 で埋めて転写するので、一辺の上限の
/// 内でも 1 GiB 近い絵を作らない。2^25 画素＝128 MiB で、8K の画面（7680×4320≒3,300 万画素）より大きい。
pub(crate) const MAX_AREA: u64 = 1 << 25;

/// キャンバス外形（[`Extent`]）を**有効 bind 集合に依存せず静的に**算出する（要件 6.5・議題2裁定 (A)）。
///
/// 母集合は **surface の全定義層**＝全 element ＋**全 bind・`always` animation の描くコマ**（`always` と
/// 純 `bind` は全部の pattern・`bind+random` は番号の最も小さい 1 つ・`binds` に含まれるか否かに関係なく全部）。
/// 各層 element の「累積配置オフセット＋原寸（[`AtlasEntry::original`]）」の和集合を、原点 (0,0)
/// 固定・負オフセット分は原点でクリップして取る。`extent = (max_x, max_y)`、
/// 各層で `max_x = max(max_x, max(0, offset_x + original.w))`・同様に `max_y`。**`placement: None`
/// （全透明）の element も原寸は既知ゆえ外形へ寄与する**（design 明記）。
///
/// これにより **bind のオン/オフでサイズが変わらない**（同一 surface へ異なる `BindSet` を渡しても
/// [`Extent`] が不変・emo-present のバッファ再利用/窓サイズ安定に必須）。入れ子 surface 参照は
/// シェルの外形と共有する [`crate::outline_walk::walk`]（(x,y) オフセット累積＋経路にある面の集合による
/// 循環検出）で走査する（命令列の経路との違いは「有効かどうかに関係なく全部の bind を母集合とし、コマを見ない」点）。
///
/// element ゼロ・bind ゼロの surface（外形へ寄与する層が皆無）や不在 surface では `Extent { w:0, h:0 }`
/// を返す（0×0 退化の Err 分類は [`build_plan`] の責務・本関数は分類しない）。
///
/// 広がりの予算は根の面 1 つにつき [`VISIT_BUDGET`]。作業場や覚えを渡す口は [`compute_extent_limited`]
/// （本番で呼ぶのは [`build_extents`] だけ・これは外形を直に確かめるテストの口）。
#[cfg(test)]
pub(crate) fn compute_extent(world: &EmoWorld, atlas: &AtlasTable, surface_id: u32) -> Extent {
    compute_extent_limited(
        &mut Scratch::default(),
        world,
        atlas,
        surface_id,
        crate::budget::VISIT_BUDGET,
        None,
    )
}

/// 全部の面の外形の表を作る（指摘 8・[`EmoWorld::bind_atlas`] の最後に 1 度だけ）。予算は根の面 1 つにつき
/// [`VISIT_BUDGET`]（範囲は合成 1 回と同じ。この表は入る面とその行を数え、命令列はさらに段の積み荷も数えるので、命令列は
/// この表より先に切れることがある）。きれいに終わった面の覚えは根の間で共有する
/// （覚えの有る無しで答えと記録は変わらない）。予算が尽きた `error!` は表 1 つにつき 1 度だけ（作業場を根の間で
/// 共有する・指摘 1）。
pub(crate) fn build_extents(world: &EmoWorld, atlas: &AtlasTable) -> SurfaceExtents {
    build_extents_limited(world, atlas, crate::budget::VISIT_BUDGET)
}

/// [`build_extents`] の中身。`budget` は根の面 1 つの広がりの予算（本番は [`VISIT_BUDGET`]・テストは小さな数）。
pub(crate) fn build_extents_limited(
    world: &EmoWorld,
    atlas: &AtlasTable,
    budget: usize,
) -> SurfaceExtents {
    let (mut scratch, mut memo) = (Scratch::default(), ExtentMemo::new());
    let extents = world
        .surface_ids()
        .map(|id| {
            let extent =
                compute_extent_limited(&mut scratch, world, atlas, id, budget, Some(&mut memo));
            (id, extent)
        })
        .collect();
    SurfaceExtents(extents)
}

/// きれいに終わった面の、下で予算に数えた数と右下（その面の原点からの相対・[`crate::outline_walk::Memo`]）。
pub(crate) type ExtentMemo = Memo<(i64, i64)>;

/// 右下（その面の原点からの相対）。
impl Outline for (i64, i64) {
    /// 右下 `max` に、`(x, y)` ずらした右下 `r` を重ねる。
    fn merge(max: &mut Option<(i64, i64)>, r: Option<(i64, i64)>, x: i64, y: i64) {
        let Some((rx, ry)) = r else {
            return;
        };
        let (rx, ry) = (rx.saturating_add(x), ry.saturating_add(y));
        *max = Some(max.map_or((rx, ry), |(mx, my)| (mx.max(rx), my.max(ry))));
    }
}

/// [`compute_extent`] の中身。`limit` は根の面 1 つの広がりの予算（本番は [`VISIT_BUDGET`]・テストは
/// 小さな数）。`s` は走査の作業場（呼び手のものを借りて使い回す・入口で空へ戻し、出口でも空）。たどり方は
/// シェルの外形と共有する [`crate::outline_walk::walk`] で、ここは面ごとの寄与だけを渡す（指摘 84）。
///
/// `memo` が在れば（[`build_extents`] が全部の面を根に回すとき・外形を直に確かめるテストの口は渡さない）、きれいに終わった面の
/// 右下を覚え、後で同じ面に入るとき予算の残りが「入る分（入る 1 ＋面の行の全部）＋下で数えた数」以上ならそれだけ一度に引いて
/// 覚えた右下を使う（足りなければ今どおり入る＝答えと記録は覚えが無いときと同じ）。段ごとの右下を
/// その面の原点からの相対で持ち、段が終わったら親の原点へずらして重ねるので、覚えた右下も同じ形で重なる。
///
/// 当 surface の全定義層を走査し `max_x`/`max_y` を更新する。命令列の経路（`plan.rs` の `flatten_surface`）と
/// 同じくオフセットを累積して明示のスタックで入れ子へ入るが（たどり方は別に書いてある）、
/// **bind の母集合が異なる**: ops は有効 bind（`binds` ∩ bind animation）のみ、本関数は**全 bind・
/// `always` animation の描くコマ**（`binds` 非依存・有効/非有効を問わない）を辿る。これが「外形は有効 bind
/// 集合に依存しない静的量」（要件 6.5）の実装核心である。
///
/// 各静的 element については `AtlasBinding` が `Some(ElementId)` のもののみ、`atlas.entry(id).original`
/// を「累積オフセット＋原寸」として外形へ寄与させる（未束縛 None は原寸不明ゆえ寄与しない）。
/// `placement` が None でも `original` は既知ゆえ寄与する（ops ではスキップされる層も外形は数える）。
///
/// element定義の子（[`ElementKind::Surface`]）は element定義の X,Y を足して入る（surface-element-
/// nesting 要件 2.5）。コマ（[`PatternState`]）は見ないので、部品のアニメーションでも外形は動かない
/// （要件 2.6）。飛ばす子は外形に数えず、記録は命令の経路だけが出す。
///
/// 子は element → bind の id 昇順の pattern の順に積み、逆にして上から取り出す（深さ優先）。
/// 取り出す時点の祖先の列は今の段までの経路そのもの（先に取り出した兄弟の段は出る印で外れている）。
pub(crate) fn compute_extent_limited(
    s: &mut Scratch<(i64, i64)>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    limit: usize,
    memo: Option<&mut ExtentMemo>,
) -> Extent {
    let answer = walk(
        s,
        world,
        memo,
        surface_id,
        limit,
        "extent",
        |surface, stack| {
            let (master, binding) = surface_and_binding(world, surface)?;
            let mut max = None;
            // 当 surface の静的 element を外形へ寄与させる（束縛済み・placement 有無を問わず原寸で数える）。
            for (i, element) in master.elements.iter().enumerate() {
                // element定義の子（surface-element-nesting 要件 2.5）: 位置を足して子へ入る。範囲を
                // 超える数・先祖は黙って飛ばし（記録は命令の経路の debug! が合成 1 回につき 1 度出す）、
                // 面の表に無い子は入った先で何も足さない。
                if let ElementKind::Surface(child) = element.kind {
                    let (ex, ey) = element.transform.offset();
                    stack.push(Step::Enter {
                        surface: child,
                        x: ex,
                        y: ey,
                        quiet: true,
                    });
                    continue;
                }
                // 原寸を先に決め、数える式は下の 1 行（animated-image-playback design b-1）: 画像は束縛した
                // 絵の原寸、動く絵の子は子の原寸（全部のコマに共通＝静止画だったときと同じ値）。未束縛
                // （原寸不明）は外形に寄与できない（ops 側でも skip 済み）。子の定義が無い（起きないはず）
                // ときは数えず、`error!` は命令の経路が合成 1 回につき 1 行出す。
                let original = match element.kind {
                    ElementKind::Film(film) => world.film_sheet(film).map(|sheet| sheet.original),
                    _ => binding.0.get(i).copied().flatten().map(|element_id| {
                        let original = atlas.entry(element_id).original;
                        (original.w, original.h)
                    }),
                };
                let Some((w, h)) = original else {
                    continue;
                };
                // 段の原点＋原寸を数える（今までどおり element の X,Y は足さない）。負オフセットは根で原点に
                // クリップする（外形は (0,0) を左上に固定・負方向はみ出しは転写時クリップ）。
                Outline::merge(&mut max, Some((w as i64, h as i64)), 0, 0);
            }

            // **全** bind animation を母集合として辿る（有効/非有効を問わない・6.5 の核心）。
            // `always` の animation は表示中ずっと全部のコマを通り、純 `bind` は全部のコマを重ねるので、
            // **全部**の pattern を同じやり方で辿る（animated-image-playback design b-2・seriko-pattern-calls
            // M6・外形はコマに依らない静的な量のまま）。
            // 描画順は外形に無関係（max の和集合ゆえ順序不変）だが、決定性のため id 昇順で走査する。
            // 番号ごとに書いた順で最初の 1 つ（安定ソートの後の dedup・番号で探し直さない・指摘 28）。
            let mut anims: Vec<&Animation> = master
                .animations
                .iter()
                .filter(|a| is_bind_interval(&a.interval) || is_always_interval(&a.interval))
                .collect();
            anims.sort_by_key(|a| a.id);
            anims.dedup_by_key(|a| a.id);

            for anim in anims {
                // `always` と純 `bind` は全部の pattern（着せ替えの手順と同じ・seriko-pattern-calls 6.8・M6）。
                // `bind+random` は今のまま番号の最も小さい 1 つ（pattern0 の無い再生のコマも大きさに数える）。
                let all =
                    is_always_interval(&anim.interval) || matches!(anim.interval, Interval::Bind);
                let patterns = if all {
                    &anim.patterns[..]
                } else {
                    let pattern0 = anim.patterns.iter().min_by_key(|p| p.index);
                    pattern0.map_or(&[][..], std::slice::from_ref)
                };
                for pattern in patterns {
                    // センチネル（非描画）は外形に寄与しない（ops 経路と一致）。欄 2 が animation の
                    // 番号になる 7 語（`start` 等）の先は面ではなく、`move` はサーフェスの番号を無視する
                    // （ukadoc）ので数えない（8 語のどれでもない語だけが面を描く・指摘 83）。
                    // u32 に収まらない番号は先が無い（ほかの走査と同じ）。
                    let Ok(child) = u32::try_from(pattern.surface_id) else {
                        continue;
                    };
                    if pattern_word(pattern.method.as_str()) != PatternWord::Draw {
                        continue;
                    }
                    stack.push(Step::Enter {
                        surface: child,
                        x: pattern.x,
                        y: pattern.y,
                        quiet: false,
                    });
                }
            }
            max
        },
    );
    // 右下は根で原点にクリップして常に >= 0。u32 へは飽和で写す（大きすぎる外形を小さな値に
    // 化けさせず、上限の判定へ渡す）。
    let (max_x, max_y) = answer.unwrap_or((0, 0));
    let to_u32 = |v: i64| u32::try_from(v.max(0)).unwrap_or(u32::MAX);
    Extent {
        w: to_u32(max_x),
        h: to_u32(max_y),
    }
}

#[cfg(test)]
#[path = "plan_extent_tests.rs"]
mod extent_tests;

#[cfg(test)]
#[path = "plan_nesting_extent_tests.rs"]
mod nesting_extent_tests;

#[cfg(test)]
#[path = "plan_extent_film_tests.rs"]
mod film_tests;
