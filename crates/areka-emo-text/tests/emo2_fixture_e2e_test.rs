//! # emo2_fixture_e2e_test — 実 emo2 fixture のメニュー cue 列 headless 統合（R6.1/6.2/6.3）
//!
//! 出典 spec: `areka-P0-choice-render`（このファイルが挙げる要件の番号と design.md はこの spec のもの）。
//!
//! design.md「Testing Strategy > E2E/実機 item 1（emo2_boot 統合）」の正典述語を張る:
//! **実 emo2 fixture ＋ menu.pasta 経路でメニュー cue 列 →選択肢描画〔headless readback〕**。
//!
//! `choice_fixture_test.rs` は tests 配下の **test-local** 最小 fixture で parse→resolve と
//! 実フォント render を固定する。本ファイルは **リポジトリに実在する emo2 fixture**
//! （`検体 emo2 の同梱バルーン emo2-kakukaku`）の実 descript／実 menu.pasta を
//! 用い、cue 配送 →選択肢描画をエンドツーエンドで検証する（実窓は起動しない・headless readback のみ）。
//!
//! ## 実 fixture の実態（2 層バルーン descript ＋ pasta メニュー台本）
//!
//! - `descript.txt`（基層）: `cursor.*` スタイル（`style,square`・`brush.color,105/25/25`・
//!   `font.color,255/255/255`）と `font.name,Yu Gothic UI`・`font.height,28` を持つが、`validrect` は
//!   **全 0＝退化**（基層のみでは描画領域が空）。
//! - `balloons0s.txt`（画像別上書き層）: `balloons0.png`（実測 400×224）向けに `validrect`
//!   （top,46／bottom,-56／left,36／right,-44）と `wordwrappoint.x,-49` を与え、2 層マージ後に
//!   **非退化領域 (36,46)-(356,168)** を成立させる（region.rs の `two_layer_merged_fixture…` と同機構）。
//! - `menu.pasta`（メニュー台本）: `＊メインメニュー選択肢` の本文に
//!   `\q[おしゃべり頻度,…]\n\q[エモの位置調整,…]\_l[5em,2lh]\q[閉じる,…]` という
//!   **実さくらスクリプト断片**（`\n` 改行 3 項目＋`\_l` 字下げ）を持つ。
//!
//! ## 経路（実 fixture → 実 style / 実 cue 列 → 描画 → readback）
//!
//! 1. 実 `descript.txt`＋`balloons0s.txt` を **実 balloon パーサ**（`parse_str`・2 層マージ）で読み、
//!    実 `cursor.*` を `ResolvedChoiceStyle::SquareFill{(105,25,25),(255,255,255)}` へ解決（値は実
//!    descript から来る——in-code ハードコードでない）。
//! 2. 実 `menu.pasta` の `＊メインメニュー選択肢` 本文からさくらスクリプト断片を抽出し、**実 sakura
//!    パイプライン**（`areka_parsers::sakura::parse` →`areka_sakura::compile`）で cue 列へコンパイルする
//!    （Choice/NewLine/Cursor cue ＋選択待ち barrier——手組み注入でなく実台本由来）。
//! 3. コンパイル済み Command cue を `apply_cue` で runtime へ載せ、**headless GPU 供給面**
//!    （`com_world` 相当・実窓なし）へ present して read_back する。
//! 4. 選択肢が描かれ（行帯にインク・数＝メニュー項目数）、ヒット行が実台本の id／label を反映し、
//!    ordinal 1（先頭でない行）の hover が実 fixture の SquareFill ハイライト（塗り(105,25,25)＋白文字）を
//!    当該行にのみ出すことを固定する。
//!
//! これが 6.1（E2E 証明）を実 fixture 内容で満たし、型／語彙シーム（6.2・Cursor/references/barrier の
//! 保持）を保ち、スクロール完全対応は追わない（6.3・短メニュー＝領域内に収まる）。

#[path = "support/mod.rs"]
mod support;

use support::{
    Band, assert_hover_band, dump_png_on_white, fill_pixels_in_rect, make_world_with_gpu,
    opaque_count, read_back, spawn_reserved_slot, surface_size, to_canvas_local,
    white_pixels_in_rect,
};

use support::fixture::{command_cues, emo2_balloon_root, menu_script, merged_model};

use areka_emo_text::actor::{
    ChoiceHitRow, ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame,
};
use areka_emo_text::choice::ResolvedChoiceStyle;
use areka_emo_text::region::TextRegion;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::compile;
use areka_sakura::contract::{ActorKey, CueCommand, CuePayload, SystemVarSnapshot};

