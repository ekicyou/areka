use super::*;

/// リージョンテスト共通: hit_test_in_window_ex でリージョン名を取得するヘルパー
pub(crate) fn resolve_region_name(
    world: &World,
    entity: Entity,
    state: &PointerState,
) -> Option<String> {
    // ウィンドウエンティティを探索
    let window = find_owner_window(world, entity)?;
    // hit_test_in_window_ex でリージョン名を含む結果を取得
    let result = hit_test_in_window_ex(
        world,
        window,
        PhysicalPoint::new(state.client_point.x as f32, state.client_point.y as f32),
    )?;
    result.region
}

/// リージョンに基づく色を返す（視覚フィードバック用）
pub(crate) fn region_color(region: Option<&str>) -> D2D1_COLOR_F {
    match region {
        Some("top-left") => D2D1_COLOR_F {
            r: 1.0,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        }, // 赤
        Some("top-right") => D2D1_COLOR_F {
            r: 0.2,
            g: 0.8,
            b: 0.2,
            a: 1.0,
        }, // 緑
        Some("bottom-left") => D2D1_COLOR_F {
            r: 0.2,
            g: 0.2,
            b: 1.0,
            a: 1.0,
        }, // 青
        Some("bottom-right") => D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 0.2,
            a: 1.0,
        }, // 黄
        Some(other) => {
            println!("[Region] 不明なリージョン: {}", other);
            D2D1_COLOR_F {
                r: 0.9,
                g: 0.9,
                b: 0.9,
                a: 1.0,
            }
        }
        None => D2D1_COLOR_F {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        }, // 無名（フォールバック）
    }
}

/// リージョンテストボックス共通の OnPointerPressed ハンドラ
///
/// クリック時に hit_test_in_window_ex でリージョン名を取得し、色を変更＋ログ出力
pub(crate) fn on_region_box_pressed(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble フェーズでのみ処理
    if !ev.is_bubble() {
        return false;
    }

    let state = ev.value();
    if !state.left_down {
        return false;
    }

    let entity_name = world
        .get::<Name>(entity)
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| format!("{:?}", entity));

    // リージョン名を取得
    let region = resolve_region_name(world, entity, state);

    info!(
        "[Region] {} pressed: region={:?}, client=({:.1},{:.1}), local=({:.1},{:.1})",
        entity_name,
        region,
        state.client_point.x,
        state.client_point.y,
        state.local_point.x,
        state.local_point.y,
    );

    // リージョンに応じた色に変更
    let color = region_color(region.as_deref());
    if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
        brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(color);
    }

    true
}

/// リージョンテストボックス共通の OnPointerMoved ハンドラ
///
/// ホバー時にリージョン名をログ出力（30フレームに1回）
pub(crate) fn on_region_box_moved(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    if !ev.is_bubble() {
        return false;
    }

    let state = ev.value();

    // 頻度制限: 30回に1回ログ出力
    static REGION_MOVE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let count = REGION_MOVE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if count % 30 != 0 {
        return false;
    }

    let entity_name = world
        .get::<Name>(entity)
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| format!("{:?}", entity));

    let region = resolve_region_name(world, entity, state);

    debug!(
        "[Region] {} hover: region={:?}, client=({:.1},{:.1})",
        entity_name, region, state.client_point.x, state.client_point.y,
    );

    false // 伝播続行
}

/// 通常ヒットテスト領域の OnPointerPressed ハンドラ
///
/// クリックスルーテストの通常領域がクリックされたことを確認するためのログ出力
pub(crate) fn on_normal_hit_box_pressed(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    if !ev.is_bubble() {
        return false;
    }

    let state = ev.value();
    if !state.left_down {
        return false;
    }

    let entity_name = world
        .get::<Name>(entity)
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| format!("{:?}", entity));

    info!(
        "[ClickThrough] Normal region clicked: {} at ({:.1},{:.1})",
        entity_name, state.client_point.x, state.client_point.y,
    );

    false
}
