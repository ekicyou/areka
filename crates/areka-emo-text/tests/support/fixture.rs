//! # support/fixture.rs — `tests/` の検体・フォント・バルーンの検体の組の共有の部品
//!
//! `support/mod.rs` の子（`support::fixture`）。中身:
//!
//! 1. **検体の置き場と読み**——`emo2` の検体とその同梱バルーン・`examples/fixtures/` の変種・
//!    ファイルの読み（バルーンの定義は本番と同じ ANSI の規則・台本は UTF-8）・2 層のマージ。
//! 2. **実台本**——`menu.pasta` の見出しで指した本文と、台本を cue 列・文字の状態へ落とす道。
//! 3. **画像とフォント**——PNG の IHDR の寸・DirectWrite の工場・本物のフォントの門・
//!    バルーンの書体の文字の寸法。
//! 4. **バルーンの検体の組**（`areka_balloon_fixture`・`staysee_balloon_fixture`）——COM の初期化・
//!    焼き上がった面 0 の α の数え上げ・読み込みの記録の確かめ・配置の道と内包の確かめ。
//!
//! 読み込みの失敗と空のファイルは名指しで落とす（「読めなかったから対象 0 件で緑」を作らない）。

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use areka_emo_atlas::{AtlasPage, AtlasTable, Rect, SetId, Size};
use areka_emo_present::balloon::{ResolvedFace, load_scope_balloon_model, resolve_balloon_faces};
use areka_emo_text::actor::ResolvedBalloonText;
use areka_emo_text::draw::{DWriteMetrics, FontCatalog};
use areka_emo_text::layout::{LayoutEngine, PositionedLine, WrapPlan};
use areka_emo_text::state::{TextItem, TextLayerConfig, TextLayerState};
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_sakura::compile;
use areka_sakura::contract::{ActorKey, CuePayload, SystemVarSnapshot, TalkCue};
use log_capture_kit::{CapturedEvent, LineFormat, capture, format_line};
use sample_ghost_kit::SampleRoot;
use windows::Win32::Graphics::DirectWrite::{DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory2};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::com::dwrite::dwrite_create_factory;

// ══ 検体の置き場 ═══════════════════════════════════════════════════════════════════════

/// emo2 検体。段 ③ で `Drop` が複製を消すため、一時値にせずプロセス寿命で保持する。
static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// emo2 検体のゴーストフォルダ（pasta 辞書は `ghost/master/dic/` 下に在る）。
pub fn emo2_folder() -> PathBuf {
    EMO2.folder().to_path_buf()
}

/// emo2 が同時にインストールするバルーンのフォルダ（自分でパスを継ぎ足さない）。
pub fn emo2_balloon_root() -> PathBuf {
    EMO2.balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}

/// `examples/fixtures/<name>/`（`emo2-vertical` などの変種）。
pub fn example_fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("fixtures")
        .join(name)
}

// ══ ファイルの読み ═════════════════════════════════════════════════════════════════════

/// `dir` からの相対パス（`/` 区切り）のファイルを生バイトで読む。失敗と空は名指しで落とす。
pub fn read_fixture_bytes(dir: &Path, rel: &str) -> Vec<u8> {
    let path = rel.split('/').fold(dir.to_path_buf(), |p, seg| p.join(seg));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "検体 {} の読取に失敗した（このテストはこのファイルの実在が前提）: {e}",
            path.display()
        )
    });
    assert!(
        !bytes.is_empty(),
        "検体 {} が空である（空ファイルでは読み取り結果が主張と無関係になる）",
        path.display()
    );
    bytes
}

/// バルーンの定義ファイルを本番と同じ規約で読む。
///
/// 本番経路 `areka_emo_present::balloon::load_scope_balloon_model` は
/// `decode(&bytes, DefaultEncoding::Ansi)`（**既定 Ansi・ファイル内の `charset` 宣言優先**）で
/// 読む。本番と 1 文字でも違う規約で読むと「テストは緑だが本番は別物」になり得るので、
/// バルーンの定義はすべてここを通す。
pub fn read_balloon_file(dir: &Path, rel: &str) -> String {
    decode(&read_fixture_bytes(dir, rel), DefaultEncoding::Ansi)
}

/// 台本（pasta 辞書・メニュー台本）を UTF-8 として読む。
///
/// 台本は `charset` 宣言を持たない（ゴーストの宣言は `master/descript.txt` の `charset,UTF-8` が担う）。
pub fn read_script_file(dir: &Path, rel: &str) -> String {
    decode(&read_fixture_bytes(dir, rel), DefaultEncoding::Utf8)
}

