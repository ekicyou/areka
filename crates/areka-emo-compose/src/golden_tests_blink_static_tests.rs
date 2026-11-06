//! 要件 9.1・9.5・11.2: emo2 まばたき bind+random の静的不活性テーマ。

use super::test_support::{bake_emo2, parse_emo2_shell, shell_master_dir};
use super::{AtlasTable, BindSet, MemoryDecoder, Path, PatternState, SetId, Shell};
use crate::plan_test_support::{bound_world, opaque_wxh, plan_of};

// ============================================================================
// emo2 まばたき bind+random（1400・pattern0 なし）の静的不活性回帰檻（要件 **9.1**・**9.5**・11.2）。
//
// 実機サインオフ#2: むらさきの目が常時閉じている。まばたき animation 1400（`interval,bind+random`・
// pattern1/2/3）の閉じ目フレーム（surface 1412）が静的土台に積まれ、ベースの目（1302）を覆っていた。
// 実 pasta はまばたきを on-repeat で bind するため BindSet に残る。`bind+random` はコマのあるときだけ
// 描く（コマは seriko が再生する）ので、静的土台には寄与しない。純 `bind` の 1403（pattern2＝1414 の
// null）は有効なら全部のコマを重ねる（seriko-pattern-calls 6.8・M6）。
//
// 本檻: emo2 surface1000 を [1302,1403] と [1302,1400,1403]（＋まばたき）で build_plan し、生成描画
// 命令列（BlitOp）が**同一**であることを固定する＝「常時閉じ目」退行を直接ガードする。
//
// 非空虚性: 閉じ目パーツ（1412=eyebase+toji・1414=null）の画像を atlas へ挿入するため、番号の最も
// 小さい pattern を静的に描く実装なら 1400→surface1412 が積まれて命令列が伸び、assert が破れる。
// ============================================================================

/// まばたき檻用の `AtlasTable` を構築する（COM/WIC 非依存・要件 11.4）。
///
/// [`build_atlas_for_surface1000`] と異なり、目/通常（1302→purple/4/normal.png）に加えて **閉じ目
/// パーツ**の画像（surface1412 の eyebase/toji・surface1414 の null）も MemoryDecoder へ挿入する。
/// これにより番号の最も小さい pattern を静的に描く実装なら閉じ目フレームが解決され命令が生じる（＝非空虚）。
fn build_atlas_for_blink_cage(shell: &Shell, base: &Path) -> AtlasTable {
    let mut dec = MemoryDecoder::new();
    // 目/通常（1302 = surface1302 → purple/4/normal.png・pattern0 あり）。
    let (w, h, s, b, a) = opaque_wxh(20, 90, 0, 0, 255);
    dec.insert(base.join("purple/4/normal.png"), w, h, s, b, a);
    // 閉じ目パーツ（1400→surface1412=eyebase+toji、1403→surface1414=null）を解決可能にする。
    // これらが解決できることで、番号の最も小さい pattern を静的に描く実装なら閉じ目が積まれ命令列が伸びる（非空虚）。
    for rel in [
        "purple/a/eyebase.png",
        "purple/4/toji.png",
        "purple/a/null.png",
    ] {
        let (w, h, s, b, a) = opaque_wxh(20, 90, 128, 128, 128);
        dec.insert(base.join(rel), w, h, s, b, a);
    }
    bake_emo2(shell, base, &dec)
}

