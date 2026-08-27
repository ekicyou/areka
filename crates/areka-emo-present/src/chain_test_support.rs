//! `chain.rs` テストの共有ヘルパ（WUC apartment・既知パターンの本物合成）。
//!
//! 往復の檻（`tests`）と失敗注入（`fault_tests`）の複数モジュールが同じ材料を使うため、生成手順を
//! ここへ集約する（`structure.md`「テーマ間で共有するヘルパは `<stem>_test_support.rs`」）。
//! 新規 dev-dep は持ち込まない（既存 `mod tests` が使っていた依存のみ）。

use super::*;

use areka_emo_atlas::{
    AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, Composer, EmoWorld, PatternState};
use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};
use std::path::Path;

use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
use wintf::com::wuc::create_dispatcher_queue_controller;

/// テスト用の WUC apartment / dispatcher を組む（spike / wintf 1.2 テストと同一方針）。
///
/// cargo test の各テストは専用スレッドで走り COM 未初期化ゆえ、design §2.1「未初期化なら ASTA」
/// に従い ASTA を第一候補・失敗時 NONE を保険とする。controller は Compositor より長寿命を要する
/// ため呼び出し側で保持する。
pub(super) fn make_dispatcher_and_compositor()
-> (windows::System::DispatcherQueueController, Compositor) {
    let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
        .or_else(|e_asta| create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta))
        .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
    let compositor = Compositor::new().expect("Compositor::new 失敗");
    (dq, compositor)
}

// ── ComposedSurface 生成補助（cache.rs テストと同技法）─────────────────────
// `ComposedSurface::bytes_mut` は emo-compose の pub(crate) ゆえ本クレートから画素を直接焼けない。
// よって「既知の非退化パターン」は上流公開 API（atlas bake → EmoWorld → Composer::compose）で
// 本物を合成して得る（模造バッファでの偽陽性を避ける）。

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

/// `w×h` の**全不透明・座標由来グラデーション**を単一 element として本物合成する。
///
/// α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は正確に `w×h` になる。各画素は
/// 座標＋`salt` から決定論的に作り（成分 ≤ α=255 で premultiplied 不変を自明に満たす）、
/// リサイズ前後で異なるパターンとして区別できる。全 0 でない＝非退化を呼び出し側が assert する。
pub(super) fn composed_of_size(w: u32, h: u32, salt: u8) -> ComposedSurface {
    let base = Path::new("shell/master");
    let surfaces = vec![surface(1000, vec![elem("p.png", 0, 0)])];

    let mut dec = MemoryDecoder::new();
    let stride = w * 4;
    let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let a: u8 = 0xFF;
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            img.push(b);
            img.push(g);
            img.push(r);
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

    let mut composer = Composer::new();
    composer
        .compose(
            &world,
            &baked.table,
            1000,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("静的 element 単体の合成は Ok")
}
