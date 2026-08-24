use super::*;

use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::BindSet;

use wintf::ecs::WucGraphicsResource;

use super::test_support::{
    build_target_assets, build_two_face_assets, elem, make_world_with_gpu, scaled_golden,
    set_window_dpi, shell_of, show_ok, spawn_window_with_dpi, surface,
};

// ── DPI 追従（k 適用の単一漏斗）: タスク 3.2／3.3 の檻 ────────────────────────────────────
// k は「target ごとの政策（author_dpi）× 窓ごとの実 DPI」から **show 適用ごと**に導出される。
// 檻は (a) 政策が窓単位で保たれること、(b) 導出 k が実際に合成結果へ掛かって表示寸・表示バイトを
// 変えること、(c) k がキャッシュキーへ届くこと、(d) DPI 不在が縮退分岐として独立に成立すること。

/// タスク 3.2・要件 1.5 観測完了（窓ごとの k 基底）: `attach_target` は target ごとに拡大政策を
/// 保持し、**別窓・別 author_dpi の 2 target が互いの政策を汚さない**。同一の窓 DPI（192）を与えて
/// も政策が異なれば導出 k が異なる＝政策が k の基底として実際に効いている。
///
/// `attach_target` は skeleton 登録のみで World に触れないため GPU 不要（素の `World` で決定論固定）。
#[test]
fn attach_target_keeps_scale_policy_per_window() {
    let mut world = World::new();
    let win_96 = spawn_window_with_dpi(&mut world, 96);
    let win_144 = spawn_window_with_dpi(&mut world, 144);
    let (w0, a0, _g) = build_target_assets(3, 2, 0x91);
    let (w1, a1, _g) = build_target_assets(3, 2, 0x92);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), win_96, w0, a0, 96)
        .expect("attach_target(0) 失敗");
    presenter
        .attach_target(&mut world, TargetId(1), win_144, w1, a1, 144)
        .expect("attach_target(1) 失敗");

    let p0 = presenter.targets.get(&TargetId(0)).unwrap().policy;
    let p1 = presenter.targets.get(&TargetId(1)).unwrap().policy;
    assert_eq!(p0.author_dpi, 96, "target 0 は自分の author_dpi を保つ");
    assert_eq!(p1.author_dpi, 144, "target 1 は自分の author_dpi を保つ");
    assert_eq!(
        p0.app_scale,
        ScaleRatio::ONE,
        "アプリ管理拡大率は ONE 固定シーム（要件 1.6）"
    );
    assert_eq!(p1.app_scale, ScaleRatio::ONE);
    assert_eq!(
        presenter.targets.get(&TargetId(0)).unwrap().window,
        win_96,
        "政策は target＝窓の対応ごとに保たれる"
    );

    // 同一の窓 DPI を与えても政策が違えば k が違う（政策が k の基底＝要件 1.5 の窓ごと k）。
    assert_eq!(
        derive_scale(p0, Some((192, 192))),
        ScaleRatio::new(2, 1).unwrap()
    );
    assert_eq!(
        derive_scale(p1, Some((192, 192))),
        ScaleRatio::new(4, 3).unwrap()
    );

    // 表示前は実適用 k・native 原寸とも未確定（照会は「まだ何も適用していない」を 1.0 で塗らない）。
    for id in [TargetId(0), TargetId(1)] {
        let t = presenter.targets.get(&id).unwrap();
        assert_eq!(t.applied, None, "表示成立前の applied は None");
        assert_eq!(t.native_size, None, "表示成立前の native_size は None");
        assert!(t.last_show.is_none(), "表示成立前の last_show は None");
        assert!(
            presenter.text_slot_view(id).is_none(),
            "表示成立前は照会不可"
        );
    }
}

