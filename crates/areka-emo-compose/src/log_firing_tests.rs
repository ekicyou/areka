//! ログ発火経路の決定論的檻（要件 1.4/3.3/4.3/7.3/8.4/10.5 が明示的に義務づける `warn!`/`error!`）。
//!
//! 既存テストは観測可能な副作用（None/skip/Err/非パニック）のみを固定していたが、要件は
//! **ログの発火**そのものを義務づける。本モジュールは [`crate::log_capture::capture_logs`] で
//! 各失敗経路のログを捕捉し、(a) 決定論的入力で経路を起こし、(b) 捕捉テキストに **level
//! （WARN/ERROR）＋ target（`areka_emo_compose`）＋ discriminating field**（surface_id/method/
//! key/path/id）が現れることを assert し、(c) 従来どおり観測可能な副作用も併せて固定する。
//!
//! # 非空虚性（RED プローブ）
//!
//! 各ログ assert は level と discriminating field を同時に突くため、当該 `warn!`/`error!` 行を
//! 削れば必ず失敗する。cycle・unimplemented-method・error! の各ケースについては、production の
//! ログ行をコメントアウトすると本モジュールの対応 assert が RED になることを確認済み（本番は
//! `git diff` で無改変を確認）。

use super::*;

use areka_emo_atlas::{
    AlphaParams, AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, MemoryDecoder, PackConfig,
    Placement, Point, Rect, SetId, Size, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::{
    Animation, AppendTarget, DefRef, DrawMethod, Element, ElementPath, Interval, Pattern, Shell,
    Surface, SurfaceAppend,
};
use std::path::Path;
use std::sync::Arc;

use crate::log_capture::capture_logs;
use crate::method::{BlendKind, BlendMode, ComposeMethod};
use crate::normalized::Transform;
use crate::plan::{BlitOp, build_plan};

// ── 合成モデルビルダ（plan.rs/composer_tests と同一パターン・headless bake 経路）──────────

fn elem(layer: u32, path: &str, x: i64, y: i64) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

fn surface(id: u32, elements: Vec<Element>) -> Surface {
    surface_with_anims(id, elements, Vec::new())
}

fn surface_with_anims(id: u32, elements: Vec<Element>, animations: Vec<Animation>) -> Surface {
    Surface {
        id,
        targets: vec![AppendTarget::Single(id)],
        elements,
        collisions: Vec::new(),
        animations,
    }
}

/// bind animation 1 本（interval=`Interval::Bind`・pattern0 が入れ子 surface_id を x,y で参照）。
fn bind_anim(id: u32, ref_surface_id: i64, x: i64, y: i64) -> Animation {
    Animation {
        id,
        interval: Interval::Bind,
        patterns: vec![Pattern {
            index: 0,
            method: DrawMethod::new("overlay".to_string()),
            surface_id: ref_surface_id,
            wait: 0,
            x,
            y,
        }],
    }
}

fn shell_of(surfaces: Vec<Surface>) -> Shell {
    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    }
}

/// surface/append 混在の登場順 definitions で `Shell` を組む（呼び手が DefRef を直接並べる）。
fn shell_mixed(surfaces: Vec<Surface>, appends: Vec<SurfaceAppend>, defs: Vec<DefRef>) -> Shell {
    Shell {
        surfaces,
        appends,
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions: defs,
    }
}

/// tightly-packed 2×2 の premultiplied BGRA・不透明画像スペック。
fn opaque_2x2() -> (u32, u32, u32, Vec<u8>, bool) {
    let (w, h) = (2u32, 2u32);
    let stride = w * 4;
    let bgra = vec![64u8; (stride * h) as usize];
    (w, h, stride, bgra, true)
}

fn register(dec: &mut MemoryDecoder, base: &Path, rel: &str) {
    let (w, h, stride, bgra, has_alpha) = opaque_2x2();
    dec.insert(base.join(rel), w, h, stride, bgra, has_alpha);
}

