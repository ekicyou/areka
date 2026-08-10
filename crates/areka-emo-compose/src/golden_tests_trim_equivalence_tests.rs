//! task 8.3（要件 11.3・6.2）: トリム等価の pixel テーマ。

use super::{
    AtlasTable, BindSet, Composer, EmoWorld, MemoryDecoder, PackConfig, Path, PatternState, SetId,
    Shell, SurfaceSet, bake,
};
use super::test_support::{on_params, parse_emo2_shell, shell_master_dir};

// ============================================================================
// task 8.3: トリム等価の pixel テスト（要件 **11.3**・**6.2**）。
//
// bake は透明マージンを α-bbox トリムで切り落とし、原画像内の bbox 左上を
// `Placement.trim_offset`・原寸を `AtlasEntry.original` として記録する（`trim.rs`）。
// blit は転写先を `transform.offset() + placement.trim_offset` として算出する（`blit.rs`）。
// この 2 つが噛み合うと「不透明コアが原画像内で占めていた絶対位置」へコアが着弾し、
// トリムは合成結果を一切変えない＝**トリムありの合成 == トリムなし理論配置**（要件 6.2）。
//
// 本テストはこれを byte-equality で固定する（要件 11.3 の pixel テスト）:
//   - Scenario A（トリムあり）: 透明マージン付き 40×40 画像（不透明 10×10 コアが内側
//     オフセット (12,15)・残り α=0）を surface0.png として挿入 → bake がトリム
//     （uv=10×10・trim_offset=(12,15)・original=40×40）→ surface0 の element0（配置 (0,0)）
//     として合成する。
//   - Scenario B（トリムなし理論配置＝期待バッファ）: 原画像そのもの（40×40・コアは
//     (12,15) に premultiplied・残り 0）。element 配置 (0,0) ＋ trim_offset (12,15) = (12,15)
//     ＝原画像内でコアが占める位置ゆえ、透明マージンへの SourceOver は恒等で、合成結果は
//     **原画像バイトそのもの**になる。
//
// これは blit テスト④（`trim_offset_shifts_destination`）を fixture 駆動の full-pipeline
// 経路（parse→fold→bake トリム→bind→plan→blit）へ引き上げ、bake が現に記録する trim_offset を
// blit が現に再加算することを end-to-end で保証する（design「Integration Tests item 3」・
// 「BlitExecutor（転写先 = transform.offset()+trim_offset）」・Requirements Traceability 6.2/11.3）。
// すべて MemoryDecoder+bake の CPU-only 経路（COM/WIC/表示なし・要件 11.4）。
// ============================================================================

/// 透明マージン付き画像スペック（原寸・不透明コアのオフセット/寸法/色）。
///
/// 40×40 の全透明キャンバスに、内側 (INNER_X, INNER_Y) を左上とする CORE_W×CORE_H の
/// **全不透明（α=255）**単色コアを置く。全不透明ゆえ premultiplied 済み（色 ≤ α）で、
/// α=0 マージンは全バイト 0（premultiplied 透明）。この構成で bake は α-bbox トリムにより
/// マージンを落とし、trim_offset=(INNER_X, INNER_Y)・original=(ORIG_W, ORIG_H) を記録する。
const ORIG_W: u32 = 40;
const ORIG_H: u32 = 40;
const INNER_X: u32 = 12;
const INNER_Y: u32 = 15;
const CORE_W: u32 = 10;
const CORE_H: u32 = 10;
// コアの単色（判別可能・全不透明）。premultiplied（B,G,R ≤ A=255）。
const CORE_B: u8 = 30;
const CORE_G: u8 = 170;
const CORE_R: u8 = 220;

