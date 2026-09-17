use super::*;

use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::BindSet;
use areka_parsers::shell::{Animation, AppendTarget, DrawMethod, Interval, Pattern, Surface};

use wintf::ecs::widget::bitmap_source::AlphaMask;
use wintf::ecs::{Arrangement, GraphicsCommandList};

use super::test_support::{
    build_target_assets, elem, make_world_with_gpu, native_golden, native_golden_with,
    pattern_overlay_at, set_window_dpi, shell_of, show_ok, spawn_window_with_dpi, surface,
};

// ── task 6.3: 端数 k（5/4）の表示の作り方・αマスクの由来・縮小方向の自動追従 ───────────────
// 既存の k≠1 檻は **k=2/1**（整数倍・端数丸めが発火しない）か、k=7/6 の照会 API 群である。
// ここで足すのは (A) 端数を伴う k での**配置（原寸＋係数）と照会物理寸**、(B) **αマスクの由来**、
// (C) **縮小方向**の `refresh_scale`——の 3 点。
//
// **表示の作り方が変わった（`areka-P0-present-gpu-transform-scale` 裁定 D）**: 絵もマスクも常に
// native 原寸で、拡大は wintf の描画経路（`render_surface` の `SetTransform`）が `Arrangement.scale`
// ＝k で掛ける。ゆえに (A) は「配置の寸＝原寸・係数＝k・照会値＝丸め権威」の形、(B) は
// 「マスクは**原寸バイト**由来」（旧 canon の正反対）、(C) は「縮小では係数と照会値だけが変わり、
// 絵・マスク・表示記録は据え置かれる」の形になる。k≠1 の画素そのものは GPU 補間の出力ゆえ
// 決定論 golden に取らない（要件 6.6・見た目は実機 2 水準サインオフが担う）。

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

/// surface entity の `Arrangement` を `(寸, 係数)` で読む（**論理寸＝原寸・係数＝拡大率 k**）。
///
/// 物理寸そのもの（`GlobalArrangement.bounds`）は wintf の伝播段（`PostLayout`）が導くので、
/// 本ファイルのテストは走らせない。丸めの一致は `display_tests.rs` の T-N3 が表で固定する。
fn arrangement_of(world: &World, surface_entity: Entity) -> ((u32, u32), (f32, f32)) {
    let arr = world
        .get::<Arrangement>(surface_entity)
        .expect("surface entity に Arrangement が無い");
    (
        (arr.size.width as u32, arr.size.height as u32),
        (arr.scale.x, arr.scale.y),
    )
}

/// surface 1000 ＝ **α が画素ごとに変わる** `w×h` element の `(EmoWorld, AtlasTable)`。
///
/// α は市松に `0xFF`（マスク hit）と `0x20`（閾値 128 未満＝非 hit）を置く。**α=0 を含まない**ため
/// atlas の α=0 除外トリムは全域を残し、合成外形は正確に `w×h` である。色は α を掛けた
/// premultiplied 値で焼く（`B,G,R ≤ A` の不変条件を崩さない）。
///
/// 全不透明の `build_target_assets` では αマスクが**全ビット 1 の一様マスク**になり、
/// 「マスク内容が表示に渡した原寸バイト由来か」の検査が空虚になる（寸法しか弁別できない）。
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

