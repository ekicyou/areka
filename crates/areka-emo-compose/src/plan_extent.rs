//! キャンバス外形（[`Extent`]）の算出（`plan.rs` から中身を変えずに移した・animated-image-playback）。

use std::collections::{HashMap, HashSet};

use areka_emo_atlas::AtlasTable;
use areka_parsers::shell::Interval;

use super::{Extent, is_bind_interval, surface_and_binding};
use crate::budget::Budget;
use crate::nesting::{ElementKind, is_always_interval};
use crate::world::{EmoWorld, targets_animation_id};

#[cfg(test)]
use super::*;

/// 外形の一辺の上限（画素）。合成の絵は表示の側が原寸のまま GPU の面へ上げるので、Direct3D 11 の
/// 機能水準 11 の 2D テクスチャの一辺の最大（`D3D11_REQ_TEXTURE2D_U_OR_V_DIMENSION`）を写す。
pub(crate) const MAX_EXTENT: u32 = 16_384;

/// キャンバス外形（[`Extent`]）を**有効 bind 集合に依存せず静的に**算出する（要件 6.5・議題2裁定 (A)）。
///
/// 母集合は **surface の全定義層**＝全 element ＋**全 bind animation の描くコマ**（純 `bind` は全部の
/// pattern・`bind+random` は番号の最も小さい 1 つ・`binds` に含まれるか否かに関係なく全部）。
/// 各層 element の「累積配置オフセット＋原寸（[`AtlasEntry::original`]）」の和集合を、原点 (0,0)
/// 固定・負オフセット分は原点でクリップして取る。`extent = (max_x, max_y)`、
/// 各層で `max_x = max(max_x, max(0, offset_x + original.w))`・同様に `max_y`。**`placement: None`
/// （全透明）の element も原寸は既知ゆえ外形へ寄与する**（design 明記）。
///
/// これにより **bind のオン/オフでサイズが変わらない**（同一 surface へ異なる `BindSet` を渡しても
/// [`Extent`] が不変・emo-present のバッファ再利用/窓サイズ安定に必須）。入れ子 surface 参照は
/// [`flatten_surface`] と同一の pattern の (x,y) オフセット累積＋visited 祖先スタックによる循環検出で
/// 走査する（ops 経路との違いは「全 bind pattern0 を母集合とする」点のみ）。
///
/// element ゼロ・bind ゼロの surface（外形へ寄与する層が皆無）や不在 surface では `Extent { w:0, h:0 }`
/// を返す（0×0 退化の Err 分類は [`build_plan`] の責務・本関数は分類しない）。
///
/// 広がりの予算は根の面 1 つにつき [`VISIT_BUDGET`]。命令列と同じ予算・作業場や覚えを渡す口は
/// [`compute_extent_limited`]（本番の口はそちらだけ・これはテストの口）。
#[cfg(test)]
pub(crate) fn compute_extent(world: &EmoWorld, atlas: &AtlasTable, surface_id: u32) -> Extent {
    compute_extent_limited(
        &mut HashSet::new(),
        world,
        atlas,
        surface_id,
        crate::budget::VISIT_BUDGET,
        None,
    )
}

/// 外形の走査の積み荷: 面へ親の原点から (x, y) ずらして入る（`element` は element定義の子＝循環を黙って
/// 飛ばす）か、面から出る。
enum Step {
    Enter {
        surface: u32,
        x: i64,
        y: i64,
        element: bool,
    },
    Exit,
}

/// 外形の走査の段: 面と、親の原点からの位置と、段の下で数えた右下（その面の原点からの相対・何も数えて
/// いなければ `None`）と、きれいに終わるかと、段の下で予算に数えた数。
struct Level {
    surface: u32,
    x: i64,
    y: i64,
    max: Option<(i64, i64)>,
    clean: bool,
    spent: usize,
}

/// きれいに終わった面（その下で先祖へ戻る辺を捨てず、予算にも断られなかった面）の、下で予算に数えた数と
/// 右下（その面の原点からの相対）。きれいに終わった面の下に循環は無いので、どの経路から入っても答えは同じ。
pub(crate) type ExtentMemo = HashMap<u32, (usize, Option<(i64, i64)>)>;

/// 右下 `max` に、`(x, y)` ずらした右下 `r` を重ねる。
fn merge(max: &mut Option<(i64, i64)>, r: Option<(i64, i64)>, x: i64, y: i64) {
    let Some((rx, ry)) = r else {
        return;
    };
    let (rx, ry) = (rx.saturating_add(x), ry.saturating_add(y));
    *max = Some(max.map_or((rx, ry), |(mx, my)| (mx.max(rx), my.max(ry))));
}