/// 透明マージン付き原画像を tightly-packed premultiplied BGRA で生成する。
///
/// 全画素をまず 0（透明）で埋め、内側 [INNER_X, INNER_X+CORE_W) × [INNER_Y, INNER_Y+CORE_H)
/// にだけ全不透明の単色コアを書く。返り値は Scenario B の「トリムなし理論配置＝期待バッファ」
/// そのものでもある（コアが原画像内で占める絶対位置に置かれた原画像）。
fn margined_core_image() -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = ORIG_W * 4;
    let mut bgra = vec![0u8; (stride * ORIG_H) as usize];
    for y in INNER_Y..(INNER_Y + CORE_H) {
        for x in INNER_X..(INNER_X + CORE_W) {
            let off = (y * stride + x * 4) as usize;
            bgra[off] = CORE_B;
            bgra[off + 1] = CORE_G;
            bgra[off + 2] = CORE_R;
            bgra[off + 3] = 255;
        }
    }
    // has_alpha=true（α チャンネル採用腕＝use_self_alpha,On・normalize 恒等）。
    (ORIG_W, ORIG_H, stride, bgra, true)
}

/// margined_core_image を surface0.png として挿入した surface0 用 AtlasTable を構築する。
///
/// bake は α-bbox トリムでマージンを落とし、element0 エントリに
/// trim_offset=(INNER_X,INNER_Y)・original=(ORIG_W,ORIG_H)・uv=CORE_W×CORE_H を記録する。
/// 挿入した原画像バイト（Scenario B の期待バッファ）も併せて返す。
fn build_atlas_margined_surface0(shell: &Shell, base: &Path) -> (AtlasTable, Vec<u8>) {
    let mut dec = MemoryDecoder::new();
    let (w, h, stride, bgra, has_alpha) = margined_core_image();
    dec.insert(base.join("surface0.png"), w, h, stride, bgra.clone(), has_alpha);
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    let table = bake(std::slice::from_ref(&set), &dec, PackConfig::default()).table;
    (table, bgra)
}

/// task 8.3（要件 11.3・6.2）: トリムありの合成結果 == トリムなし理論配置（原画像）とバイト等価。
///
/// surface0 の element0 は配置 (0,0)（実 fixture・既存テストで固定）。透明マージン付き 40×40 を
/// 挿入すると bake はトリム（trim_offset=(12,15)・original=40×40・uv=10×10）する。合成では
/// dest = 配置 (0,0) ＋ trim_offset (12,15) = (12,15) へ不透明コアが着弾し、その他は透明のまま。
/// これは原画像（コアが (12,15) に premultiplied・残り 0）とバイト単位で一致する。
/// あわせて外形が**原寸 40×40**（トリム後コア 10×10 ではない・task 5.4/要件 6.5）であることを固定する。
#[test]
fn surface0_trimmed_composite_equals_untrimmed_placement() {
    let shell = parse_emo2_shell();

    // surface0 が element0 単層・配置 (0,0)・bind 無しであることを実データで確認する（前提固定）。
    let s0 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 0)
        .expect("emo2 surfaces.txt に surface0 が存在する");
    assert_eq!(s0.elements.len(), 1, "surface0 は element0 単層");
    assert_eq!(
        (s0.elements[0].x, s0.elements[0].y),
        (0, 0),
        "element0 は原点配置（配置 (0,0)＋trim_offset で着弾位置が決まる）"
    );

    let base = shell_master_dir();
    let (atlas, original_bytes) = build_atlas_margined_surface0(&shell, &base);

    // bake が現に記録したトリム情報を診断的に固定する（trim_offset/original/uv が期待どおり）。
    // element0 は fold 後の登場順で ElementId(0)（surface0 単層・既存 8.1 harness と同一束縛）。
    let entry = atlas.entry(areka_emo_atlas::ElementId(0));
    assert_eq!(
        entry.original,
        areka_emo_atlas::Size { w: ORIG_W, h: ORIG_H },
        "original は原寸 40×40（トリム後コアではない・task 5.4 の外形母集合）"
    );
    let placement = entry
        .placement
        .as_ref()
        .expect("透明マージン付きでも不透明コアがあるので placement は Some");
    assert_eq!(
        placement.trim_offset,
        areka_emo_atlas::Point { x: INNER_X as i32, y: INNER_Y as i32 },
        "bake は原画像内 bbox 左上 (12,15) を trim_offset に記録"
    );
    assert_eq!(
        (placement.uv_rect.w, placement.uv_rect.h),
        (CORE_W, CORE_H),
        "uv はトリム後コア 10×10（マージンが落ちている＝現にトリムされた）"
    );

    // fold → bind → compose（surface0・空 BindSet）。
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));
    let out = Composer::new()
        .compose(&world, &atlas, 0, &BindSet::default(), &PatternState::default())
        .expect("surface0 の単層合成は Ok（要件 11.1）");

    // 外形は原寸 40×40（有効 bind 非依存の静的外形＝原点+original・トリム後コアではない）。
    assert_eq!(out.width(), ORIG_W, "合成幅 == 原寸 40（トリム後 10 ではない・要件 6.5）");
    assert_eq!(out.height(), ORIG_H, "合成高 == 原寸 40（トリム後 10 ではない・要件 6.5）");
    assert_eq!(out.stride(), ORIG_W * 4, "stride == 原寸*4");

    // トリム等価の中核（要件 6.2/11.3）: トリムありの合成結果は原画像（トリムなし理論配置）と
    // バイト単位で一致する。配置 (0,0)＋trim_offset (12,15) がコアを原画像内の位置へ戻すため、
    // トリムは合成結果を一切変えない。
    assert_eq!(
        out.bytes(),
        original_bytes.as_slice(),
        "トリムありの合成結果 == トリムなし理論配置（原画像）とバイト等価（要件 6.2/11.3）"
    );

    // 着弾位置の要点確認: コア左上 (12,15) は不透明・素の配置 (0,0) は透明。
    let core_tl = ((INNER_Y * out.stride() + INNER_X * 4) as usize, [CORE_B, CORE_G, CORE_R, 255u8]);
    assert_eq!(
        &out.bytes()[core_tl.0..core_tl.0 + 4],
        &core_tl.1,
        "配置+trim=(12,15) に不透明コアが着弾"
    );
    assert_eq!(
        &out.bytes()[0..4],
        &[0, 0, 0, 0],
        "素の配置 (0,0)（trim を足さない位置）は透明のまま（トリムが見た目を変えない）"
    );
}