/// タスク 3.3 の名指し受け入れ基準・要件 2.1/2.2 観測完了（k=2/1 の実拡大表示）: 窓 `DPI`=192・
/// author_dpi=96（k=2/1）でキャッシュミスの `ShowSurface` を適用すると——(a) 供給面寸が
/// `scaled_extent(2/1, native)` と一致し、(b) `read_back` のバイト長がその寸に一致し、
/// (c) `read_back` バイトが **native 合成 → `resample(2/1)`** の独立再現と全バイト一致する。
///
/// k=1.0 固定の途中状態なら (a) が native 寸のまま残るため RED になる（要件 2.2 の「両水準が同一
/// 物理寸にならない」ことを、96 水準の既存 golden 檻と対で担保する）。
#[test]
fn show_surface_scales_display_to_scaled_extent_at_k2() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);

    let (emo_world, atlas, native_golden) = build_target_assets(3, 2, 0x81);
    // 同一入力を独立に再現して k 適用後の golden を作る（presenter の内部値の追認ではない）。
    let (probe_world, probe_atlas, _) = build_target_assets(3, 2, 0x81);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let (scaled_golden_bytes, native_size, scaled_size) =
        scaled_golden(&probe_world, &probe_atlas, 1000, k2);
    assert_eq!(native_size, (3, 2), "fixture の native 原寸");
    assert_eq!(
        scaled_size,
        k2.scaled_extent(3, 2),
        "golden の外形は丸め権威 scaled_extent に従う"
    );
    assert_eq!(scaled_size, (6, 4));

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    // (a) 供給面（swap chain）寸＝k 倍後の物理寸（既存の「composed 外形従属」連鎖が k 追従した証跡）。
    let chain_size = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.chain.as_ref())
        .expect("表示成立後は供給面が生成済み")
        .size();
    assert_eq!(
        chain_size, scaled_size,
        "供給面寸が scaled_extent(k, native) と一致しない（k が表示へ届いていない）"
    );

    // (b) readback の画素数が k 倍後の寸に一致（stride = width*4 の密配列）。
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb.len(),
        (scaled_size.0 * scaled_size.1 * 4) as usize,
        "readback の画素数が k 倍後の寸と一致しない"
    );
    assert_ne!(
        rb.len(),
        native_golden.len(),
        "k=2/1 なのに native 寸のまま（k=1.0 固定の途中状態が残っている・要件 2.2）"
    );

    // (c) バイトそのものが native→resample(k) の独立再現と一致（寸だけ合わせた偽物を弾く）。
    assert_eq!(
        rb, scaled_golden_bytes,
        "表示バイトが native 合成の k 倍リサンプル結果と一致しない"
    );

    // 実適用 k・native 原寸が表示成立点で記録される（照会契約の単一真実源）。
    let t = presenter.targets.get(&TargetId(0)).unwrap();
    assert_eq!(t.applied, Some(k2), "applied が実適用 k と一致しない");
    assert_eq!(
        t.native_size,
        Some(native_size),
        "native_size は k 適用前の原寸"
    );
    assert_eq!(
        t.last_show.as_ref().map(|(id, _, _)| *id),
        Some(1000),
        "last_show は最後に成立した show 入力を保持する"
    );
}

/// タスク 3.2・要件 1.2 観測完了（照会契約の更新）: k=2/1 の表示確立後、`TextSlotView::scale()` は
/// **実適用 k（2.0）**を返し（恒常 1.0 の廃止）、`surface_size()` は **native 原寸**を返す
/// （供給面が持つ k 適用後の物理寸ではない）。物理寸との関係は
/// `scaled_extent(scale(), surface_size()) == chain.size()` として成立する。
#[test]
fn text_slot_view_reports_applied_scale_and_native_surface_size() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x82);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");
    assert_eq!(
        view.scale(),
        2.0,
        "scale() が実適用 k を返さない（恒常 1.0 の定数返しが残っている）"
    );
    assert_eq!(
        view.surface_size(),
        (3, 2),
        "surface_size() は native 原寸（k 適用後の供給面寸ではない）"
    );

    // 契約式: 物理寸 == scaled_extent(k, native)。供給面の実寸で裏取りする。
    let chain_size = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.chain.as_ref())
        .unwrap()
        .size();
    let k = ScaleRatio::new(2, 1).unwrap();
    assert_eq!(
        k.scaled_extent(view.surface_size().0, view.surface_size().1),
        chain_size,
        "物理寸 = scaled_extent(scale(), surface_size()) の契約が成立しない"
    );
    assert_ne!(
        view.surface_size(),
        chain_size,
        "k≠1 では native 原寸と物理寸が一致しない（供給面寸を返していれば同値になる）"
    );
}

