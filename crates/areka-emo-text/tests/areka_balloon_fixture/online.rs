//! 取り出した `online0.webp` を areka の動く絵の読み手で読み戻し、コマと灯る点の α を固定する。
//!
//! 出典 spec: `default-balloon-selfmade`（要件 **8.9**・**3.2**・**3.3**）。
//!
//! 期待値は設計「絵柄の値」の表から手で書く: 77×36・中心 (26,18)・(38.5,18)・(51,18)・半径 4 の
//! 点 3 つ・コマ k で点 k が α 1、ほかが α 0.25・3 コマ・各 300 ミリ秒・繰り返し無限。

use areka_emo_atlas::{ElementDecoder, LoopCount, WicDecoderArm};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::test_support::{EXPECTED_ONLINE_SIZE, balloon_root};

/// 点の中心を指す画素 `(x, y)`。画素 i は座標 [i, i+1) を覆うので、中心の座標の切り捨てを取る。
/// 画素の中心（i+0.5）と点の中心のずれは各軸 0.5 以下で、半径 4 の縁（ぼかしの効く 1 画素ほど）
/// から十分に内側にある。座標を画素の中心と読む流儀でも、ずれは同じく 0.5 以下に収まる。
const DOT_PIXELS: [(u32, u32); 3] = [(26, 18), (38, 18), (51, 18)];

/// 点の外の画素（左上の隅・どの点からも 20 以上離れる）。
const OUTSIDE_PIXEL: (u32, u32) = (2, 2);

/// 灯る点の α（1.0 × 255）。
const LIT_ALPHA: u8 = 255;

/// 灯らない点の α（0.25 × 255 = 63.75 を最も近い整数へ丸めて 64）。
const UNLIT_ALPHA: u8 = 64;

/// コマの枚数と各コマの待ち時間（要件 3.3）。
const FRAME_COUNT: u32 = 3;
const FRAME_DELAY_MS: u32 = 300;

/// `online0.webp` が 77×36・3 コマ・各 300 ミリ秒・無限・α ありで、コマ k で点 k だけが灯る
/// （要件 8.9・3.2・3.3）。
#[test]
fn online_marker_frames_light_dots_in_turn() {
    // SAFETY: COM の MTA 初期化。並列のテストで既に張られていても使えるので戻り値は見ない。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let decoder = WicDecoderArm::new().expect("WIC の読み手を作れない（COM の初期化が前提）");
    let (name, width, height) = EXPECTED_ONLINE_SIZE;
    let path = balloon_root().join(name);

    let info = decoder
        .probe_animation(&path)
        .unwrap_or_else(|| panic!("{}: areka の読み手が動く絵として読めない", path.display()));
    assert_eq!(
        (info.width, info.height, info.frame_count),
        (width, height, FRAME_COUNT),
        "{}: 見出しが期待 {width}×{height}・{FRAME_COUNT} コマと違う",
        path.display()
    );

    let anim = decoder
        .decode_frames(&path, info)
        .unwrap_or_else(|e| panic!("{}: 全コマを読めない: {e}", path.display()));
    assert_eq!(
        anim.loop_count,
        LoopCount::Infinite,
        "{}: 繰り返しが無限でない",
        path.display()
    );
    assert_eq!(
        anim.frames.len(),
        FRAME_COUNT as usize,
        "{}: コマの枚数",
        path.display()
    );

    for (k, frame) in anim.frames.iter().enumerate() {
        let img = &frame.image;
        assert_eq!(
            frame.delay_ms,
            FRAME_DELAY_MS,
            "{}: コマ {k} の待ち時間",
            path.display()
        );
        assert!(img.has_alpha, "{}: コマ {k} に α が無い", path.display());
        assert_eq!(
            (img.width, img.height),
            (width, height),
            "{}: コマ {k} の寸法",
            path.display()
        );
        // BGRA8 の 4 バイト目が α（乗算済みでも α の値はそのまま）。
        let alpha = |(x, y): (u32, u32)| img.bgra[(y * img.stride + x * 4 + 3) as usize];
        for (d, &px) in DOT_PIXELS.iter().enumerate() {
            let expected = if d == k { LIT_ALPHA } else { UNLIT_ALPHA };
            assert_eq!(
                alpha(px),
                expected,
                "{}: コマ {k} の点 {d} の中心 {px:?} の α",
                path.display()
            );
        }
        assert_eq!(
            alpha(OUTSIDE_PIXEL),
            0,
            "{}: コマ {k} の点の外 {OUTSIDE_PIXEL:?} の α",
            path.display()
        );
    }
}