/// [`compute_extent`] の中身。`limit` は根の面 1 つの広がりの予算（本番は [`VISIT_BUDGET`]・テストは
/// 小さな数）。`on_path` は経路にある面の集合（呼び手の作業場を借りて使い回す・入口で空へ戻し、出口でも空）。
///
/// `memo` が在れば（シェルの外形が全部の面を根に回すとき・合成からは渡さない）、きれいに終わった面の
/// 右下を覚え、後で同じ面に入るとき予算の残りが「入る 1 ＋下で数えた数」以上ならそれだけ一度に引いて
/// 覚えた右下を使う（足りなければ今どおり入る＝答えと記録は覚えが無いときと同じ）。段ごとの右下を
/// その面の原点からの相対で持ち、段が終わったら親の原点へずらして重ねるので、覚えた右下も同じ形で重なる。
///
/// 当 surface の全定義層を走査し `max_x`/`max_y` を更新する。[`flatten_surface`]（ops 経路）と同一の
/// 入れ子 flatten 構造（オフセット累積＋visited 祖先スタックによる循環検出）を明示のスタックで回すが、
/// **bind の母集合が異なる**: ops は有効 bind（`binds` ∩ bind animation）のみ、本関数は**全 bind
/// animation の pattern0**（`binds` 非依存・有効/非有効を問わない）を辿る。これが「外形は有効 bind
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
/// 子は今までの再帰と同じ順（element → bind の id 昇順の pattern）に積み、逆にして上から取り出す。
/// 取り出す時点の祖先の列は再帰の入口と同じ（先に取り出した兄弟の段は出る印で外れている）。
pub(crate) fn compute_extent_limited(
    on_path: &mut HashSet<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    limit: usize,
    mut memo: Option<&mut ExtentMemo>,
) -> Extent {
    // ops 経路と同じ祖先の規律で循環検出する（別走査・別集約器だが構造は共有）。走査開始前に空へ戻す。
    on_path.clear();
    let mut budget = Budget::new(limit);
    let mut levels: Vec<Level> = Vec::new();
    let mut answer = None;
    let mut stack = vec![Step::Enter {
        surface: surface_id,
        x: 0,
        y: 0,
        element: false,
    }];
    while let Some(step) = stack.pop() {
        let (surface, x, y, element) = match step {
            // 枝離脱: 経路から外す（非循環の重複参照は別経路で再走査可能・要件 7.1）。段の右下を親の
            // 原点へずらして重ねる（根なら答え）。
            Step::Exit => {
                let Some(done) = levels.pop() else {
                    break;
                };
                let removed = on_path.remove(&done.surface);
                debug_assert!(removed, "外形算出: 経路の集合は段と揃う");
                if done.clean
                    && let Some(m) = memo.as_deref_mut()
                {
                    m.insert(done.surface, (done.spent, done.max));
                }
                match levels.last_mut() {
                    Some(up) => {
                        up.spent += done.spent + 1;
                        up.clean &= done.clean;
                        merge(&mut up.max, done.max, done.x, done.y);
                    }
                    None => answer = done.max,
                }
                continue;
            }
            Step::Enter {
                surface,
                x,
                y,
                element,
            } => (surface, x, y, element),
        };
        // 循環検出（ops 経路と同一規律・要件 7.2/7.3）: 現在の祖先経路に既出なら打ち切り。element定義の
        // 子は黙って飛ばし（記録は命令の経路の debug! が合成 1 回につき 1 度出す）、pattern の先の
        // 知らせは ops 経路と同じく debug。
        if on_path.contains(&surface) {
            if !element {
                tracing::debug!(
                    target: "areka_emo_compose",
                    surface_id = surface,
                    "外形算出: 入れ子参照の循環を検出・枝を打ち切り"
                );
            }
            if let Some(up) = levels.last_mut() {
                up.clean = false;
            }
            continue;
        }
        // 根の面（段が無い）は予算に数えない。
        if let Some(up) = levels.last_mut() {
            if let Some(&(n, max)) = memo.as_deref().and_then(|m| m.get(&surface))
                && budget.take(n + 1)
            {
                up.spent += n + 1;
                merge(&mut up.max, max, x, y);
                continue;
            }
            if !budget.enter("extent", surface_id) {
                up.clean = false;
                continue;
            }
        }
        on_path.insert(surface);
        stack.push(Step::Exit);
        // 子の積み荷はここから先へ今までの順に積み、最後に逆にする（先に取り出すものを上へ）。
        let children = stack.len();
        let mut max = None;

        if let Some((master, binding)) = surface_and_binding(world, surface) {
            // 当 surface の静的 element を外形へ寄与させる（束縛済み・placement 有無を問わず原寸で数える）。
            for (i, element) in master.elements.iter().enumerate() {
                // element定義の子（surface-element-nesting 要件 2.5）: 位置を足して子へ入る。範囲を
                // 超える数・先祖は黙って飛ばし、面の表に無い子は入った先で何も足さない。
                if let ElementKind::Surface(child) = element.kind {
                    let (ex, ey) = element.transform.offset();
                    stack.push(Step::Enter {
                        surface: child,
                        x: ex,
                        y: ey,
                        element: true,
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
                merge(&mut max, Some((w as i64, h as i64)), 0, 0);
            }

            // **全** bind animation を母集合として辿る（有効/非有効を問わない・6.5 の核心）。
            // `always` の animation は表示中ずっと全部のコマを通り、純 `bind` は全部のコマを重ねるので、
            // **全部**の pattern を同じやり方で辿る（animated-image-playback design b-2・seriko-pattern-calls
            // M6・外形はコマに依らない静的な量のまま）。
            // 描画順は外形に無関係（max の和集合ゆえ順序不変）だが、決定性のため id 昇順で走査する。
            let mut bind_ids: Vec<u32> = master
                .animations
                .iter()
                .filter(|a| is_bind_interval(&a.interval) || is_always_interval(&a.interval))
                .map(|a| a.id)
                .collect();
            bind_ids.sort_unstable();
            bind_ids.dedup();

            for id in bind_ids {
                let Some(anim) = master.animations.iter().find(|a| a.id == id) else {
                    continue;
                };
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
                    // （ukadoc）ので数えない。
                    if pattern.surface_id < 0
                        || targets_animation_id(pattern.method.as_str())
                        || crate::method::is_move_name(pattern.method.as_str())
                    {
                        continue;
                    }
                    stack.push(Step::Enter {
                        surface: pattern.surface_id as u32,
                        x: pattern.x,
                        y: pattern.y,
                        element: false,
                    });
                }
            }
        }
        levels.push(Level {
            surface,
            x,
            y,
            max,
            clean: true,
            spent: 0,
        });
        stack[children..].reverse();
    }
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