/// 指定 rel_path 群を含む単一 SurfaceSet を bake し `AtlasTable` を得る（COM/WIC 非依存・SetId(0)）。
fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
    let elements: Vec<Element> = rels.iter().map(|r| elem(0, r, 0, 0)).collect();
    let surfaces = vec![surface(0, elements)];
    let mut dec = MemoryDecoder::new();
    for r in rels {
        register(&mut dec, base, r);
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let result = bake(&[set], &dec, PackConfig::default());
    assert!(result.errors.is_empty(), "bake セットアップは失敗しない");
    result.table
}

// ── 7.3: 循環検出 warn（ops 経路・flatten_surface）────────────────────────────────────

/// 要件 7.3: 自己参照 surface の合成で循環 WARN が surface_id つきで発火する（＋部分結果は非パニック）。
///
/// surface 1000 が自身の bind pattern0 で 1000 を参照する（自己参照）。build_plan（→derive_ops→
/// flatten_surface）は循環枝を打ち切り WARN を出す。要件 7.3 はこのログが本体。
#[test]
fn cycle_detection_warn_fires_with_surface_id() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["self.png"]);

    let host = surface_with_anims(
        1000,
        vec![elem(0, "self.png", 0, 0)],
        vec![bind_anim(1, 1000, 0, 0)], // 自己参照＝循環。
    );
    let mut world = EmoWorld::build(&shell_of(vec![host]));
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut ops = Vec::new();
    let mut visited = Vec::new();
    let logs = capture_logs(|| {
        // build_plan は compute_extent（extent 経路）→ derive_ops（ops 経路）の両方を走らせる。
        // ops 経路（flatten_surface）の循環 WARN を狙って結果自体も固定する。
        let extent = build_plan(
            &mut ops,
            &mut visited,
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("自己参照でも部分結果で Ok（非パニック・要件 7.1）");
        // 部分結果: 静的 self.png 1 本が積まれ、自己参照枝は打ち切られる。
        assert_eq!(ops.len(), 1, "静的層のみ（自己参照枝は打ち切り）");
        assert_ne!(extent.w, 0, "外形は非ゼロ");
    });

    // ops 経路（flatten_surface）の循環 WARN を **行単位で判別**して突く。build_plan は
    // compute_extent（extent 経路の循環 WARN・「外形算出:」接頭）も走らせるため、単純な
    // contains では extent 経路の WARN に masking されうる。ops 経路の WARN は「外形算出」を
    // **含まない** 循環行として一意に判別する（この行を消せば必ず落ちる・非空虚性）。
    let ops_cycle_line = logs.lines().find(|l| {
        l.contains("level=WARN")
            && l.contains("target=areka_emo_compose")
            && l.contains("surface_id=1000")
            && l.contains("循環")
            && !l.contains("外形算出")
    });
    assert!(
        ops_cycle_line.is_some(),
        "ops 経路（flatten_surface）の循環 WARN（外形算出を含まない）が発火する: {logs}",
    );
}

/// 要件 7.3: 外形算出経路（flatten_extent）の循環 WARN も surface_id つきで発火する。
///
/// build_plan は compute_extent を先に走らせるため、flatten_extent の循環 WARN（メッセージに
/// 「外形算出」を含む）が別途発火する。extent 経路の循環ログを個別に固定する。
#[test]
fn extent_cycle_warn_fires_with_surface_id() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["self.png"]);

    let host = surface_with_anims(
        2000,
        vec![elem(0, "self.png", 0, 0)],
        vec![bind_anim(1, 2000, 0, 0)], // 自己参照。
    );
    let mut world = EmoWorld::build(&shell_of(vec![host]));
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::default(); // extent は有効 bind 非依存ゆえ空 bind でも全 pattern0 を辿る。
    let mut ops = Vec::new();
    let mut visited = Vec::new();
    let logs = capture_logs(|| {
        build_plan(
            &mut ops,
            &mut visited,
            &world,
            &atlas,
            2000,
            &binds,
            &PatternState::default(),
        )
        .expect("自己参照でも Ok（外形は非ゼロ）");
    });

    // extent 経路（flatten_extent）特有の循環 WARN を **行単位で判別**する（「外形算出」を含む
    // 循環行）。ops 経路の循環 WARN とは別の行として一意に突く（この行を消せば必ず落ちる）。
    let extent_cycle_line = logs.lines().find(|l| {
        l.contains("level=WARN")
            && l.contains("target=areka_emo_compose")
            && l.contains("surface_id=2000")
            && l.contains("循環")
            && l.contains("外形算出")
    });
    assert!(
        extent_cycle_line.is_some(),
        "extent 経路（flatten_extent）の循環 WARN（外形算出を含む）が発火する: {logs}",
    );
}