/// balloons0.png の実測原寸（region.rs `FIXTURE_IMAGE_SIZE` と同値・image px）。
const IMAGE_SIZE: (u32, u32) = (400, 224);

/// 実 `descript.txt`（基層）＋`balloons0s.txt`（画像別上書き層）を 2 層マージした BalloonModel。
/// 実機の balloon-parse と同一機構（`parse_str(descript, Some(image))`）——非退化領域が成立する。
fn merged_balloon_model() -> BalloonModel {
    merged_model(&emo2_balloon_root(), Some("balloons0s.txt"))
}

/// 実 `menu.pasta` の `＊メインメニュー選択肢` 本文（純さくらスクリプト断片）。
fn extract_main_menu_sakura() -> String {
    menu_script("＊メインメニュー選択肢")
}

/// 実台本のメインメニュー 3 項目（`\q[disp,target]` の (target=id, disp=label)）。
/// compile は `id=target`・`text=disp` へ写す（sakura compile.rs の Choice アーム）。
const EXPECTED_MENU: &[(&str, &str)] = &[
    ("Onおしゃべり頻度メニュー", "おしゃべり頻度"),
    ("Onエモの位置調整メニュー", "エモの位置調整"),
    ("Onメニュー閉じる", "閉じる"),
];

// ══ テスト 1: 実 fixture の cursor.* が実 parse で SquareFill へ解決される（GPU 不要） ══════════

/// Observable（R6.1/6.2）: 実 emo2 `descript.txt`＋`balloons0s.txt` を**実 balloon パーサ**で 2 層
/// マージすると、(a) 実 `cursor.*` が `SquareFill{fill:(105,25,25), text:(255,255,255)}` へ解決され、
/// (b) 実フォント `Yu Gothic UI`／`font.height,28` を持ち、(c) validrect が非退化領域
/// (36,46)-(356,168) へ解決される（基層のみでは退化・画像別層マージで成立）。
#[test]
fn real_emo2_fixture_descript_resolves_square_fill_style() {
    let model = merged_balloon_model();

    // 実フォント（既定 ＭＳ ゴシック盲点の回避）。
    assert_eq!(
        model.font().name(),
        Some("Yu Gothic UI"),
        "実 descript は font.name に実フォントを指定する"
    );
    assert_eq!(
        model.font().height(),
        Some(28),
        "実 descript の font.height は 28（in-code でなく実 parse 値）"
    );

    // 実 cursor.* → SquareFill（値は実 descript の brush/font から来る——ハードコードでない）。
    let default_font = {
        let c = model.font().color();
        (c.r().unwrap_or(0), c.g().unwrap_or(0), c.b().unwrap_or(0))
    };
    let style = ResolvedChoiceStyle::resolve(Some(model.cursor()), default_font);
    assert_eq!(
        style,
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        },
        "実 emo2 fixture の cursor.* は SquareFill（塗り(105,25,25)＋白文字(255,255,255)）へ解決される"
    );
    assert_eq!(
        ResolvedBalloonText::resolve(&model, IMAGE_SIZE).choice_style,
        style,
        "ResolvedBalloonText.choice_style（通し経路が読む値）も同一 SquareFill"
    );
}

// ══ テスト 2: 実 menu.pasta を実 sakura パイプラインで cue 列へコンパイルする（GPU 不要） ══════