/// 要件 9.1・9.5・11.2: emo2 まばたき bind+random（1400・pattern0 なし）は静的合成へ寄与しない＝
/// BindSet に足しても BlitOp 列が [1302,1403] のときと**同一**。純 `bind` の 1403 は全部のコマを重ねる。
///
/// 「むらさきの目が常時閉じている」実機第2欠陥の直接回帰檻。まばたき（閉じ目）フレームが静的
/// 土台へ積まれないことを命令列の同一性で固定する。
#[test]
fn emo2_blink_binds_are_statically_inactive() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_blink_cage(&shell, &base);

    let world = bound_world(&shell, &[], &atlas);

    // [1302] のみ（目/通常・pattern0 あり）。
    let (ops_base, result) = plan_of(
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([1302]),
        &PatternState::default(),
    );
    result.expect("目/通常のみでも surface1000 存在＋外形非ゼロで Ok");

    // [1302,1400,1403]（＋まばたき・pattern0 なし）。
    let (ops_blink, result) = plan_of(
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([1302, 1400, 1403]),
        &PatternState::default(),
    );
    result.expect("まばたき込みでも Ok");

    // [1302,1403]（純 `bind` の 1403 だけを足す）。
    let (ops_pure, result) = plan_of(
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([1302, 1403]),
        &PatternState::default(),
    );
    result.expect("純 bind 込みでも Ok");

    // `bind+random` のまばたき 1400（pattern0 なし）は静的合成へ寄与しない＝BlitOp 列が完全一致。
    assert_eq!(
        ops_pure, ops_blink,
        "まばたき bind+random を足しても BlitOp 列は不変（pattern0 なしは静的不活性・要件 9.1/9.5）",
    );
    // 純 `bind` の 1403 は有効なら全部のコマ（pattern2＝1414 の null）を重ねる（seriko-pattern-calls
    // 6.8・SSP の測り M6 で期待を直した）。閉じ目（1412）は `bind+random` 側なので目は覆われない。
    assert_eq!(ops_pure[..1], ops_base[..], "土台の 1302 はそのまま");
    assert_eq!(
        ops_pure.len(),
        2,
        "1403 の pattern2（1414）が 1 命令足される（M6）"
    );
    // 非空虚: 目/通常（1302）は pattern0 ありゆえ現に 1 命令を生む（空同士の空虚一致でない）。
    assert_eq!(
        ops_base.len(),
        1,
        "目/通常 1302 は pattern0 ありで 1 命令（檻は空虚でない）"
    );
}

/// まばたき檻の非空虚性の実データ確認: まばたきの 1400 は `bind+random`・1403 は純 `bind` で、どちらも
/// pattern0（index==0）を持たず、番号の最も小さい pattern の先（1400 は閉じ目の 1412・1403 は null の 1414）の
/// 画像が atlas に解決されることを固定する。
///
/// これにより上檻 [`emo2_blink_binds_are_statically_inactive`] の assert_eq が「まばたきが元々
/// 何も生まないから一致」という空虚一致でないこと（番号の最も小さい pattern を静的に描く実装なら
/// 閉じ目が積まれ破れる）を実データで裏づける。
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

    for (aid, interval, first_frame_surface) in [
        (1400u32, Interval::BindRandom { k: 4 }, 1412i64),
        (1403, Interval::Bind, 1414),
    ] {
        let anim = s1000
            .animations
            .iter()
            .find(|a| a.id == aid)
            .unwrap_or_else(|| panic!("surface1000 に animation{aid} が存在する"));
        assert_eq!(anim.interval, interval, "animation{aid} の interval");
        assert!(
            !anim.patterns.iter().any(|p| p.index == 0),
            "animation{aid} は pattern0（index==0）を持たない",
        );
        let min_pat = anim
            .patterns
            .iter()
            .min_by_key(|p| p.index)
            .expect("pattern を 1 本以上持つ");
        assert_eq!(
            min_pat.surface_id, first_frame_surface,
            "animation{aid} の最小 index pattern は surface{first_frame_surface} を参照（静的に描けばこれを合成）",
        );
    }

    // 閉じ目（1412=eyebase+toji）と null（1414）の element 画像が atlas に解決される（placement Some）。
    // ＝静的に描く実装なら実際に積まれ得た＝上檻の assert_eq は非空虚。
    for rel in [
        "purple/a/eyebase.png",
        "purple/4/toji.png",
        "purple/a/null.png",
    ] {
        let id = atlas
            .resolve(SetId(0), rel)
            .unwrap_or_else(|| panic!("{rel} が atlas に解決される"));
        assert!(
            atlas.entry(id).placement.is_some(),
            "{rel} は不透明コアあり＝placement Some（静的に描けば積まれ得た）",
        );
    }
    // 閉じ目の面 1412 そのものを合成すると命令が生じる（静的に描けば上檻の命令列が実際に伸びる）。
    let world = bound_world(&shell, &[], &atlas);
    let (ops_1412, result) = plan_of(
        &world,
        &atlas,
        1412,
        &BindSet::default(),
        &PatternState::default(),
    );
    result.expect("閉じ目の面 1412 は合成できる");
    assert!(!ops_1412.is_empty(), "閉じ目の面 1412 の命令列は空でない");
}
