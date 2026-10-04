//! `dump_surface` の実際の描画を通るテスト（spec: areka-P0-mcp-dump-images・要件 1.1・1.3・1.4・
//! 5.2・5.3・7.4 ⑴・x64 だけ接続）。土台は [`super::gpu_test_support`]。

use areka_mcp::ToolContent;

use super::gpu_test_support::{GpuRig, decode_png};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::target_map::shell_target;

/// 検体（emo2）の `surfaces.txt` に在る、アニメーションも着せ替えも持たない surface（`element0` 1 枚）。
/// 起動のときの 0 と違う番号で、台本で出したことが ID に現れる。
const SHOWN: u32 = 1101;

/// 今の見た目（要件 1.1・1.3・1.4）: 台本 `\0\s[1101]` の後の `dump_surface`（省略）の PNG を
/// 復号した画素が、同じ surface を合成器で合成して乗算を戻した絵と一致し、本文が今の見た目の逐語。
/// 窓の `DPI` を 144（拡大率 1.5）にして、拡大を掛ける前の原寸であることも同時に固定する。
///
/// # 非空虚性
/// 乗算の戻しで色の並びを取り違えると画素が一致せず赤。拡大した絵を返すと大きさが合わず赤。
/// 本文が別の言い方（指定の文言など）だと逐語の比較で赤。
#[test]
fn shown_surface_matches_the_composed_pixels_at_native_size() {
    let mut gpu = GpuRig::new(r"\0\s[1101]\e", 144);
    let shown = gpu.frames_until(100, |world| {
        world
            .get_non_send::<Emo2Wiring>()
            .and_then(|w| w.presenter().last_shown(shell_target(0)))
            .is_some_and(|(id, pic)| id == SHOWN && pic.is_some())
    });
    let answer = gpu.dump_surface(None, None).try_answer().ok().flatten();
    let expected = gpu.composed_rgba(SHOWN);
    let down = gpu.shutdown();

    let answer = answer.expect("装着の後はその場で答える");
    let (text, (w, h, pixels), mime) = match answer.outcome.content.as_slice() {
        [
            ToolContent::Text(text),
            ToolContent::Image { data, mime_type },
        ] => (text.clone(), decode_png(data), mime_type.clone()),
        other => panic!("本文＋画像 1 枚の形ではない: {other:?}"),
    };
    let (ew, eh, expected_pixels) = expected;
    // 画素の列は大きいので、食い違った画素の数で比べる（大きさが違えば数えずに大きさで赤）。
    let differing = (pixels.len() == expected_pixels.len()).then(|| {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected_pixels.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count()
    });
    assert_eq!(
        (
            shown,
            down,
            answer.outcome.is_error,
            text,
            mime,
            (w, h),
            differing
        ),
        (
            true,
            true,
            false,
            "OK:scope 0, surface 1101 as currently shown (with running animations and dressups, \
             before scaling and transparency)"
                .to_owned(),
            "image/png".to_owned(),
            (ew, eh),
            Some(0),
        ),
        "（台本の surface が出た, 降ろせた, isError, 本文, MIME, 復号した（幅, 高さ）, 食い違った画素の数）"
    );
    assert!(ew > 0 && eh > 0, "比べる絵が空ではない");
}
