use super::*;

/// RedBox の OnPointerPressed ハンドラ
///
/// 左クリックで色をトグル（赤 ⇔ 黄）する。
/// αマスクヒットテストのデモ: 画像の透明部分をクリックすると
/// イベントが親(RedBox)に伝播してこのハンドラが呼ばれる。
pub(crate) fn on_red_box_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let wlabel = window_label(world, entity);
    // Bubble フェーズでのみ処理
    if !ev.is_bubble() {
        info!(
            "[Tunnel] RedBox: Passing through, sender={:?}, entity={:?}, {}",
            sender, entity, wlabel,
        );
        return false;
    }

    let state = ev.value();

    // 左クリック検出
    if state.left_down {
        info!(
            "[Bubble] RedBox: Left-click, sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1}), L={}, R={}, Ctrl={}",
            sender,
            entity,
            state.client_point.x,
            state.client_point.y,
            state.local_point.x,
            state.local_point.y,
            state.left_down,
            state.right_down,
            state.ctrl_down,
        );

        // 色をトグル（赤 ⇔ 黄）
        if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
            let is_red = match brushes.foreground.as_color() {
                Some(c) => c.r > 0.9 && c.g < 0.1,
                None => false,
            };
            if is_red {
                // 黄色に変更
                brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                });
                info!(
                    "[AlphaMask Demo] BACKGROUND clicked (transparent area) - color: RED -> YELLOW"
                );
            } else {
                // 赤に戻す
                brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                });
                info!(
                    "[AlphaMask Demo] BACKGROUND clicked (transparent area) - color: YELLOW -> RED"
                );
            }
        }

        return true; // イベント処理済み、親に伝播しない
    }

    false
}

/// SeikatuImage の OnPointerPressed ハンドラ
///
/// αマスクヒットテストのデモ用。
/// 不透明部分がクリックされた場合のみこのハンドラが呼ばれる。
/// イベントを消費して親(RedBox)に伝播させない。
pub(crate) fn on_image_pressed(
    _world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble フェーズでのみ処理
    if !ev.is_bubble() {
        return false;
    }

    let state = ev.value();

    // 左クリック検出
    if state.left_down {
        info!(
            "[AlphaMask Demo] IMAGE clicked (opaque area) - event consumed, background unchanged"
        );
        return true; // イベント処理済み、親(RedBox)に伝播しない
    }

    false
}

/// GreenBox の OnPointerPressed ハンドラ
///
/// **Tunnelフェーズ**: 左クリックでキャプチャし、子（GreenBoxChild）に到達させない
/// **Bubbleフェーズ**: 右クリックで色を変更
/// **ダブルクリック**: サイズを変更（100x100 ⇔ 150x150）
///
/// # stopPropagation使用例
/// Tunnelフェーズでtrueを返すことで、親エンティティが子のイベント処理前に
/// 介入できます。これはWinUI3/WPFの`PreviewMouseDown`やDOMの`Capture Phase`と
/// 同じ動作です。
///
/// # sender vs entity
/// - `sender`: 常にイベント発生元（例: GreenBoxChild）
/// - `entity`: 現在処理中のエンティティ（この場合はGreenBox）
pub(crate) fn on_green_box_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(state) => {
            // 左クリックでキャプチャ
            if state.left_down {
                tracing::info!(
                    double_click = ?state.double_click,
                    left_down = state.left_down,
                    "[Tunnel] GreenBox: Button pressed, checking double-click"
                );

                // ダブルクリック判定
                if state.double_click == wintf::ecs::pointer::DoubleClick::Left {
                    info!(
                        "[Tunnel] GreenBox: DOUBLE-CLICK detected, toggling size, sender={:?}, entity={:?}",
                        sender, entity,
                    );

                    // サイズをトグル（100x100 ⇔ 150x150）
                    if let Some(mut box_style) =
                        world.get_mut::<wintf::ecs::layout::BoxStyle>(entity)
                    {
                        let current_width = box_style
                            .size
                            .and_then(|s| s.width)
                            .and_then(|w| {
                                if let Dimension::Px(px) = w {
                                    Some(px)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(100.0);

                        let new_size = if current_width < 125.0 { 150.0 } else { 100.0 };
                        box_style.size = Some(wintf::ecs::layout::BoxSize {
                            width: Some(Dimension::Px(new_size)),
                            height: Some(Dimension::Px(new_size)),
                        });
                        info!(
                            "[Tunnel] GreenBox: Size changed {} -> {}",
                            current_width, new_size
                        );
                    }

                    return true;
                }

                // 通常の左クリック：色をトグル（緑 ⇔ 黄緑）
                info!(
                    "[Tunnel] GreenBox: Captured event, stopping propagation (Left), sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    let is_green = match brushes.foreground.as_color() {
                        Some(c) => c.r < 0.1 && c.g > 0.9,
                        None => false,
                    };
                    if is_green {
                        // 黄緑に変更
                        brushes.foreground =
                            wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                                r: 0.5,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            });
                        info!("[Tunnel] GreenBox: Color changed GREEN -> YELLOW-GREEN");
                    } else {
                        // 緑に戻す
                        brushes.foreground =
                            wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                                r: 0.0,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            });
                        info!("[Tunnel] GreenBox: Color changed YELLOW-GREEN -> GREEN");
                    }
                }

                return true; // イベント停止、子に到達しない
            }

            info!(
                "[Tunnel] GreenBox: Passing through, sender={:?}, entity={:?}",
                sender, entity,
            );
            false
        }
        Phase::Bubble(state) => {
            // 右クリック処理
            if state.right_down {
                info!(
                    "[Bubble] GreenBox: Right-click, sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                // 色を変更
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.8,
                        b: 0.8,
                        a: 1.0,
                    });
                }

                return true;
            }

            false
        }
    }
}