/// `descript.txt`（基層）に面別上書き層を重ねた `BalloonModel`（本番と同じ `parse_str`）。
pub fn merged_model(dir: &Path, overlay: Option<&str>) -> BalloonModel {
    let overlay = overlay.map(|name| read_balloon_file(dir, name));
    parse_str(&read_balloon_file(dir, "descript.txt"), overlay.as_deref())
}

// ══ 実台本 ═════════════════════════════════════════════════════════════════════════════

/// 見出しで指した台本の本文（純さくらスクリプト断片）を実 `menu.pasta` から取り出す。
///
/// pasta の見出しは `＊<名前>` の行で、本文はその直後の `　　　エモ：＠通常　<さくらスクリプト>`
/// 行である。話者接頭辞は最初の ASCII `\`（さくらスクリプトの開始）より前ゆえ、最初の `\` 以降が
/// 純さくらスクリプト断片になる。
pub fn menu_script(heading: &str) -> String {
    let pasta = read_script_file(&emo2_folder(), "ghost/master/dic/menu.pasta");
    let mut lines = pasta.lines();
    lines
        .find(|l| l.trim_end() == heading)
        .unwrap_or_else(|| panic!("menu.pasta に見出し {heading} が在る"));
    let body = lines
        .find(|l| l.contains('\\'))
        .unwrap_or_else(|| panic!("見出し {heading} の直後にさくらスクリプトを含む本文行が在る"));
    let start = body
        .find('\\')
        .expect("本文行はさくらスクリプト（\\）を含む");
    body[start..].to_string()
}

/// 実 sakura パイプライン（`parse` → `compile`）で台本を cue 列へ落とし、文字状態機械が
/// 消費する Command cue だけを配送エンベロープへ無変形複写する（Barrier/Routing は対象外）。
pub fn command_cues(script: &str) -> Vec<TalkCue> {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = compile(&instructions, &SystemVarSnapshot::default());
    let cues: Vec<TalkCue> = compiled
        .sheet
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
        .collect();
    assert!(
        !cues.is_empty(),
        "台本 `{script}` から Command cue が 1 つも出なかった（cue 0 件では以降の観測が空振りになる）"
    );
    cues
}

/// cue 列を実 `TextLayerState` へ載せ、（状態, 唯一の actor）を返す。
pub fn state_of(cues: &[TalkCue]) -> (TextLayerState, ActorKey) {
    let mut state = TextLayerState::default();
    for cue in cues {
        state.apply_cue(cue);
    }
    let actors: Vec<ActorKey> = state.actors().map(|(k, _)| k.clone()).collect();
    assert_eq!(
        actors.len(),
        1,
        "台本は 1 スコープぶんの cue 列のはず（実測の actor: {actors:?}）"
    );
    (state, actors[0].clone())
}

// ══ 画像とフォント ═════════════════════════════════════════════════════════════════════

/// PNG の IHDR から原寸 `(width, height)` を読む（署名 8B ＋ 長さ 4B ＋ `IHDR` 4B の直後）。
///
/// 画像デコーダ（WIC）を持ち込まないのは、呼ぶテストを非 GPU・非 COM に保つため。IHDR が
/// ストリーム先頭に在ることは PNG 仕様が定めているので固定オフセットで足りる。
pub fn png_native_size(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("バルーンの絵 {} の読取に失敗した: {e}", path.display()));
    assert!(
        bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR",
        "{} が PNG（先頭 IHDR チャンク付き）として読めない（{} バイト）",
        path.display(),
        bytes.len()
    );
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    (w, h)
}

/// DirectWrite の factory（文字の寸法を測るのに要るのはこれだけ——実 GPU も実窓も要らない）。
pub fn dwrite_factory() -> IDWriteFactory2 {
    dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)
        .expect("DirectWrite factory を生成できる（文字の寸法の観測はこれが前提）")
}

/// 期待値の前提にしている本物のフォント。
pub const REAL_FONT: &str = "Yu Gothic UI";

/// 当該のフォント名がこの機械にインストールされているか——**本番の判断関数**
/// `FontCatalog::family_for` で判定する。
pub fn font_is_installed(name: &str) -> bool {
    let catalog = FontCatalog::new(&dwrite_factory()).expect("FontCatalog::new");
    catalog.family_for(&[name.to_owned()]).is_some()
}

