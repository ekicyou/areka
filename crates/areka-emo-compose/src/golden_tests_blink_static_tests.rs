//! task 11.2（要件 9.1・9.5・11.2）: emo2 まばたき bind の静的不活性テーマ。

use crate::plan::build_plan;

use super::test_support::{on_params, parse_emo2_shell, shell_master_dir, solid_opaque};
use super::{
    AtlasTable, BindSet, EmoWorld, MemoryDecoder, PackConfig, Path, PatternState, SetId, Shell,
    SurfaceSet, bake,
};

// ============================================================================
// task 11.2: emo2 まばたき bind（1400/1403・pattern0 なし）の静的不活性回帰檻
// （要件 **9.1**・**9.5**・11.2）。
//
// 実機サインオフ#2: むらさきの目が常時閉じている。真因＝`flatten_surface` の pattern 選択が
// 最小 index フォールバック（min_by_key）だったため、pattern0（index==0）を持たない まばたき
// animation 1400（`interval,bind+random`・pattern1/2/3）と 1403（`interval,bind`・pattern2）の
// 閉じ目フレーム（surface 1412/1414）が静的土台に積まれ、ベースの目（1302）を覆っていた。実 pasta は
// まばたきを on-repeat で bind するため BindSet に残る。canon では pattern0 を持たない bind
// animation は seriko-loop（M-life）が再生する再生専用フレームで、静的土台には寄与しない。
//
// 本檻: emo2 surface1000 を [1302]（目/通常・pattern0 あり）と [1302,1400,1403]（＋まばたき）で
// build_plan し、生成描画命令列（BlitOp）が**同一**であることを固定する。まばたき bind を足しても
// 静的合成が一切変わらない＝「常時閉じ目」退行を直接ガードする。
//
// 非空虚性: 閉じ目パーツ（1412=eyebase+toji・1414=null）の画像を atlas へ挿入するため、旧
// min_by_key 実装なら 1400→surface1412・1403→surface1414 が composed され命令列が伸びて assert が
// 破れる。修正後（find(index==0)）はこれらが skip され命令列が [1302] と一致する。
// ============================================================================

/// まばたき檻用の `AtlasTable` を構築する（COM/WIC 非依存・要件 11.4）。
///
/// [`build_atlas_for_surface1000`] と異なり、目/通常（1302→purple/4/normal.png）に加えて **閉じ目
/// パーツ**の画像（surface1412 の eyebase/toji・surface1414 の null）も MemoryDecoder へ挿入する。
/// これにより旧 min_by_key 実装なら 1400/1403 の閉じ目フレームが解決され命令が生じる（＝非空虚）。
fn build_atlas_for_blink_cage(shell: &Shell, base: &Path) -> AtlasTable {
    let mut dec = MemoryDecoder::new();
    // 目/通常（1302 = surface1302 → purple/4/normal.png・pattern0 あり）。
    let (w, h, s, b, a) = solid_opaque(20, 90, 0, 0, 255);
    dec.insert(base.join("purple/4/normal.png"), w, h, s, b, a);
    // 閉じ目パーツ（1400→surface1412=eyebase+toji、1403→surface1414=null）を解決可能にする。
    // これらが解決できることで、旧 min_by_key 実装なら閉じ目が composed され命令列が伸びる（非空虚）。
    for rel in [
        "purple/a/eyebase.png",
        "purple/4/toji.png",
        "purple/a/null.png",
    ] {
        let (w, h, s, b, a) = solid_opaque(20, 90, 128, 128, 128);
        dec.insert(base.join(rel), w, h, s, b, a);
    }
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    bake(std::slice::from_ref(&set), &dec, PackConfig::default()).table
}