/// 要件 3.1（物理寸の照会契約・丸め権威の単一化）: `physical_size()` は
/// `scaled_extent(applied, surface_size())` と厳密に一致し、供給面の実寸とも一致する。
///
/// # なぜ **7/6**（窓 DPI 112 ／ author_dpi 96）と native 27px なのか
///
/// 既約分母が 2 冪でない k を選ぶ。`ScaleRatio::as_f32()` は `7/6` を厳密に表現できず
/// `1.16666662693…`（真値より下）へ丸まるため、`27 × as_f32()` は `31.4999989…` となり
/// round half away from zero が **31** へ切り下がる。一方、丸め権威 `scaled_extent` は整数演算
/// `(2·27·7 + 6) / (2·6)` で `31.5` を **32** へ正しく丸める。すなわち本ケースは
/// 「`as_f32()` 経由で寸法を計算した実装」と「権威経由の実装」を**数値で弁別**する
/// （両者が一致する 0.25 刻みの k＝分母 2 冪だけを見る檻では、この差は構造的に観測できない）。
#[test]
fn text_slot_view_physical_size_uses_rounding_authority_not_f32_scale() {
    let mut world = make_world_with_gpu();
    // 窓 DPI 112 ÷ author_dpi 96 = 7/6（既約分母 6＝非 2 冪・f32 で非厳密）。
    let window = spawn_window_with_dpi(&mut world, 112);
    let (emo_world, atlas, _golden) = build_target_assets(27, 27, 0x5B);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");
    let k = ScaleRatio::new(112, 96).expect("非ゼロ比");

    // (a) 契約: physical_size() == scaled_extent(applied, surface_size())。
    assert_eq!(view.surface_size(), (27, 27), "前提: native 原寸");
    assert_eq!(
        view.physical_size(),
        k.scaled_extent(27, 27),
        "physical_size() は丸め権威 scaled_extent と一致しなければならない"
    );
    assert_eq!(
        view.physical_size(),
        (32, 32),
        "27 × 7/6 = 31.5 → round half away from zero = 32（権威の検算値）"
    );

    // (b) 供給面の実寸とも一致する（照会値＝実表示の担保・要件 4.2）。
    let chain_size = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.chain.as_ref())
        .expect("表示確立後は chain がある")
        .size();
    assert_eq!(
        view.physical_size(),
        chain_size,
        "physical_size() が実際の供給面寸と食い違う"
    );

    // (c) **非空虚性の核**: `as_f32()` から掛け算で復元した値は権威と食い違う（31 ≠ 32）。
    //     physical_size() が `as_f32` 経由で実装されていれば (a)(b) ごと落ちる。
    let via_f32 = (27.0f32 * view.scale()).round() as u32;
    assert_eq!(
        via_f32, 31,
        "前提: as_f32 経由の掛け算はこの k で 31 へ切り下がる（弁別の前提が崩れていないこと）"
    );
    assert_ne!(
        view.physical_size().0,
        via_f32,
        "physical_size() が as_f32 経由の掛け算と同値＝丸め権威を通っていない"
    );

    // (d) k≠1 ゆえ native 原寸とも一致しない（surface_size をそのまま返していれば落ちる）。
    assert_ne!(
        view.physical_size(),
        view.surface_size(),
        "k≠1 では物理寸と native 原寸は一致しない"
    );
}

