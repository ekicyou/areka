use super::*;

use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::BindSet;
use areka_parsers::shell::{Animation, AppendTarget, DrawMethod, Interval, Pattern, Surface};

use wintf::ecs::Arrangement;
use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::test_support::{
    ScaledGolden, build_target_assets, elem, make_world_with_gpu, pattern_overlay_at, px_at,
    scaled_golden, scaled_golden_with, set_window_dpi, shell_of, show_ok, spawn_window_with_dpi,
    surface,
};

// ── task 6.3: 端数 k（5/4）の実表示・αマスクの k 寸/内容・縮小方向の自動追従 ───────────────
// 既存の k≠1 檻は **k=2/1**（整数倍・端数丸めが発火しない）か、k=7/6 の照会 API 群
// （`physical_size`／`target_physical_size`。これらは `chain.size()` との一致まで見るので
// 供給面寸は無檻ではない——ただし **readback バイト**は見ていない）である。ここで足すのは
// (A) 端数を伴う k での**実表示バイト＋供給面寸＋visual bounds**、
// (B) **αマスクが k 適用後バイト由来**であること（寸だけでなくビット内容）、
// (C) **縮小方向**の `refresh_scale`——の 3 点。なお (C) の `ResizeBuffers` 縮み追従自体は
// 既存 2 本と共倒れで、本テストの排他キルは**再表示経路のマスク寸・visual bounds 追従**にある。

/// target の surface entity（表示器＝visual/αマスク/bounds の宿主）を取り出す。
fn surface_entity_of(presenter: &EmoPresenter, target: TargetId) -> Entity {
    presenter
        .targets
        .get(&target)
        .and_then(|t| t.mount.as_ref())
        .expect("表示成立後は mount が生成済み")
        .surface_entity()
}

/// surface entity に供給済みの αマスク寸（未供給なら `None`）。
fn mask_dims(world: &World, surface_entity: Entity) -> Option<(u32, u32)> {
    world
        .get::<AlphaMaskResource>(surface_entity)
        .and_then(|r| r.mask().map(|m| (m.width(), m.height())))
}

/// surface entity の `Arrangement` 寸（＝visual bounds・物理 px で直接設定される）。
fn arrangement_size(world: &World, surface_entity: Entity) -> Option<(u32, u32)> {
    world
        .get::<Arrangement>(surface_entity)
        .map(|a| (a.size.width as u32, a.size.height as u32))
}

/// surface 1000 ＝ **α が画素ごとに変わる** `w×h` element の `(EmoWorld, AtlasTable)`。
///
/// α は市松に `0xFF`（マスク hit）と `0x20`（閾値 128 未満＝非 hit）を置く。**α=0 を含まない**ため
/// atlas の α=0 除外トリムは全域を残し、合成外形は正確に `w×h` である。色は α を掛けた
/// premultiplied 値で焼く（`B,G,R ≤ A` の不変条件を崩さない）。
///
/// 全不透明の `build_target_assets` では αマスクが**全ビット 1 の一様マスク**になり、
/// 「マスク内容が k 適用後バイト由来か」の検査が空虚になる（寸法しか弁別できない）。
fn build_alpha_varying_assets(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![surface(1000, vec![elem("p.png", 0, 0)])];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let a: u8 = if (x + y) % 2 == 0 { 0xFF } else { 0x20 };
            let pm = |c: u8| ((c as u16 * a as u16) / 255) as u8;
            img.push(pm((x as u8).wrapping_mul(3).wrapping_add(salt)));
            img.push(pm((y as u8).wrapping_mul(5).wrapping_add(salt)));
            img.push(pm(((x + y) as u8).wrapping_mul(7).wrapping_add(salt)));
            img.push(a);
        }
    }
    dec.insert(base.join("p.png"), w, h, stride, img, true);

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