// ── 11.2: pattern0（index==0）なし bind の良性 skip DEBUG（ops 経路・flatten_surface）──────

/// 要件 9.1/9.3: pattern0（index==0）を持たない有効 bind は **DEBUG**（surface_id＋animation_id つき）を
/// 出して skip する（非パニック・WARN/ERROR は出さない）。まばたき等 `interval,bind+random` 再生
/// アニメの静的不活性（実機第2欠陥の是正・task 11.2）。
///
/// bind id=1400 は pattern0 を持たず index=1 のみ（surface 1412＝閉じ目相当）。build_plan（→derive_ops
/// →flatten_surface）は index==0 不在ゆえ命令を積まず DEBUG を出す。extent 経路（flatten_extent・:529）は
/// サイズ不変契約のため全 bind pattern（min_by_key）を辿るが DEBUG は出さない＝DEBUG は 1 本だけ。
#[test]
fn pattern0_less_bind_skip_fires_debug_and_no_warn_error() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["closed.png"]);

    // pattern0 なし・index=1 のみが surface 1412 を参照する再生アニメ（まばたき相当）。
    let blink = Animation {
        id: 1400,
        interval: Interval::BindRandom { k: 4 },
        patterns: vec![Pattern {
            index: 1,
            method: DrawMethod::new("overlay".to_string()),
            surface_id: 1412,
            wait: 0,
            x: 0,
            y: 0,
        }],
    };
    let host = surface_with_anims(1000, Vec::new(), vec![blink]);
    let part = surface(1412, vec![elem(0, "closed.png", 0, 0)]);
    let mut world = EmoWorld::build(&shell_of(vec![host, part]));
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1400]);
    let mut ops = Vec::new();
    let mut visited = Vec::new();
    let logs = capture_logs(|| {
        build_plan(
            &mut ops,
            &mut visited,
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("pattern0 なし bind でも surface1000 存在＋外形非ゼロで Ok（要件 6.6）");
    });

    // 副作用: pattern0 なし bind は静的合成へ寄与しない＝描画命令ゼロ（要件 9.1）。
    assert!(
        ops.is_empty(),
        "pattern0（index==0）なし bind は描画命令に現れない（要件 9.1）"
    );

    // DEBUG skip ログを level＋target＋discriminating field（surface_id/animation_id）で突く
    // （この DEBUG 行を消せば必ず落ちる＝非空虚）。
    let skip_line = logs.lines().find(|l| {
        l.contains("level=DEBUG")
            && l.contains("target=areka_emo_compose")
            && l.contains("surface_id=1000")
            && l.contains("animation_id=1400")
    });
    assert!(
        skip_line.is_some(),
        "pattern0 なし skip の DEBUG（surface_id＋animation_id つき）が発火する: {logs}",
    );
    // DEBUG は厳密に 1 本（ops 経路の skip のみ・extent 経路は DEBUG を出さない）。
    let debug_count = logs.lines().filter(|l| l.contains("level=DEBUG")).count();
    assert_eq!(
        debug_count, 1,
        "pattern0 なし skip の DEBUG は 1 本のみ: {logs}"
    );
    // 良性 skip ゆえ WARN/ERROR は出さない（要件 9.3）。
    assert!(
        !logs.contains("level=WARN"),
        "良性 skip は WARN を出さない: {logs}"
    );
    assert!(
        !logs.contains("level=ERROR"),
        "良性 skip は ERROR を出さない: {logs}"
    );
}

// ── 8.4: 未実装メソッド warn（blit.rs execute）────────────────────────────────────────

/// premultiplied BGRA 1 画素頁を組む。
fn page_1px(b: u8, g: u8, r: u8, a: u8) -> AtlasPage {
    AtlasPage {
        width: 1,
        height: 1,
        stride: 4,
        bytes: Arc::from(vec![b, g, r, a]),
    }
}

