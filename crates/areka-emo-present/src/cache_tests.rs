use super::*;
use crate::presenter::test_support::{bake_world, elem, pattern_overlay_at, shell_of, surface};
use areka_emo_compose::plan::Extent;
use areka_emo_compose::{BindSet, Composer};

/// 外形だけを持つ計画（表は与えられた計画を持つだけで中身を見ない。外形は「どのエントリか」の弁別に使う）。
fn sized(w: u32, h: u32) -> DrawPlan {
    DrawPlan {
        surface_id: 0,
        base_surface: 0,
        extent: Extent { w, h },
        items: Vec::new(),
    }
}

/// 計画を挿入する。
fn insert_plan(
    cache: &mut ComposeCache,
    surface_id: u32,
    binds: BindSet,
    pattern: PatternState,
    plan: DrawPlan,
) {
    cache.insert(surface_id, binds, pattern, plan);
}

/// 3×1 の単一画像（不透明赤 / 透明 / 不透明赤）を 1 element とする面の計画を、**与えられた出力先へ書き戻す**。
///
/// 出力先を受け取るのは、回収した計画への書き戻しで確保が引き継がれるかを測るため
/// （`Composer::plan_into` は出力先の項の列の確保を再利用する）。
fn plan_small_into(out: &mut DrawPlan) {
    // 3×1 premultiplied BGRA: (0,0)不透明赤・(1,0)透明・(2,0)不透明赤。
    let img: Vec<u8> = vec![
        0, 0, 200, 255, // (0,0) 不透明赤
        0, 0, 0, 0, // (1,0) 全透明
        0, 0, 200, 255, // (2,0) 不透明赤
    ];
    let (world, atlas) = bake_world(
        shell_of(vec![surface(1000, vec![elem("otr.png", 0, 0)])]),
        vec![("otr.png", 3, 1, img)],
    );

    Composer::new()
        .plan_into(
            out,
            &world,
            &atlas,
            1000,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("静的 element 単体の計画は Ok");
}

/// 上の計画をまっさらな器へ書いて値で返す（出力先を持たない檻用の便宜）。
fn small_plan() -> DrawPlan {
    let mut out = sized(0, 0);
    plan_small_into(&mut out);
    out
}

// ── 容量政策の檻用の最小補助 ─────────────────────────────────────────────
//
// 既定キー（`BindSet::default()`／`PatternState::default()`）で **surface id だけ**を変える。
// 容量・置換方式は「キーが違う」ことだけで駆動される。

/// 既定キーの surface id `id` で挿入する。
fn insert_id(cache: &mut ComposeCache, id: u32) {
    insert_plan(
        cache,
        id,
        BindSet::default(),
        PatternState::default(),
        sized(4, 4),
    );
}

/// 既定キーの surface id `id` が保持されているか。
///
/// [`ComposeCache::get`] は最近使用順を動かさない（動かすのは [`ComposeCache::touch`] だけ）ので、
/// この観測は LRU の状態を乱さない。
fn holds(cache: &ComposeCache, id: u32) -> bool {
    cache
        .get(id, &BindSet::default(), &PatternState::default())
        .is_some()
}

/// 既定キーの surface id `id` を引き当てて**最近使用へ引き上げる**（本番のヒット経路と同じ口）。
fn touch_id(cache: &mut ComposeCache, id: u32) -> bool {
    cache.touch(id, &BindSet::default(), &PatternState::default())
}

/// 回帰檻（キャッシュ仕様バグ）: **同一 surface id でも bind 集合が異なればミス**する。
///
/// surface id のみをキーにした旧キーの形では、着せ替え・まばたき（bind 差分）が古い合成結果に
/// ヒットし表示が更新されなかった。合成入力の完全一致のみをヒットとすることを固定する。
#[test]
fn different_binds_on_same_surface_must_miss() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let eyes_open = BindSet::from_ids([1101, 1302]);
    let eyes_closed = BindSet::from_ids([1101, 1302, 1400]);

    insert_plan(
        &mut cache,
        id,
        eyes_open.clone(),
        PatternState::default(),
        sized(4, 4),
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

    // 異なる binds を挿入すると**別エントリ**として共存する（容量 3）。まばたきの
    // 開閉のように行き来する入力で再合成を省けることが、容量 3 の裁定の狙いそのものである。
    insert_plan(
        &mut cache,
        id,
        eyes_closed.clone(),
        PatternState::default(),
        sized(4, 4),
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
        insert_plan(
            &mut cache,
            id,
            BindSet::from_ids([1101, 1302, extra]),
            PatternState::default(),
            sized(4, 4),
        );
    }
    assert!(
        cache
            .get(id, &eyes_open, &PatternState::default())
            .is_none(),
        "容量 3 の上限: 異なるキーを入れ続ければ最も古い引き当てから落ちる（無限堆積しない）"
    );
}

/// 回帰檻（pattern がキー要素）: **surface id ＋ bind 集合が完全同一でも pattern が異なれば
/// ミス**する。pattern を合成入力キーへ加えたことの load-bearing な証拠——この 1 点が
/// 欠けると seriko のアニメ pattern 進行が古い合成結果に衝突し表示が更新されない。
#[test]
fn different_pattern_on_same_surface_and_binds_must_miss() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1101, 1302]);
    let pattern_a = pattern_overlay_at(2000, 1001, 0, 0);
    let pattern_b = pattern_overlay_at(2000, 1002, 0, 0);
    assert_ne!(pattern_a, pattern_b, "前提: 2 つの pattern 状態は異なる");

    insert_plan(
        &mut cache,
        id,
        binds.clone(),
        pattern_a.clone(),
        sized(4, 4),
    );

    assert!(
        cache.get(id, &binds, &pattern_a).is_some(),
        "surface id・binds・pattern が完全一致すればヒットする"
    );
    assert!(
        cache.get(id, &binds, &pattern_b).is_none(),
        "surface id・binds 同一でも pattern が異なればミスしなければならない（R5.2・pattern がキー要素）"
    );

    // 2 つの pattern は**別エントリ**として共存し、それぞれ**自分の絵**を返す（古い絵を返さない）。
    // 外形を変えてあるので、キーだけ通ってエントリが取り違わる形はここで赤になる。
    insert_plan(
        &mut cache,
        id,
        binds.clone(),
        pattern_b.clone(),
        sized(5, 5),
    );
    assert_eq!(
        cache
            .get(id, &binds, &pattern_b)
            .map(|e| (e.plan.extent.w, e.plan.extent.h)),
        Some((5, 5)),
        "pattern_b の引き当ては pattern_b の絵を返す"
    );
    assert_eq!(
        cache
            .get(id, &binds, &pattern_a)
            .map(|e| (e.plan.extent.w, e.plan.extent.h)),
        Some((4, 4)),
        "容量 3: pattern_a のエントリは残り、しかも**自分の絵**を返す（取り違えない）"
    );
}

