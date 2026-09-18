//! 表示スケールの扱いを固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.7**・設計 **C2** の I 行）。
//!
//! ## ここで固定するもの
//!
//! 表示スケール k を 3 通り（1.0／1.25／2.0）で与えて検体の枠の配置を作り、
//!
//! - 画像の寸法が k に依らず原寸（335×205）のままであること・保持される k が渡した値であること、
//! - 物理寸法への変換が、描画範囲の幅（遠辺 − 左辺）と左辺のそれぞれで k 倍になること、
//! - 文字の領域解決の入力が**物理寸法ではなく画像の寸法**であること
//!   （＝解いた領域が k に依らず [`super::region`] の値と同値になること）
//!
//! を固定する。
//!
//! ## 配置計算そのものはここで比べない
//!
//! `LayoutEngine::layout` は画像座標のまま解くので k を引数に取らない。したがって
//! 「k ごとに解き直して同じ結果になる」は入力が同じである以上**必ず真**であり、
//! 固定しても何も守らない。同じ理由で `image_size` の保持だけを見ても足りない
//! （渡した値を素通しで持つ実装では常に真になる）。実質の守りは
//!
//! - 変換が k 倍になる側（[`physical_conversion_scales_with_the_display_scale`]）と、
//! - 領域が k に依らない側（[`text_region_resolves_from_the_image_size_not_the_physical_surface`]）
//!
//! が担う。後者が空振りでないこと——すなわち物理寸法を渡せば答えが変わること——は
//! 対照 [`resolving_from_the_physical_surface_moves_the_far_edges`] が示す。
//! **k=1.0 では物理寸法と画像の寸法が一致する**ので、k=1.0 だけでは対照が成立しない。
//! そのため 3 通りには k≠1 を必ず含め、含まれていること自体を先に確かめる。
//!
//! ## 決定論
//!
//! ファイル読み込みと純粋層の解決のみ。実 GPU・実窓・DirectWrite を要さず、
//! 同一入力に対して常に同一の結果を返す。

use areka_emo_text::actor::{ResolvedBalloonText, TextSlotBinding};
use areka_emo_text::region::{ImagePx, ScaleContract};
use bevy_ecs::prelude::World;

use super::test_support::{
    EXPECTED_LEFT, EXPECTED_RIGHT, EXPECTED_TOP, expected_bottom, scope_image_size, staysee_model,
};

// ── 観測する表示スケール（設計 C2 の I 行）──────────────────────────────────────

/// 表示スケール k の 3 通り（設計 C2 の I 行が定める値。既存の検体の観測と同じ 3 値）。
const SCALES: [f32; 3] = [1.0, 1.25, 2.0];

/// 踏むべき表示スケールの本数。ループが痩せて 0 件のまま緑になる形を塞ぐ。
const EXPECTED_SCALE_COUNT: usize = SCALES.len();

/// 本ファイルが観測する scope（設計 C2 の I 行の入力は本体側の面 0＝335×205）。
const SCOPE: u32 = 0;

/// 描画範囲の幅（遠辺 309 − 左辺 22 ＝ 287・image px）。
///
/// [`super::region`] が固定している 4 辺から導く（値を書き写すと、片方だけ直したときに
/// 同一性が黙って壊れる）。
const EXPECTED_INLINE_EXTENT: f32 = EXPECTED_RIGHT - EXPECTED_LEFT;

// ── 土台 ─────────────────────────────────────────────────────────────────────

/// 画像の原寸 `image` を k で物理化した供給面の寸（`物理 = ceil(画像 × k)`）。
///
/// 上流（emo-present／配置層）が同じバルーンを k 別に物理化した状況の再現である。
fn physical_surface(image: (u32, u32), k: f32) -> (u32, u32) {
    (
        (image.0 as f32 * k).ceil() as u32,
        (image.1 as f32 * k).ceil() as u32,
    )
}

/// 検体の面 0 を表示スケール k で配置したときの枠の配置を、本番と同じ構築口で組む。
///
/// slot／窓の entity の値は本ファイルの関心外なので `World` から採番するだけである。
fn staysee_binding(world: &mut World, k: f32) -> TextSlotBinding {
    let image = scope_image_size(SCOPE);
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();
    TextSlotBinding::new(slot, window, k, physical_surface(image, k), image)
}