/// 要件 3.1（窓 client 物理寸の照会・消費点の単一口）: `EmoPresenter::target_physical_size` は
/// 丸め権威 `scaled_extent` を通した物理寸を返し、`TextSlotView::physical_size()` とも供給面の
/// 実寸（`chain.size()`）とも一致する。未登録・表示成立前は `None`。
///
/// k は `TextSlotView` 側の檻と同じ **7/6**（窓 DPI 112 ／ author_dpi 96）× native 27px を使う——
/// `as_f32()` 経由の掛け算（31）と権威（32）が**数値で弁別**できる唯一種の k であり、
/// 分母が 2 冪の k（0.25 刻み）だけを見る檻では両実装の差が構造的に観測できないため。
#[test]
fn target_physical_size_uses_rounding_authority_and_matches_view_and_chain() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 112);
    let (emo_world, atlas, _golden) = build_target_assets(27, 27, 0x6C);

    let mut presenter = EmoPresenter::new();
    // 未登録 target は None（「まだ何も適用していない」を原寸で塗り潰さない）。
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        None,
        "未登録 target の物理寸は None"
    );

    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    // 装着済みでも表示成立前（applied/native_size 未確定）は None。
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        None,
        "初回 ShowSurface 前の物理寸は None"
    );

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let k = ScaleRatio::new(112, 96).expect("非ゼロ比");
    let physical = presenter
        .target_physical_size(TargetId(0))
        .expect("表示確立後は Some");

    // (a) 丸め権威との一致（27 × 7/6 = 31.5 → round half away from zero = 32）。
    assert_eq!(
        physical,
        k.scaled_extent(27, 27),
        "target_physical_size は丸め権威 scaled_extent と一致しなければならない"
    );
    assert_eq!(physical, (32, 32), "権威の検算値");

    // (b) TextSlotView::physical_size() と同値（2 つの照会口が食い違わない）。
    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");
    assert_eq!(
        physical,
        view.physical_size(),
        "2 つの物理寸照会口が食い違う（同一の applied/native から同一権威で導くはず）"
    );

    // (c) 供給面の実寸とも一致する（照会値＝実表示・要件 4.2）。
    let chain_size = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.chain.as_ref())
        .expect("表示確立後は chain がある")
        .size();
    assert_eq!(physical, chain_size, "物理寸が実際の供給面寸と食い違う");

    // (d) 非空虚性: native 原寸とも、`as_f32` 経由の掛け算とも異なる（両実装ミスを弾く）。
    assert_ne!(
        physical,
        view.surface_size(),
        "k≠1 では物理寸と native 原寸は一致しない（native を返していれば落ちる）"
    );
    let via_f32 = (27.0f32 * view.scale()).round() as u32;
    assert_eq!(
        via_f32, 31,
        "前提: as_f32 経由の掛け算はこの k で 31 へ切り下がる"
    );
    assert_ne!(
        physical.0, via_f32,
        "target_physical_size が as_f32 経由の掛け算と同値＝権威を通っていない"
    );
}

/// 要件 1.3/7.2（恒等 k の等価）: k=1/1 では `target_physical_size` が native 原寸と一致し、
/// `TextSlotView::physical_size()` とも揃う（恒等ゆえ既存挙動と等価）。
#[test]
fn target_physical_size_equals_native_at_identity_scale() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0xA4);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");
    assert_eq!(view.scale(), 1.0, "前提: 恒等 k");
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some((3, 2)),
        "k=1/1 では物理寸＝native 原寸（恒等・既存等価）"
    );
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(view.physical_size()),
        "恒等 k でも 2 つの照会口は一致する"
    );
}

/// 要件 1.3/7.2（恒等 k の等価）: k=1/1 では `physical_size()` と `surface_size()` が一致する。
#[test]
fn text_slot_view_physical_size_equals_native_at_identity_scale() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x91);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("表示確立後の text_slot_view は Some");
    assert_eq!(view.scale(), 1.0, "前提: 恒等 k");
    assert_eq!(
        view.physical_size(),
        view.surface_size(),
        "k=1/1 では物理寸＝native 原寸（恒等・既存等価）"
    );
    assert_eq!(view.physical_size(), (3, 2), "恒等ゆえ原寸そのまま");
}

