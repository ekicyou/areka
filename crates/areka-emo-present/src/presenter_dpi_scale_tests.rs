use super::*;

use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::BindSet;

use wintf::ecs::{Arrangement, GraphicsCommandList};

use super::test_support::{
    build_target_assets, build_two_face_assets, elem, make_world_with_gpu, mount_entities,
    native_golden, set_window_dpi, shell_of, show_ok, spawn_window_with_dpi, surface,
};

// ── DPI 追従（k 適用の単一漏斗）: タスク 3.2／3.3 の檻 ────────────────────────────────────
// k は「target ごとの政策（author_dpi）× 窓ごとの実 DPI」から **show 適用ごと**に導出される。
// 檻は (a) 政策が窓単位で保たれること、(b) 導出 k が実際に**配置の係数**として表示へ届くこと、
// (c) k が引き当てに関与しないこと（k だけの変化はヒット）、(d) DPI 不在が縮退分岐として独立に
// 成立すること。
//
// **表示の作り方が変わった（`areka-P0-present-gpu-transform-scale` 裁定 D）**: 提示段が抱える絵は
// 常に native 原寸で、拡大は wintf の描画経路（`render_surface` の `SetTransform`）が
// `Arrangement.scale`＝k で掛ける。ゆえに本ファイルの期待値は「配置の寸＝原寸・配置の係数＝k・
// 照会値＝丸め権威 `scaled_extent`」の 3 点で書く。物理寸そのもの（`GlobalArrangement.bounds`）は
// wintf の伝播段（`PostLayout`）が導くため
// 本ファイルでは走らせず、丸めの一致は `display_tests.rs` の T-N3 が表で固定する。

/// surface entity の `Arrangement` を `(寸, 係数)` で読む（**論理寸＝原寸・係数＝拡大率 k**）。
fn arrangement_of(world: &World, surface_entity: Entity) -> ((u32, u32), (f32, f32)) {
    let arr = world
        .get::<Arrangement>(surface_entity)
        .expect("surface entity に Arrangement が無い");
    (
        (arr.size.width as u32, arr.size.height as u32),
        (arr.scale.x, arr.scale.y),
    )
}

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

