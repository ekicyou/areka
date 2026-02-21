use super::*;

/// FlexContainer の OnPointerPressed ハンドラ（拡張版）
///
/// **Tunnelフェーズ**: Ctrl+左クリックでキャプチャ（条件付き前処理の例）
/// **Bubbleフェーズ**: 右クリックで色変更（既存）
///
/// # パラメータ
/// - `sender`: イベント発生元エンティティ（e.OriginalSource相当）
/// - `entity`: 現在処理中のエンティティ（e.currentTarget相当）
/// - `ev`: Tunnel/Bubbleフェーズを含むイベント情報
///
/// # 戻り値
/// - `true`: イベント伝播を停止（stopPropagation相当）
/// - `false`: イベント伝播を継続
pub(crate) fn on_container_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let wlabel = window_label(world, entity);
    match ev {
        Phase::Tunnel(state) => {
            // Ctrl+左クリックでイベントを停止
            if state.ctrl_down && state.left_down {
                info!(
                    "[Tunnel] FlexContainer: Event stopped at Container (Ctrl+Left), sender={:?}, entity={:?}, {}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    wlabel,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                // コンテナの色をピンクに変更
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                        r: 1.0,
                        g: 0.4,
                        b: 0.8,
                        a: 1.0,
                    });
                }

                return true; // イベント停止、子に到達しない
            }

            info!(
                "[Tunnel] FlexContainer: Passing through, sender={:?}, entity={:?}, {}",
                sender, entity, wlabel,
            );
            false
        }
        Phase::Bubble(state) => {
            // 右クリック検出
            if state.right_down {
                info!(
                    "[Bubble] FlexContainer: Right-click detected! sender={:?}, entity={:?}, {}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    wlabel,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                // コンテナの色をピンクに変更
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                        r: 1.0,
                        g: 0.7,
                        b: 0.8,
                        a: 1.0,
                    });
                }

                return true; // イベント処理済み
            }

            false
        }
    }
}

/// FlexContainer の OnDragStart ハンドラ
///
/// ドラッグ開始時に初期inset値を記録する。
pub(crate) fn on_container_drag_start(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragStartEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let sender_name = world
                .get::<Name>(sender)
                .map(|n| n.as_str())
                .unwrap_or("unknown");
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str())
                .unwrap_or("unknown");

            info!(
                "[Drag] DragStart: sender={}, entity={}, pos=({},{})",
                sender_name, entity_name, event.position.x, event.position.y
            );

            // ウィンドウエンティティを探索してドラッグ開始位置を記録
            // これはDraggingStateとして保存される（DraggingStateには既にdrag_start_posがある）

            false
        }
    }
}

/// FlexContainer の OnDrag ハンドラ
///
/// ドラッグ中のログ出力を行う。
/// ウィンドウ位置の更新はフレームワークのWndProcレベル直接SetWindowPosが
/// 自動的に処理する（DragConfig.move_window = true）。
pub(crate) fn on_container_drag(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let sender_name = world
                .get::<Name>(sender)
                .map(|n| n.as_str())
                .unwrap_or("unknown");
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str())
                .unwrap_or("unknown");

            // start_positionとpositionから移動量を計算（ログ出力用）
            let delta_x = event.position.x - event.start_position.x;
            let delta_y = event.position.y - event.start_position.y;

            debug!(
                "[Drag] Drag: sender={}, entity={}, pos=({},{}), delta=({},{})",
                sender_name, entity_name, event.position.x, event.position.y, delta_x, delta_y
            );

            false
        }
    }
}

/// FlexContainer の OnDragEnd ハンドラ
pub(crate) fn on_container_drag_end(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragEndEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let sender_name = world
                .get::<Name>(sender)
                .map(|n| n.as_str())
                .unwrap_or("unknown");
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str())
                .unwrap_or("unknown");

            info!(
                "[Drag] DragEnd: sender={}, entity={}, pos=({},{}), cancelled={}",
                sender_name, entity_name, event.position.x, event.position.y, event.cancelled
            );
            false
        }
    }
}
