//! # emo2_fixture_e2e_test — 実 emo2 fixture のメニュー cue 列 headless 統合（task 11.2・R6.1/6.2/6.3）
//!
//! design.md「Testing Strategy > E2E/実機 item 1（emo2_boot 統合）」の正典述語を張る:
//! **実 emo2 fixture ＋ menu.pasta 経路でメニュー cue 列 →選択肢描画〔headless readback〕**。
//!
//! task 11.1（`choice_fixture_test.rs`）は tests 配下の **test-local** 最小 fixture で parse→resolve と
//! 実フォント render を固定した。task 11.2 は **リポジトリに実在する emo2 fixture**
//! （`crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/`）の実 descript／実 menu.pasta を
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
//!    ordinal 0 の hover が実 fixture の SquareFill ハイライト（塗り(105,25,25)＋白文字）を出すことを固定する。
//!
//! これが 6.1（E2E 証明）を実 fixture 内容で満たし、型／語彙シーム（6.2・Cursor/references/barrier の
//! 保持）を保ち、スクロール完全対応は追わない（6.3・短メニュー＝領域内に収まる）。

use std::path::PathBuf;

use areka_emo_text::actor::{
    ChoiceHitRow, ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame,
};
use areka_emo_text::choice::ResolvedChoiceStyle;
use areka_emo_text::region::TextRegion;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_sakura::compile;
use areka_sakura::contract::{ActorKey, CueCommand, CuePayload, SystemVarSnapshot, TalkCue};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};

// ── 実 emo2 fixture の所在（リポジトリ実在ファイル・test-local でない） ─────────────────────

/// 実 emo2 fixture ルート（`crates/pilot/…/fixtures/emo2/`）。
/// CARGO_MANIFEST_DIR（=`crates/areka-emo-text`）からの相対で辿る。
/// バルーン descript は `emo2-kakukaku/` 下・メニュー台本は `ghost/master/dic/` 下に在る。
fn emo2_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("pilot")
        .join("examples")
        .join("shiori-host-32")
        .join("fixtures")
        .join("emo2")
}

/// 実 fixture ファイル（fixtures/emo2 ルートからの相対パス）を読み、`charset` 宣言に従いデコードする
/// （parser-foundation の decode 経路）。descript／pasta とも UTF-8 ゆえ既定 UTF-8 で読む。
fn read_fixture(rel: &str) -> String {
    let mut path = emo2_fixture_root();
    for seg in rel.split('/') {
        path = path.join(seg);
    }
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "実 emo2 fixture {} の読取に失敗した（task 11.2 は実在 fixture を用いる）: {e}",
            path.display()
        )
    });
    decode(&bytes, DefaultEncoding::Utf8)
}

/// balloons0.png の実測原寸（region.rs `FIXTURE_IMAGE_SIZE` と同値・image px）。
const IMAGE_SIZE: (u32, u32) = (400, 224);

/// 実 `descript.txt`（基層）＋`balloons0s.txt`（画像別上書き層）を 2 層マージした BalloonModel。
/// 実機の balloon-parse と同一機構（`parse_str(descript, Some(image))`）——非退化領域が成立する。
fn merged_balloon_model() -> BalloonModel {
    let descript = read_fixture("emo2-kakukaku/descript.txt");
    let image_layer = read_fixture("emo2-kakukaku/balloons0s.txt");
    parse_str(&descript, Some(&image_layer))
}

/// 実 `menu.pasta` の `＊メインメニュー選択肢` 本文からさくらスクリプト断片を抽出する。
///
/// pasta 本文行は `　　　エモ：＠通常　<さくらスクリプト>` の形——pasta 話者接頭辞は最初の ASCII
/// `\`（さくらスクリプト開始）より前ゆえ、最初の `\` 以降を取れば純さくらスクリプト断片になる。
/// メインメニュー本文行は `\q[おしゃべり頻度` と `\q[閉じる` を同時に含む唯一の行で一意に識別できる。
fn extract_main_menu_sakura() -> String {
    let pasta = read_fixture("ghost/master/dic/menu.pasta");
    let line = pasta
        .lines()
        .find(|l| l.contains("\\q[おしゃべり頻度") && l.contains("\\q[閉じる"))
        .expect("menu.pasta に メインメニュー選択肢 本文行（\\q[おしゃべり頻度…\\q[閉じる…）が在る");
    let start = line.find('\\').expect("本文行はさくらスクリプト（\\）を含む");
    line[start..].to_string()
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

    // 実フォント（既定 ＭＳ ゴシック盲点の回避・記憶 emo-text-byte-equiv-default-font-blindspot）。
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
    assert!(sakura.contains("\\_l[5em,2lh]"), "実台本は字下げ \\_l[5em,2lh] を含む: {sakura}");
    assert_eq!(sakura.matches("\\q[").count(), 3, "実台本は \\q 選択肢 3 項目: {sakura}");

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
    assert_eq!(newlines, 1, "メインメニュー本文の \\n は 1 個（項目1と2の間）");

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
    assert_eq!(barriers, 1, "\\q を持つ台本は WaitForChoice barrier を 1 個持つ");
}