/// タスク 6.3 の名指し受け入れ基準・要件 2.1/2.5/3.1/3.2 観測完了（**端数を伴う k=5/4 の実拡大表示**）:
/// 窓 `DPI`=120（125%）・author_dpi=96 で `ShowSurface` を適用すると——(a) 供給面寸が
/// `scaled_extent(5/4, native)`、(b) `read_back` が **native 合成 → `resample(5/4)`** の独立再現と
/// 全バイト一致、(c) αマスク寸が k 適用後の物理寸、(d) visual bounds（`Arrangement`）も同寸、
/// (e) 窓寸 reconcile 要求も同寸で積まれる。
///
/// # なぜ k=2/1 の既存檻に加えて 5/4 が要るのか
///
/// k=2/1 は**整数倍**ゆえ `scaled_extent` の丸めが一度も発火しない。native 6×5 に 5/4 を掛けると
/// `7.5 → 8`・`6.25 → 6` で**両軸とも端数**になり、丸め規約（round half away from zero）を
/// 切り捨て実装（`7`）から数値で弁別できる。実機の常用水準（125%）そのものでもある
/// （Implementation Notes 4.3 の実測 `k_shell_ratio=ScaleRatio{num:5,den:4}`）。
#[test]
fn show_surface_scales_display_mask_and_bounds_at_k_five_quarters() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 120);

    let (emo_world, atlas, native_golden) = build_target_assets(6, 5, 0x71);
    // 同一入力を独立に再現して k 適用後の golden を作る（presenter の内部値の追認ではない）。
    let (probe_world, probe_atlas, _) = build_target_assets(6, 5, 0x71);
    let k54 = ScaleRatio::new(5, 4).unwrap();
    let (scaled_bytes, native_size, scaled_size) =
        scaled_golden(&probe_world, &probe_atlas, 1000, k54);
    assert_eq!(native_size, (6, 5), "fixture の native 原寸");
    assert_eq!(
        scaled_size,
        k54.scaled_extent(6, 5),
        "golden の外形は丸め権威 scaled_extent に従う"
    );
    assert_eq!(
        scaled_size,
        (8, 6),
        "6×5/4=7.5→8・5×5/4=6.25→6（両軸とも端数・切り捨て実装なら 7×6 になる）"
    );

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    // (a) 供給面寸＝k 倍後の物理寸。
    let chain_size = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.chain.as_ref())
        .expect("表示成立後は供給面が生成済み")
        .size();
    assert_eq!(
        chain_size, scaled_size,
        "供給面寸が scaled_extent(5/4, native) と一致しない（端数 k が表示へ届いていない）"
    );

    // (b) 表示バイトそのものが native→resample(5/4) の独立再現と一致（寸だけ合わせた偽物を弾く）。
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb.len(),
        (scaled_size.0 * scaled_size.1 * 4) as usize,
        "readback の画素数が k 倍後の寸と一致しない"
    );
    assert_eq!(
        rb, scaled_bytes,
        "表示バイトが native 合成の 5/4 リサンプル結果と一致しない"
    );
    assert_ne!(
        rb, native_golden,
        "前提: k=5/4 と等倍は弁別可能（native のまま表示していれば同値）"
    );

    // (c) αマスクは k 適用後の物理寸で供給される（native 寸のマスクを載せていれば落ちる）。
    let surface_entity = surface_entity_of(&presenter, TargetId(0));
    assert_eq!(
        mask_dims(&world, surface_entity),
        Some(scaled_size),
        "αマスク寸が k 適用後の物理寸でない（native 寸のマスクが表示器へ載っている）"
    );
    assert_ne!(
        mask_dims(&world, surface_entity),
        Some(native_size),
        "前提: k≠1 ゆえ native 寸とマスク寸は弁別可能"
    );

    // (d) 合成先 visual の bounds も同寸（R3.2・見切れ／余白を作らない）。
    //     **初回表示では `VisualMount::attach` が k 適用後の外形で `Arrangement` を組む**ため、
    //     ここは契約の明文化であって `set_bounds` 欠落変異の排他キルではない（その変異を殺すのは
    //     再表示側の `refresh_scale_shrinks_display_mask_and_bounds_to_smaller_k`）。
    assert_eq!(
        arrangement_size(&world, surface_entity),
        Some(scaled_size),
        "visual bounds（Arrangement）が k 倍後の表示寸へ整合していない"
    );

    // (e) 照会契約・窓寸 reconcile 要求も同一の物理寸。
    assert_eq!(presenter.applied_scale(TargetId(0)), Some(1.25));
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(scaled_size),
        "照会物理寸が供給面寸と乖離している"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(scaled_size),
        "初回表示が k 倍後の物理寸を報告していない"
    );
}

