//! design D8 の「揃えの残差の許容量」の実測（要件 4.4／7.6・research §12 の宿題）。
//!
//! キーワードの中央揃えは `(char_w − balloon_w) / 2` であり、両寸は作者寸から
//! **軸ごとの拡大率で個別に**丸められる。拡大率遷移でその基本位置を追随させたとき、
//! 遷移後の寸から導出し直した中央揃えとどれだけ食い違うか——それが D8 の残差である。
//!
//! design D8 は上限 3px を**式からの見積り**として置いた（research §12 が「決定論テストが
//! DPI 行列で実値を固定した時点で上限の妥当性を再評価すること」を宿題に残している）。
//! 本ファイルがその実値であり、上限だけでなく**実測の最大値**も逐語で固定する。
//!
//! **実測最大 2px が上界の証明ではないこと**に注意する。全数なのは作者基準 DPI（3×3）と
//! 表示 DPI 遷移（20 通り）の側だけで、作者寸は下の 4×4＝16 通りの手選びである。
//! 逐語固定した `worst` は、寸法集合を広げて 3px が出れば必ず赤くなり、D8 の再評価を強制する。
//!
//! 判断分岐の網羅そのものは兄弟ファイル `follow_offset_space_tests.rs` が持つ。
//! ここを分けているのは 1,000 行制限（要件 9.6）に収めるためである。

use areka_emo_compose::ScaleRatio;
use wintf::ecs::DPI;

use super::offset_space::{OffsetBase, OffsetRescale, rescale_follow_offset};
use crate::placement::resolver::PointPx;

/// 表示 DPI（両軸同値）。
fn dpi(v: u16) -> DPI {
    DPI::from_dpi(v, v)
}

/// 表示 DPI `d` における軸ごとの拡大率（`k_axis(d) = d ÷ author_dpi_axis`・app_scale=1）。
fn k_axis(display_dpi: u32, author_dpi: u32) -> ScaleRatio {
    ScaleRatio::new(display_dpi, author_dpi).expect("非ゼロ DPI")
}

/// 中央揃えの基本位置（キャラ窓左上相対）——`resolver::keyword_balloon_pos` と同じ式。
///
/// `char_pos` に原点を渡したときの `base_x` にあたる（`(char_w − balloon_w) / 2`・
/// 整数除算は 0 方向へ切り捨て）。両寸は**作者寸から軸ごとの拡大率で個別に**丸められる
/// ——シェルとバルーンで作者基準 DPI が違えば、丸めの落ち方も軸ごとに違う。
fn center_offset_x(
    char_w_author: u32,
    balloon_w_author: u32,
    shell_author_dpi: u32,
    balloon_author_dpi: u32,
    display_dpi: u32,
) -> i32 {
    let char_w = k_axis(display_dpi, shell_author_dpi).scale_len(char_w_author) as i32;
    let balloon_w = k_axis(display_dpi, balloon_author_dpi).scale_len(balloon_w_author) as i32;
    (char_w - balloon_w) / 2
}

/// 追随後の揃えの残差が design D8 の許容量（1 軸あたり ≤ 3px）に収まり、
/// **実測の最大は 2px** であること（要件 4.4／design D8・research §12 の宿題の再評価）。
///
/// 残差＝「遷移前の中央揃えを追随させた値」と「遷移後の寸から導出し直した中央揃え」の差。
/// シェル／バルーンの作者基準 DPI の 9 通り × 作者寸 16 通り × 表示 DPI 比が 1/2〜2 に
/// 収まる遷移 20 通り＝**2,880 組を全数列挙**する。上限だけでなく実測値も逐語で固定する
/// のは、上限内での悪化を見逃さないためである（D8 の意図）。
#[test]
fn center_alignment_residual_stays_within_d8_bound() {
    const DISPLAY_DPIS: [u32; 5] = [96, 120, 144, 168, 192];
    const AUTHOR_DPIS: [u32; 3] = [96, 120, 144];
    const CHAR_WIDTHS: [u32; 4] = [120, 241, 382, 937];
    const BALLOON_WIDTHS: [u32; 4] = [180, 301, 434, 813];

    let mut cases = 0usize;
    let mut worst = 0i32;
    for &shell_author in &AUTHOR_DPIS {
        for &balloon_author in &AUTHOR_DPIS {
            for &char_w in &CHAR_WIDTHS {
                for &balloon_w in &BALLOON_WIDTHS {
                    for &d0 in &DISPLAY_DPIS {
                        for &d1 in &DISPLAY_DPIS {
                            if d0 == d1 || d1 * 2 < d0 || d1 > d0 * 2 {
                                continue;
                            }
                            let before = center_offset_x(
                                char_w,
                                balloon_w,
                                shell_author,
                                balloon_author,
                                d0,
                            );
                            let after = center_offset_x(
                                char_w,
                                balloon_w,
                                shell_author,
                                balloon_author,
                                d1,
                            );
                            let base = OffsetBase {
                                offset: PointPx { x: before, y: 0 },
                                dpi: Some(DPI::from_dpi(d0 as u16, d0 as u16)),
                            };
                            let OffsetRescale::Rescaled { offset, saturated } =
                                rescale_follow_offset(base, DPI::from_dpi(d1 as u16, d1 as u16))
                            else {
                                panic!("{d0} → {d1}: 追随の腕に入らなかった");
                            };
                            assert!(!saturated, "{d0} → {d1}: 現実的な寸で飽和した");
                            worst = worst.max((offset.x - after).abs());
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 2_880, "列挙が痩せたら上限の主張も痩せる");
    assert!(worst <= 3, "D8 の許容量 3px を超えた: {worst}px");
    assert_eq!(
        worst, 2,
        "実測の最大残差が動いた（悪化も改善も設計の再評価が要る）"
    );
}

/// D8 の残差が実際に出る最悪の 1 組を逐語で固定する（残差 0 の組ばかりでは檻にならない）。
#[test]
fn center_alignment_residual_worst_case_is_pinned_verbatim() {
    // シェル作者基準 144dpi・バルーン作者基準 120dpi、作者寸 241×813 を 96 → 192 へ。
    let before = center_offset_x(241, 813, 144, 120, 96);
    let after = center_offset_x(241, 813, 144, 120, 192);
    assert_eq!((before, after), (-244, -490));
    assert_eq!(
        rescale_follow_offset(
            OffsetBase {
                offset: PointPx { x: before, y: 0 },
                dpi: Some(dpi(96)),
            },
            dpi(192)
        ),
        OffsetRescale::Rescaled {
            offset: PointPx { x: -488, y: 0 },
            saturated: false,
        }
    );
    // 残差 2px（許容量 3px の内側・上限に 1px しか余裕が無いことを記録する）。
    assert_eq!((-488 - after).abs(), 2);
}