/// 要件 1.4 観測完了（DPI 取得不能の縮退・専用檻）: 窓 entity に `DPI` component が**無い**target
/// でも表示は成立し、k は 1.0 へ縮退する（表示を失わない）。
///
/// # `author_dpi` に **192**（非 96）を使う理由＝縮退の**帰属可能性**
///
/// author_dpi=96 で組むと、縮退の答（`app_scale × 1/1` ＝ 1/1）と「component 不在を 96 で捏造した
/// 場合の答」（`96/96` ＝ 1/1）が**数値として区別できない**。すなわち `world.get::<DPI>(..)` に
/// `.or(Some((96, 96)))` を足す実装ミス——本体コメントが名指しで禁じている当のもの——を素通し
/// させてしまい、檻が空虚になる。author_dpi=192 なら捏造時の k は `96/192 = 1/2` となり、
/// 適用 k・readback 寸（`scaled_extent(1/2, (4,3)) = (2,2)`）・`scale()` の 3 つがすべて外れる。
/// したがって本テストの緑は「縮退分岐を通った」ことに帰属する。
///
/// 縮退時の表示は k=1.0 の等倍＝native 合成 golden と全バイト一致であり、`scale()` は 1.0 を返す。
/// 他テストは `DPI` を明示挿入する規律ゆえ、この分岐は本テストだけが踏む（縮退が「正常系のふり」で
/// 通らないことの保証）。`derive_scale` 側の `error!` 発火自体は同関数の in-crate テストが檻に入れる。
#[test]
fn show_surface_without_dpi_component_degrades_to_identity() {
    let mut world = make_world_with_gpu();
    // 意図的に DPI component 無しの窓（本番では起こらない＝取得不能の代替）。
    let window = world.spawn_empty().id();
    assert!(
        world.get::<DPI>(window).is_none(),
        "前提: DPI component 不在"
    );

    let (emo_world, atlas, native_golden) = build_target_assets(4, 3, 0x83);

    let mut presenter = EmoPresenter::new();
    // author_dpi=192（非 96）: 縮退の 1/1 と「96 捏造」の 96/192=1/2 を数値で弁別する（上記 doc）。
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 192)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let t = presenter.targets.get(&TargetId(0)).unwrap();
    assert_eq!(
        t.applied,
        Some(ScaleRatio::ONE),
        "DPI 不在は author_dpi に依らず app_scale×1/1 へ縮退する（要件 1.4）"
    );
    assert_eq!(t.native_size, Some((4, 3)));
    assert!(t.visible, "縮退しても表示を失わない");
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        native_golden,
        "k=1.0 縮退の表示は等倍 native 合成と全バイト一致（96 捏造なら 1/2 縮小で 2×2 になる）"
    );
    assert_eq!(
        presenter.text_slot_view(TargetId(0)).unwrap().scale(),
        1.0,
        "縮退時の照会値も実適用 k（1.0）"
    );
}

