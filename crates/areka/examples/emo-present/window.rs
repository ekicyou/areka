use super::{
    BalloonWindowMarker, Entity, HitTest, Name, OnPointerPressed, Point, ShellWindowMarker, SizeI,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE, Window, WindowPos,
    WindowStyle, World, on_shell_pressed,
};

// ---------------------------------------------------------------------------
// Window creation（mock-shell donor から移植・内容供給だけ差し替え）
// ---------------------------------------------------------------------------

/// シェル窓 Entity を構築する（WS_POPUP 透過窓・物理 px 採寸・αマスク当たりは emo-surface 子が担う）。
///
/// mock-shell と異なり `BitmapSource`／`BoxStyle` は使わない。表示内容は `EmoPresenter` が
/// `attach_target`→`apply` で装着する原寸のコマンドリスト（拡大は wintf の `SetTransform`）。窓クライアント寸は
/// 起動時点では surface の **native 原寸**（物理 px 単位）を `WindowPos.size` へ直接与える（DPI 表示契約・
/// taffy 非経由）——表示成立後に `reconcile_window_size` が `target_physical_size`（`scaled_extent`）へ合わせ直す。
pub(super) fn create_shell_window(world: &mut World, x: i32, y: i32, w: u32, h: u32) -> Entity {
    world
        .spawn((
            Name::new("Emo-Shell-Window"),
            ShellWindowMarker,
            Window {
                title: "areka emo shell".to_string(),
                // WUC 合成固定。factory の compute_ex_style が WS_EX_LAYERED を剥がし
                // WS_EX_NOREDIRECTIONBITMAP を付与するため ex_style は据え置きでよい。
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point { x, y }),
                // 窓クライアント寸 ≔ surface 原寸（物理 px）。
                size: Some(SizeI {
                    width: w as i32,
                    height: h as i32,
                }),
                ..Default::default()
            },
            // 窓自身はヒット対象外（全面ヒットで透過を殺さない）。当たりは emo-surface 子（αマスク）が担う。
            HitTest::none(),
            // ダブルクリックで全窓を閉じて終了（手動観測の利便）。
            OnPointerPressed(on_shell_pressed),
        ))
        .id()
}

/// バルーン窓 Entity を構築する（シェルと同一機構・内容は EmoPresenter が装着）。
pub(super) fn create_balloon_window(world: &mut World, x: i32, y: i32, w: u32, h: u32) -> Entity {
    world
        .spawn((
            Name::new("Emo-Balloon-Window"),
            BalloonWindowMarker,
            Window {
                title: "areka emo balloon".to_string(),
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point { x, y }),
                size: Some(SizeI {
                    width: w as i32,
                    height: h as i32,
                }),
                ..Default::default()
            },
            HitTest::none(),
        ))
        .id()
}