/// 単一エントリ（`ElementId(0)`）＋単一頁の AtlasTable を組む（uv/trim 指定）。
fn single_entry_table(page: AtlasPage, uv: Rect, trim: Point) -> AtlasTable {
    let keys = vec![AtlasKey {
        set: SetId(0),
        rel_path: "e.png".into(),
    }];
    let entries = vec![AtlasEntry {
        original: Size { w: uv.w, h: uv.h },
        placement: Some(Placement {
            page: 0,
            uv_rect: uv,
            trim_offset: trim,
        }),
    }];
    AtlasTable::new(keys, entries, vec![page])
}

/// 要件 8.4: 未実装メソッド命令（Replace/Base/Blend）は WARN（method＋element つき）を出して skip する。
///
/// blit::execute へ未実装メソッド命令を渡すと、method の Debug 値と element id を載せた WARN が
/// 発火し、その命令は合成へ寄与しない（skip）。3 種の未実装メソッドで method フィールドの判別も突く。
#[test]
fn unimplemented_method_warn_fires_with_method_and_element() {
    let page = page_1px(200, 150, 100, 255);
    let atlas = single_entry_table(
        page,
        Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        },
        Point { x: 0, y: 0 },
    );

    let cases = [
        (ComposeMethod::Replace, "Replace"),
        (ComposeMethod::Base, "Base"),
        (
            ComposeMethod::Blend(BlendMode::new(BlendKind::Multiply)),
            "Blend",
        ),
    ];
    for (method, marker) in cases {
        let ops = vec![BlitOp {
            element: ElementId(0),
            transform: Transform::translate(0, 0),
            method,
        }];
        let mut out = ComposedSurface::new(0, 0);
        let logs = capture_logs(|| {
            crate::blit::execute(&mut out, crate::plan::Extent { w: 1, h: 1 }, &ops, &atlas);
        });

        // 副作用: 未実装ゆえ何も合成されず全透明のまま（skip・非パニック）。
        assert!(
            out.bytes().iter().all(|&b| b == 0),
            "{marker}: 未実装は合成へ寄与しない（skip）",
        );
        // ログ: WARN・target・method（Debug 判別子）・element を同時に突く。
        assert!(
            logs.contains("level=WARN"),
            "{marker}: 未実装は WARN: {logs}"
        );
        assert!(
            logs.contains("target=areka_emo_compose"),
            "{marker}: target: {logs}"
        );
        assert!(
            logs.contains("element=0"),
            "{marker}: element id を載せる: {logs}"
        );
        assert!(
            logs.contains(&format!("method={marker}")),
            "{marker}: method の判別子を載せる: {logs}",
        );
    }
}

/// 要件 1.4（防御ログ）: 破損アトラス（placement.page が範囲外）は WARN（element＋page つき）で skip。
///
/// design「Error Handling」表の頁欠落 warn を檻に入れる（cheap にトリガ可能ゆえ併せて固定）。
#[test]
fn missing_page_warn_fires_with_element_and_page() {
    let keys = vec![AtlasKey {
        set: SetId(0),
        rel_path: "e.png".into(),
    }];
    let entries = vec![AtlasEntry {
        original: Size { w: 1, h: 1 },
        placement: Some(Placement {
            page: 5, // 頁が無い（pages 空）。
            uv_rect: Rect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            trim_offset: Point { x: 0, y: 0 },
        }),
    }];
    let atlas = AtlasTable::new(keys, entries, Vec::new());
    let ops = vec![BlitOp {
        element: ElementId(0),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }];

    let mut out = ComposedSurface::new(0, 0);
    let logs = capture_logs(|| {
        crate::blit::execute(&mut out, crate::plan::Extent { w: 1, h: 1 }, &ops, &atlas);
    });

    assert!(
        out.bytes().iter().all(|&b| b == 0),
        "頁欠落は skip（非パニック）"
    );
    assert!(logs.contains("level=WARN"), "頁欠落は WARN: {logs}");
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(logs.contains("element=0"), "element id: {logs}");
    assert!(logs.contains("page=5"), "欠落頁番号を載せる: {logs}");
}