/// Observable（R6.2）: 実 `menu.pasta` の メインメニュー本文（`\q…\n\q…\_l[5em,2lh]\q…`）を
/// **実 sakura パイプライン**（`parse`→`compile`）へ通すと、(a) 3 つの `Choice` cue が id=target・
/// label=disp を欠落なく転写し、(b) `\_l[5em,2lh]` が `Cursor{x:"5em", y:"2lh"}` cue へ（不透明転写・
/// 字下げシーム）、(c) `\n` が `NewLine` cue へ、(d) `\q` 存在ゆえ末尾に `WaitForChoice` barrier が
/// ちょうど 1 個現れる（型／語彙シームの保持）。手組み注入でなく実台本由来であることの証明。
#[test]
fn real_menu_pasta_compiles_to_choice_cursor_cue_sequence() {
    let sakura = extract_main_menu_sakura();
    // 実台本断片は 3 項目＋字下げ（\_l[5em,2lh]）＋改行（\n）を持つ。
    assert!(
        sakura.contains("\\_l[5em,2lh]"),
        "実台本は字下げ \\_l[5em,2lh] を含む: {sakura}"
    );
    assert_eq!(
        sakura.matches("\\q[").count(),
        3,
        "実台本は \\q 選択肢 3 項目: {sakura}"
    );

    let instructions = areka_parsers::sakura::parse(&sakura);
    let compiled = compile(&instructions, &SystemVarSnapshot::default());
    let cues = compiled.sheet.cues();

    // (a) Choice cue が実台本の id/label を順序どおり転写する。
    let choices: Vec<(&str, &str)> = cues
        .iter()
        .filter_map(|c| match &c.payload {
            CuePayload::Command(CueCommand::Choice { id, text, .. }) => {
                Some((id.as_str(), text.as_str()))
            }
            _ => None,
        })
        .collect();
    let expected: Vec<(&str, &str)> = EXPECTED_MENU.to_vec();
    assert_eq!(
        choices, expected,
        "実 menu.pasta の Choice cue は id=target・label=disp を順序どおり転写する"
    );

    // (b) \_l[5em,2lh] → Cursor cue（不透明転写・字下げシーム・6.2）。
    let cursor = cues.iter().find_map(|c| match &c.payload {
        CuePayload::Command(CueCommand::Cursor { x, y }) => Some((x.as_str(), y.as_str())),
        _ => None,
    });
    assert_eq!(
        cursor,
        Some(("5em", "2lh")),
        "\\_l[5em,2lh] は Cursor{{x:\"5em\", y:\"2lh\"}} cue へ不透明転写される（字下げ）"
    );

    // (c) \n → NewLine cue（メインメニューは項目 1-2 間に 1 個）。
    let newlines = cues
        .iter()
        .filter(|c| matches!(&c.payload, CuePayload::Command(CueCommand::NewLine { .. })))
        .count();
    assert_eq!(
        newlines, 1,
        "メインメニュー本文の \\n は 1 個（項目1と2の間）"
    );

    // (d) \q 存在ゆえ末尾に WaitForChoice barrier がちょうど 1 個（語彙シーム・選択待ち）。
    let barriers = cues
        .iter()
        .filter(|c| {
            matches!(
                &c.payload,
                CuePayload::Barrier(areka_sakura::contract::BarrierKind::WaitForChoice { .. })
            )
        })
        .count();
    assert_eq!(
        barriers, 1,
        "\\q を持つ台本は WaitForChoice barrier を 1 個持つ"
    );
}

// ══ テスト 3: 実 fixture ＋ 実 cue 列を headless 描画し readback で検証する（GPU・headless） ══════