/// タスク 3.3 の名指し受け入れ基準・要件 2.1/2.2 観測完了（k=2/1 の表示の作り方）: 窓 `DPI`=192・
/// author_dpi=96（k=2/1）でキャッシュミスの `ShowSurface` を適用すると——(a) 配置の寸が **native
/// 原寸**・係数が **k**、(b) 照会物理寸が丸め権威 `scaled_extent(2/1, native)` と一致し native とは
/// 弁別でき、(c) `read_back` が **native 合成**の独立再現と全バイト一致し画素数も原寸のまま、
/// (d) 表示記録（描画命令）が 1 枚だけ載る。
///
/// k が表示へ届いていない実装（係数 1.0 のまま）なら (a) の係数と (b) の照会値が同時に落ちる。
/// 逆に CPU で画素を拡大する実装が戻れば (c) の画素数で落ちる（要件 1.1・6.2）。
#[test]
fn show_surface_scales_display_to_scaled_extent_at_k2() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);

    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x81);
    // 同一入力を独立に再現して原寸 golden を作る（presenter の内部値の追認ではない）。
    let (probe_world, probe_atlas, _) = build_target_assets(3, 2, 0x81);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let (golden, native_size) = native_golden(&probe_world, &probe_atlas, 1000);
    assert_eq!(native_size, (3, 2), "fixture の native 原寸");
    let physical = k2.scaled_extent(3, 2);
    assert_eq!(physical, (6, 4), "丸め権威が返す k=2/1 の物理寸");
    assert_ne!(
        physical, native_size,
        "前提: k≠1 ゆえ原寸と物理寸は弁別可能（要件 2.2）"
    );

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    // (a) 配置＝原寸の寸 × k の係数（拡大は wintf の変換行列が掛ける）。
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (2.0, 2.0)),
        "配置が「原寸の寸・k の係数」になっていない（k が表示へ届いていない）"
    );

    // (b) 照会物理寸は丸め権威の式ひとつで出る（消費点＝窓寸 reconcile が見る唯一の値）。
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(physical),
        "照会物理寸が scaled_extent(k, native) と一致しない"
    );

    // (c) 読み戻しは **原寸の合成バイトそのもの**（k に依らない・要件 6.2）。
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb.len(),
        (native_size.0 * native_size.1 * 4) as usize,
        "読み戻しの画素数が原寸と一致しない（CPU 拡大の経路が残っている）"
    );
    assert_eq!(
        rb, golden,
        "読み戻しが native 合成の独立再現と一致しない（k が画素へ混ざっている）"
    );

    // (d) 表示記録は 1 枚（原寸 bitmap を丸ごと描く命令）。k は記録に入らない。
    assert!(
        world
            .get::<GraphicsCommandList>(surface_entity)
            .and_then(|l| l.command_list().cloned())
            .is_some(),
        "surface entity に表示記録（描画命令）が載っていない"
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
/// （配置の係数を掛けた後の物理寸ではない）。物理寸との関係は
/// `scaled_extent(scale(), surface_size()) == target_physical_size()` として成立する。
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
        "surface_size() は native 原寸（k を掛けた後の物理寸ではない）"
    );

    // 契約式: 物理寸 == scaled_extent(k, native)。照会口と実際の配置の両方で裏取りする。
    let k = ScaleRatio::new(2, 1).unwrap();
    let physical = presenter
        .target_physical_size(TargetId(0))
        .expect("表示確立後は Some");
    assert_eq!(
        k.scaled_extent(view.surface_size().0, view.surface_size().1),
        physical,
        "物理寸 = scaled_extent(scale(), surface_size()) の契約が成立しない"
    );
    assert_ne!(
        view.surface_size(),
        physical,
        "k≠1 では native 原寸と物理寸が一致しない（原寸をそのまま物理寸として返していれば同値になる）"
    );
    // 実表示の作り方（配置の寸＝原寸・係数＝k）とも一致する——照会値だけが正しくて画面が
    // 原寸のまま、という食い違いをここで弾く。
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (view.surface_size(), (view.scale(), view.scale())),
        "照会値（原寸・k）と実際の配置が食い違う"
    );
}

/// 要件 3.1（物理寸の照会契約・丸め権威の単一化）: `physical_size()` は
/// `scaled_extent(applied, surface_size())` と厳密に一致し、実際の配置（原寸・係数 k）とも整合する。
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

    // (b) 実表示の作り方（配置の寸＝原寸・係数＝k）と整合する（照会値＝実表示の担保・要件 4.2）。
    //     物理寸そのもの（`GlobalArrangement.bounds`）との丸めの一致は display_tests.rs の T-N3。
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        ((27, 27), (k.as_f32(), k.as_f32())),
        "配置が「原寸 27×27・係数 7/6」になっていない（照会値と実表示が食い違う）"
    );
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(view.physical_size()),
        "2 つの物理寸照会口が食い違う"
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
/// 丸め権威 `scaled_extent` を通した物理寸を返し、`TextSlotView::physical_size()` とも実際の配置
/// （原寸の寸・k の係数）とも整合する。未登録・表示成立前は `None`。
///
/// k は `TextSlotView` 側の檻と同じ **7/6**（窓 DPI 112 ／ author_dpi 96）× native 27px を使う——
/// `as_f32()` 経由の掛け算（31）と権威（32）が**数値で弁別**できる唯一種の k であり、
/// 分母が 2 冪の k（0.25 刻み）だけを見る檻では両実装の差が構造的に観測できないため。
#[test]
fn target_physical_size_uses_rounding_authority_and_matches_view_and_arrangement() {
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

    // (c) 実際の配置とも整合する（照会値＝実表示・要件 4.2）。表示は「原寸の絵 × k の係数」で
    //     作られるので、照会物理寸はその 2 つを丸め権威に通した値でなければならない。
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    let (arr_size, arr_scale) = arrangement_of(&world, surface_entity);
    assert_eq!(arr_size, (27, 27), "配置の寸は原寸");
    assert_eq!(arr_scale, (k.as_f32(), k.as_f32()), "配置の係数は k");
    assert_eq!(
        physical,
        k.scaled_extent(arr_size.0, arr_size.1),
        "物理寸が実際の配置（原寸 × k）から導けない"
    );

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
/// 適用 k・照会物理寸（`scaled_extent(1/2, (4,3)) = (2,2)`）・配置の係数（0.5）・`scale()` の
/// 4 つがすべて外れる。したがって本テストの緑は「縮退分岐を通った」ことに帰属する。
///
/// **読み戻しは弁別に使えない**（`areka-P0-present-gpu-transform-scale` 裁定 D）——提示段が抱える
/// 絵は k に依らず原寸だからである。ゆえに捏造の弁別は上の 4 つが担い、読み戻しは「縮退しても
/// 絵は等倍の native 合成そのもの」という別の主張として残す。`scale()` は 1.0 を返す。
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

    let (emo_world, atlas, native_bytes) = build_target_assets(4, 3, 0x83);

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
        native_bytes,
        "縮退の表示は native 合成と全バイト一致"
    );
    assert_eq!(
        presenter.text_slot_view(TargetId(0)).unwrap().scale(),
        1.0,
        "縮退時の照会値も実適用 k（1.0）"
    );
    // 捏造の弁別（doc §帰属可能性）: 96 捏造なら k=1/2 ゆえ照会物理寸が 2×2・配置の係数が 0.5。
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some((4, 3)),
        "縮退の物理寸は原寸そのもの（96 捏造なら 1/2 で 2×2 になる）"
    );
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        ((4, 3), (1.0, 1.0)),
        "縮退の配置は原寸・係数 1.0（96 捏造なら係数 0.5 になる）"
    );
}

