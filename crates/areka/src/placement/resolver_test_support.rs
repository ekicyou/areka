use super::*;

/// DPI パラメタ水準（3.4・U5）。
pub(super) const DPIS: [i32; 4] = [96, 120, 144, 192];

/// 論理基準値 → 各 DPI の物理 px（整数演算のみ・厳密整除を強制）。
///
/// resolver 自体は物理 px しか見ない（U1）。テストは「同じ論理形状を
/// 各 DPI の物理値で与えたとき、期待値も同じ物理式から出る」ことを固定する
/// （隠れた `/96` 変換があれば 96 以外の水準で崩れる＝07-05 欠陥の檻）。
pub(super) fn px(logical: i32, dpi: i32) -> i32 {
    assert_eq!(
        (logical * dpi) % 96,
        0,
        "テスト入力は厳密整除になる論理値（4 の倍数）で構築する"
    );
    logical * dpi / 96
}

/// プライマリモニタ相当の work area（左上原点・物理 px）。
pub(super) fn work_area(dpi: i32) -> RectPx {
    RectPx {
        left: 0,
        top: 0,
        right: px(1920, dpi),
        bottom: px(1080, dpi),
    }
}