/// task 11.2（要件 9.1・9.5・11.2）: emo2 まばたき bind（1400/1403・pattern0 なし）は静的合成へ
/// 寄与しない＝BindSet に足しても BlitOp 列が [1302] のときと**同一**。
///
/// 「むらさきの目が常時閉じている」実機第2欠陥の直接回帰檻。まばたき（閉じ目）フレームが静的
/// 土台へ積まれない（pattern0 を持たないゆえ）ことを命令列の同一性で固定する。
#[test]
fn emo2_blink_binds_are_statically_inactive() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_blink_cage(&shell, &base);

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // [1302] のみ（目/通常・pattern0 あり）。
    let mut ops_base = Vec::new();
    let mut vis_base = Vec::new();
    build_plan(
        &mut ops_base,
        &mut vis_base,
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([1302]),
        &PatternState::default(),
    )
    .expect("目/通常のみでも surface1000 存在＋外形非ゼロで Ok");

    // [1302,1400,1403]（＋まばたき・pattern0 なし）。
    let mut ops_blink = Vec::new();
    let mut vis_blink = Vec::new();
    build_plan(
        &mut ops_blink,
        &mut vis_blink,
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([1302, 1400, 1403]),
        &PatternState::default(),
    )
    .expect("まばたき込みでも Ok");

    // まばたき bind（1400/1403・pattern0 なし）は静的合成へ寄与しない＝BlitOp 列が完全一致。
    assert_eq!(
        ops_base, ops_blink,
        "まばたき bind を足しても BlitOp 列は不変（pattern0 なしは静的不活性・要件 9.1/9.5）",
    );
    // 非空虚: 目/通常（1302）は pattern0 ありゆえ現に 1 命令を生む（空同士の空虚一致でない）。
    assert_eq!(
        ops_base.len(),
        1,
        "目/通常 1302 は pattern0 ありで 1 命令（檻は空虚でない）"
    );
}

/// まばたき檻の非空虚性の実データ確認: 1400/1403 が pattern0（index==0）を持たず、閉じ目パーツ
/// 画像が atlas に解決される（旧実装なら composed されうる）ことを固定する。
///
/// これにより上檻 [`emo2_blink_binds_are_statically_inactive`] の assert_eq が「まばたきが元々
/// 何も生まないから一致」という空虚一致でないこと（旧 min_by_key なら閉じ目が composed され破れる）
/// を実データで裏づける。
#[test]
fn emo2_blink_binds_lack_pattern0_but_frames_are_resolvable() {
    use areka_parsers::shell::Interval;

    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_blink_cage(&shell, &base);

    let s1000 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 1000)
        .expect("emo2 surfaces.txt に surface1000 が存在する");

    // 1400（bind+random）と 1403（bind）は pattern0（index==0）を持たない再生アニメである。
    for (aid, closed_frame_surface) in [(1400u32, 1412i64), (1403, 1414)] {
        let anim = s1000
            .animations
            .iter()
            .find(|a| a.id == aid)
            .unwrap_or_else(|| panic!("surface1000 に animation{aid} が存在する"));
        assert!(
            matches!(anim.interval, Interval::Bind | Interval::BindRandom { .. }),
            "animation{aid} は bind 種 interval（BindSet に載る）",
        );
        assert!(
            !anim.patterns.iter().any(|p| p.index == 0),
            "animation{aid} は pattern0（index==0）を持たない（まばたき再生アニメ）",
        );
        // 旧 min_by_key が採るであろう最小 index pattern の参照先（閉じ目 surface）が存在する。
        let min_pat = anim
            .patterns
            .iter()
            .min_by_key(|p| p.index)
            .expect("pattern を 1 本以上持つ");
        assert_eq!(
            min_pat.surface_id, closed_frame_surface,
            "animation{aid} の最小 index pattern は閉じ目 surface{closed_frame_surface} を参照（旧実装ならこれを合成）",
        );
    }

    // 閉じ目パーツ surface（1412=eyebase+toji）の element 画像が atlas に解決される（placement Some）。
    // ＝旧実装なら実際に composed され得た＝上檻の assert_eq は非空虚。
    for rel in [
        "purple/a/eyebase.png",
        "purple/4/toji.png",
        "purple/a/null.png",
    ] {
        let id = atlas
            .resolve(SetId(0), rel)
            .unwrap_or_else(|| panic!("閉じ目パーツ {rel} が atlas に解決される"));
        assert!(
            atlas.entry(id).placement.is_some(),
            "{rel} は不透明コアあり＝placement Some（旧実装なら composed され得た）",
        );
    }
}