// ══ テスト 3: 実 fixture ＋ 実 cue 列を headless 描画し readback で検証する（GPU・headless） ══════

/// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World（headless・MTA・実窓なし）。
fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

/// emo-present `VisualMount` と同型の予約スロット（(window, slot) を返す）。
fn spawn_reserved_slot(world: &mut World) -> (bevy_ecs::entity::Entity, bevy_ecs::entity::Entity) {
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((Name::new("emo-text-layer-slot"), Visual::default(), ChildOf(window)))
        .id();
    world.flush();
    (window, slot)
}

/// コンパイル済み CueSheet の Command cue を配送エンベロープ `TalkCue` へ無変形複写する
/// （Barrier/Routing は runtime の apply_cue 対象外——選択待ち／配送制御は本 spec スコープ外）。
fn command_talk_cues(sheet: &areka_sakura::contract::CueSheet) -> Vec<TalkCue> {
    sheet
        .cues()
        .iter()
        .filter_map(|c| match &c.payload {
            CuePayload::Command(cmd) => Some(TalkCue {
                at: c.start_time,
                actor: c.actor.clone(),
                command: cmd.clone(),
                duration: c.duration,
            }),
            _ => None,
        })
        .collect()
}

/// ヒット行（窓物理 px）→ 供給面 readback（validrect-local＝canvas-local）矩形へ変換する。
///
/// 供給面は `ceil(validrect 寸 × k)` サイズで validrect 原点×k の窓内 offset へ mount される
/// （surface.rs「Arrangement offset＝validrect 原点×k」）。ゆえに read_back は canvas-local——
/// 窓物理ヒット矩形から validrect 原点（`region` の left/top×k）を差し引いて probe 座標へ戻す。
fn to_canvas_local(r: &ChoiceHitRow, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (r.rect.left - ox, r.rect.top - oy, r.rect.right - ox, r.rect.bottom - oy)
}

/// SquareFill 塗り色（105,25,25・premultiplied α=255 ゆえ BGRA=(25,25,105,255)）を矩形内で数える。
fn fill_pixels_in_rect(bytes: &[u8], width: u32, height: u32, rect: (f32, f32, f32, f32)) -> usize {
    count_in_rect(bytes, width, height, rect, |px| {
        px[0] == 25 && px[1] == 25 && px[2] == 105 && px[3] == 255
    })
}

/// 白文字（≈255,255,255・全チャネル閾値で AA 端を除いた芯）を矩形内で数える。
fn white_pixels_in_rect(bytes: &[u8], width: u32, height: u32, rect: (f32, f32, f32, f32)) -> usize {
    count_in_rect(bytes, width, height, rect, |px| {
        px[0] >= 200 && px[1] >= 200 && px[2] >= 200 && px[3] == 255
    })
}

fn count_in_rect(
    bytes: &[u8],
    width: u32,
    height: u32,
    rect: (f32, f32, f32, f32),
    pred: impl Fn(&[u8]) -> bool,
) -> usize {
    let x0 = rect.0.floor().max(0.0) as u32;
    let x1 = (rect.2.ceil() as u32).min(width);
    let y0 = rect.1.floor().max(0.0) as u32;
    let y1 = (rect.3.ceil() as u32).min(height);
    let mut n = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if pred(&bytes[i..i + 4]) {
                n += 1;
            }
        }
    }
    n
}

fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// 指定 x 帯 `[x0, x1)` × y 帯 `[y0, y1)` の中で、述語に合う画素が現れた **y の最小/最大**を返す
/// （1 つも無ければ `None`）。ハイライト帯とインクの縦範囲を突き合わせるための走査。
fn y_span_where(
    bytes: &[u8],
    width: u32,
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    pred: impl Fn(&[u8]) -> bool,
) -> Option<(u32, u32)> {
    let mut span: Option<(u32, u32)> = None;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if pred(&bytes[i..i + 4]) {
                span = Some(match span {
                    None => (y, y),
                    Some((lo, _)) => (lo, y),
                });
                break;
            }
        }
    }
    span
}