/// 要件 2.4/4.1 観測完了（k のキー参加）: 同一合成入力の再 show は **キャッシュヒット**（再合成
/// しない）が、窓 DPI が変われば k が変わって**必ずミス**し、新しい k で再サンプルされる。
///
/// ヒットの判定は間接推測ではなく**改竄プローブ**で行う: 表示成立後のキャッシュスロットを同一キー
/// のまま別の絵（面 3000 由来）で上書きし、再 show の表示がその絵になるなら presenter は確かに
/// キャッシュを引いた（再合成していれば面 1000 の絵に戻る）。続けて窓 DPI を 192→96 へ変えると、
/// k が 2/1→1/1 になりキー相違でミス＝再合成されて面 1000 の等倍 golden へ戻る。
#[test]
fn same_scale_hits_cache_and_window_dpi_change_misses_and_resamples() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);

    let (emo_world, atlas, golden_1000, _golden_3000) = build_two_face_assets(6, 5);
    // 改竄プローブ用に同一 fixture を独立生成（決定論ゆえ同一資産）。
    let (probe_world, probe_atlas, _, _) = build_two_face_assets(6, 5);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let (scaled_1000, native_size, scaled_size) =
        scaled_golden(&probe_world, &probe_atlas, 1000, k2);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 1 回目（ミス→合成→k=2/1 リサンプル）。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        scaled_1000,
        "初回表示が k=2/1 のリサンプル結果と一致しない"
    );
    {
        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert!(
            t.cache
                .get(1000, &BindSet::default(), &PatternState::default(), k2)
                .is_some(),
            "導出 k がキャッシュキーへ届いていない（k=2/1 で引けない）"
        );
        assert!(
            t.cache
                .get(
                    1000,
                    &BindSet::default(),
                    &PatternState::default(),
                    ScaleRatio::ONE
                )
                .is_none(),
            "k=1/1 で引けてしまう（k がキー要素になっていない）"
        );
    }

    // 改竄プローブ: 同一キーのスロットを別の絵（面 3000 の k 適用結果）で上書きする。
    let tampered = {
        let mut composer = Composer::new();
        let native = composer
            .compose(
                &probe_world,
                &probe_atlas,
                3000,
                &BindSet::default(),
                &PatternState::default(),
            )
            .expect("面 3000 の合成は Ok");
        let mut scaled = ComposedSurface::new(0, 0);
        resample(&native, k2, &mut scaled);
        scaled
    };
    let tampered_bytes = tampered.bytes().to_vec();
    // 設計 D4 で `insert` は生成済みマスクを引数で受ける形になった（署名追随）。改竄した絵と
    // **同一 bytes 由来**のマスクを渡し、スロットの原子対をプローブ側でも崩さない。
    let tampered_mask =
        std::sync::Arc::new(wintf::ecs::widget::bitmap_source::AlphaMask::from_pbgra32(
            tampered.bytes(),
            tampered.width(),
            tampered.height(),
            tampered.stride(),
        ));
    assert_ne!(
        tampered_bytes, scaled_1000,
        "プローブ前提: 別の絵であること"
    );
    // 原寸は改竄前のエントリのものを引き継ぐ（キーも原寸も変えず**絵だけ**を差し替えるプローブ
    // である）。原寸は `CacheEntry` の中に在るため、差し替えるときも対で渡す必要がある。
    let tampered_native = presenter
        .targets
        .get(&TargetId(0))
        .unwrap()
        .cache
        .get(1000, &BindSet::default(), &PatternState::default(), k2)
        .expect("初回表示でエントリが在る")
        .native;
    presenter
        .targets
        .get_mut(&TargetId(0))
        .unwrap()
        .cache
        .insert(
            1000,
            BindSet::default(),
            PatternState::default(),
            k2,
            tampered,
            tampered_mask,
            tampered_native,
        );

    // 2 回目（同一入力・同一 k）: ヒットゆえ再合成せず、改竄された絵がそのまま表示される。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        tampered_bytes,
        "同一入力・同一 k の再 show でキャッシュを引いていない（無駄な再合成）"
    );

    // 窓 DPI 変化（192→96）: k=1/1 へ変わりキー相違＝必ずミス→再合成→等倍 golden へ戻る。
    set_window_dpi(&mut world, window, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb, golden_1000,
        "窓 DPI 変化後も旧 k の絵が出ている（k がキーに参加していない）"
    );
    assert_eq!(
        rb.len(),
        (native_size.0 * native_size.1 * 4) as usize,
        "k=1/1 の表示寸は native 原寸"
    );
    assert_ne!(
        scaled_size, native_size,
        "前提: 2 水準の物理寸は異なる（要件 2.2）"
    );

    let t = presenter.targets.get(&TargetId(0)).unwrap();
    assert_eq!(
        t.applied,
        Some(ScaleRatio::ONE),
        "照会値が新 k へ追随していない"
    );
    assert_eq!(
        t.native_size,
        Some(native_size),
        "native 原寸は k に依らず不変"
    );
    // 容量 3（要件 7.1）では旧 k のエントリも表に残る。**正しさはキー完全一致だけに依っている**
    // ——上の照会値・表示バイトが新 k のものであることが、旧 k の絵が載っていないことの証拠である。
    // 残った旧 k のエントリが**旧 k の絵のまま**であること（新 k で上書きされていないこと）を見る。
    assert_eq!(
        t.cache
            .get(1000, &BindSet::default(), &PatternState::default(), k2)
            .map(|e| (e.composed.width(), e.composed.height())),
        Some(scaled_size),
        "旧 k のエントリは旧 k の表示寸のまま残る（k をまたいで上書きしない・設計 D6）"
    );
}