/// GreenBoxChild の OnPointerPressed ハンドラ
///
/// 親（GreenBox）がTunnelでキャプチャした場合、このハンドラは呼ばれない。
/// 右クリック時は親がキャプチャしないため、Tunnel/Bubble両方で実行される。
///
/// # ev.value()の使用例
/// `Phase::Tunnel(state)` や `Phase::Bubble(state)` でパターンマッチする代わりに、
/// `ev.value()` で `PointerState` を直接取得できます。
pub(crate) fn on_green_child_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let state = ev.value();

    match ev {
        Phase::Tunnel(_) => {
            info!(
                "[Tunnel] GreenBoxChild: This should NOT be called if parent captured (Left), sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1}), L={}, R={}, Ctrl={}",
                sender,
                entity,
                state.client_point.x,
                state.client_point.y,
                state.local_point.x,
                state.local_point.y,
                state.left_down,
                state.right_down,
                state.ctrl_down,
            );
            false
        }
        Phase::Bubble(_) => {
            // 右クリック処理
            if state.right_down {
                info!(
                    "[Bubble] GreenBoxChild: Right-click detected, changing to orange, sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                // 色をオレンジに変更
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                        r: 1.0,
                        g: 0.5,
                        b: 0.0,
                        a: 1.0,
                    });
                }

                return true;
            }

            false
        }
    }
}

/// GreenBox の OnPointerMoved ハンドラ
///
/// マウス移動時にログを出力する（デバッグ用）。
pub(crate) fn on_green_box_moved(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble フェーズでのみ処理（Tunnel でログ出力すると冗長）
    if !ev.is_bubble() {
        return false;
    }

    let wlabel = window_label(world, entity);
    let state = ev.value();

    // 10フレームに1回程度ログ出力（頻繁すぎないように）
    static MOVE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let count = MOVE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if count % 30 == 0 {
        info!(
            sender = ?sender,
            entity = ?entity,
            window = %wlabel,
            x = state.client_point.x,
            y = state.client_point.y,
            "[Bubble] GreenBox: Pointer moved"
        );
    }

    false // 伝播続行（親にも通知）
}

/// BlueBox の OnPointerPressed ハンドラ
///
/// 左クリックでサイズをトグル（100 ⇔ 150）する。
pub(crate) fn on_blue_box_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble フェーズでのみ処理
    if !ev.is_bubble() {
        info!(
            "[Tunnel] BlueBox: Passing through, sender={:?}, entity={:?}",
            sender, entity,
        );
        return false;
    }

    let state = ev.value();

    // 左クリック検出
    if state.left_down {
        info!(
            "[Bubble] BlueBox: Left-click detected! Toggling size, sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1}), L={}, R={}, Ctrl={}",
            sender,
            entity,
            state.client_point.x,
            state.client_point.y,
            state.local_point.x,
            state.local_point.y,
            state.left_down,
            state.right_down,
            state.ctrl_down,
        );

        // サイズをトグル
        if let Some(mut style) = world.get_mut::<BoxStyle>(entity) {
            if let Some(ref mut size) = style.size {
                let new_size = if size.width == Some(Dimension::Px(100.0)) {
                    150.0
                } else {
                    100.0
                };
                size.width = Some(Dimension::Px(new_size));
                size.height = Some(Dimension::Px(new_size));
                info!(new_size = new_size, "[PointerEvent] BlueBox: New size");
            }
        }

        return true; // イベント処理済み
    }

    false
}