/// 要件 2.1/2.5 観測完了（**αマスクは k 適用後バイト由来**）: α が画素ごとに変わる surface を
/// k=5/4 で表示すると、表示器へ供給される `AlphaMask` は——(a) 寸法が k 適用後の物理寸、
/// (b) **全ビットが「実際に表示されたバイト列から独立に組んだマスク」と一致**する。
///
/// # なぜ寸法だけでは足りないのか
///
/// `build_target_assets` は α=255 一様ゆえ、そこから作るマスクは**全ビット 1**である。寸法しか
/// 弁別できず、「k 適用前バイトを k 適用後の寸へ引き伸ばして作ったマスク」のような内容の誤りが
/// 素通りする。本テストは α に 0xFF（hit）と 0x20（閾値 128 未満＝非 hit）を市松に置き、
/// hit/非 hit が**両方存在すること**を前提として明示検査したうえでビット全走査する。
///
/// マスクの**座標契約**（点÷k・ヒット規約）は本 spec の領分ではない（R7.9・W5
/// `areka-P0-collision-dpi-hittest`）。ここで固定するのは「表示バッファと同一 bytes・同一寸の
/// マスクが供給される」という emo-present 側の生成契約だけである。
///
/// 実測の変異キル: 寸は正しいまま**内容だけ**を表示バイト由来でなくする変異（全画素 α=255 で
/// マスクを組む）は**本テストのみ**が落とす（他 89 本は全生存）——既存 fixture はすべて α=255
/// 一様ゆえ、そのマスクは元から全ビット 1 で当該変異と観測上区別できないからである。
#[test]
fn alpha_mask_bits_come_from_k_scaled_display_bytes() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 120);

    let (emo_world, atlas) = build_alpha_varying_assets(8, 6, 0x72);
    let (probe_world, probe_atlas) = build_alpha_varying_assets(8, 6, 0x72);
    let k54 = ScaleRatio::new(5, 4).unwrap();
    let (scaled_bytes, native_size, scaled_size) =
        scaled_golden(&probe_world, &probe_atlas, 1000, k54);
    assert_eq!(native_size, (8, 6), "前提: α≠0 ゆえトリムは全域を残す");
    assert_eq!(
        scaled_size,
        (10, 8),
        "8×5/4=10・6×5/4=7.5→8（高さは端数・丸め権威）"
    );

    // 表示されるはずのバイト列から独立にマスクを組む（presenter の内部値の追認ではない）。
    let expected = AlphaMask::from_pbgra32(
        &scaled_bytes,
        scaled_size.0,
        scaled_size.1,
        scaled_size.0 * 4,
    );
    // 非空虚性の前提: hit と非 hit が両方在る（全ビット 1 のマスクでは内容比較が空虚になる）。
    let mut hits = 0usize;
    let mut misses = 0usize;
    for y in 0..scaled_size.1 {
        for x in 0..scaled_size.0 {
            if expected.is_hit(x, y) {
                hits += 1;
            } else {
                misses += 1;
            }
        }
    }
    assert!(
        hits > 0 && misses > 0,
        "fixture 前提が崩れた: 期待マスクが一様（hit={hits} miss={misses}）＝内容比較が空虚"
    );

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    // 前提: 画面に載ったバイトが k 適用後 golden そのもの（マスクの由来と同一の bytes）。
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        scaled_bytes,
        "表示バイトが k=5/4 のリサンプル結果と一致しない"
    );

    let surface_entity = surface_entity_of(&presenter, TargetId(0));
    let mask_res = world
        .get::<AlphaMaskResource>(surface_entity)
        .expect("surface entity に AlphaMaskResource が無い");
    let mask = mask_res.mask().expect("表示成立後は αマスクが供給済み");

    // (a) 寸法が k 適用後の物理寸。
    assert_eq!(
        (mask.width(), mask.height()),
        scaled_size,
        "αマスク寸が k 適用後の物理寸でない"
    );

    // (b) 全ビット一致（k 適用前バイト由来・別解像度からの引き伸ばしをここで弾く）。
    for y in 0..scaled_size.1 {
        for x in 0..scaled_size.0 {
            assert_eq!(
                mask.is_hit(x, y),
                expected.is_hit(x, y),
                "αマスク ({x},{y}) のビットが k 適用後の表示バイト由来でない"
            );
        }
    }
}