/// surface 1000（`w1×h1`）と surface 3000（`w2×h2`）＝**native 原寸が互いに異なる** 2 面を
/// 同一 world へ載せた `(EmoWorld, AtlasTable)`。
///
/// `build_two_face_assets` は同寸 2 面（供給面リサイズ経路を踏まない檻）だが、こちらは
/// 「照会契約の native 原寸が**表示中の面**を指しているか」を弁別するために寸法を変えてある
/// （同寸では取り違えが観測できない）。両面とも α=255 ゆえトリムは全域を残し、合成外形は宣言どおり。
fn build_two_sized_face_assets(w1: u32, h1: u32, w2: u32, h2: u32) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![
        surface(1000, vec![elem("p.png", 0, 0)]),
        surface(3000, vec![elem("q.png", 0, 0)]),
    ];

    let gradient = |w: u32, h: u32, salt: u8| -> Vec<u8> {
        let mut img: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.extend_from_slice(&[b, g, r, 0xFF]);
            }
        }
        img
    };

    let mut dec = MemoryDecoder::new();
    dec.insert(
        base.join("p.png"),
        w1,
        h1,
        w1 * 4,
        gradient(w1, h1, 0x21),
        true,
    );
    dec.insert(
        base.join("q.png"),
        w2,
        h2,
        w2 * 4,
        gradient(w2, h2, 0x5C),
        true,
    );

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
    (world, baked.table)
}