/// Observable（R6.1/6.2/6.3）: 実 emo2 fixture（実 descript の SquareFill＋実 validrect）へ、
/// 実 menu.pasta 由来の cue 列（Choice/NewLine/Cursor）を通し経路
/// （`register_actor`→`apply_cue`→`present_frame`→`inject_choice_hover`→`read_back`）で描くと、
/// (a) 3 選択肢が描かれ（非退化インク）ヒット行が実台本の id／label を反映し、(b) `\_l[5em,2lh]`
/// 由来で最終項目（ordinal 2）が字下げされ（rect.left が先頭項目より右）、(c) ordinal 0 の hover が
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
    let instructions = areka_parsers::sakura::parse(&sakura);
    let compiled = compile(&instructions, &SystemVarSnapshot::default());
    let talk_cues = command_talk_cues(&compiled.sheet);

    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, IMAGE_SIZE),
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
        assert_eq!(&row.label, label, "ordinal {ordinal} の label は実台本 disp");
    }

    let (w, h) = rt.surface(&actor).expect("供給面").size();
    let base = rt.surface(&actor).expect("供給面").read_back().expect("read_back");
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

    // ── (c) ordinal 0 を hover 注入 → 再提示 → read_back ──
    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, 100.0).expect("hover 提示");
    let hover = rt.surface(&actor).expect("供給面").read_back().expect("read_back");

    // read_back（canvas-local）を probe するため、窓物理ヒット矩形から validrect 原点（×k=1.0）を差し引く。
    let region = TextRegion::resolve(&model, IMAGE_SIZE, WritingMode::HorizontalTb);
    let (ox, oy) = (region.left(), region.top());
    let row0_cl = to_canvas_local(row0, ox, oy);
    let row1 = rows.iter().find(|r| r.ordinal == 1).unwrap();
    let row1_cl = to_canvas_local(row1, ox, oy);

    assert!(
        fill_pixels_in_rect(&hover, w, h, row0_cl) > 0,
        "hover 行に実 fixture の SquareFill 塗り色(105,25,25)画素が載る: canvas-local {row0_cl:?}"
    );
    assert!(
        white_pixels_in_rect(&hover, w, h, row0_cl) > 0,
        "hover 行に白文字(255,255,255)画素が載る: canvas-local {row0_cl:?}"
    );
    // 非 hover 行（ordinal 1）へは塗り色画素が載らない（hover 行のみハイライト）。
    assert_eq!(
        fill_pixels_in_rect(&hover, w, h, row1_cl),
        0,
        "非 hover 行には SquareFill 塗り色画素が載らない: canvas-local {row1_cl:?}"
    );

    // ── (d) 実機不具合の回帰檻: hover 行のインクがハイライト矩形から**縦にはみ出さない** ──
    //
    // 実機サインオフで「hover 中の選択肢の文字の下が切れる」が出た。真因は帯（ハイライト矩形／
    // ヒット矩形のブロック軸寸）が em ボックス丈（font.height=28）だったこと——DirectWrite は行を
    // ascent+descent（Yu Gothic UI 実測 1.3301em＝37.24px）で描くため descent のインクが帯の外に落ち、
    // 白背景バルーン＋白 hover 文字では「下が消えた」ように見える。
    //
    // 檻: hover セグメントの x 帯 × 「この行の block 起点〜次行の block 起点」（帯寸に依存しない
    // 独立の y 窓）を走査し、**インク（α>0）の縦範囲が塗り（fill 色）の縦範囲の内側**に収まることを
    // 要求する。帯を font.height に戻すとインク下端が塗り下端より下に出て赤くなる。
    let x0 = row0_cl.0.floor().max(0.0) as u32;
    let x1 = (row0_cl.2.ceil() as u32).min(w);
    let y0 = row0_cl.1.floor().max(0.0) as u32;
    // y 窓の下端＝次行（ordinal 1）の block 起点——帯寸に依存しないため RED/GREEN で同一窓になる。
    let y1 = (row1_cl.1.floor().max(0.0) as u32).min(h);
    assert!(x0 < x1 && y0 < y1, "走査窓が非退化: x{x0}..{x1} y{y0}..{y1}");
    let fill_span = y_span_where(&hover, w, x0, x1, y0, y1, |px| {
        px[0] == 25 && px[1] == 25 && px[2] == 105 && px[3] == 255
    })
    .expect("hover 行に塗り画素が在る");
    let ink_span = y_span_where(&hover, w, x0, x1, y0, y1, |px| px[3] != 0)
        .expect("hover 行にインク（不透明画素）が在る");
    assert!(
        ink_span.0 >= fill_span.0,
        "hover 行のインク上端 y={} がハイライト矩形の上端 y={} より上に出ている（帯が痩せている）",
        ink_span.0,
        fill_span.0
    );
    assert!(
        ink_span.1 <= fill_span.1,
        "hover 行のインク下端 y={} がハイライト矩形の下端 y={} より下に出ている\
         ＝**文字の下が切れる**（帯が em ボックス丈のまま＝descent 分が塗りの外・R3.3/4.2）",
        ink_span.1,
        fill_span.1
    );
    eprintln!(
        "[emo2-fixture-e2e] hover 行の縦範囲: 塗り y{}..{} ⊇ インク y{}..{}（font.height=28・\
         Yu Gothic UI 行ボックス 37.24 → 帯はピッチ 35 で頭打ち）",
        fill_span.0, fill_span.1, ink_span.0, ink_span.1
    );

    // ── 診断: read_back（premultiplied BGRA）を白背景へ合成して PNG を保存する（実ジオメトリ目視） ──
    let out_dir = std::env::var("AREKA_DIAG_OUT")
        .unwrap_or_else(|_| env!("CARGO_TARGET_TMPDIR").to_string());
    let rgba = composite_on_white(&hover, w, h);
    let png = encode_png_rgba(&rgba, w, h);
    let path = format!("{out_dir}/emo2_fixture_menu_hover.png");
    std::fs::write(&path, &png).expect("PNG 書き込み");
    eprintln!(
        "[emo2-fixture-e2e] 実 fixture 統合 PNG を保存: {path} ({w}x{h}・emo2-kakukaku descript＋\
         menu.pasta メインメニュー 3 項目・ordinal 0 hover＝SquareFill 塗り(105,25,25)＋白文字)"
    );
}

