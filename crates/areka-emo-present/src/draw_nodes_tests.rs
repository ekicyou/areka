use super::*;

use areka_emo_compose::plan::Extent;
use bevy_ecs::hierarchy::Children;

use crate::gpu_atlas::tests::{two_picture_atlas, warp};

#[test]
fn each_mix_maps_to_its_d2d_composite_mode() {
    let table = [
        (Mix::SourceOver, D2D1_COMPOSITE_MODE_SOURCE_OVER),
        (Mix::SourceAtop, D2D1_COMPOSITE_MODE_SOURCE_ATOP),
        (Mix::DestinationOver, D2D1_COMPOSITE_MODE_DESTINATION_OVER),
        (Mix::Copy, D2D1_COMPOSITE_MODE_SOURCE_COPY),
        (Mix::DestinationIn, D2D1_COMPOSITE_MODE_DESTINATION_IN),
    ];
    for (mix, mode) in table {
        assert_eq!(mix_mode_of(mix), mode, "{mix:?}");
    }
}

fn image(element: u32, x: i64, y: i64, mix: Mix) -> PlanItem {
    PlanItem::Image {
        element: ElementId(element),
        transform: Transform::translate(x, y),
        mix,
        range: None,
    }
}

/// `replace` の絵（範囲は絵の位置と原寸）。
fn copy_image(element: u32, x: i64, y: i64, (w, h): (u32, u32)) -> PlanItem {
    PlanItem::Image {
        element: ElementId(element),
        transform: Transform::translate(x, y),
        mix: Mix::Copy,
        range: Some(IRect { x, y, w, h }),
    }
}

fn fill(x: i64, y: i64, w: u32, h: u32) -> PlanItem {
    PlanItem::Fill {
        rect: IRect { x, y, w, h },
    }
}

fn group(x: i64, y: i64, mix: Mix, range: Option<(u32, u32)>) -> PlanItem {
    PlanItem::GroupBegin {
        transform: Transform::translate(x, y),
        mix,
        range: range.map(|(w, h)| IRect { x, y, w, h }),
    }
}

fn plan(items: Vec<PlanItem>) -> DrawPlan {
    DrawPlan {
        surface_id: 0,
        base_surface: 0,
        extent: Extent { w: 32, h: 32 },
        items,
    }
}

/// 絵 2 つ・`replace`・黒の塗り・範囲つきのまとめ（中に絵）・範囲なしのまとめ（中に絵）。
fn mixed_items() -> Vec<PlanItem> {
    vec![
        image(0, 1, 2, Mix::SourceOver),
        copy_image(1, 3, 4, (5, 5)),
        fill(6, 7, 8, 9),
        group(10, 11, Mix::Copy, Some((12, 13))),
        image(0, 2, 3, Mix::SourceAtop),
        PlanItem::GroupEnd,
        group(4, 5, Mix::DestinationIn, None),
        image(1, 6, 7, Mix::DestinationOver),
        PlanItem::GroupEnd,
    ]
}

struct Bed {
    world: World,
    root: Entity,
    gpu: GpuAtlas,
    nodes: DrawNodes,
}

impl Bed {
    fn new() -> Self {
        let mut world = World::new();
        let root = world.spawn(Visual::default()).id();
        Self {
            world,
            root,
            gpu: GpuAtlas::new(&warp(), &two_picture_atlas()).expect("GpuAtlas"),
            nodes: DrawNodes::new(root),
        }
    }

    fn sync(&mut self, items: Vec<PlanItem>) {
        self.nodes
            .sync(&mut self.world, &mut self.gpu, &plan(items))
            .expect("写し");
    }

    /// 根の下の節を描く順（深さ優先・`Children` の順）に。
    fn order(&self) -> Vec<Entity> {
        let mut out = Vec::new();
        let mut stack = vec![self.root];
        while let Some(e) = stack.pop() {
            if e != self.root {
                out.push(e);
            }
            if let Some(kids) = self.world.get::<Children>(e) {
                stack.extend(kids.iter().rev());
            }
        }
        out
    }

    fn parent(&self, e: Entity) -> Entity {
        self.world.get::<ChildOf>(e).expect("親").parent()
    }

    fn offset(&self, e: Entity) -> (f32, f32) {
        let a = self.world.get::<Arrangement>(e).expect("配置");
        (a.offset.x, a.offset.y)
    }

    fn size(&self, e: Entity) -> (f32, f32) {
        let a = self.world.get::<Arrangement>(e).expect("配置");
        (a.size.width, a.size.height)
    }

    fn mix(&self, e: Entity) -> MixMode {
        self.world
            .get::<VisualDrawContent>(e)
            .map(|c| c.mix_mode)
            .or_else(|| self.world.get::<VisualGroup>(e).map(|g| g.mix))
            .expect("描く中身かまとめ")
    }