/// 逆側檻（空 pattern はキーへ寄与しない＝pattern を持たない入力と観測等価）と、非空 pattern の
/// 同値ヒットを固定する。空と非空は別キーである。
#[test]
fn empty_vs_nonempty_pattern_are_distinct_keys() {
    let mut cache = ComposeCache::new();
    let id = 42;
    let binds = BindSet::default();
    let pat = pattern_overlay_at(3000, 5000, 0, 0);

    insert_plan(
        &mut cache,
        id,
        binds.clone(),
        PatternState::default(),
        sized(4, 4),
    );
    assert!(
        cache.get(id, &binds, &PatternState::default()).is_some(),
        "空 pattern で挿入 → 空 pattern の get はヒット（拡張前と観測等価・R5.4）"
    );
    assert!(
        cache.get(id, &binds, &pat).is_none(),
        "空 pattern で挿入 → 非空 pattern の get はミス（空と非空は別キー）"
    );

    // 逆向き: 非空 pattern **だけ**を入れた**新しい表**で測る。同じ表へ足すと容量 3 では両方が共存し、
    // 「空と非空が別キーである」という弁別が観測できなくなる（測っているのは弁別であって置換ではない）。
    let mut cache = ComposeCache::new();
    insert_plan(&mut cache, id, binds.clone(), pat.clone(), sized(4, 4));
    assert!(
        cache.get(id, &binds, &pat).is_some(),
        "非空 pattern で挿入 → 同値 pattern の get はヒット"
    );
    assert!(
        cache.get(id, &binds, &PatternState::default()).is_none(),
        "非空 pattern で挿入 → 空 pattern の get はミス（空と非空は別キー）"
    );
}

