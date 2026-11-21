use super::*;

/// DPI パラメタ水準。
pub(crate) const DPIS: [i32; 4] = [96, 120, 144, 192];

/// 論理基準値 → 各 DPI の物理 px（整数演算のみ・厳密整除を強制）。
///
/// resolver 自体は物理 px しか見ない。テストは「同じ論理形状を
/// 各 DPI の物理値で与えたとき、期待値も同じ物理式から出る」ことを固定する
/// （隠れた `/96` 変換があれば 96 以外の水準で崩れる）。
pub(crate) fn px(logical: i32, dpi: i32) -> i32 {
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

/// 左上が原点でない work area（クランプ境界の left/top 依存を暴く）。
pub(crate) fn offset_work_area(dpi: i32) -> RectPx {
    RectPx {
        left: px(64, dpi),
        top: px(40, dpi),
        right: px(64 + 1920, dpi),
        bottom: px(40 + 1080, dpi),
    }
}