/// Observable（R6.1/6.2/6.3）: 実 emo2 fixture（実 descript の SquareFill＋実 validrect）へ、
/// 実 menu.pasta 由来の cue 列（Choice/NewLine/Cursor）を通し経路
/// （`register_actor`→`apply_cue`→`present_frame`→`inject_choice_hover`→`read_back`）で描くと、
/// (a) 3 選択肢が描かれ（非退化インク）ヒット行が実台本の id／label を反映し、(b) `\_l[5em,2lh]`
/// 由来で最終項目（ordinal 2）が字下げされ（rect.left が先頭項目より右）、(c) ordinal 1 の hover が
/// 実 fixture の SquareFill ハイライト（塗り(105,25,25)＋白文字）を当該行にのみ出す（非 hover 行は塗らない）。
/// 短メニュー＝領域内に収まりスクロールを要さない（6.3）。
///
/// 診断 PNG（実ジオメトリの実フォント出力）を `AREKA_DIAG_OUT`（無指定は `CARGO_TARGET_TMPDIR`）へ保存する。
#[test]
fn real_emo2_menu_cue_sequence_renders_and_hovers_headless() {
    let mut world = make_world_with_gpu();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let actor = ActorKey::from("0");

    // ── 実 fixture → 実 style ／ 実 menu.pasta → 実 cue 列 ──
    let model = merged_balloon_model();
    let sakura = extract_main_menu_sakura();
    let talk_cues = command_cues(&sakura);

    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, IMAGE_SIZE, IMAGE_SIZE),
        ResolvedBalloonText::resolve(&model, IMAGE_SIZE),
    );
    for cue in &talk_cues {
        rt.apply_cue(cue);
    }

    // 全リビール済みフレーム（t 大）。hover 前も選択肢テキストが描かれる。
    present_frame(&mut rt, &mut world, 100.0).expect("ベースライン提示");
    let rows: Vec<ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();

    // (a) ヒット行が実台本の 3 項目を id／label で反映する（distinct ordinal＝項目数）。
    let mut ordinals: Vec<usize> = rows.iter().map(|r| r.ordinal).collect();
    ordinals.sort_unstable();
    ordinals.dedup();
    assert_eq!(
        ordinals.len(),
        EXPECTED_MENU.len(),
        "distinct ヒット行 ordinal 数＝実メニュー項目数（3）: rows={rows:?}"
    );
    for (ordinal, (id, label)) in EXPECTED_MENU.iter().enumerate() {
        let row = rows
            .iter()
            .find(|r| r.ordinal == ordinal)
            .unwrap_or_else(|| panic!("ordinal {ordinal} のヒット行が在る: {rows:?}"));
        assert_eq!(&row.id, id, "ordinal {ordinal} の id は実台本 target");
        assert_eq!(
            &row.label, label,
            "ordinal {ordinal} の label は実台本 disp"
        );
    }

    let (w, h) = surface_size(&rt, &actor);
    let base = read_back(&rt, &actor);
    assert!(
        opaque_count(&base) > 0,
        "実フォント＋実 validrect で選択肢テキストが描画される（非退化）"
    );

    // (b) \_l[5em,2lh] 字下げ: 最終項目（ordinal 2）は先頭項目（ordinal 0）より右へ字下げされる。
    let row0 = rows.iter().find(|r| r.ordinal == 0).unwrap();
    let row2 = rows.iter().find(|r| r.ordinal == 2).unwrap();
    assert!(
        row2.rect.left > row0.rect.left,
        "\\_l[5em,2lh] により最終項目が字下げされる（ordinal2.left={} > ordinal0.left={}）",
        row2.rect.left,
        row0.rect.left
    );

    // ── (c) ordinal 1（「エモの位置調整」）を hover 注入 → 再提示 → read_back ──
    //
    // ホバーするのを**先頭でない行**にするのは、帯を行ボックスの中央へ寄せる量（先頭行の帯は
    // 面の縁より 4 画素下の y4 から）が行送りの先の行にも効くことを、下の縦範囲の検査で
    // 確かめるためである。ordinal 1 の帯の上端は行送り 1 つぶんとその寄せの分だけ下にあり、
    // 窓はその 4 画素上から見られる。
    rt.inject_choice_hover(&actor, Some(1));
    present_frame(&mut rt, &mut world, 100.0).expect("hover 提示");
    let hover = read_back(&rt, &actor);

    // read_back（canvas-local）を probe するため、窓物理ヒット矩形から validrect 原点（×k=1.0）を差し引く。
    let region = TextRegion::resolve(&model, IMAGE_SIZE, WritingMode::HorizontalTb);
    let (ox, oy) = (region.left(), region.top());
    let row0_cl = to_canvas_local(row0, ox, oy);
    let row1 = rows.iter().find(|r| r.ordinal == 1).unwrap();
    let row1_cl = to_canvas_local(row1, ox, oy);
    let row2_cl = to_canvas_local(row2, ox, oy);

    assert!(
        fill_pixels_in_rect(&hover, w, h, row1_cl) > 0,
        "hover 行に実 fixture の SquareFill 塗り色(105,25,25)画素が載る: canvas-local {row1_cl:?}"
    );
    assert!(
        white_pixels_in_rect(&hover, w, h, row1_cl) > 0,
        "hover 行に白文字(255,255,255)画素が載る: canvas-local {row1_cl:?}"
    );
    // 非 hover 行（ordinal 0＝hover 行の 1 つ上）へは塗り色画素が載らない
    //（hover 行のみハイライト。帯が上の行へ伸びていないことも同時に言える）。
    assert_eq!(
        fill_pixels_in_rect(&hover, w, h, row0_cl),
        0,
        "非 hover 行には SquareFill 塗り色画素が載らない: canvas-local {row0_cl:?}"
    );

    // ── (d) 実機不具合の回帰検査: hover 行のインクがハイライト矩形から**縦にほぼはみ出さない** ──
    //
    // 実機サインオフで「hover 中の選択肢の文字の下が切れる」が出た。真因は帯（ハイライト矩形／
    // ヒット矩形のブロック軸寸）が em ボックス丈（font.height=28）だったこと——DirectWrite は行を
    // ascent+descent（Yu Gothic UI 実測 1.3301em＝37.24px）で描くため descent のインクが帯の外に落ち、
    // 白背景バルーン＋白 hover 文字では「下が消えた」ように見える。
    //
    // 検査: hover セグメント（ordinal 1）の x 帯 × 「この行のヒット矩形の上端−余白 4 〜 次行
    // （ordinal 2）のヒット矩形の上端＋余白 4」（帯寸に依存しない独立の y 窓）を走査し、
    // **帯の丈が行送り 30 ちょうど**で、**帯が行ボックスの中央へ 4 画素寄って置かれ**、
    // **文字のインクの上端が塗り（fill 色）の上端の内側**にあり、**下端のはみ出しが 0** で
    // あることを要求する。帯を font.height に戻すと丈が 28 になって赤くなり、寄せを 0 へ戻すと
    // 帯の位置が y30..59 へ上がって（はみ出し 2 画素）やはり赤くなる。
    //
    // 窓を帯の上下へ 4 画素ずつ開けるのは、窓を帯とぴったり同じにすると帯からはみ出したインクが
    // 窓の外へ落ち、**測れないまま緑になる**からである（spec
    // `areka-P0-emo-text-line-height-canon` 要件 7.3）。これは上端にも同じく効く——窓の上端を帯の
    // 上端と揃えると「字の上端は帯の上端以上」が窓の取り方だけで必ず成り立ち、字が上へずれても
    // 緑のままになる。上の行（ordinal 0）の字は黒（premultiplied の青チャネルは 0）ゆえ白文字の
    // 述語に当たらず、塗り色 (25,25,105,255) でもないので窓へ入っても数えられない。次行の字は
    // 自分の block 起点から 7 画素下で始まるので、余白 4 では次行の字も窓へ入らない。
    //
    // インクは**文字だけ**を色で拾う（[`is_hover_text`]）。α だけで拾うと不透明な塗り帯そのものに
    // 当たり、返る縦範囲が帯の縦範囲になって「インクは帯の内」が字の有無に関わらず成り立つ。
    //
    // **裁定 2026-09-06（第 2 回・`areka-P0-emo2-conformance-e2e` 要件 13）**: 行送りが正典
    // （`font.height` ＋ 行間 2）へ確定した後、実フォントの読み戻しで帯の下端から文字のインクが
    // 下へ出た。同日の第 1 の裁定は「2 画素のはみ出しを許容する」だったが、開発者は実機の目視で
    // これを覆した——**帯を行ボックスの中央へ寄せる**のがあるべき姿である。本 fixture
    // （font.height 28・ordinal 1「エモの位置調整」）の実測は、寄せる前が 帯 y30..59（丈 30）に
    // 対し白文字 y37..61 ＝ 上の余白 7・はみ出し 2 画素、寄せた後が 帯 y34..63 に対し同じ白文字
    // y37..61 ＝ 上の余白 3・はみ出し **0** である（丈は変えていない）。
    // **帯を広げてはならない**（隣接行の帯と重なり、どの選択肢を指しているかの一意性が壊れる）。
    // **はみ出しが出た場合は許容幅を上げず、画素数を添えて改めて開発者の裁定を仰ぐこと（R12.2）。**
    /// ハイライト帯の丈＝行送り（`font.height` 28 ＋ 行間 2）。
    const BAND_HEIGHT: u32 = 30;
    /// 帯を行ボックスの中央へ寄せる量（画素・`font.height,28` の Yu Gothic UI）。
    /// `round(max(0, 37.24 − 30) / 2) = round(3.62) = 4`（要件 13.1）。
    const BAND_OFFSET: u32 = 4;

    assert_hover_band(
        "emo2-fixture-e2e",
        &hover,
        (w, h),
        row1_cl,
        row2_cl,
        Band {
            // ordinal 1 の行矩形の上端は行送り 1 つぶん下（y30）。寄せを 0 へ戻すと帯 y30..59 で赤。
            line_top: BAND_HEIGHT,
            height: BAND_HEIGHT,
            offset: BAND_OFFSET,
        },
    );

    // ── 診断: read_back（premultiplied BGRA）を白背景へ合成して PNG を保存する（実ジオメトリ目視） ──
    let path = dump_png_on_white("emo2_fixture_menu_hover.png", &hover, w, h);
    eprintln!(
        "[emo2-fixture-e2e] 実 fixture 統合 PNG を保存: {path} ({w}x{h}・emo2-kakukaku descript＋\
         menu.pasta メインメニュー 3 項目・ordinal 1「エモの位置調整」hover＝SquareFill 塗り(105,25,25)＋白文字)"
    );
}