/// 変わらず: `invalidate_all` は**保持中の全エントリ**を破棄する（挙動不変）。
///
/// 1 件だけの表では「先頭 1 件しか消さない」実装を通してしまうため、満杯の表で測る。
#[test]
fn invalidate_all_clears_every_entry() {
    let mut cache = ComposeCache::new();
    let binds = BindSet::from_ids([1100]);
    let pattern = pattern_overlay_at(2000, 1001, 0, 0);

    insert_plan(&mut cache, 7, binds.clone(), pattern.clone(), sized(5, 5));
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

// ── 容量回収（`take_recycled`） ─────────────────────────

/// **容量回収を挟んでも承認済み意味論は不変**である。
///
/// 承認済み意味論（容量は 2026-08-15 の裁定で 3）は ⑴上限 3 件のメモ化表 ⑵キー完全一致のみヒット
/// ⑶挿した計画がそのまま返る、の 3 点である。`take_recycled` は計画の容量を回収するだけで
/// 政策を変えないため、回収 → 挿入を経ても 3 点とも成立しなければならない。
#[test]
fn insert_after_take_recycled_preserves_approved_semantics() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1101, 1302]);
    let pattern = pattern_overlay_at(2000, 1001, 0, 0);

    // 回収は満杯のときだけ成立する。追跡対象（このキー）が最も古い引き当てになるよう先に入れる。
    insert_plan(&mut cache, id, binds.clone(), pattern.clone(), sized(4, 4));
    insert_id(&mut cache, 8001);
    insert_id(&mut cache, 8002);

    let recycled = cache.take_recycled().expect("満杯なら 1 本追い出される");
    assert_eq!(
        (recycled.plan.extent.w, recycled.plan.extent.h),
        (4, 4),
        "回収したエントリは計画を保持している"
    );
    assert!(
        cache.get(id, &binds, &pattern).is_none(),
        "回収されたキーは以後ミスする"
    );
    assert!(
        holds(&cache, 8001) && holds(&cache, 8002),
        "回収は 1 本だけ——残りのエントリは剥がれない"
    );

    // 回収後の挿入。
    let plan = small_plan();
    let (w, h) = (plan.extent.w, plan.extent.h);
    insert_plan(&mut cache, id, binds.clone(), pattern.clone(), plan);

    let entry = cache
        .get(id, &binds, &pattern)
        .expect("回収後の挿入でも完全一致キーはヒットする");
    assert_eq!((entry.plan.extent.w, entry.plan.extent.h), (w, h));

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
        cache
            .get(id, &binds, &pattern_overlay_at(2000, 1002, 0, 0))
            .is_none(),
        "pattern が異なればミスする"
    );

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