/// 非空虚性ガード（要件 6.2/11.3）: 「trim_offset を足さない」誤配置は合成結果と一致しない。
///
/// byte-equality が tautology でないことを示すため、コアを **trim_offset を加算せず**素の配置
/// (0,0) に置いた「誤った期待バッファ」を手組みし、それが実際の合成結果と**不一致**になることを
/// 固定する。もし blit が trim_offset を再加算していなければ（要件 6.2 のバグ）、コアは (0,0) に
/// 着弾しこの誤バッファと一致してしまう——本 assert_ne! はその退行を検出する。
#[test]
fn surface0_trim_equivalence_is_non_vacuous() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let (atlas, _original) = build_atlas_margined_surface0(&shell, &base);

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));
    let out = Composer::new()
        .compose(&world, &atlas, 0, &BindSet::default(), &PatternState::default())
        .expect("Ok");

    // 誤った期待: コアを trim_offset を足さず原点 (0,0) に置いた 40×40 バッファ。
    // 実際の合成は配置+trim=(12,15) にコアを置くので、両者は一致してはならない。
    let stride = ORIG_W * 4;
    let mut wrong = vec![0u8; (stride * ORIG_H) as usize];
    for y in 0..CORE_H {
        for x in 0..CORE_W {
            let off = (y * stride + x * 4) as usize;
            wrong[off] = CORE_B;
            wrong[off + 1] = CORE_G;
            wrong[off + 2] = CORE_R;
            wrong[off + 3] = 255;
        }
    }

    assert_ne!(
        out.bytes(),
        wrong.as_slice(),
        "trim_offset を足さない誤配置は合成結果と不一致＝byte-equality は非空虚（blit が現に trim_offset を再加算・要件 6.2）"
    );

    // 誤バッファでコアが在る (0,0) は、実合成では透明（trim で (12,15) へ移動済み）＝差の直接証拠。
    assert_eq!(&wrong[0..4], &[CORE_B, CORE_G, CORE_R, 255], "誤バッファは (0,0) にコア");
    assert_eq!(&out.bytes()[0..4], &[0, 0, 0, 0], "実合成の (0,0) は透明（差が生じる画素）");
}
