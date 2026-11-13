//! 信用できない読み手の答えを焼きの段で断るテスト（spec: audit-emo-atlas 指摘 2・18）。
//!
//! 替え玉の読み手 `MemoryDecoder` は登録どおりに返す（約束を確かめない）ので、一辺の上限を越える絵や
//! 長さが約束より短い絵を作れる。焼きの入口がそれを読めなかった絵として扱い、panic しないことを確かめる。
use crate::decode::{AnimatedImage, AnimationFrame, AnimationInfo, DecodeError, MemoryDecoder};
use crate::error::BakeError;
use crate::limits::AnimationLimits;
use crate::pack::PackConfig;
use crate::table::{LoopCount, SetId};
use crate::test_support::{BROKEN, elem, image, set, surface};
use crate::{BakeResult, bake_with_limits};
use std::path::Path;

const BASE: &str = "shell/master";
const OPAQUE: [u8; 4] = [10, 20, 30, 255];

/// `rel` の絵 1 つを持つシェルを焼き、記録を添えて返す。
fn bake_logged(dec: &MemoryDecoder, rel: &str) -> (BakeResult, String) {
    let surfaces = vec![surface(0, vec![elem(0, rel)])];
    let sets = [set(&surfaces, Path::new(BASE))];
    let mut result = None;
    let log = crate::log_capture::capture_logs(|| {
        result = Some(bake_with_limits(
            &sets,
            dec,
            PackConfig::default(),
            AnimationLimits::default(),
        ));
    });
    (result.unwrap(), log)
}

/// 失敗の一覧がちょうど 1 件の `Decode` で、その文を返す。
fn single_decode_error(result: &BakeResult) -> &str {
    match result.errors.as_slice() {
        [BakeError::Decode(DecodeError::Decode { source, .. })] => source,
        other => panic!("expected one Decode error, got {other:?}"),
    }
}

/// 指摘 2: 見出しが 16,385×1 の動く絵は縮めずに読めなかった絵になり、「1 枚だけ読みます」の記録は出ない。
#[test]
fn animated_header_over_max_side_fails_without_shrinking() {
    let path = Path::new(BASE).join("wide.png");
    let wide = image(16_385, 1, true, &vec![OPAQUE; 16_385]);
    let mut dec = MemoryDecoder::new();
    dec.insert_animated(
        &path,
        AnimationInfo {
            width: 16_385,
            height: 1,
            frame_count: 2,
        },
        Ok(wide.clone()),
        Ok(AnimatedImage {
            frames: (0..2)
                .map(|_| AnimationFrame {
                    image: wide.clone(),
                    delay_ms: 100,
                })
                .collect(),
            loop_count: LoopCount::Infinite,
        }),
    );
    dec.insert(&path, 16_385, 1, 16_385 * 4, wide.bgra, true);

    let (result, log) = bake_logged(&dec, "wide.png");
    assert_eq!(result.table.resolve(SetId(0), "wide.png"), None);
    let source = single_decode_error(&result);
    assert!(
        source.contains("16385") && source.contains("16384"),
        "{source}"
    );
    assert!(!log.contains("1 枚だけ読みます"), "no shrink warn: {log}");
}

/// 指摘 2: 16,385×1 の静止画は読めなかった絵になり、表に載らない。
#[test]
fn still_over_max_side_fails() {
    let mut dec = MemoryDecoder::new();
    let wide = image(16_385, 1, true, &vec![OPAQUE; 16_385]);
    dec.insert(
        Path::new(BASE).join("wide.png"),
        16_385,
        1,
        16_385 * 4,
        wide.bgra,
        true,
    );

    let (result, _) = bake_logged(&dec, "wide.png");
    assert_eq!(result.table.resolve(SetId(0), "wide.png"), None);
    let source = single_decode_error(&result);
    assert!(source.contains("16384"), "{source}");
}

/// 2×2・stride 8 なのに 12 バイトしかない絵（右上と左下に α を置き、切り詰めが 2 行目を 8 バイト写そうとする形）。
fn short_bgra() -> Vec<u8> {
    let mut bgra = vec![0u8; 12];
    bgra[4..8].copy_from_slice(&OPAQUE);
    bgra[8..12].copy_from_slice(&OPAQUE);
    bgra
}

/// 指摘 18: 長さが約束より短い静止画は panic せず読めなかった絵になり、文に長さが出る。
#[test]
fn still_shorter_than_contract_fails_without_panic() {
    let mut dec = MemoryDecoder::new();
    dec.insert(
        Path::new(BASE).join("short.png"),
        2,
        2,
        8,
        short_bgra(),
        true,
    );

    let (result, _) = bake_logged(&dec, "short.png");
    assert_eq!(result.table.resolve(SetId(0), "short.png"), None);
    let source = single_decode_error(&result);
    assert!(source.contains("12"), "{source}");
}

/// 指摘 18: コマの長さが約束に合わない動く絵は「読めなかった」の記録を 1 行出して 1 枚へ縮める。
#[test]
fn animated_frame_shorter_than_contract_shrinks() {
    let path = Path::new(BASE).join("anim.png");
    let good = image(2, 2, true, &[OPAQUE; 4]);
    let mut short = good.clone();
    short.bgra = short_bgra();
    let mut dec = MemoryDecoder::new();
    dec.insert_animated(
        &path,
        AnimationInfo {
            width: 2,
            height: 2,
            frame_count: 2,
        },
        Ok(good.clone()),
        Ok(AnimatedImage {
            frames: [good, short]
                .into_iter()
                .map(|image| AnimationFrame {
                    image,
                    delay_ms: 100,
                })
                .collect(),
            loop_count: LoopCount::Infinite,
        }),
    );

    let (result, log) = bake_logged(&dec, "anim.png");
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let id = result
        .table
        .resolve(SetId(0), "anim.png")
        .expect("shrinks to one picture");
    assert!(result.table.animation(id).is_none(), "not animated");
    assert_eq!(log.matches(BROKEN).count(), 1, "{log}");
}

/// 指摘 6⑶: 見出しが 1 コマの動く絵は panic せず 1 枚へ縮み、「読めなかった」の記録を 1 行出す
/// （`AtlasTable::with_frames` は 2 コマ以上しか受けない）。
#[test]
fn animated_with_one_frame_shrinks_without_panic() {
    let path = Path::new(BASE).join("one.png");
    let good = image(2, 2, true, &[OPAQUE; 4]);
    let mut dec = MemoryDecoder::new();
    dec.insert_animated(
        &path,
        AnimationInfo {
            width: 2,
            height: 2,
            frame_count: 1,
        },
        Ok(good.clone()),
        Ok(AnimatedImage {
            frames: vec![AnimationFrame {
                image: good,
                delay_ms: 100,
            }],
            loop_count: LoopCount::Infinite,
        }),
    );

    let (result, log) = bake_logged(&dec, "one.png");
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let id = result
        .table
        .resolve(SetId(0), "one.png")
        .expect("shrinks to one picture");
    assert!(result.table.animation(id).is_none(), "not animated");
    assert_eq!(log.matches(BROKEN).count(), 1, "{log}");
}
