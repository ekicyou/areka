//! task 7.3（要件 5.4）: 採録まばたきコマがベース surface の外形内に収まることの実測テーマ。

use super::{
    AtlasTable, EmoWorld, MemoryDecoder, PackConfig, Path, SetId, Shell, SurfaceSet, bake,
};
use super::test_support::{on_params, parse_emo2_shell, shell_master_dir};

// ============================================================================
// task 7.3: 外形前提の実測檻（要件 **5.4**・design「合成合流と method ゲート」の
// `compute_extent` 外形前提）。
//
// design 決定（design.md「合成合流」）: `compute_extent`（外形）は transient コマを寄与させず、
// 「まばたきコマ（1410-1412/2106-2110）はベース外形内に収まる前提」を維持する。コマがベース外形を
// 越えると越えた分がクリップされる（許容劣化）。**この前提を宣言に留めず emo2 fixture の実測で裏取り
// する**（design「Testing Strategy / Integration Tests item 3: emo-compose golden」）。
//
// 本檻は emo2 の採録まばたきアニメ 2 系統——
//   - kero: `animation0`（`interval,random,4`・`surface.append10,2100`）→ フレーム surface 2106/2110、
//   - sakura: `animation1400`（`interval,bind+random,4`・surface1000 内）→ フレーム surface 1410/1411/1412
// ——の**全コマ**について、各コマが参照する surface の **原寸**（`AtlasEntry.original`＝実 PNG の
// IHDR 寸法）＋コマの (x,y) オフセットが、当該アニメをホストするベース surface（kero=2100／
// sakura=1000）の `Extent` 内に収まる（`x + frame.w <= base.w` かつ `y + frame.h <= base.h`）ことを
// production `compute_extent` で検証する。
//
// ## real fixture 経路（COM/WIC 回避）
// アニメ構造・(x,y) は**実 surfaces.txt を parse＋fold した実データ**（`EmoWorld` の SurfaceMaster）から
// 読む。各 surface の原寸は**実 PNG の IHDR を直読**して得る（COM/WIC デコード不要）。実寸の全不透明
// バッファを `MemoryDecoder` へ挿入して bake するため `AtlasEntry.original` は実 fixture 寸法どおりに
// なり（全不透明ゆえ placement も full＝original）、`compute_extent` はベース／コマの外形を実寸で算出する。
// ＝実アニメ構造・実オフセット・実原寸に基づく real-fixture 検証（既存 golden の合成画素檻が headless
// 合成画素を扱うのに対し、本檻は headless で**実寸外形**を扱う）。
// ============================================================================

use crate::plan::{compute_extent, Extent};

/// PNG ファイルの IHDR から `(width, height)` を読む（COM/WIC 非依存の実原寸取得）。
///
/// PNG は先頭 8 バイトが署名（`\x89PNG\r\n\x1a\n`）、続く IHDR チャンク（length[4]+"IHDR"[4]）の
/// 直後に width[4]・height[4]（いずれもビッグエンディアン u32）が並ぶ。ゆえに width はバイト
/// オフセット 16、height は 20 から読める（RFC 2083 / PNG 仕様）。
fn png_size(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("実 PNG を読めない: {}: {e}", path.display()));
    assert!(
        bytes.len() >= 24 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR",
        "PNG 署名／IHDR が期待どおり: {}",
        path.display()
    );
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    assert!(w > 0 && h > 0, "実 PNG 原寸は非ゼロ: {}", path.display());
    (w, h)
}

/// 実 shell の全 element を**実 PNG 原寸の全不透明バッファ**で `MemoryDecoder` へ挿入し bake する。
///
/// `compute_extent` は `AtlasEntry.original`（＝挿入した実寸）のみを用いるため、これでベース／コマ
/// surface の外形が実 fixture 寸法どおりに算出される。全不透明（α=255）ゆえ α-bbox トリムは恒等で
/// placement=full・original=(w,h)。挿入キーは bake が引くキー（`base_dir.join(element.path)`）と同一に
/// するため、実 shell の element パス文字列（CityPop は `\` 区切りを含む）をそのまま用いる。
fn build_atlas_real_sizes(shell: &Shell, base: &Path) -> AtlasTable {
    let mut dec = MemoryDecoder::new();
    let mut seen = std::collections::HashSet::new();
    for s in &shell.surfaces {
        for e in &s.elements {
            let abs = base.join(e.path.as_str());
            if !seen.insert(abs.clone()) {
                continue; // 同一 PNG を複数 surface が参照する（eyebase 等）: 一度だけ挿入。
            }
            let (w, h) = png_size(&abs);
            let stride = w * 4;
            let bgra = vec![255u8; (stride * h) as usize]; // 全不透明→ original=(w,h)・placement full。
            dec.insert(abs, w, h, stride, bgra, true);
        }
    }
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    bake(std::slice::from_ref(&set), &dec, PackConfig::default()).table
}

