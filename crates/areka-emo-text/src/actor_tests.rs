use super::TextSlotBinding;
use bevy_ecs::prelude::World;

/// k=1.0（現行契約の物理 1:1）: `image_size` は `surface_size` と同値、
/// slot/window/scale/surface_size は透過保持される。
#[test]
fn binding_identity_scale_keeps_surface_size() {
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();

    let binding = TextSlotBinding::new(slot, window, 1.0, (434, 687), (434, 687));
    assert_eq!(binding.slot, slot);
    assert_eq!(binding.window, window);
    assert_eq!(binding.scale, 1.0);
    assert_eq!(binding.surface_size, (434, 687));
    assert_eq!(
        binding.image_size,
        (434, 687),
        "k=1.0 のとき image_size == surface_size（k≠1.0 では乖離する＝本 fixture は k=1.0）"
    );
}

/// image px 原寸は**導出せず透過する**（2026-07-30・旧「`round(物理 / k)` 一点導出」の後継檻）。
///
/// k と物理寸から image px 原寸を計算し直す変異（＝旧実装の復活）を殺す。k=1.25・物理
/// (127, 94) に対し旧導出は (102, 75) を返したが、正しい原寸は**呼び手が渡した値**である。
/// 端数を含む物理寸でも原寸は 1bit も動かない——これが「作者画像空間は k 不変」の意味。
#[test]
fn binding_passes_image_size_through_without_deriving_it() {
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();

    let binding = TextSlotBinding::new(slot, window, 1.25, (127, 94), (101, 75));
    assert_eq!(
        binding.image_size,
        (101, 75),
        "image px 原寸は透過（旧導出 round(127/1.25)=102 を復活させると落ちる）"
    );
    assert_eq!(binding.surface_size, (127, 94), "物理原寸はそのまま保持");
}

/// k<1 の透過檻（2026-07-30 新設）: k=4/5・物理 (114, 62)・原寸 (142, 77)。
///
/// k<1 の順写像は**縮小写像＝単射でない**: `scale_len` は 142 も 143 も物理 114 へ、
/// 77 も 78 も 62 へ潰す。潰れた情報は割り算では戻らず、旧導出 `round(物理 / k)` は
/// この衝突対のどちらかで必ず 1px 間違える（実測では 114 から 143 を返すため 142 側が落ちる）。
/// 透過にした今は k の大小に依らず厳密。
#[test]
fn binding_passes_image_size_through_at_sub_unity_scale() {
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();

    let binding = TextSlotBinding::new(slot, window, 0.8, (114, 62), (142, 77));
    assert_eq!(
        binding.image_size,
        (142, 77),
        "k<1 でも原寸は透過（旧導出は (143, 78) を返して 1px ずれた）"
    );
    assert_eq!(binding.surface_size, (114, 62), "物理原寸はそのまま保持");
}

/// 不正な k（0 以下・非有限）は ScaleContract の縮退規約（warn!＋1.0）へ乗り、
/// binding の scale が物理 1:1 で自己整合する（k の多重適用・混在の構造排除）。
#[test]
fn binding_degrades_invalid_scale_to_identity() {
    let mut world = World::new();
    let slot = world.spawn_empty().id();
    let window = world.spawn_empty().id();

    let binding = TextSlotBinding::new(slot, window, 0.0, (320, 240), (320, 240));
    assert_eq!(
        binding.scale, 1.0,
        "不正 k は 1.0 へ縮退（log-first・panic なし）"
    );
    assert_eq!(binding.image_size, (320, 240));
}