// ── 3.3: 未解決 alias warn（world.rs resolve_alias）─────────────────────────────────

/// 要件 3.3: 未解決 alias キーの解決で WARN（key つき）が発火し、返り値は None。
#[test]
fn unresolved_alias_warn_fires_with_key() {
    let world = EmoWorld::build(&shell_of(Vec::new()));

    let mut result = Some(&[][..]);
    let logs = capture_logs(|| {
        result = world.resolve_alias("nope");
    });

    // 副作用: 未解決は None（非パニック・要件 3.3）。
    assert_eq!(result, None, "未解決 alias は None");
    // ログ: WARN・target・key を同時に突く。
    assert!(logs.contains("level=WARN"), "未解決 alias は WARN: {logs}");
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(logs.contains("key=\"nope\""), "未解決 key を載せる: {logs}");
}

// ── 10.5: SurfaceNotFound / EmptyComposition error（plan.rs build_plan / Composer::compose）──

/// 要件 10.5: 不在 surface の合成で ERROR（surface_id つき）が発火し、Err(SurfaceNotFound)。
#[test]
fn surface_not_found_error_fires_with_surface_id() {
    let world = EmoWorld::build(&shell_of(Vec::new())); // surface 皆無。
    let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);

    let binds = BindSet::default();
    let mut composer = Composer::new();
    let mut result = Ok(ComposedSurface::new(0, 0));
    let logs = capture_logs(|| {
        result = composer.compose(&world, &atlas, 9999, &binds, &PatternState::default());
    });

    // 副作用: Err(SurfaceNotFound)（要件 10.5）。
    assert_eq!(
        result.err(),
        Some(ComposeError::SurfaceNotFound(9999)),
        "不在 surface は SurfaceNotFound",
    );
    // ログ: ERROR・target・surface_id を同時に突く。
    assert!(
        logs.contains("level=ERROR"),
        "不在 surface は ERROR: {logs}"
    );
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(
        logs.contains("surface_id=9999"),
        "不在 surface_id を載せる: {logs}"
    );
    assert!(logs.contains("SurfaceNotFound"), "分類名が載る: {logs}");
}

/// 要件 10.5: 定義層皆無で外形 0×0 の退化で ERROR（surface_id つき）が発火し、Err(EmptyComposition)。
#[test]
fn empty_composition_error_fires_with_surface_id() {
    // element ゼロ・animation ゼロの surface（存在するが定義層皆無）。
    let mut world = EmoWorld::build(&shell_of(vec![surface(7000, Vec::new())]));
    let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::default();
    let mut composer = Composer::new();
    let mut result = Ok(ComposedSurface::new(0, 0));
    let logs = capture_logs(|| {
        result = composer.compose(&world, &atlas, 7000, &binds, &PatternState::default());
    });

    // 副作用: Err(EmptyComposition)（要件 10.5・議題2裁定）。
    assert_eq!(
        result.err(),
        Some(ComposeError::EmptyComposition(7000)),
        "定義層皆無 0×0 は EmptyComposition",
    );
    // ログ: ERROR・target・surface_id を同時に突く。
    assert!(logs.contains("level=ERROR"), "退化データは ERROR: {logs}");
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(
        logs.contains("surface_id=7000"),
        "退化 surface_id を載せる: {logs}"
    );
    assert!(logs.contains("EmptyComposition"), "分類名が載る: {logs}");
}

// ── 1.4: fold の重なり warn（fold.rs upsert_surface / fold_append）────────────────────

/// 要件 1.4/2.1: surface id 重複（後勝ち全置換）で WARN（id つき）が発火する。
///
/// 同一 id を 2 度 plain 定義すると、後の定義が前を全置換し、de-facto 挙動として WARN を出す。
#[test]
fn duplicate_surface_id_replacement_warn_fires_with_id() {
    let first = surface(7, vec![elem(0, "old.png", 0, 0)]);
    let second = surface(7, vec![elem(0, "new.png", 5, 6)]); // 同 id 再定義＝後勝ち置換。

    let logs = capture_logs(|| {
        // fold は EmoWorld::build 内で走る（populate_from_shell→fold_shell→upsert_surface）。
        let world = EmoWorld::build(&shell_of(vec![first, second]));
        // 副作用: 後勝ちで new.png が残る（前提の実証）。
        let master = world.surface(7).expect("id=7 常駐");
        assert_eq!(master.elements.len(), 1, "全置換で 1 本のみ");
        assert_eq!(master.elements[0].path.as_str(), "new.png", "後勝ち");
    });

    assert!(logs.contains("level=WARN"), "id 重複は WARN: {logs}");
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(logs.contains("id=7"), "重複 id を載せる: {logs}");
    assert!(logs.contains("重複"), "id 重複メッセージ: {logs}");
}