    /// 版を上げる部品（`Visual`・描く中身・まとめ）か配置が前の `clear_trackers` の後に書かれたか。
    fn touched(&self, e: Entity) -> bool {
        let r = self.world.entity(e);
        r.get_ref::<Visual>().is_some_and(|c| c.is_changed())
            || r.get_ref::<VisualDrawContent>()
                .is_some_and(|c| c.is_changed())
            || r.get_ref::<VisualGroup>().is_some_and(|c| c.is_changed())
            || r.get_ref::<Arrangement>().is_some_and(|c| c.is_changed())
    }

    fn touched_nodes(&self) -> Vec<Entity> {
        self.order()
            .into_iter()
            .filter(|&e| self.touched(e))
            .collect()
    }
}

fn composite(mode: D2D1_COMPOSITE_MODE) -> MixMode {
    MixMode::Composite(mode)
}

#[test]
fn items_map_to_nodes_in_draw_order_with_clips_and_groups() {
    let mut bed = Bed::new();
    bed.sync(mixed_items());

    let n = bed.order();
    // 絵・(切り抜き・絵)・塗り・(切り抜き・まとめ・絵)・(まとめ・絵)
    let [img0, clip1, img1, fill2, clip3, grp3, img4, grp6, img7] =
        <[Entity; 9]>::try_from(n.clone()).expect("節は 9 つ");

    let root = bed.root;
    assert_eq!(bed.parent(img0), root);
    assert_eq!(bed.mix(img0), composite(D2D1_COMPOSITE_MODE_SOURCE_OVER));
    assert_eq!(bed.offset(img0), (1.0, 2.0));
    assert_eq!(bed.size(img0), (4.0, 3.0), "絵 0 の原寸");

    assert_eq!(bed.parent(clip1), root);
    assert_eq!(
        bed.world.get::<Visual>(clip1).expect("切り抜き").clip,
        Some(ClipShape::Rectangle)
    );
    assert!(bed.world.get::<VisualTransform>(clip1).is_some());
    assert_eq!(bed.offset(clip1), (3.0, 4.0));
    assert_eq!(bed.size(clip1), (5.0, 5.0));
    assert_eq!(bed.parent(img1), clip1);
    assert_eq!(bed.offset(img1), (0.0, 0.0), "切り抜きの左上からの位置");
    assert_eq!(bed.mix(img1), composite(D2D1_COMPOSITE_MODE_SOURCE_COPY));

    assert_eq!(bed.parent(fill2), root);
    assert_eq!(bed.offset(fill2), (6.0, 7.0));
    assert_eq!(bed.size(fill2), (8.0, 9.0));
    assert_eq!(bed.mix(fill2), composite(D2D1_COMPOSITE_MODE_SOURCE_OVER));

    assert_eq!(bed.parent(clip3), root);
    assert_eq!(bed.offset(clip3), (10.0, 11.0));
    assert_eq!(bed.size(clip3), (12.0, 13.0));
    assert_eq!(bed.parent(grp3), clip3);
    assert_eq!(bed.offset(grp3), (0.0, 0.0));
    assert_eq!(bed.mix(grp3), composite(D2D1_COMPOSITE_MODE_SOURCE_COPY));
    assert_eq!(bed.parent(img4), grp3, "まとめの中の絵はまとめの子");
    assert_eq!(bed.offset(img4), (2.0, 3.0), "まとめの左上からの位置のまま");
    assert_eq!(bed.mix(img4), composite(D2D1_COMPOSITE_MODE_SOURCE_ATOP));

    assert_eq!(bed.parent(grp6), root, "閉じた後の項は根へ戻る");
    assert_eq!(bed.offset(grp6), (4.0, 5.0));
    assert_eq!(bed.mix(grp6), composite(D2D1_COMPOSITE_MODE_DESTINATION_IN));
    assert_eq!(bed.parent(img7), grp6);
    assert_eq!(
        bed.mix(img7),
        composite(D2D1_COMPOSITE_MODE_DESTINATION_OVER)
    );

    for e in n {
        assert_eq!(
            bed.world.get::<HitTest>(e).map(|h| h.mode),
            Some(HitTest::none().mode),
            "節は当たりを持たない"
        );
    }
}

#[test]
fn same_plan_twice_keeps_every_node_untouched() {
    let mut bed = Bed::new();
    bed.sync(mixed_items());
    let before = bed.order();
    assert_eq!(before.len(), 9);
    bed.world.clear_trackers();

    bed.sync(mixed_items());

    assert_eq!(bed.order(), before, "同じ実体のまま");
    assert_eq!(
        bed.touched_nodes(),
        Vec::<Entity>::new(),
        "版を上げる書き込みが無い"
    );
}