/// 要件 1.2/4.4 観測完了（**insert 済みのまま失敗 → 後からヒットで成立**した表示でも照会契約が
/// 正しい）: 供給面生成に失敗した初回 show は `Err` を返すが、その回の合成結果は既にキャッシュへ
/// 入っている。資源が復旧した後の再 show は**キャッシュヒット**（＝今回は合成しない）でありながら
/// 表示が成立する——このとき native 原寸を供給できなければ、確立済みの表示に対して
/// `text_slot_view` が永続的に `None` を返してしまう。
///
/// 「合成した回だけ `native_size` を書く」実装ではここが RED になる（`native_size` が `None` のまま）。
/// [`CacheEntry::native`]（エントリと同じ入れ物に在る原寸）を表示成立点で**無条件に**写す実装
/// だけが緑になる。
///
/// [`CacheEntry::native`]: crate::cache::CacheEntry::native
///
/// device 失敗は `WucGraphicsResource` を**一時的に外す**ことで再現する（2 個目の Compositor を
/// 生成しない＝要件 5.3 の AV 非再導入を守る）。
#[test]
fn native_size_recovers_when_failed_show_is_followed_by_cache_hit() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x84);
    let k2 = ScaleRatio::new(2, 1).unwrap();

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 供給面生成の前提資源を一時退避（合成→insert の**後**で失敗する経路へ入る）。
    let wuc = world
        .remove_resource::<WucGraphicsResource>()
        .expect("前提: make_world_with_gpu が WucGraphicsResource を載せている");

    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    let outcome = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("reply（供給面生成失敗）を受信できない");
    assert!(
        matches!(
            outcome,
            Err(PresentError::Device {
                context: "WucGraphicsResource::compositor",
                ..
            })
        ),
        "供給面生成の前提資源が無ければ Device エラー: {outcome:?}"
    );

    {
        let t = presenter.targets.get(&TargetId(0)).unwrap();
        let entry = t
            .cache
            .get(1000, &BindSet::default(), &PatternState::default(), k2)
            .expect("失敗前に insert 済み＝次回の同一入力は必ずキャッシュヒットになる（本テストの前提）");
        assert_eq!(
            entry.native,
            (4, 3),
            "native 原寸は絵・マスクと同じエントリへ束ねて控えられている（要件 7.1・容量 3）"
        );
        assert_eq!(t.applied, None, "表示は成立していない（R4.4: 前値のまま）");
        assert_eq!(t.native_size, None, "表示未成立ゆえ照会値も未確定");
        assert!(
            presenter.text_slot_view(TargetId(0)).is_none(),
            "表示未成立の間は照会不可"
        );
    }

    // 資源を戻して同一入力を再 show（＝キャッシュヒット経由で表示が成立する）。
    world.insert_resource(wuc);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("ヒット経由で成立した表示でも照会可能でなければならない（欠陥の RED 点）");
    assert_eq!(
        view.surface_size(),
        (4, 3),
        "ヒット経由の成立でも native 原寸が正しく供給される"
    );
    assert_eq!(view.scale(), 2.0, "実適用 k は 2.0");

    let t = presenter.targets.get(&TargetId(0)).unwrap();
    assert_eq!(t.native_size, Some((4, 3)));
    assert_eq!(
        k2.scaled_extent(4, 3),
        t.chain
            .as_ref()
            .expect("表示成立後は供給面が生成済み")
            .size(),
        "物理寸 = scaled_extent(applied, native_size) の契約が回復後も成立する"
    );
}

/// 要件 1.2 観測完了（照会 native 原寸は**表示中の面**を指す）: native 原寸の異なる 2 面を切り替え
/// ながら表示すると、`surface_size()` は常に**いま画面に出ている面**の原寸を返し、
/// `scaled_extent(scale(), surface_size()) == 供給面寸` が各時点で成立する。
///
/// 3 回目は 2 回目と同一入力＝**キャッシュヒット**であり、ヒット回でも照会値が前の面へ巻き戻ったり
/// 失われたりしないことを固定する（`native_size` を「合成した回だけ書く」実装が生む取り違えの檻）。
/// 同寸 fixture では取り違えが観測できないため、寸法の異なる 2 面を専用に用意している。
#[test]
fn native_size_tracks_displayed_surface_across_size_changing_switch() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas) = build_two_sized_face_assets(6, 5, 4, 3);
    let k2 = ScaleRatio::new(2, 1).unwrap();

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 3 回目は 2 回目と同一入力＝キャッシュヒット（ヒット回の照会値を固定する）。
    for (step, (surface_id, native)) in [(1000u32, (6u32, 5u32)), (3000, (4, 3)), (3000, (4, 3))]
        .into_iter()
        .enumerate()
    {
        show_ok(&mut presenter, &mut world, TargetId(0), surface_id);

        let view = presenter
            .text_slot_view(TargetId(0))
            .expect("表示成立後は照会可能");
        assert_eq!(
            view.surface_size(),
            native,
            "step {step}: surface_size() が表示中の面（{surface_id}）の native 原寸を指していない"
        );
        assert_eq!(view.scale(), 2.0, "step {step}: 実適用 k");

        let chain_size = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.chain.as_ref())
            .expect("表示成立後は供給面が生成済み")
            .size();
        assert_eq!(
            k2.scaled_extent(native.0, native.1),
            chain_size,
            "step {step}: 物理寸 = scaled_extent(scale(), surface_size()) が成立しない"
        );
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(surface_id),
            "step {step}: 現サーフェス id"
        );
    }
}