/// 要件 1.4/2.2: append 対象 id が未存在なら WARN（id つき）が発火し、新設しない（存在条件付き）。
#[test]
fn nonexistent_append_target_warn_fires_with_id() {
    // 事前 surface は id=10 のみ。append は 10（存在）と 999（未存在）を狙う。
    let s10 = surface(10, vec![elem(0, "base.png", 0, 0)]);
    let ap = SurfaceAppend {
        targets: vec![AppendTarget::Single(10), AppendTarget::Single(999)],
        elements: vec![elem(1, "added.png", 3, 4)],
        collisions: Vec::new(),
        animations: Vec::new(),
    };

    let logs = capture_logs(|| {
        let world = EmoWorld::build(&shell_mixed(
            vec![s10],
            vec![ap],
            vec![DefRef::Surface(0), DefRef::Append(0)],
        ));
        // 副作用: 10 には added.png が届き、999 は新設されない（存在条件付き）。
        let m10 = world.surface(10).expect("id=10 常駐");
        assert!(
            m10.elements.iter().any(|e| e.path.as_str() == "added.png"),
            "既存 10 には追記が届く",
        );
        assert!(world.surface(999).is_none(), "未存在 999 は新設されない");
    });

    assert!(
        logs.contains("level=WARN"),
        "未存在 append 対象は WARN: {logs}"
    );
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(
        logs.contains("id=999"),
        "未存在 append 対象 id を載せる: {logs}"
    );
    assert!(logs.contains("未存在"), "未存在メッセージ: {logs}");
}

// ── 4.3: 未束縛 element warn（atlas_bind.rs bind_atlas）───────────────────────────────

/// 要件 4.3: atlas に無い element path を bind すると WARN（path つき）が発火し、AtlasBinding は None。
#[test]
fn unbound_element_warn_fires_with_path() {
    let base = Path::new("shell/master");
    // atlas には known.png のみ。bogus.png は未束縛＝None になる。
    let atlas = bake_atlas(base, &["known.png"]);

    let shell = shell_of(vec![surface(
        1000,
        vec![elem(0, "known.png", 0, 0), elem(1, "bogus.png", 0, 0)],
    )]);
    let mut world = EmoWorld::build(&shell);

    let logs = capture_logs(|| {
        world.bind_atlas(&atlas, SetId(0));
    });

    // 副作用: 未知 path は None（非パニック・要件 4.3）。
    let master = world.surface(1000).expect("surface1000 常駐");
    let entity = *world
        .world()
        .resource::<crate::world::SurfaceIndex>()
        .0
        .get(&1000)
        .expect("entity");
    let binding = world
        .world()
        .get::<crate::world::AtlasBinding>(entity)
        .expect("AtlasBinding");
    let bogus_idx = master
        .elements
        .iter()
        .position(|e| e.path.as_str() == "bogus.png")
        .expect("bogus element");
    assert_eq!(binding.0[bogus_idx], None, "未束縛 element は None");

    // ログ: WARN・target・path を同時に突く。
    assert!(
        logs.contains("level=WARN"),
        "未束縛 element は WARN: {logs}"
    );
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(
        logs.contains("path=\"bogus.png\""),
        "未束縛 path を載せる: {logs}"
    );
    // surface_id フィールドも載る（判別強化）。
    assert!(
        logs.contains("surface_id=1000"),
        "surface_id も載る: {logs}"
    );
}

// ── Part 3-9: ComposeError Display 文字列（決定論的・error.rs #[error("...")]）──────────