/// 3 通りの表示スケールに k≠1 が含まれ、本数が期待どおりであることを先に確かめる。
///
/// k=1.0 では物理寸法と画像の寸法が一致するため、k=1.0 しか踏まない観測は
/// 「物理寸法を渡しても同じ答えになる」ことを見逃す。本ファイルの主張はすべて
/// この前提の上に立つので、前提の側を最初に固定する。
#[test]
fn the_observed_scale_set_has_three_values_including_non_unit_ones() {
    assert_eq!(
        SCALES.len(),
        EXPECTED_SCALE_COUNT,
        "観測する表示スケールの本数が {EXPECTED_SCALE_COUNT} ではなく {}（実測 {SCALES:?}）",
        SCALES.len()
    );
    let non_unit: Vec<f32> = SCALES.iter().copied().filter(|k| *k != 1.0).collect();
    assert_eq!(
        non_unit.len(),
        2,
        "k≠1 が 2 通り無い（実測 {non_unit:?}）。k=1.0 では物理寸法と画像の寸法が一致するため、\
         k=1.0 だけでは「物理寸法を渡しても答えが変わらない」実装を見逃す"
    );
}

// ── 画像の寸法と k の保持（設計 C2 の I 行）─────────────────────────────────────

/// 枠の配置が持つ画像の寸法が k に依らず原寸のままで、保持される k が渡した値であること。
///
/// 供給面の寸だけが k で膨らみ、画像の寸法は膨らまない——この 2 つが同じ配置の中で
/// 分かれていることが、下の領域解決が k に依らない土台である。
#[test]
fn binding_keeps_the_native_image_size_and_the_given_scale() {
    let image = scope_image_size(SCOPE);
    let mut world = World::new();
    let mut visited = 0usize;
    for k in SCALES {
        let binding = staysee_binding(&mut world, k);
        let surface = physical_surface(image, k);

        assert_eq!(
            binding.image_size, image,
            "k={k}: 画像の寸法が原寸 {image:?} ではなく {:?}（k に依らず原寸のままのはず）",
            binding.image_size
        );
        assert_eq!(
            binding.scale, k,
            "k={k}: 保持される表示スケールが {k} ではなく {}",
            binding.scale
        );
        assert_eq!(
            binding.surface_size, surface,
            "k={k}: 供給面の寸が {surface:?} ではなく {:?}（`ceil(原寸 × k)`）",
            binding.surface_size
        );
        visited += 1;
    }
    assert_eq!(
        visited, EXPECTED_SCALE_COUNT,
        "踏んだ表示スケールが {EXPECTED_SCALE_COUNT} 通りではなく {visited} 通り（ループが痩せている）"
    );
}

// ── 物理寸法への変換（設計 C2 の I 行）──────────────────────────────────────────

/// 物理寸法への変換が、描画範囲の幅と左辺のそれぞれで k 倍になること。
///
/// 期待値は手計算の絶対値である。幅 287 の側は `ceil(287k)` なので k=1.25 で
/// 358.75 → **359**（切り捨て・四捨五入なら 358 になる端数）、左辺 22 の側は切り上げを
/// 挟まない `22k` なので k=1.25 で **27.5**——両者で丸めの有無が違うことまで込みで固定する。
#[test]
fn physical_conversion_scales_with_the_display_scale() {
    // (k, ceil(287k), 22k)。いずれも f32 で正確に表現できる値。
    let expected: [(f32, u32, f32); EXPECTED_SCALE_COUNT] = [
        (1.0, 287, 22.0),
        (1.25, 359, 27.5), // 287×1.25=358.75→ceil 359・22×1.25=27.5
        (2.0, 574, 44.0),
    ];
    assert_eq!(
        expected.map(|(k, _, _)| k),
        SCALES,
        "期待値の表の表示スケールが観測する 3 通り {SCALES:?} と食い違う"
    );

    let mut visited = 0usize;
    for (k, want_extent, want_left) in expected {
        let contract = ScaleContract::new(k, None);

        let extent = contract.physical_extent(ImagePx(EXPECTED_INLINE_EXTENT));
        assert_eq!(
            extent, want_extent,
            "k={k}: 描画範囲の幅 {EXPECTED_INLINE_EXTENT} の物理寸法が {want_extent} ではなく \
             {extent}（`ceil(幅 × k)`）"
        );
        let left = contract.to_physical(ImagePx(EXPECTED_LEFT)).0;
        assert_eq!(
            left, want_left,
            "k={k}: 描画範囲の左辺 {EXPECTED_LEFT} の物理座標が {want_left} ではなく {left}\
             （`左辺 × k`）"
        );
        visited += 1;
    }
    assert_eq!(
        visited, EXPECTED_SCALE_COUNT,
        "踏んだ表示スケールが {EXPECTED_SCALE_COUNT} 通りではなく {visited} 通り（ループが痩せている）"
    );
}

// ── 領域解決の入力（設計 C2 の I 行の要）────────────────────────────────────────