/// 採録アニメ 1 本の全コマ（surface_id>=0 のみ・センチネル -1 は非描画で除外）を
/// `(frame_surface_id, x, y)` 列として、fold 済み world から**実データで**取り出す。
///
/// `world.surface(base_id)` の SurfaceMaster.animations（append 適用済み）から `anim_id` を引き、
/// その pattern 群を返す。実 surfaces.txt の転記そのものを検証入力にするため、コマ座標を本テストへ
/// ハードコードしない。
fn recorded_frames(world: &EmoWorld, base_id: u32, anim_id: u32) -> Vec<(u32, i64, i64)> {
    let master = world
        .surface(base_id)
        .unwrap_or_else(|| panic!("ベース surface{base_id} が fold 済み world に存在する"));
    let anim = master
        .animations
        .iter()
        .find(|a| a.id == anim_id)
        .unwrap_or_else(|| panic!("surface{base_id} に animation{anim_id}（採録まばたき）が存在する"));
    let frames: Vec<(u32, i64, i64)> = anim
        .patterns
        .iter()
        .filter(|p| p.surface_id >= 0) // -1 停止センチネルは非描画コマ＝外形対象外。
        .map(|p| (p.surface_id as u32, p.x, p.y))
        .collect();
    assert!(!frames.is_empty(), "採録アニメ animation{anim_id} は描画コマを 1 枚以上持つ");
    frames
}

/// 採録まばたきアニメの**全コマ**が原寸＋(x,y) でベース surface の Extent 内に収まることをアサートし、
/// 実測値（ベース外形・各コマ外形＋オフセット）を返す（EVIDENCE 用）。
///
/// 各コマ surface の原寸はそれ自身の `compute_extent`（当該 surface の element 群の (0,0) 起点原寸和集合）
/// で得る。コマ surface（1410/2106 等）は animation を持たない葉 surface ゆえ、その Extent は原寸和集合＝
/// 「コマの原寸」に一致する。
fn assert_frames_fit_base(
    world: &EmoWorld,
    atlas: &AtlasTable,
    base_id: u32,
    anim_id: u32,
) -> (Extent, Vec<(u32, i64, i64, Extent)>) {
    let mut visited = Vec::new();
    let base_extent = compute_extent(&mut visited, world, atlas, base_id);
    assert!(
        base_extent.w > 0 && base_extent.h > 0,
        "ベース surface{base_id} の外形は非ゼロ（実 PNG 原寸由来）"
    );

    let frames = recorded_frames(world, base_id, anim_id);
    let mut measured = Vec::new();
    for (fid, x, y) in frames {
        let mut fvisited = Vec::new();
        let fextent = compute_extent(&mut fvisited, world, atlas, fid);
        assert!(
            fextent.w > 0 && fextent.h > 0,
            "コマ surface{fid} の原寸外形は非ゼロ（実 PNG 由来）"
        );
        // 収まり（クリップ非発生）: x + frame.w <= base.w かつ y + frame.h <= base.h。x/y は非負前提
        // （emo2 の採録コマは (x,y)>=0）だが、負値でも i64 比較で安全側（左上原点クリップは別議論）。
        let right = x + fextent.w as i64;
        let bottom = y + fextent.h as i64;
        assert!(
            right <= base_extent.w as i64 && bottom <= base_extent.h as i64,
            "コマ surface{fid}（原寸 {}x{} ＋オフセット ({x},{y})）は base surface{base_id} 外形 \
             {}x{} 内に収まる（右端 {right}<=幅{}／下端 {bottom}<=高{}）＝クリップ非発生（要件 5.4）",
            fextent.w, fextent.h, base_extent.w, base_extent.h, base_extent.w, base_extent.h
        );
        measured.push((fid, x, y, fextent));
    }
    (base_extent, measured)
}