// ── PNG エンコード（依存追加なし・choice_fixture_test.rs と同一アルゴリズム） ──────────────────

/// premultiplied BGRA read_back を白背景へ合成した RGBA8 を返す。
fn composite_on_white(bgra: &[u8], w: u32, h: u32) -> Vec<u8> {
    let bg = [255u8, 255, 255];
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for (idx, px) in bgra.chunks_exact(4).enumerate() {
        let (b, g, r, a) = (px[0] as u32, px[1] as u32, px[2] as u32, px[3] as u32);
        let inv = 255 - a;
        let o = idx * 4;
        rgba[o] = (r + bg[0] as u32 * inv / 255).min(255) as u8;
        rgba[o + 1] = (g + bg[1] as u32 * inv / 255).min(255) as u8;
        rgba[o + 2] = (b + bg[2] as u32 * inv / 255).min(255) as u8;
        rgba[o + 3] = 255;
    }
    rgba
}

/// CRC-32（PNG チャンク用・多項式 0xEDB88320）。
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Adler-32（zlib ストリーム末尾）。
fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// PNG チャンク（長さ＋種別＋データ＋CRC）を書き足す。
fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut kd = Vec::with_capacity(4 + data.len());
    kd.extend_from_slice(kind);
    kd.extend_from_slice(data);
    out.extend_from_slice(&kd);
    out.extend_from_slice(&crc32(&kd).to_be_bytes());
}

/// RGBA8 を無圧縮 deflate（stored ブロック）で PNG エンコードする（依存追加なし）。
fn encode_png_rgba(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut raw = Vec::with_capacity((h * (1 + w * 4)) as usize);
    for y in 0..h {
        raw.push(0u8); // フィルタ 0
        let row = ((y * w * 4) as usize)..(((y + 1) * w * 4) as usize);
        raw.extend_from_slice(&rgba[row]);
    }
    let mut zlib = vec![0x78u8, 0x01];
    let mut i = 0usize;
    while i < raw.len() {
        let chunk = (raw.len() - i).min(65535);
        let bfinal = if i + chunk >= raw.len() { 1u8 } else { 0u8 };
        zlib.push(bfinal); // BFINAL ＋ BTYPE=00（stored）
        zlib.extend_from_slice(&(chunk as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(chunk as u16)).to_le_bytes());
        zlib.extend_from_slice(&raw[i..i + chunk]);
        i += chunk;
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8bit・color type 6（RGBA）
    png_chunk(&mut out, b"IHDR", &ihdr);
    png_chunk(&mut out, b"IDAT", &zlib);
    png_chunk(&mut out, b"IEND", &[]);
    out
}