/// 文字の領域解決の入力が画像の寸法であること——解いた領域が k に依らず
/// [`super::region`] が固定している値と同値になる。
///
/// 期待値は `super::test_support` の定数をそのまま引く（書き写すと、片方だけ直したときに
/// 同一性が黙って壊れる）。本番の解決へ供給面の寸（物理寸法）を渡す形へ変われば、
/// k≠1 でここが赤になる。
#[test]
fn text_region_resolves_from_the_image_size_not_the_physical_surface() {
    let model = staysee_model();
    let image = scope_image_size(SCOPE);
    let bottom = expected_bottom(SCOPE);
    let mut world = World::new();
    let mut visited = 0usize;
    for k in SCALES {
        let binding = staysee_binding(&mut world, k);
        let region = ResolvedBalloonText::resolve(&model, binding.image_size).region;

        assert_eq!(
            (region.left(), region.top(), region.right(), region.bottom()),
            (EXPECTED_LEFT, EXPECTED_TOP, EXPECTED_RIGHT, bottom),
            "k={k}: 描画範囲の 4 辺が {:?} ではなく {:?}（領域解決へ供給面の寸 {:?} が渡っている疑い）",
            (EXPECTED_LEFT, EXPECTED_TOP, EXPECTED_RIGHT, bottom),
            (region.left(), region.top(), region.right(), region.bottom()),
            binding.surface_size
        );
        assert_eq!(
            region.start(),
            (EXPECTED_LEFT, EXPECTED_TOP),
            "k={k}: 描画開始点が {:?} ではなく {:?}",
            (EXPECTED_LEFT, EXPECTED_TOP),
            region.start()
        );
        assert_eq!(
            region.image_size(),
            (image.0 as f32, image.1 as f32),
            "k={k}: 解決が保持する画像の原寸が {:?} ではなく {:?}",
            (image.0 as f32, image.1 as f32),
            region.image_size()
        );
        visited += 1;
    }
    assert_eq!(
        visited, EXPECTED_SCALE_COUNT,
        "踏んだ表示スケールが {EXPECTED_SCALE_COUNT} 通りではなく {visited} 通り（ループが痩せている）"
    );
}

/// 対照: 同じ解決へ**供給面の寸**（物理寸法）を渡すと、k≠1 では遠辺と下辺が動く。
///
/// `validrect.right,-26`／`bottom,-47` は反対辺基準なので、渡した寸法がそのまま答えへ乗る。
/// この対照が緑であることは、上の
/// [`text_region_resolves_from_the_image_size_not_the_physical_surface`] が
/// 「何を渡しても同じ答えが返る」実装の上での空振りではないことの裏取りである。
/// k=1.0 では供給面の寸が原寸と一致するので**答えも一致する**——k=1.0 だけの観測では
/// 入力の取り違えを検出できないことを、その一致そのもので示す。
#[test]
fn resolving_from_the_physical_surface_moves_the_far_edges() {
    // (k, 供給面の寸を渡したときの遠辺, 同じく下辺)。
    // k=1.25: ceil(335×1.25)=419 → 419−26=393・ceil(205×1.25)=257 → 257−47=210。
    // k=2.0 : 670−26=644・410−47=363。
    let expected: [(f32, f32, f32); EXPECTED_SCALE_COUNT] = [
        (1.0, EXPECTED_RIGHT, 158.0),
        (1.25, 393.0, 210.0),
        (2.0, 644.0, 363.0),
    ];
    assert_eq!(
        expected.map(|(k, _, _)| k),
        SCALES,
        "対照の表の表示スケールが観測する 3 通り {SCALES:?} と食い違う"
    );

    let model = staysee_model();
    let bottom = expected_bottom(SCOPE);
    let mut world = World::new();
    let mut moved = 0usize;
    for (k, want_right, want_bottom) in expected {
        let binding = staysee_binding(&mut world, k);
        let region = ResolvedBalloonText::resolve(&model, binding.surface_size).region;

        assert_eq!(
            (region.right(), region.bottom()),
            (want_right, want_bottom),
            "k={k}: 供給面の寸 {:?} を渡したときの遠辺・下辺が {:?} ではなく {:?}",
            binding.surface_size,
            (want_right, want_bottom),
            (region.right(), region.bottom())
        );
        if k == 1.0 {
            assert_eq!(
                (region.right(), region.bottom()),
                (EXPECTED_RIGHT, bottom),
                "k=1.0 では供給面の寸が原寸と一致するので答えも一致するはず\
                 （ここが分かれるなら供給面の寸の作り方が原寸と別物になっている）"
            );
        } else {
            assert_ne!(
                (region.right(), region.bottom()),
                (EXPECTED_RIGHT, bottom),
                "k={k}: 供給面の寸を渡しても答えが変わらない\
                 ——解決が渡された寸法を使っておらず、入力の取り違えを検出できない"
            );
            moved += 1;
        }
    }
    assert_eq!(
        moved, 2,
        "答えが動いた k≠1 が 2 通りではなく {moved} 通り（対照が空振りしている）"
    );
}
