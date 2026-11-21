//! ID の描き先を crate の外の使い手と同じ公開の口だけで実 GPU の上で使う。

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use wintf::ecs::{GraphicsCore, IdForm, IdMask, IdMaskPictures, IdRead, IdShape, IdTarget};

fn id(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}

fn painted(size: (u32, u32), shapes: &[IdShape]) -> IdTarget {
    let core = GraphicsCore::new().expect("GraphicsCore");
    let mut target = IdTarget::new(&core, size).expect("IdTarget::new");
    target
        .paint(&core, shapes, &mut IdMaskPictures::new())
        .expect("paint");
    target
}

fn shapes() -> Vec<IdShape> {
    let mask = IdMask::new(2, 1, |x, _| x == 0).unwrap();
    vec![
        IdShape {
            id: id(7),
            form: IdForm::Rect {
                left: 100.0,
                top: 200.0,
                right: 200.0,
                bottom: 260.0,
            },
        },
        IdShape {
            id: id(0x01_02_03),
            form: IdForm::Mask {
                mask: Arc::new(mask),
                left: 10,
                top: 10,
            },
        },
    ]
}

#[test]
fn paints_and_reads_back_on_a_real_gpu() {
    let target = painted((512, 512), &shapes());
    assert_eq!(target.read(100, 200), IdRead::Id(id(7)));
    assert_eq!(target.read(199, 259), IdRead::Id(id(7)));
    assert_eq!(target.read(200, 260), IdRead::Empty);
    assert_eq!(target.read(10, 10), IdRead::Id(id(0x01_02_03)));
    assert_eq!(target.read(11, 10), IdRead::Empty);
    assert_eq!(target.read(512, 0), IdRead::Outside);
}

/// 数字を見るだけの測り（上限は置かない）。
#[test]
fn measure_one_pixel_read_latency() {
    let target = painted((512, 512), &shapes());
    let mut micros: Vec<f64> = (0..300u32)
        .map(|i| {
            let (x, y) = (i % 512, (i * 7) % 512);
            let start = Instant::now();
            assert_ne!(target.read(x, y), IdRead::Unreadable);
            start.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    micros.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "id_target 1px read x300: median {:.0} us, p95 {:.0} us, max {:.0} us",
        micros[150], micros[285], micros[299]
    );
}