/// **本題**: 回収した計画の容量が次の計画へ引き継がれる。
///
/// 返るのが挿入したその確保でなければ「容量回収」にならない——複製や新規確保を返す実装は、キーの
/// 意味論だけを見る檻ではすべて緑のまま素通りする。ゆえに ⑴挿入した計画の項の列の先頭ポインタが回収
/// エントリにそのまま現れること ⑵回収した計画を**より小さい計画**の書き戻し先に使っても再確保が
/// 起きないこと（同じ大きさの反復だけを見る檻が見逃す新規確保形を捕まえる）⑶その確保のまま再挿入できること、
/// の 3 点で固定する。書き戻しは本番と同じ `Composer::plan_into` で測る。
#[test]
fn take_recycled_carries_over_plan_capacity() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1100]);
    let pattern = PatternState::default();

    // 大きい計画（項 64 個）。以後この 1 本の確保だけを追う。
    let mut big = sized(64, 64);
    big.items.extend(std::iter::repeat_n(
        areka_emo_compose::PlanItem::GroupEnd,
        64,
    ));
    let big_ptr = big.items.as_ptr();
    insert_plan(&mut cache, id, binds.clone(), pattern.clone(), big);
    // 回収は満杯のときだけ成立する（容量 3）。追跡対象が最も古い引き当てのまま
    // 残るよう、別キーで表を埋める。
    insert_id(&mut cache, 9001);
    insert_id(&mut cache, 9002);

    let recycled = cache.take_recycled().expect("満杯なら 1 本追い出される");
    assert_eq!(
        recycled.plan.items.as_ptr(),
        big_ptr,
        "回収したのは挿入したその計画でなければならない（複製・新規確保は容量回収ではない）"
    );
    assert_eq!(recycled.plan.items.len(), 64);

    let mut reused = recycled.plan;
    plan_small_into(&mut reused);
    assert!(
        reused.items.len() < 64,
        "前提: 書き戻し先は回収時より小さい計画になる（項 {}）",
        reused.items.len()
    );
    assert_eq!(
        reused.items.as_ptr(),
        big_ptr,
        "回収した容量が引き継がれていれば、より小さい計画への書き戻しで再確保は起きない"
    );

    cache.insert(id, binds.clone(), pattern.clone(), reused);
    let entry = cache
        .get(id, &binds, &pattern)
        .expect("再挿入したキーはヒットする");
    assert_eq!(
        entry.plan.items.as_ptr(),
        big_ptr,
        "回収 → 使い回し → 再挿入で確保は 1 本のまま（新規確保が挟まっていない）"
    );
}