/// 要件 5.2/5.3 観測完了（**k はキーに参加しない**）: 同一合成入力の再 show はキャッシュヒットで
/// あり、**窓 DPI が変わって k だけが変わった適用もヒットする**（再合成しない）。k 変化で変わるのは
/// 配置の係数と照会物理寸だけで、絵・表示記録は原寸のまま据え置かれる。
///
/// ヒットの判定は間接推測ではなく**改竄プローブ**で行う: 表示成立後のキャッシュスロットを同一キー
/// のまま別の絵（面 3000 の native 合成）で上書きし、再 show の表示がその絵になるなら presenter は
/// 確かにキャッシュを引いた（再合成していれば面 1000 の絵に戻る）。続けて窓 DPI を 192→96 へ変えても
/// **改竄した絵のまま**であることが「k 変化がミスを起こさない」ことの直接の証拠である——k がキー
/// 要素だった旧仕様（完了 spec `emo-dpi-scaling` 設計 D6）なら、ここで面 1000 の絵へ戻っていた。
#[test]
fn same_scale_hits_cache_and_window_dpi_change_still_hits() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);

    let (emo_world, atlas, golden_1000, _golden_3000) = build_two_face_assets(6, 5);
    // 改竄プローブ用に同一 fixture を独立生成（決定論ゆえ同一資産）。
    let (probe_world, probe_atlas, _, _) = build_two_face_assets(6, 5);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let native_size = (6u32, 5u32);
    assert_ne!(
        k2.scaled_extent(native_size.0, native_size.1),
        native_size,
        "前提: 2 水準（192／96 dpi）の物理寸は異なる（要件 2.2）"
    );

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 1 回目（ミス→合成）。絵は原寸のまま・k は配置の係数として載る。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden_1000,
        "初回表示が面 1000 の native 合成と一致しない"
    );
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (2.0, 2.0)),
        "初回の配置が「原寸・係数 2.0」になっていない"
    );
    // 表示記録は k を含まない 1 枚。k 変化の前後で**同一の記録**が載り続けることを末尾で見る。
    let display_before = world
        .get::<GraphicsCommandList>(surface_entity)
        .cloned()
        .expect("表示成立後は表示記録が載っている");
    {
        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert!(
            t.cache
                .get(1000, &BindSet::default(), &PatternState::default())
                .is_some(),
            "合成入力（id＋binds＋pattern）でエントリが引けない"
        );
    }

    // 改竄プローブ: 同一キーのスロットの**絵とマスクだけ**を別の絵（面 3000 の native 合成）へ
    // 差し替える。表示記録は差し替えない——記録を据え置くことで、末尾の「記録が作り直されて
    // いない」主張が同一 COM ポインタの比較として成立する。
    let tampered = {
        let mut composer = Composer::new();
        composer
            .compose(
                &probe_world,
                &probe_atlas,
                3000,
                &BindSet::default(),
                &PatternState::default(),
            )
            .expect("面 3000 の合成は Ok")
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
        tampered_bytes, golden_1000,
        "プローブ前提: 別の絵であること"
    );
    presenter
        .targets
        .get_mut(&TargetId(0))
        .unwrap()
        .cache
        .insert(
            1000,
            BindSet::default(),
            PatternState::default(),
            tampered,
            tampered_mask,
            display_before.clone(),
        );

    // 2 回目（同一入力・同一 k）: ヒットゆえ再合成せず、改竄された絵がそのまま表示される。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        tampered_bytes,
        "同一入力・同一 k の再 show でキャッシュを引いていない（無駄な再合成）"
    );

    // 窓 DPI 変化（192→96）: k=2/1→1/1。**k はキーに参加しない**ゆえここでもヒットし、改竄した絵の
    // ままである（再合成していれば面 1000 の golden へ戻る）。
    set_window_dpi(&mut world, window, 96);
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        tampered_bytes,
        "k が変わっただけでミスして再合成している（k がキー要素に残っている・要件 5.2/5.3）"
    );

    // 変わるのは配置の係数と照会物理寸だけ。
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (1.0, 1.0)),
        "配置の係数が新 k（1.0）へ追随していない"
    );
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(native_size),
        "k=1/1 の物理寸は原寸そのもの"
    );
    assert_eq!(
        world.get::<GraphicsCommandList>(surface_entity),
        Some(&display_before),
        "k が変わっただけで表示記録を作り直している（記録は k を含まない・要件 1.2/5.6）"
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
    // エントリは k をまたいで 1 つのまま（k 別のエントリが生えない・要件 5.1/5.6）。改竄した絵が
    // 残っていることが「新 k で上書きされていない」ことの証拠である。
    let entry = t
        .cache
        .get(1000, &BindSet::default(), &PatternState::default())
        .expect("k が変わってもキーは同じ＝同じエントリが引ける");
    assert_eq!(
        (entry.composed.width(), entry.composed.height()),
        native_size,
        "エントリの外形は原寸（k 適用済みの面を持たない）"
    );
    assert_eq!(
        entry.composed.bytes(),
        tampered_bytes.as_slice(),
        "k 変化でエントリの絵が差し替わっている（再合成が走った）"
    );
}