/// タスク 6.3 の名指し受け入れ基準・要件 2.1/2.5/3.1/3.2 観測完了（**端数を伴う k=5/4 の表示の
/// 作り方**）: 窓 `DPI`=120（125%）・author_dpi=96 で `ShowSurface` を適用すると——(a) 配置の寸が
/// native 原寸・係数が 1.25、(b) `read_back` が **native 合成**の独立再現と全バイト一致（k に
/// 依らない）、(c) αマスクが **原寸**で供給される、(d) 照会物理寸が `scaled_extent(5/4, native)`、
/// (e) 窓寸 reconcile 要求も同じ物理寸で積まれる。
///
/// # なぜ k=2/1 の既存檻に加えて 5/4 が要るのか
///
/// k=2/1 は**整数倍**ゆえ `scaled_extent` の丸めが一度も発火しない。native 6×5 に 5/4 を掛けると
/// `7.5 → 8`・`6.25 → 6` で**両軸とも端数**になり、丸め規約（round half away from zero）を
/// 切り捨て実装（`7`）から数値で弁別できる。実機の常用水準（125%）そのものでもある
/// （Implementation Notes 4.3 の実測 `k_shell_ratio=ScaleRatio{num:5,den:4}`）。
#[test]
fn show_surface_keeps_native_display_and_mask_and_scales_layout_at_k_five_quarters() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 120);

    let (emo_world, atlas, _golden) = build_target_assets(6, 5, 0x71);
    // 同一入力を独立に再現して原寸 golden を作る（presenter の内部値の追認ではない）。
    let (probe_world, probe_atlas, _) = build_target_assets(6, 5, 0x71);
    let k54 = ScaleRatio::new(5, 4).unwrap();
    let (golden, native_size) = native_golden(&probe_world, &probe_atlas, 1000);
    assert_eq!(native_size, (6, 5), "fixture の native 原寸");
    let physical = k54.scaled_extent(6, 5);
    assert_eq!(
        physical,
        (8, 6),
        "6×5/4=7.5→8・5×5/4=6.25→6（両軸とも端数・切り捨て実装なら 7×6 になる）"
    );
    assert_ne!(
        physical, native_size,
        "前提: k≠1 ゆえ原寸と物理寸は弁別可能"
    );

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    // (a) 配置＝原寸の寸 × 端数を伴う係数（拡大は wintf の変換行列が掛ける）。
    let surface_entity = surface_entity_of(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (1.25, 1.25)),
        "配置が「原寸 6×5・係数 1.25」になっていない（端数 k が表示へ届いていない）"
    );

    // (b) 読み戻しは原寸の合成バイトそのもの（k に依らない・要件 6.2）。
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

    // (c) αマスクは **原寸**で供給される（k 適用後の寸で作る旧 canon はここで落ちる・要件 4.1）。
    assert_eq!(
        mask_dims(&world, surface_entity),
        Some(native_size),
        "αマスク寸が原寸でない（k 倍バイト由来のマスクを載せている）"
    );
    assert_ne!(
        mask_dims(&world, surface_entity),
        Some(physical),
        "前提: k≠1 ゆえ原寸と物理寸は弁別可能（マスクが物理寸なら旧 canon）"
    );

    // (d)(e) 照会契約・窓寸 reconcile 要求は丸め権威の物理寸。
    assert_eq!(presenter.applied_scale(TargetId(0)), Some(1.25));
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(physical),
        "照会物理寸が丸め権威と乖離している"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(physical),
        "初回表示が k 倍後の物理寸を報告していない"
    );
}