/// 本物のフォントが無ければ赤で止める（代替フォントへ縮退したまま緑にしない）。
pub fn assert_real_font_present() {
    assert!(
        font_is_installed(REAL_FONT),
        "実フォント {REAL_FONT} がこの機械に無い。期待値は当該フォントの実測を前提にしているので、\
         代替フォントへ縮退したまま緑にしない"
    );
}

/// バルーンを解いた書体で実測の文字の寸法を組む（正典の行間・GPU 不要）。
pub fn balloon_metrics(resolved: &ResolvedBalloonText) -> DWriteMetrics {
    DWriteMetrics::new(
        &dwrite_factory(),
        &resolved.font,
        resolved.mode,
        &TextLayerConfig::default(),
    )
    .expect("バルーンの書体で文字の寸法を組める")
}

// ══ バルーンの検体の組: 焼き込みと α ═══════════════════════════════════════════════════

/// 各 scope で面 0 として焼き込まれる枠画像の名前（本体側 `balloons0`・相方側 `balloonk0`）。
pub const SCOPE_FACE0_FILES: [(u32, &str); 2] = [(0, "balloons0.png"), (1, "balloonk0.png")];

/// COM を当スレッドで MTA として初期化する。
///
/// テストは既定で並列に走るので `S_FALSE`／`RPC_E_CHANGED_MODE` が返りうるが、どちらも
/// 「このスレッドで COM が使える」ことに変わりはないので無視する。
pub fn init_com_for_this_thread() {
    // SAFETY: COM の MTA 初期化。戻り値は上記のとおり無視してよい。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// 走査した画素の α の内訳。
#[derive(Debug, Default)]
pub struct AlphaCensus {
    /// 走査した画素の総数。
    pub total: u64,
    /// α ＝ 0（完全な透明）。
    pub transparent: u64,
    /// 0 < α < 255（半透明）。
    pub partial: u64,
    /// α ＝ 255（完全な不透明）。
    pub opaque: u64,
}

/// アトラス頁の矩形 `uv` を 1 画素ずつ走査して α の内訳を数える。
///
/// 頁は premultiplied BGRA（1 画素 4 バイト・α は 4 バイト目）で、行の先頭は `stride`
/// バイトごとに並ぶ（頁の幅 × 4 とは限らない）。
fn census_alpha(page: &AtlasPage, uv: Rect) -> AlphaCensus {
    assert!(
        uv.x + uv.w <= page.width && uv.y + uv.h <= page.height,
        "走査する矩形 {uv:?} が頁 {}×{} の外へ出ている（はみ出したまま数えると内訳が意味を失う）",
        page.width,
        page.height
    );
    let stride = page.stride as usize;
    let mut census = AlphaCensus::default();
    for row in 0..uv.h {
        let row_head = (uv.y + row) as usize * stride;
        for col in 0..uv.w {
            census.total += 1;
            match page.bytes[row_head + (uv.x + col) as usize * 4 + 3] {
                0 => census.transparent += 1,
                255 => census.opaque += 1,
                _ => census.partial += 1,
            }
        }
    }
    census
}

/// 焼き上がった面 0 のエントリの原寸が `size` であることを確かめ、頁の矩形の α を数える。
///
/// 走査した総数が矩形の面積と一致し、母数が 0 でなく、内訳の合計が総数と一致することまで言う。
pub fn census_face0(
    table: &AtlasTable,
    scope: u32,
    file_name: &str,
    size: (u32, u32),
) -> AlphaCensus {
    let id = table.resolve(SetId(0), file_name).unwrap_or_else(|| {
        panic!(
            "scope {scope}: 焼き上がりの索引に面 0 の `{file_name}` が無い（索引の項目は {} 件）",
            table.len()
        )
    });
    let entry = table.entry(id);
    let (w, h) = size;
    assert_eq!(
        entry.original,
        Size { w, h },
        "scope {scope}: 面 0 `{file_name}` の原寸が期待 {w}×{h} と違う"
    );
    let placement = entry.placement.as_ref().unwrap_or_else(|| {
        panic!("scope {scope}: 面 0 `{file_name}` に割り当て結果が無い（全画素が透明と判断された）")
    });
    let page = table.page(placement.page).unwrap_or_else(|| {
        panic!(
            "scope {scope}: 面 0 `{file_name}` の割り当て先の頁 {} が無い（頁は {} 枚）",
            placement.page,
            table.pages().len()
        )
    });
    let uv = placement.uv_rect;
    let census = census_alpha(page, uv);
    assert_eq!(
        census.total,
        u64::from(uv.w) * u64::from(uv.h),
        "scope {scope}: 面 0 `{file_name}` で走査した画素の総数が矩形 {}×{} の面積と違う（実測 {}）",
        uv.w,
        uv.h,
        census.total
    );
    assert!(
        census.total > 0,
        "scope {scope}: 面 0 `{file_name}` の走査範囲が空である（母数 0 では内訳の主張は何も確かめない）"
    );
    assert_eq!(
        census.transparent + census.partial + census.opaque,
        census.total,
        "scope {scope}: 面 0 `{file_name}` の内訳の合計が走査総数と合わない（実測 {census:?}）"
    );
    census
}

/// 半透明が残り、走査が矩形の一部分に偏っていない（不透明・透明・半透明の 3 種がすべて実在する）。
pub fn assert_three_alpha_kinds(scope: u32, file_name: &str, census: &AlphaCensus) {
    assert!(
        census.partial > 0,
        "scope {scope}: 面 0 `{file_name}` に半透明の画素が 1 つも無い（α が 2 値へ潰れている）。内訳 {census:?}"
    );
    assert!(
        census.opaque > 0,
        "scope {scope}: 面 0 `{file_name}` に完全な不透明の画素が 1 つも無い（中身が焼けていないか走査が偏っている）。内訳 {census:?}"
    );
    assert!(
        census.transparent > 0,
        "scope {scope}: 面 0 `{file_name}` に完全な透明の画素が 1 つも無い（角の抜けが焼けていないか走査が偏っている）。内訳 {census:?}"
    );
}

// ══ バルーンの検体の組: 読み込みの記録 ═════════════════════════════════════════════════

/// 本番の読み込みの記録の宛先（`resolve_balloon_faces`／`load_scope_balloon_model` の発行元）。
///
/// 宛先まで確かめるのは、別の層が出した無関係な記録を数え込んで「件数が合っている」ように
/// 見えるのを防ぐためである。
pub const BALLOON_LOG_TARGET: &str = "areka_emo_present::balloon";

/// 本番の読み込みの経路（面の系列解決→面 0 に `descript.txt` と面別の設定を重ねたモデル）を
/// 走らせ、（面の系列, 面 0 のモデル）を返す。系列解決の失敗と面 0 の不在は名指しで落とす。
pub fn load_face0(root: &Path, scope: u32) -> (Vec<ResolvedFace>, BalloonModel) {
    let faces = resolve_balloon_faces(root, scope)
        .unwrap_or_else(|e| panic!("scope {scope} の面の系列解決が失敗した: {e}"));
    let face0 = faces
        .iter()
        .find(|f| f.surface_id == 0)
        .unwrap_or_else(|| panic!("scope {scope}: 面 0 が解決されない"));
    let model = load_scope_balloon_model(root, scope, face0);
    (faces, model)
}

/// [`load_face0`] を**同じ捕捉窓**で走らせ、（面の系列, 面 0 のモデル, 記録）を返す。
pub fn load_face0_with_records(
    root: &Path,
    scope: u32,
) -> (Vec<ResolvedFace>, BalloonModel, Vec<CapturedEvent>) {
    let ((faces, model), events) = capture(|| load_face0(root, scope));
    (faces, model, events)
}

/// 記録を 1 件 1 行の読める形へ落とす（assert の失敗文言に実測を添えるため）。
pub fn record_lines(events: &[CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| format_line(e, LineFormat::LevelTargetFields))
        .collect()
}

/// 読み込みの窓の記録を突き合わせる。
///
/// - 対照: 宛先の情報の記録がちょうど 2 件で、系列解決の完了（欄 `chain`）と 2 層マージの確定
///   （欄 `validrect_left`）が 1 件ずつ（「0 件」の主張が捕捉の空振りで真にならないため）。
/// - `ERROR` は 0 件。`WARN` は面の縮退の記録だけで、欄（宛先・scope・面 id・接頭辞）まで
///   `expected_fallback` と一致する。
pub fn assert_balloon_loading_records(
    scope: u32,
    events: &[CapturedEvent],
    expected_fallback: &[(u32, &str)],
) {
    let lines = record_lines(events);
    let infos: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::INFO && e.target == BALLOON_LOG_TARGET)
        .collect();
    assert_eq!(
        infos.len(),
        2,
        "scope {scope}: 宛先 {BALLOON_LOG_TARGET} の情報の記録が期待 2 件に対し実測 {} 件（0 件なら捕捉が空振りしている）: {lines:?}",
        infos.len()
    );
    for field in ["chain", "validrect_left"] {
        assert_eq!(
            infos.iter().filter(|e| e.field(field).is_some()).count(),
            1,
            "scope {scope}: 宛先 {BALLOON_LOG_TARGET} の情報の記録で欄 `{field}` を持つものが 1 件でない（発行点の変化）: {lines:?}"
        );
    }
    let errors: Vec<&String> = events
        .iter()
        .zip(&lines)
        .filter(|(e, _)| e.level == tracing::Level::ERROR)
        .map(|(_, l)| l)
        .collect();
    assert!(
        errors.is_empty(),
        "scope {scope}: 失敗の記録は 0 件であること。実測 {} 件: {errors:?}",
        errors.len()
    );
    let actual: Vec<(String, String, String, String)> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .map(|e| {
            let field = |name: &str| e.field(name).unwrap_or("<欄なし>").to_string();
            (
                e.target.clone(),
                field("scope"),
                field("surface_id"),
                field("prefix"),
            )
        })
        .collect();
    let expected: Vec<(String, String, String, String)> = expected_fallback
        .iter()
        .map(|(surface_id, prefix)| {
            (
                BALLOON_LOG_TARGET.to_string(),
                scope.to_string(),
                surface_id.to_string(),
                (*prefix).to_string(),
            )
        })
        .collect();
    assert_eq!(
        actual, expected,
        "scope {scope}: WARN の記録が面の縮退の期待どおりでない（縮退のほかは 0 件のはず）: {lines:?}"
    );
}