/// タスク 6.3 の名指し受け入れ基準・要件 4.1/4.2 観測完了（**DPI 差替 → `refresh_scale` の縮小追従**）:
/// k=2/1 で表示を確立したのち窓 `DPI` を 192→120（k=2/1→5/4）へ差し替えて `refresh_scale` を呼ぶと、
/// 供給面が `ResizeBuffers` で**小さい物理寸へ**自動追従し、表示・αマスク・visual bounds・照会値・
/// 報告値がすべて新 k で揃う。
///
/// # 既存 `refresh_scale_after_dpi_change_reapplies_new_k` との差
///
/// 既存檻は **1/1 → 2/1（拡大方向・整数倍）** のみで、しかも観測は戻り値・照会値・readback バイトに
/// 閉じている。本テストは (1) **縮小方向**（`ResizeBuffers` が縮む側・source_tex/staging の再作成寸が
/// 縮む側）、(2) **端数を伴う遷移先 k**、(3) `refresh_scale` 経由でも **αマスクと visual bounds が
/// 追従すること**——を足す。
///
/// 実測の変異キル: `set_bounds` を落とす変異は**本テストのみ**が落とす（他 89 本は全生存）——
/// 初回表示では `VisualMount::attach` が bounds を組むため、`set_bounds` が load-bearing なのは
/// 再表示経路だけだからである。`ResizeBuffers` を拡大方向のみへ落とす変異では本テストと既存 2 本
/// （`same_scale_hits_cache_and_window_dpi_change_misses_and_resamples`・
/// `native_size_tracks_displayed_surface_across_size_changing_switch`）が共倒れする。
#[test]
fn refresh_scale_shrinks_display_mask_and_bounds_to_smaller_k() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _native_golden) = build_target_assets(6, 5, 0x73);
    let (probe_world, probe_atlas, _) = build_target_assets(6, 5, 0x73);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let k54 = ScaleRatio::new(5, 4).unwrap();
    let (grown_bytes, native_size, grown_size) =
        scaled_golden(&probe_world, &probe_atlas, 1000, k2);
    let (shrunk_bytes, _, shrunk_size) = scaled_golden(&probe_world, &probe_atlas, 1000, k54);
    assert_eq!(native_size, (6, 5));
    assert_eq!(grown_size, (12, 10), "前提: k=2/1 の物理寸");
    assert_eq!(
        shrunk_size,
        (8, 6),
        "前提: k=5/4 の物理寸（両軸とも端数・遷移先が遷移元より小さい）"
    );
    assert!(
        shrunk_size.0 < grown_size.0 && shrunk_size.1 < grown_size.1,
        "前提: 縮小方向の遷移（ResizeBuffers の縮み追従を踏む）"
    );

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        grown_bytes,
        "前提: k=2/1 の表示が確立している"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(grown_size),
        "前提: 初回表示の要求を取り出しておく"
    );

    // モニタ跨ぎ移動（200% → 125%）の決定論的代替。
    set_window_dpi(&mut world, window, 120);

    assert_eq!(
        presenter.refresh_scale(&mut world, TargetId(0)),
        Some(shrunk_size),
        "縮小方向の DPI 変化で新物理寸が返らない（再導出・再表示が走っていない）"
    );

    // 供給面が縮み側へ追従（ResizeBuffers ＋ source_tex/staging 再作成）。
    let chain_size = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.chain.as_ref())
        .expect("表示成立後は供給面が生成済み")
        .size();
    assert_eq!(
        chain_size, shrunk_size,
        "供給面が縮み側の新物理寸へ追従していない"
    );

    // 画面へ載った画素が新 k のリサンプル結果（旧 k の絵が残っていない）。
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb, shrunk_bytes,
        "表示バイトが k=5/4 のリサンプル結果と一致しない"
    );
    assert_ne!(rb, grown_bytes, "前提: 2 水準の絵は弁別可能");

    // αマスク・visual bounds も新 k へ追従（表示バッファだけ更新する実装をここで弾く）。
    let surface_entity = surface_entity_of(&presenter, TargetId(0));
    assert_eq!(
        mask_dims(&world, surface_entity),
        Some(shrunk_size),
        "refresh_scale 後の αマスクが旧 k の寸のまま（表示だけ更新している）"
    );
    assert_eq!(
        arrangement_size(&world, surface_entity),
        Some(shrunk_size),
        "refresh_scale 後の visual bounds が旧 k の寸のまま（余白が残る）"
    );

    // 照会契約と drain 契約。
    assert_eq!(presenter.applied_scale(TargetId(0)), Some(1.25));
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(shrunk_size)
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        None,
        "refresh_scale が返した要求が drain 側にも残っている（同一フレームで二重 resize になる）"
    );
}