/// 要件 2.1/2.5/4.1 観測完了（**αマスクは原寸バイト由来**）: α が画素ごとに変わる surface を
/// k=5/4 で表示すると、表示器へ供給される `AlphaMask` は——(a) 寸法が **native 原寸**（k 適用後の
/// 物理寸ではない）、(b) **全ビットが「原寸の合成バイト列から独立に組んだマスク」と一致**する。
///
/// # 旧 canon の正反対である
///
/// かつてここは「マスクは k 適用後バイト由来」を固定していた。裁定 D
/// （`areka-P0-present-gpu-transform-scale`）でマスクは原寸に戻り、÷k は wintf の `alpha_mask_hit`
/// が物理寸の境界に対する比例写像として 1 回だけ掛ける（要件 4.2）。ゆえに主張も名も反転した。
///
/// # なぜ寸法だけでは足りないのか
///
/// `build_target_assets` は α=255 一様ゆえ、そこから作るマスクは**全ビット 1**である。寸法しか
/// 弁別できず、「別解像度のバイトから作ったマスクを原寸へ縮めた」ような内容の誤りが素通りする。
/// 本テストは α に 0xFF（hit）と 0x20（閾値 128 未満＝非 hit）を市松に置き、hit/非 hit が
/// **両方存在すること**を前提として明示検査したうえでビット全走査する。
///
/// マスクの**座標契約**（点÷k・ヒット規約）は本 spec の領分ではない（R7.9・W5
/// `areka-P0-collision-dpi-hittest`）。ここで固定するのは「表示に渡した原寸バイトと同一・同寸の
/// マスクが供給される」という emo-present 側の生成契約だけである。
#[test]
fn alpha_mask_bits_come_from_native_bytes() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 120);

    let (emo_world, atlas) = build_alpha_varying_assets(8, 6, 0x72);
    let (probe_world, probe_atlas) = build_alpha_varying_assets(8, 6, 0x72);
    let k54 = ScaleRatio::new(5, 4).unwrap();
    let (native_bytes, native_size) = native_golden(&probe_world, &probe_atlas, 1000);
    assert_eq!(native_size, (8, 6), "前提: α≠0 ゆえトリムは全域を残す");
    let physical = k54.scaled_extent(8, 6);
    assert_eq!(
        physical,
        (10, 8),
        "8×5/4=10・6×5/4=7.5→8（高さは端数・丸め権威）"
    );

    // 表示に渡されるはずの原寸バイト列から独立にマスクを組む（presenter の内部値の追認ではない）。
    let expected = AlphaMask::from_pbgra32(
        &native_bytes,
        native_size.0,
        native_size.1,
        native_size.0 * 4,
    );
    // 非空虚性の前提: hit と非 hit が両方在る（全ビット 1 のマスクでは内容比較が空虚になる）。
    let mut hits = 0usize;
    let mut misses = 0usize;
    for y in 0..native_size.1 {
        for x in 0..native_size.0 {
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

    // 前提: 表示に渡ったバイトが原寸 golden そのもの（マスクの由来と同一の bytes）。
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        native_bytes,
        "読み戻しが原寸の合成バイトと一致しない"
    );

    let surface_entity = surface_entity_of(&presenter, TargetId(0));
    let mask_res = world
        .get::<AlphaMaskResource>(surface_entity)
        .expect("surface entity に AlphaMaskResource が無い");
    let mask = mask_res.mask().expect("表示成立後は αマスクが供給済み");

    // (a) 寸法は native 原寸。物理寸（10×8）と弁別できる。
    assert_eq!(
        (mask.width(), mask.height()),
        native_size,
        "αマスク寸が原寸でない"
    );
    assert_ne!(
        (mask.width(), mask.height()),
        physical,
        "αマスクが k 適用後の物理寸で作られている（旧 canon）"
    );

    // (b) 全ビット一致（別解像度のバイト由来・引き伸ばし／縮めをここで弾く）。
    for y in 0..native_size.1 {
        for x in 0..native_size.0 {
            assert_eq!(
                mask.is_hit(x, y),
                expected.is_hit(x, y),
                "αマスク ({x},{y}) のビットが原寸の合成バイト由来でない"
            );
        }
    }

    // 配置と照会値は端数 k のまま（マスクが原寸でも表示の大きさは k 倍で決まる）。
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (1.25, 1.25)),
        "配置が「原寸 8×6・係数 1.25」になっていない"
    );
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(physical),
        "照会物理寸が scaled_extent(5/4, native) と一致しない"
    );
}