/// 空きのある表からの回収は `None` を返し、キャッシュは以後も従来どおり使える。
///
/// 回収は「満杯なら最も古い引き当てを 1 本外す」だけの操作で、未挿入・回収直後・無効化後など空きが
/// あれば何も起こらない（べき等）。
#[test]
fn take_recycled_without_a_full_table_returns_none_and_leaves_cache_usable() {
    let mut cache = ComposeCache::new();
    let id = 7;
    let binds = BindSet::from_ids([1100]);
    let pattern = pattern_overlay_at(2000, 1001, 0, 0);

    assert!(
        cache.take_recycled().is_none(),
        "未挿入の表からは何も回収できない"
    );

    insert_plan(&mut cache, id, binds.clone(), pattern.clone(), sized(4, 4));
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

    insert_plan(&mut cache, id, binds.clone(), pattern.clone(), sized(4, 4));
    assert!(
        cache.get(id, &binds, &pattern).is_some(),
        "回収後に挿入し直せば再びヒットする"
    );

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

// ── 容量 3・LRU 置換（2026-08-15 開発者裁定） ──────────────────────

/// （**容量の上限そのもの**）: 4 本目の挿入は**ちょうど 1 本**を追い出す。
///
/// 容量は 2026-08-15 の開発者裁定で 1 → 3 に決まった。**追い出しが起きること**と**落ちるのが
/// 1 本だけであること**を対で固定して初めて容量が 3 に決まる（容量 1・2・4 以上のどれへ変えても
/// 保持の列が変わって赤になる）。
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

/// （**置換方式そのもの**）: 追い出されるのは**最近最も使われていない**もので
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
/// 容量 3 の裁定は、実走行の適用列を **LRU で再生**した命中率（キャラ面 56.2%）を根拠にしている。
/// FIFO へ退化すると、まばたきのように「直前に使った面へ戻る」列で命中せず、数字の出所が実装と
/// 対応しなくなる。**表示バイトも確保計数も 1 つも変わらない**ため、この檻だけが退化を捕まえる。
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

/// （**回収も LRU に従う**）: [`ComposeCache::take_recycled`] が返すのも
/// 最近最も使われていない 1 本である。
///
/// 上の檻と同じ「挿入順と最近使用順が食い違う」状態を作り、回収された実体を**外形で**弁別する。
#[test]
fn take_recycled_picks_the_least_recently_used_entry() {
    let mut cache = ComposeCache::new();
    // 外形を変えて「どのエントリが回収されたか」を回収物から直接読めるようにする。
    for (id, side) in [(1000_u32, 4_u32), (1001, 5), (1002, 6)] {
        insert_plan(
            &mut cache,
            id,
            BindSet::default(),
            PatternState::default(),
            sized(side, side),
        );
    }
    assert!(touch_id(&mut cache, 1000), "前提: 1000 は保持されている");

    let recycled = cache.take_recycled().expect("満杯なら 1 本追い出される");
    assert_eq!(
        (recycled.plan.extent.w, recycled.plan.extent.h),
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
/// `show.rs` の表示成立点も `read.rs` の `read_back` も、引き当て済みエントリを `get` で読む。
/// 読み取りが順序を動かす形だと観測しただけで最近使用を打ち直してしまい、檻もまた自分の観測で
/// LRU の状態を書き換えてしまう（本ファイルの [`holds`] がそれである）。
///
/// 逆向きの退化（`touch` が順序を動かさない）は上の 2 本の檻が捕まえる。本檻はその対である。
#[test]
fn get_does_not_disturb_the_recency_order() {
    let mut cache = ComposeCache::new();
    for id in [1000, 1001, 1002] {
        insert_id(&mut cache, id);
    }

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
/// `touch` が「ヒットしたか」を `bool` で返す別経路である以上、判定が `get` と食い違うと `show.rs` は
/// 「ミスだと思って合成したのに `get` では引ける」あるいはその逆に落ちる。キー 3 成分のそれぞれで
/// 両者の答えが一致することを見る。
#[test]
fn touch_and_get_agree_on_every_key_component() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1101, 1302]);
    let pattern = pattern_overlay_at(2000, 1001, 0, 0);

    insert_plan(&mut cache, id, binds.clone(), pattern.clone(), sized(5, 5));

    let probes: [(&str, u32, BindSet, PatternState); 4] = [
        ("完全一致", id, binds.clone(), pattern.clone()),
        ("surface id 相違", 1001, binds.clone(), pattern.clone()),
        ("binds 相違", id, BindSet::from_ids([1101]), pattern.clone()),
        (
            "pattern 相違",
            id,
            binds.clone(),
            pattern_overlay_at(2000, 1002, 0, 0),
        ),
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

/// （**上限が実効的であること**）: 同一キーの再挿入は重複エントリを作らず、
/// 後から入れた対で置き換える。
///
/// # なぜ専用の檻が要るのか（変異検査が見つけた穴）
///
/// `insert` が重複を許すと、同じキーが席を埋めて**実効容量が縮み**、しかも引き当ては先に見つかった
/// 側を返すので**古い絵が返る**経路までできる。本番経路では `insert` はミス時にしか呼ばれず同一キーの
/// 再挿入は起きないが、`insert` は公開 API であり、この不変条件は呼び手の規律ではなく本層の構造で守る
/// べきものである。重複を作る変異（同一キーの席を外す枝を殺す）を当てても**クレート全体が緑のまま**だった。
#[test]
fn reinserting_the_same_key_replaces_the_entry_without_duplicating_it() {
    let mut cache = ComposeCache::new();
    let id = 1000;
    let binds = BindSet::from_ids([1100]);
    let pattern = PatternState::default();

    // 同一キーで 2 度挿入する。外形を変えて「どちらが返るか」を弁別できるようにする。
    insert_plan(&mut cache, id, binds.clone(), pattern.clone(), sized(4, 4));
    insert_plan(&mut cache, id, binds.clone(), pattern.clone(), sized(5, 5));

    assert_eq!(
        cache.len(),
        1,
        "同一キーの再挿入で重複エントリができている（実効容量が縮む）"
    );
    assert_eq!(
        cache
            .get(id, &binds, &pattern)
            .map(|e| (e.plan.extent.w, e.plan.extent.h)),
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