// ── 要件 2.3（多層コンテンツの単一 k 一貫拡大）の実表示檻 ──────────────────────────────
//
// 既存の k≠1 檻は全て**単一 element** の fixture を駆動しており、「ベース surface・SERIKO アニメ
// パターン・mayuna 着せ替えパーツを単一の k で一貫拡大し、要素間の相対配置・重なりが等倍時と
// 同一の見た目関係を保つ」（要件 2.3）は *compose → 1 回だけ resample* という構造からの帰結で
// あって、**一度も観測されていなかった**。実 emo2 ゴーストの表情は全て bind part の重ねで作られる
// ため、未観測の構成こそが本番の構成である。以下の fixture／テストがその空白を閉じる。

/// bind 層 part の重ね位置（base 左上からの非対称オフセット）。
const LAYERED_BIND_AT: (i64, i64) = (2, 3);
/// pattern 層 part（SERIKO 現在コマ相当）の重ね位置（bind 層と**重なる**非対称オフセット）。
const LAYERED_PATTERN_AT: (i64, i64) = (5, 5);
/// 両 part 共通の原寸（`6×4`）。base（`16×12`）内に収まるため合成外形は base 原寸のまま。
const LAYERED_PART_SIZE: (u32, u32) = (6, 4);

/// surface 1000 に **3 層**（ベース element ＋ bind animation 2000 の重ね part ＋ `PatternState` が
/// 運ぶ現在コマ part）を**非対称位置・相互重なり**で載せた `(EmoWorld, AtlasTable)`。
///
/// - ベース: `p.png`（`w×h` 全不透明・座標由来グラデーション）を (0,0)。
/// - bind 層: animation 2000（`Interval::Bind`）の pattern0 が surface 5000（`q.png` 単色）を
///   [`LAYERED_BIND_AT`] へ overlay する。`BindSet::from_ids([2000])` で有効化される
///   （mayuna 着せ替えパーツ相当）。
/// - pattern 層: `PatternState` が animation 3000 の現在コマとして surface 6000（`r.png` 単色・
///   bind 層と異色）を [`LAYERED_PATTERN_AT`] へ overlay する（SERIKO アニメパターン相当）。
///   surface 6000 は 1000 の animation ではないため定義層（extent 母集合）に寄与しない。
///
/// 2 part は互いに重なり（native x∈[5,8)・y∈[5,7)）、かつ base 左上に対して非対称に置かれる。
/// 要素ごとに k を掛けてから合成する実装（＝要件 2.3 が禁じる形）では、各 part の拡大と
/// 非対称オフセットの丸めが独立に動くため、**合成後に 1 回だけ resample した** golden とは
/// バイトが一致しない。両 part とも base 内（`(2,3)+(6,4)=(8,7)`・`(5,5)+(6,4)=(11,9)` ≤ `(16,12)`）
/// ゆえ合成外形は base の `w×h` のまま——外形変化ではなく**中身の相対配置**だけを観測できる。
fn build_layered_assets(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let (pw, ph) = LAYERED_PART_SIZE;
    let base_surface = Surface {
        id: 1000,
        targets: vec![AppendTarget::Single(1000)],
        elements: vec![elem("p.png", 0, 0)],
        collisions: Vec::new(),
        animations: vec![Animation {
            id: 2000,
            interval: Interval::Bind,
            patterns: vec![Pattern {
                index: 0,
                method: DrawMethod::new("overlay".to_string()),
                surface_id: 5000,
                wait: 0,
                x: LAYERED_BIND_AT.0,
                y: LAYERED_BIND_AT.1,
            }],
        }],
    };
    let surfaces = vec![
        base_surface,
        surface(5000, vec![elem("q.png", 0, 0)]),
        surface(6000, vec![elem("r.png", 0, 0)]),
    ];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            img.extend_from_slice(&[b, g, r, 0xFF]);
        }
    }
    dec.insert(base.join("p.png"), w, h, stride, img, true);
    // 2 part は単色不透明で互いに異色（重なり順と相対配置を画素で弁別できる）。α=255 ゆえ
    // premultiplied 不変条件は自明に成立する。
    let solid = |bgr: [u8; 3]| {
        let mut v = Vec::with_capacity((pw * ph * 4) as usize);
        for _ in 0..(pw * ph) {
            v.extend_from_slice(&[bgr[0], bgr[1], bgr[2], 0xFF]);
        }
        v
    };
    dec.insert(
        base.join("q.png"),
        pw,
        ph,
        pw * 4,
        solid([0x11, 0x99, 0x22]),
        true,
    );
    dec.insert(
        base.join("r.png"),
        pw,
        ph,
        pw * 4,
        solid([0xEE, 0x33, 0xCC]),
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

/// 要件 2.3 観測完了（**多層コンテンツの単一 k**・k=3/2）: ベース surface ＋ mayuna 着せ替え相当の
/// bind part ＋ SERIKO アニメパターン相当の現在コマ part を**非対称・相互重なり**で載せた面を
/// k≠1 で表示すると——(a) 供給面寸が `scaled_extent(3/2, native)`、(b) `read_back` バイトが
/// **同一 `(binds, pattern)` で合成した native → `resample(3/2)`** の独立再現と全バイト一致し、
/// (c) k 適用後の各 part 画素が **native の対応画素と厳密に同値**（＝相対配置・重なりが等倍時と
/// 同じ関係で保たれている）。
///
/// # なぜ既存 k≠1 檻では足りないのか
///
/// 既存の k≠1 檻は全て単一 element の fixture を駆動する。単一 element では「要素ごとに k を
/// 掛けてから合成」と「合成してから 1 回 k を掛ける」が同じ絵になり得るため、要件 2.3 の
/// **層をまたぐ**主張は一度も観測されない。実 emo2 ゴーストの表情は bind part の重ねで構成される
/// ので、未観測の構成が本番の構成そのものだった。
///
/// # (c) の座標算術（`resample` の有理逆写像から導く固定値）
///
/// `resample` は画素中心写像 `src = (d + 1/2)·den/num − 1/2` の bilinear（エッジクランプ）。
/// k=3/2 では出力 d=5 → src=3.1667（隣接入力 {3,4}）・d=10 → src=6.5（隣接入力 {6,7}）。
/// - 出力 (5,5) の入力足跡 {3,4}×{3,4} は **bind part 単独**領域（bind: x∈[2,8) y∈[3,7)・
///   pattern: x∈[5,11) y∈[5,9)）に完全に収まる → 4 サンプルが同値ゆえ結果は native (3,3) と厳密同値。
/// - 出力 (10,10) の入力足跡 {6,7}×{6,7} は **pattern part** 領域に完全に収まる → native (6,6) と同値。
///
/// part ごとに k を掛けてから重ねる実装では part の拡大寸と非対称オフセットの丸めが独立に動くため、
/// この 2 点の色は隣接層・ベースの色へずれる。
///
/// # (b) と (c) は独立したオラクルである
///
/// (b) の golden は presenter と同じ `compose → resample` を辿るため、**`resample` 自身の
/// 幾何が壊れる変異には共倒れで盲目**である。(c) は k 適用後の画素を `resample` を通さない
/// **native の画素**と突き合わせるため、その盲点を埋める（下の実測がそれを示す）。
///
/// # 実測の変異キル（2026-07-26・本ワークツリー）
///
/// - `apply_show` が k≠1 のとき `binds`／`pattern` を既定へ落とす変異（＝層が k 経路で消える）:
///   `-p areka-emo-present` 91 本中**本テストのみ**が落ちる（他 90 本生存）。`-p areka` でも
///   `spine_dpi_change_during_live_seriko_loop_keeps_loop_progressing`（同時追加の spine 檻）以外は
///   全生存——**本テスト追加前は、この変異を落とす檻が repo 内に 1 本も無かった**。
/// - `scale.rs` の `AxisWalk::new` で画素中心写像の初期分子を `den - num` → `den + num` へずらす
///   幾何変異: `-p areka-emo-present` 91 本中**本テストのみ**が落ち、しかも落ちるのは **(c)** の
///   座標突合である（(b) は golden も同じ変異を通るため生存）。同変異は `-p areka-emo-compose` の
///   `resample` golden 6 本とは**共倒れ**（shared）——ただし emo-present 側で唯一検出できるのは本テスト。
#[test]
fn show_surface_scales_layered_bind_and_pattern_content_with_single_k() {
    let mut world = make_world_with_gpu();
    // 窓 DPI 144 / author_dpi 96 → k=3/2（150%・実機水準・両軸とも端数を伴う倍率）。
    let window = spawn_window_with_dpi(&mut world, 144);
    let k32 = ScaleRatio::new(3, 2).unwrap();

    let binds = BindSet::from_ids([2000]);
    let pattern = pattern_overlay_at(3000, 6000, LAYERED_PATTERN_AT.0, LAYERED_PATTERN_AT.1);

    let (emo_world, atlas) = build_layered_assets(16, 12, 0x4D);
    // 同一入力を独立に再現して golden を作る（presenter の内部値の追認ではない）。
    let (probe_world, probe_atlas) = build_layered_assets(16, 12, 0x4D);
    let ScaledGolden {
        scaled: scaled_bytes,
        native: native_bytes,
        native_size,
        scaled_size,
    } = scaled_golden_with(&probe_world, &probe_atlas, 1000, &binds, &pattern, k32);
    assert_eq!(
        native_size,
        (16, 12),
        "前提: 2 part とも base 内ゆえ合成外形は base 原寸（外形変化ではなく中身を観測する）"
    );
    assert_eq!(
        scaled_size,
        k32.scaled_extent(16, 12),
        "golden の外形は丸め権威 scaled_extent に従う"
    );
    assert_eq!(scaled_size, (24, 18));

    // 前提（層の非空虚性）: k≠1 の golden は「層なし」「bind のみ」「pattern のみ」と全て区別できる。
    // ここが縮退すると、presenter が k≠1 で層を握り潰しても (b) がすり抜けてしまう。
    let plain = scaled_golden_with(
        &probe_world,
        &probe_atlas,
        1000,
        &BindSet::default(),
        &PatternState::default(),
        k32,
    )
    .scaled;
    let bind_only = scaled_golden_with(
        &probe_world,
        &probe_atlas,
        1000,
        &binds,
        &PatternState::default(),
        k32,
    )
    .scaled;
    let pattern_only = scaled_golden_with(
        &probe_world,
        &probe_atlas,
        1000,
        &BindSet::default(),
        &pattern,
        k32,
    )
    .scaled;
    for (label, other) in [
        ("層なし", &plain),
        ("bind 層のみ", &bind_only),
        ("pattern 層のみ", &pattern_only),
    ] {
        assert_ne!(
            &scaled_bytes, other,
            "fixture 前提: k≠1 の 3 層 golden が「{label}」と区別できなければ層の檻にならない"
        );
    }

    // 前提（座標突合の非空虚性）: bind part／pattern part／ベースの 3 点が互いに異色。
    let bind_px = px_at(&native_bytes, 16, 3, 3);
    let pattern_px = px_at(&native_bytes, 16, 6, 6);
    let base_px = px_at(&native_bytes, 16, 13, 10);
    assert_ne!(
        bind_px, pattern_px,
        "前提: bind part と pattern part は異色"
    );
    assert_ne!(bind_px, base_px, "前提: bind part とベースは異色");
    assert_ne!(pattern_px, base_px, "前提: pattern part とベースは異色");

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        &mut world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds: binds.clone(),
            pattern: pattern.clone(),
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "3 層 ShowSurface（k=3/2）が Ok でない"
    );

    // (a) 供給面寸＝k 倍後の物理寸。
    let chain_size = presenter
        .targets
        .get(&TargetId(0))
        .and_then(|t| t.chain.as_ref())
        .expect("表示成立後は供給面が生成済み")
        .size();
    assert_eq!(
        chain_size, scaled_size,
        "供給面寸が scaled_extent(3/2, native) と一致しない"
    );

    // (b) 表示バイトが「3 層を合成した native → resample(3/2)」と全バイト一致。
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb.len(),
        (scaled_size.0 * scaled_size.1 * 4) as usize,
        "readback の画素数が k 倍後の寸と一致しない"
    );
    assert_eq!(
        rb, scaled_bytes,
        "k≠1 の表示バイトが 3 層合成 → 単一 resample の独立再現と一致しない（層の一部が k 経路で落ちた／層ごとに k が掛かった）"
    );

    // (c) 相対配置・重なりの座標突合: k 適用後の part 内部画素が native の対応画素と厳密同値。
    assert_eq!(
        px_at(&rb, scaled_size.0, 5, 5),
        bind_px,
        "k=3/2 表示の (5,5) が bind part の色でない（bind 層の相対配置が k 適用でずれている）"
    );
    assert_eq!(
        px_at(&rb, scaled_size.0, 10, 10),
        pattern_px,
        "k=3/2 表示の (10,10) が pattern part の色でない（pattern 層の相対配置・重なり順が k 適用でずれている）"
    );

    // 照会契約（native 原寸・実適用 k）も 3 層構成で成立する。
    let t = presenter.targets.get(&TargetId(0)).unwrap();
    assert_eq!(t.applied, Some(k32), "applied が実適用 k と一致しない");
    assert_eq!(
        t.native_size,
        Some(native_size),
        "native_size は k 適用前の原寸"
    );
}