/// task 7.3 ③（要件 5.4・design compute_extent 外形前提）: kero まばたき（`animation0`・
/// フレーム 2106/2110）の全コマが原寸＋(x,y) で base surface2100 の Extent 内に収まる。
///
/// kero の採録まばたき（`surface.append10,2100` の `animation0.interval,random,4`）は立ち絵ベース
/// surface2100（`CityPop\surface0100.png`）上にフレーム 2106（`surface0106.png`）／2110
/// （`surface0110.png`）を overlay する。実 PNG 原寸（いずれも 336×400 の同一キャンバス）ゆえ、各コマは
/// オフセット (0,0) で base 外形にちょうど収まる（越境クリップは発生しない）。
#[test]
fn kero_blink_frames_fit_within_base_extent() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_real_sizes(&shell, &base);

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let (base_extent, measured) = assert_frames_fit_base(&world, &atlas, 2100, 0);

    // 回帰固定（実 IHDR 由来）: base 2100 と全コマは 336×400 の同一立ち絵キャンバス（境界一致収まり）。
    assert_eq!(
        (base_extent.w, base_extent.h),
        (336, 400),
        "kero base surface2100（CityPop\\surface0100.png）の外形は実原寸 336×400"
    );
    // 採録コマは 2106/2110 の 2 枚（-1 センチネルは除外済み）。
    let ids: Vec<u32> = measured.iter().map(|(id, _, _, _)| *id).collect();
    assert_eq!(ids, vec![2106, 2110], "kero animation0 の描画コマは 2106・2110（採録順）");
    for (fid, x, y, fe) in &measured {
        assert_eq!(
            (fe.w, fe.h, *x, *y),
            (336, 400, 0, 0),
            "kero フレーム surface{fid} は 336×400・オフセット (0,0)（実 fixture 転記）"
        );
    }
}

/// task 7.3 ③（要件 5.4）: sakura まばたき（`animation1400`・フレーム 1410/1411/1412）の全コマが
/// 原寸＋(x,y) で base surface1000 の Extent 内に収まる。
///
/// sakura の採録まばたき（surface1000 内 `animation1400.interval,bind+random,4`）はフレーム
/// 1412（`eyebase+toji`）／1411（`eyebase+hanme`）／1410（`eyebase+normal`）を overlay する。各フレーム
/// surface は eye パーツ 2 枚（いずれも実原寸 382×547 の全身キャンバス）の (0,0) 和集合＝382×547。
/// base surface1000（全パーツ MAYUNA bind の本体・外形は全 bind pattern0 原寸の和集合）は 382×547 を
/// 含むため、各コマはオフセット (0,0) で base 外形内に収まる（越境クリップは発生しない）。
#[test]
fn sakura_blink_frames_fit_within_base_extent() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_real_sizes(&shell, &base);

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let (base_extent, measured) = assert_frames_fit_base(&world, &atlas, 1000, 1400);

    // 回帰固定（実 IHDR 由来）: sakura 本体 surface1000 の外形は eye/body パーツの原寸 382×547 を含む。
    assert!(
        base_extent.w >= 382 && base_extent.h >= 547,
        "sakura base surface1000 の外形（{}x{}）は eye パーツ原寸 382×547 以上",
        base_extent.w, base_extent.h
    );
    // 採録コマは 1412/1411/1410 の 3 枚（pattern1/2/3・pattern0 なしの再生専用アニメ）。
    let ids: Vec<u32> = measured.iter().map(|(id, _, _, _)| *id).collect();
    assert_eq!(
        ids,
        vec![1412, 1411, 1410],
        "sakura animation1400 の描画コマは 1412・1411・1410（採録 pattern1/2/3 順）"
    );
    for (fid, x, y, fe) in &measured {
        assert_eq!(
            (fe.w, fe.h, *x, *y),
            (382, 547, 0, 0),
            "sakura フレーム surface{fid} は 382×547・オフセット (0,0)（eyebase＋eye パーツの和集合）"
        );
    }
}