// ══ バルーンの検体の組: 配置の道 ═══════════════════════════════════════════════════════

/// 全角だけの本文（60 字）。
pub const FULL_WIDTH_BODY: &str = "あいうえおかきくけこ\
                                   あいうえおかきくけこ\
                                   あいうえおかきくけこ\
                                   あいうえおかきくけこ\
                                   あいうえおかきくけこ\
                                   あいうえおかきくけこ";
/// 半角だけの本文（120 字）。
pub const HALF_WIDTH_BODY: &str = "abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij\
                                   abcdefghijabcdefghijabcdefghijabcdefghijabcdefghijabcdefghij";
/// 全角と半角を交互に並べた本文（90 字・偶数番目が全角）。
pub const MIXED_BODY: &str = "あaいbうcえdおeかfきgくhけiこj\
                              あaいbうcえdおeかfきgくhけiこj\
                              あaいbうcえdおeかfきgくhけiこj\
                              あaいbうcえdおeかfきgくhけiこj\
                              あaいbうcえdおe";

/// 本文を 1 クラスタ 1 グリフの追記列へ写す（`\n` は行区切りとして写す）。
pub fn glyph_items(body: &str) -> Vec<TextItem> {
    areka_sakura::cluster::clusters(body)
        .map(|ch| {
            if ch == "\n" {
                TextItem::LineBreak { ratio: 1.0 }
            } else {
                TextItem::glyph(ch)
            }
        })
        .collect()
}