/// ComposeError の Display（thiserror `#[error]`）が正準文字列と厳密一致する（決定論的）。
#[test]
fn compose_error_display_strings_are_exact() {
    assert_eq!(
        ComposeError::SurfaceNotFound(7).to_string(),
        "surface 7 not found",
    );
    assert_eq!(
        ComposeError::EmptyComposition(7).to_string(),
        "surface 7 has no layers at all (extent 0x0)",
    );
}

// ── Part 3-10: blit uv_rect.x/y != 0（6.1 review が直接未カバーと指摘した源矩形オフセット）──

/// 要件 6.1/6.4: エントリの uv_rect.x/y が非ゼロ（頁内サブ矩形オフセット）でも、頁原点でなく
/// **uv オフセット位置**の源画素が転写される。
///
/// 3×1 の頁に [赤, 緑, 青] を横並びに置き、uv_rect=(x:2,y:0,w:1,h:1) で **青のみ**を切り出す
/// エントリを組む。合成結果 (0,0) が青（頁原点の赤でない）であればサンプル位置が uv.x で
/// 正しくオフセットされている証左。手計算バイトで厳密一致を突く。
#[test]
fn blit_reads_source_at_nonzero_uv_offset() {
    // 3×1 premultiplied BGRA 頁: (0,0)=赤 [0,0,255] / (1,0)=緑 [0,255,0] / (2,0)=青 [255,0,0]。
    let bytes = vec![
        0, 0, 255, 255, // (0,0) 赤
        0, 255, 0, 255, // (1,0) 緑
        255, 0, 0, 255, // (2,0) 青
    ];
    let page = AtlasPage {
        width: 3,
        height: 1,
        stride: 12,
        bytes: Arc::from(bytes),
    };
    // uv.x=2（青の列）を 1×1 で切り出す。trim_offset=0 ゆえ着弾は配置 (0,0)。
    let atlas = single_entry_table(
        page,
        Rect {
            x: 2,
            y: 0,
            w: 1,
            h: 1,
        },
        Point { x: 0, y: 0 },
    );
    let ops = vec![BlitOp {
        element: ElementId(0),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }];

    let mut out = ComposedSurface::new(0, 0);
    crate::blit::execute(&mut out, crate::plan::Extent { w: 1, h: 1 }, &ops, &atlas);

    // 頁原点(赤)でなく uv.x=2 の青が着弾する（源矩形オフセット読み取りの直接証拠）。
    let i = 0usize;
    let b = out.bytes();
    assert_eq!(
        [b[i], b[i + 1], b[i + 2], b[i + 3]],
        [255, 0, 0, 255],
        "uv.x=2 の青が転写される（頁原点の赤でない）",
    );
}

/// uv_rect.y も非ゼロで効くことを縦方向で突く（3 行頁の 3 行目を切り出す）。
#[test]
fn blit_reads_source_at_nonzero_uv_y_offset() {
    // 1×3 premultiplied BGRA 頁（縦並び）: 行0=赤 / 行1=緑 / 行2=青。
    let bytes = vec![
        0, 0, 255, 255, // (0,0) 赤
        0, 255, 0, 255, // (0,1) 緑
        255, 0, 0, 255, // (0,2) 青
    ];
    let page = AtlasPage {
        width: 1,
        height: 3,
        stride: 4,
        bytes: Arc::from(bytes),
    };
    // uv.y=2（青の行）を 1×1 で切り出す。
    let atlas = single_entry_table(
        page,
        Rect {
            x: 0,
            y: 2,
            w: 1,
            h: 1,
        },
        Point { x: 0, y: 0 },
    );
    let ops = vec![BlitOp {
        element: ElementId(0),
        transform: Transform::translate(0, 0),
        method: ComposeMethod::Overlay,
    }];

    let mut out = ComposedSurface::new(0, 0);
    crate::blit::execute(&mut out, crate::plan::Extent { w: 1, h: 1 }, &ops, &atlas);

    let b = out.bytes();
    assert_eq!(
        [b[0], b[1], b[2], b[3]],
        [255, 0, 0, 255],
        "uv.y=2 の青が転写される（頁原点の赤でない）",
    );
}