#[test]
fn changing_one_item_rewrites_only_its_node() {
    let mut bed = Bed::new();
    bed.sync(mixed_items());
    let before = bed.order();

    let cases: Vec<(usize, PlanItem, usize)> = vec![
        // (項の添字, 差し替える項, 替わる節の `order` の添字)
        (0, image(1, 1, 2, Mix::SourceOver), 0),
        (0, image(0, 9, 9, Mix::SourceOver), 0),
        (2, fill(6, 7, 1, 1), 3),
        (4, image(0, 2, 3, Mix::DestinationIn), 6),
        (6, group(4, 5, Mix::SourceAtop, None), 7),
    ];
    for (i, item, node) in cases {
        bed.sync(mixed_items());
        bed.world.clear_trackers();
        let mut items = mixed_items();
        items[i] = item.clone();

        bed.sync(items);

        assert_eq!(bed.order(), before, "{item:?}: 実体は作り直さない");
        assert_eq!(bed.touched_nodes(), vec![before[node]], "{item:?}");
    }
}

#[test]
fn changing_a_replace_range_moves_its_clip_and_keeps_the_leaf() {
    let mut bed = Bed::new();
    bed.sync(mixed_items());
    let before = bed.order();
    bed.world.clear_trackers();
    let mut items = mixed_items();
    items[1] = copy_image(1, 8, 9, (5, 5));

    bed.sync(items);

    assert_eq!(bed.order(), before);
    assert_eq!(
        bed.touched_nodes(),
        vec![before[1]],
        "切り抜きの節の位置だけ"
    );
    assert_eq!(bed.offset(before[1]), (8.0, 9.0));
    assert_eq!(bed.offset(before[2]), (0.0, 0.0));
}

#[test]
fn changing_shape_rebuilds_from_that_item_and_removes_the_rest() {
    let mut bed = Bed::new();
    bed.sync(mixed_items());
    let before = bed.order();
    let mut items = mixed_items();
    // 塗り → 絵（種類が違う）。
    items[2] = image(1, 6, 7, Mix::SourceOver);

    bed.sync(items);

    let after = bed.order();
    assert_eq!(after.len(), before.len());
    assert_eq!(after[..3], before[..3], "前の項の節は残す");
    for e in &before[3..] {
        assert!(
            bed.world.get_entity(*e).is_err(),
            "{e:?}: 違う項から後ろは消す"
        );
        assert!(!after.contains(e));
    }
    assert_eq!(
        bed.mix(after[3]),
        composite(D2D1_COMPOSITE_MODE_SOURCE_OVER)
    );
    assert_eq!(bed.size(after[3]), (5.0, 5.0), "絵 1 の原寸");
}

#[test]
fn changing_nesting_rebuilds_from_where_the_shape_differs() {
    let mut bed = Bed::new();
    bed.sync(mixed_items());
    let before = bed.order();
    let mut items = mixed_items();
    // まとめの中の絵の後ろで閉じず、次の絵も中に入れる。
    items.swap(5, 6);
    items[5] = image(1, 0, 0, Mix::SourceOver);
    items[6] = PlanItem::GroupEnd;
    items.truncate(7);

    bed.sync(items);

    let after = bed.order();
    assert_eq!(after[..7], before[..7], "同じ形の前の項は残す");
    assert_eq!(after.len(), 8);
    assert_eq!(
        bed.parent(after[7]),
        before[5],
        "新しい絵は開いたままのまとめの子"
    );
    for e in &before[7..] {
        assert!(bed.world.get_entity(*e).is_err(), "{e:?}");
    }
}

#[test]
fn a_range_appearing_rebuilds_the_item() {
    let mut bed = Bed::new();
    bed.sync(mixed_items());
    let before = bed.order();
    let mut items = mixed_items();
    items[0] = copy_image(0, 1, 2, (4, 3));

    bed.sync(items);

    let after = bed.order();
    assert!(bed.world.get_entity(before[0]).is_err(), "範囲の有無は形");
    assert_eq!(bed.parent(after[1]), after[0], "絵は新しい切り抜きの子");
    assert_eq!(after.len(), before.len() + 1);
}

#[test]
fn shorter_plan_removes_the_left_over_nodes() {
    let mut bed = Bed::new();
    bed.sync(mixed_items());
    let before = bed.order();
    bed.world.clear_trackers();

    bed.sync(mixed_items()[..3].to_vec());

    assert_eq!(bed.order(), before[..4].to_vec());
    assert_eq!(bed.touched_nodes(), Vec::<Entity>::new());
    for e in &before[4..] {
        assert!(bed.world.get_entity(*e).is_err(), "{e:?}");
    }

    bed.sync(Vec::new());
    assert!(bed.order().is_empty(), "空の計画は節を全部消す");
}