/// 追記列のうち可視グリフの数（非グリフのアイテムは序数を消費しない）。
pub fn glyph_count(items: &[TextItem]) -> usize {
    items
        .iter()
        .filter(|it| matches!(it, TextItem::Glyph { .. }))
        .count()
}

/// 解いたバルーンの描画範囲・書体で追記列を全文可視のまま本番と同じ折り返しの入口へ流す。
pub fn lay_out(resolved: &ResolvedBalloonText, items: &[TextItem]) -> Vec<PositionedLine> {
    LayoutEngine::layout(
        items,
        glyph_count(items),
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        &balloon_metrics(resolved),
        WrapPlan::CharByChar,
    )
}

/// 矩形 `(左, 上, 右, 下)` が範囲 `(左, 上, 右, 下)` の内側に収まることを 4 辺それぞれで言う。
pub fn assert_rect_inside(where_: &str, rect: (f32, f32, f32, f32), bounds: (f32, f32, f32, f32)) {
    let (left, top, right, low) = rect;
    let (b_left, b_top, b_right, b_bottom) = bounds;
    assert!(
        left >= b_left,
        "{where_} の左端 {left} が範囲の左辺 {b_left} より外に出た（超過 {} px）",
        b_left - left
    );
    assert!(
        right <= b_right,
        "{where_} の右端 {right} が範囲の右辺 {b_right} を超えた（超過 {} px）",
        right - b_right
    );
    assert!(
        top >= b_top,
        "{where_} の上端 {top} が範囲の上辺 {b_top} より外に出た（超過 {} px）",
        b_top - top
    );
    assert!(
        low <= b_bottom,
        "{where_} の下端 {low} が範囲の下辺 {b_bottom} を超えた（超過 {} px）",
        low - b_bottom
    );
}