/// タスク 6.3 の名指し受け入れ基準・要件 4.1/4.2/5.3 観測完了（**DPI 差替 → `refresh_scale` は
/// 係数だけを変える**）: k=2/1 で表示を確立したのち窓 `DPI` を 192→120（k=2/1→5/4）へ差し替えて
/// `refresh_scale` を呼ぶと、**変わるのは配置の係数と照会物理寸・報告値だけ**で、絵・αマスク・
/// 表示記録は原寸のまま据え置かれる（k はキーに参加しないゆえ再合成もしない）。
///
/// # 既存 `refresh_scale_after_dpi_change_reapplies_new_k` との差
///
/// 既存檻は **1/1 → 2/1（拡大方向・整数倍）** のみで、観測は戻り値・照会値・読み戻しに閉じている。
/// 本テストは (1) **縮小方向**、(2) **端数を伴う遷移先 k**、(3) `refresh_scale` 経由でも
/// **配置の係数が追従し、かつマスクと表示記録が作り直されないこと**——を足す。
///
/// 係数の更新を落とす変異（`set_layout` を再表示経路で呼ばない）は本テストが落とす——初回表示では
/// `VisualMount::attach` が配置を組むため、`set_layout` が load-bearing なのは再表示経路だけである。
#[test]
fn refresh_scale_to_smaller_k_changes_only_the_scale_factor() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 192);
    let (emo_world, atlas, _golden) = build_target_assets(6, 5, 0x73);
    let (probe_world, probe_atlas, _) = build_target_assets(6, 5, 0x73);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let k54 = ScaleRatio::new(5, 4).unwrap();
    let (golden, native_size) = native_golden(&probe_world, &probe_atlas, 1000);
    assert_eq!(native_size, (6, 5));
    let grown = k2.scaled_extent(6, 5);
    let shrunk = k54.scaled_extent(6, 5);
    assert_eq!(grown, (12, 10), "前提: k=2/1 の物理寸");
    assert_eq!(
        shrunk,
        (8, 6),
        "前提: k=5/4 の物理寸（両軸とも端数・遷移先が遷移元より小さい）"
    );
    assert!(
        shrunk.0 < grown.0 && shrunk.1 < grown.1,
        "前提: 縮小方向の遷移"
    );

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");

    show_ok(&mut presenter, &mut world, TargetId(0), 1000);
    let surface_entity = surface_entity_of(&presenter, TargetId(0));
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (2.0, 2.0)),
        "前提: k=2/1 の表示が確立している"
    );
    assert_eq!(
        presenter.take_pending_resize(TargetId(0)),
        Some(grown),
        "前提: 初回表示の要求を取り出しておく"
    );
    // 遷移前の絵・マスク寸・表示記録（据え置きの照合基準）。
    let display_before = world
        .get::<GraphicsCommandList>(surface_entity)
        .cloned()
        .expect("表示成立後は表示記録が載っている");
    assert_eq!(
        mask_dims(&world, surface_entity),
        Some(native_size),
        "前提: マスクは原寸で供給されている"
    );

    // モニタ跨ぎ移動（200% → 125%）の決定論的代替。
    set_window_dpi(&mut world, window, 120);

    assert_eq!(
        presenter.refresh_scale(&mut world, TargetId(0)),
        Some(shrunk),
        "縮小方向の DPI 変化で新物理寸が返らない（再導出・再表示が走っていない）"
    );

    // 変わるのは配置の係数だけ（寸は原寸のまま）。
    assert_eq!(
        arrangement_of(&world, surface_entity),
        (native_size, (1.25, 1.25)),
        "refresh_scale 後の配置の係数が旧 k のまま（余白が残る）"
    );

    // 絵・マスク・表示記録は据え置き（k はキーに参加せず再合成も再記録も起きない・要件 5.3）。
    assert_eq!(
        presenter.read_back(TargetId(0)).expect("read_back 失敗"),
        golden,
        "k 変化で絵が作り直されている（原寸の読み戻しは k に依らない）"
    );
    assert_eq!(
        mask_dims(&world, surface_entity),
        Some(native_size),
        "k 変化でマスクの寸が動いている（マスクは原寸で不変）"
    );
    assert_eq!(
        world.get::<GraphicsCommandList>(surface_entity),
        Some(&display_before),
        "k が変わっただけで表示記録を作り直している（記録は k を含まない）"
    );

    // 照会契約と drain 契約。
    assert_eq!(presenter.applied_scale(TargetId(0)), Some(1.25));
    assert_eq!(presenter.target_physical_size(TargetId(0)), Some(shrunk));
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
// 同一の見た目関係を保つ」（要件 2.3）は *3 層を 1 枚へ合成 → その 1 枚に係数 1 対を掛ける* という
// 構造からの帰結であって、**一度も観測されていなかった**。実 emo2 ゴーストの表情は bind part の重ねで作られる
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
/// 層のどれかが k 経路で落ちる実装では原寸の合成結果そのものが変わるため、下のテストの
/// **原寸 golden 突合**とバイトが一致しない。
/// 両 part とも base 内（`(2,3)+(6,4)=(8,7)`・`(5,5)+(6,4)=(11,9)` ≤ `(16,12)`）
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
/// k≠1 で表示すると——(a) 配置の寸が native 原寸・**係数が 1 対**（x と y が同一の k）、
/// (b) `read_back` が **同一 `(binds, pattern)` で合成した native** の独立再現と全バイト一致し、
/// (c) 表示記録が**エントリの 1 枚**（原寸の面を丸ごと描く命令）だけ載る。
///
/// (a)＋(c) が「単一の拡大率が 1 枚に掛かる」の新しい形である——層ごとに k を掛ける実装なら
/// 係数も命令も層の数だけ要るのに対し、ここには係数 1 対と命令 1 枚しか存在しない。
///
/// # なぜ既存 k≠1 檻では足りないのか
///
/// 既存の k≠1 檻は全て単一 element の fixture を駆動する。単一 element では「層が k 経路で
/// 落ちた」ことが観測できない。実 emo2 ゴーストの表情は bind part の重ねで構成されるので、
/// 未観測の構成が本番の構成そのものだった。(b) の非空虚性は下の「層なし／bind のみ／
/// pattern のみ」との弁別で明示検査する。
///
/// # k≠1 の画素そのものは檻に入れない（要件 6.6）
///
/// かつてここには「k 適用後の part 内部画素が native の対応画素と厳密同値」という座標突合が
/// あった。拡大が GPU の変換行列へ移った今、k 適用後の画素は wintf の `render_surface` が描く
/// 出力であり決定論 golden の対象外である（見た目は実機 2 水準サインオフが担う）。層が k 経路で
/// 落ちる変異は (b) が原寸のまま捕まえる——`apply_show` が k≠1 のとき `binds`／`pattern` を既定へ
/// 落とせば、原寸の読み戻しが「層なし」の golden と一致してしまい (b) が赤になる。
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
    let (native_bytes, native_size) =
        native_golden_with(&probe_world, &probe_atlas, 1000, &binds, &pattern);
    assert_eq!(
        native_size,
        (16, 12),
        "前提: 2 part とも base 内ゆえ合成外形は base 原寸（外形変化ではなく中身を観測する）"
    );
    let physical = k32.scaled_extent(16, 12);
    assert_eq!(physical, (24, 18), "丸め権威が返す k=3/2 の物理寸");
    assert_ne!(
        physical, native_size,
        "前提: k≠1 ゆえ原寸と物理寸は弁別可能"
    );

    // 前提（層の非空虚性）: 3 層の golden は「層なし」「bind のみ」「pattern のみ」と全て区別できる。
    // ここが縮退すると、presenter が層を握り潰しても (b) がすり抜けてしまう。
    let plain = native_golden_with(
        &probe_world,
        &probe_atlas,
        1000,
        &BindSet::default(),
        &PatternState::default(),
    )
    .0;
    let bind_only = native_golden_with(
        &probe_world,
        &probe_atlas,
        1000,
        &binds,
        &PatternState::default(),
    )
    .0;
    let pattern_only = native_golden_with(
        &probe_world,
        &probe_atlas,
        1000,
        &BindSet::default(),
        &pattern,
    )
    .0;
    for (label, other) in [
        ("層なし", &plain),
        ("bind 層のみ", &bind_only),
        ("pattern 層のみ", &pattern_only),
    ] {
        assert_ne!(
            &native_bytes, other,
            "fixture 前提: 3 層 golden が「{label}」と区別できなければ層の檻にならない"
        );
    }

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

    // (a) 配置＝原寸の寸 × **1 対の**係数。x と y に別々の倍率が入る余地が無いことを明示する。
    let surface_entity = surface_entity_of(&presenter, TargetId(0));
    let (arr_size, arr_scale) = arrangement_of(&world, surface_entity);
    assert_eq!(arr_size, native_size, "配置の寸は 3 層合成の原寸");
    assert_eq!(
        arr_scale,
        (1.5, 1.5),
        "配置の係数が k（3/2）の 1 対になっていない"
    );
    assert_eq!(
        arr_scale.0, arr_scale.1,
        "単一の拡大率＝x と y が同一の係数（層ごと・軸ごとに別の倍率が入っていない）"
    );
    assert_eq!(
        presenter.target_physical_size(TargetId(0)),
        Some(physical),
        "照会物理寸が scaled_extent(3/2, native) と一致しない"
    );

    // (b) 読み戻しが「3 層を合成した native」と全バイト一致（層の一部が k 経路で落ちていない）。
    let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert_eq!(
        rb.len(),
        (native_size.0 * native_size.1 * 4) as usize,
        "読み戻しの画素数が原寸と一致しない（CPU 拡大の経路が残っている）"
    );
    assert_eq!(
        rb, native_bytes,
        "読み戻しが 3 層合成の独立再現と一致しない（層の一部が k 経路で落ちた）"
    );

    // (c) 表示記録は**エントリの 1 枚**そのもの（層の数だけ命令を積む形になっていない）。
    let t = presenter.targets.get(&TargetId(0)).unwrap();
    let entry = t
        .cache
        .get(1000, &binds, &pattern)
        .expect("表示成立後は同一入力でエントリが引ける");
    assert!(
        entry.display.command_list().is_some(),
        "エントリの表示記録が空（原寸の面を描く命令が記録されていない）"
    );
    assert_eq!(
        world.get::<GraphicsCommandList>(surface_entity),
        Some(&entry.display),
        "surface entity に載る表示記録がエントリの 1 枚と別物（記録が層ごとに作られている）"
    );

    // 照会契約（native 原寸・実適用 k）も 3 層構成で成立する。
    assert_eq!(t.applied, Some(k32), "applied が実適用 k と一致しない");
    assert_eq!(
        t.native_size,
        Some(native_size),
        "native_size は k 適用前の原寸"
    );
}