/// surface 1000（`w1×h1`）と surface 3000（`w2×h2`）＝**native 原寸が互いに異なる** 2 面を
/// 同一 world へ載せた `(EmoWorld, AtlasTable)`。
///
/// `build_two_face_assets` は同寸 2 面（外形変化の経路を踏まない檻）だが、こちらは
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

/// 要件 1.2/4.4/5.3 観測完了（**キャッシュヒットで成立した表示でも照会契約が正しい**）: 別の面を
/// 挟んでから元の面へ戻ると、その回は合成せずヒットで表示が成立する。このとき native 原寸を
/// 供給できなければ、確立済みの表示に対して照会値が前の面のまま（あるいは `None`）になる。
///
/// 「合成した回だけ `native_size` を書く」実装ではここが RED になる——戻り先の面（4×3）と直前に
/// 出していた面（6×5）は**原寸が違う**ため、書き忘れは古い 6×5 として観測される。同寸の面で
/// 往復しても取り違えは観測できないので、寸法の異なる 2 面を使う。エントリと同じ入れ物に在る
/// 原寸（[`CacheEntry::composed`] の外形）を表示成立点で**無条件に**写す実装だけが緑になる。
///
/// [`CacheEntry::composed`]: crate::cache::CacheEntry::composed
///
/// # かつての形（`insert` 済みのまま失敗 → 後からヒットで成立）は経路ごと消えた
///
/// 旧実装は「合成して `insert` した後、供給面の生成で失敗する」経路を持っており、本テストは
/// `WucGraphicsResource` を一時的に外してそれを再現していた。裁定 D
/// （`areka-P0-present-gpu-transform-scale`）で失敗し得る GPU 呼び出しは**メモへの挿入より手前**
/// （表示の記録）へ集約され、装着は純 ECS（失敗経路なし）になったため、「insert 済みのまま失敗」
/// という状態は構造的に作れない。ゆえに前提を「別の面を挟んでからのヒット」へ置き換えた——
/// 固定している性質（ヒット回の照会契約）はそのまま残る。失敗時の前状態維持そのものは
/// `presenter_display_failure_tests.rs`（4 注入点）が担う。
#[test]
fn native_size_is_supplied_on_a_cache_hit_established_show() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    // 面 1000＝4×3・面 3000＝6×5（**原寸が異なる** 2 面）。容量 3 ゆえ 2 面とも表に残る。
    let (emo_world, atlas) = build_two_sized_face_assets(4, 3, 6, 5);
    let k2 = ScaleRatio::new(2, 1).unwrap();

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    // 1 回目（ミス）: 面 1000 を確立する。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    // 2 回目（ミス）: 別原寸の面 3000 を挟む——照会値をここで 6×5 へ動かしておく。
    show_ok(&mut presenter, &mut world, TargetId(0), 3000);
    assert_eq!(
        presenter.targets.get(&TargetId(0)).unwrap().native_size,
        Some((6, 5)),
        "前提: 直前の表示は別原寸の面（書き忘れがあればこの値が残る）"
    );

    // 3 回目: 面 1000 へ戻る。1 回目のエントリが表に残っている＝**合成しないヒット**である。
    {
        let t = presenter.targets.get(&TargetId(0)).unwrap();
        let entry = t
            .cache
            .get(1000, &BindSet::default(), &PatternState::default())
            .expect("容量 3 ゆえ 1 回目のエントリが残る＝次の同一入力は必ずヒットする（本テストの前提）");
        assert_eq!(
            (entry.composed.width(), entry.composed.height()),
            (4, 3),
            "native 原寸はエントリの面の外形そのもの（別フィールドで二重に持たない・要件 5.1）"
        );
    }
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let view = presenter
        .text_slot_view(TargetId(0))
        .expect("ヒット経由で成立した表示でも照会可能でなければならない（欠陥の RED 点）");
    assert_eq!(
        view.surface_size(),
        (4, 3),
        "ヒット経由の成立でも native 原寸が正しく供給される（合成した回だけ書く実装なら 6×5 が残る）"
    );
    assert_eq!(view.scale(), 2.0, "実適用 k は 2.0");

    let t = presenter.targets.get(&TargetId(0)).unwrap();
    assert_eq!(t.native_size, Some((4, 3)));
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(k2.scaled_extent(4, 3)),
        "物理寸 = scaled_extent(applied, native_size) の契約がヒット回でも成立する"
    );
    let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        ((4, 3), (2.0, 2.0)),
        "ヒット経由の成立でも配置は「原寸・係数 k」で書かれる"
    );
}

/// 要件 1.2 観測完了（照会 native 原寸は**表示中の面**を指す）: native 原寸の異なる 2 面を切り替え
/// ながら表示すると、`surface_size()` は常に**いま画面に出ている面**の原寸を返し、
/// `scaled_extent(scale(), surface_size()) == 照会物理寸` と「配置の寸＝その面の原寸」が各時点で
/// 成立する。
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

        assert_eq!(
            presenter.target_physical_size(TargetId(0)),
            Some(k2.scaled_extent(native.0, native.1)),
            "step {step}: 物理寸 = scaled_extent(scale(), surface_size()) が成立しない"
        );
        let (surface_entity, _) = mount_entities(&presenter, TargetId(0));
        assert_eq!(
            arrangement_of(&world, surface_entity),
            (native, (2.0, 2.0)),
            "step {step}: 配置の寸が表示中の面の原寸を指していない（係数は k のまま）"
        );
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(surface_id),
            "step {step}: 現サーフェス id"
        );
    }
}
