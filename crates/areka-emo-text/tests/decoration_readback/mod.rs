//! # decoration_readback/mod.rs — 読み戻しの共有ヘルパ（task 8.1／8.2）
//!
//! 出典 spec: `areka-P0-text-decoration-canon`（要件 **15.4**／**15.7**／**15.8**）。
//!
//! 入口 `decoration_readback_test.rs`（横書き）と、縦書きの分（[`vertical`]・task 8.2）が
//! 共有する道具だけをここに置く。中身は 4 つ:
//!
//! 1. **描画環境の用意**——headless の GPU World（WARP 可）と予約スロット。
//! 2. **実行時の組み立て**——`TextLayerRuntime` へ台本を流し、本番の 1 フレーム経路
//!    （`present_frame`）で描く。
//! 3. **画素の読み戻し**——供給面の premultiplied BGRA を密配列で取る。
//! 4. **インク判定**——α の二値化と、「素の行に無くて装飾した行にある画素」の抽出。
//!
//! ## 本番経路を通すこと
//!
//! 描画は必ず `present_frame` を通す。ここで `LayoutEngine`／`ViewboxExecutor` を直に
//! 呼んで組み直すと、「装飾が画面に出る」という主張が**本番の配線を一切踏まないまま緑**に
//! なる（`actor_decoration_frame_tests.rs` と同じ考え方）。
//!
//! ## 実フォントの門
//!
//! 期待値は実フォント `Yu Gothic UI` を実際に描いた画素に依る。当該フォントが無い環境では
//! DirectWrite が代替へ落ち、前提が崩れたまま緑になり得る。ゆえに各検査の先頭で
//! [`assert_real_font_present`] を呼ぶ。門の判定には**本番の判断関数**
//! `FontCatalog::family_for`（`\f[name,…]` の候補列を解決するのと同じ口）を使う——門そのものが
//! 正しく閉じることは、入口の `the_font_gate_closes_on_a_missing_family` が実在しない名前で
//! 確かめる。

mod vertical;

use areka_emo_text::actor::{
    ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame,
};
use areka_emo_text::draw::FontCatalog;
use areka_emo_text::state::TextLayerConfig;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_sakura::contract::{ActorKey, CueCommand, FONT_TAG_CARRIER, TalkCue};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::Graphics::DirectWrite::{DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory2};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::com::dwrite::dwrite_create_factory;
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};

// ══ 台本と描画環境の定数 ═══════════════════════════════════════════════════════════════

/// 期待値の前提になる実フォント（プロポーショナル・実物 `emo2-kakukaku` と同じ指定）。
pub(crate) const REAL_FONT: &str = "Yu Gothic UI";

/// `\f[name,…]` の差し替え先（製品既定フォント——`REAL_FONT` と字形が明らかに違う）。
pub(crate) const ALT_FONT: &str = "ＭＳ ゴシック";

/// 実在しないフォント名（門の較正専用——これで門が閉じなければ門は働いていない）。
pub(crate) const MISSING_FONT: &str = "areka-no-such-font-xyz";

/// バルーン定義の文字の大きさ（image px）。
pub(crate) const FONT_HEIGHT: u32 = 28;

/// バルーン画像の原寸（image px・k=1 ゆえ物理原寸と同値）。
pub(crate) const IMAGE: (u32, u32) = (320, 200);

/// 全グリフが出そろう十分に後の注入時刻（reveal を待たない＝実時間 sleep 不使用）。
const SETTLED: f64 = 100.0;

/// 装飾の差を見る 1 行（仮名・カタカナ・漢字・英字を混ぜ、字形の差が出やすい形にする）。
///
/// 英字を混ぜるのは `italic`／`name` の差が仮名だけでは小さいことがあるため。
pub(crate) const SAMPLE: &str = "あア亜Agほ";

/// インクとみなす α の下限（アンチエイリアスの薄い端を落とす二値化・
/// `line_pitch_readback_test.rs` と同じ閾値）。
const INK_ALPHA_MIN: u8 = 128;

/// 表示に効く 7 項目（要件 5.1・`\f[…]` のトークン列）。
///
/// 横書き（入口）と縦書き（[`vertical`]）が**同じ表**を回す——片方だけ項目が抜けるのを防ぐ。
/// 母数は入口の `the_visible_keys_are_the_seven_that_directwrite_can_range` が固定する
/// （表が空になるとどちらのループも恒真で緑になる）。
pub(crate) const VISIBLE_KEYS: &[(&str, &[&str])] = &[
    ("name", &["name", ALT_FONT]),
    ("height", &["height", "44"]),
    ("color", &["color", "255", "0", "0"]),
    ("bold", &["bold", "1"]),
    ("italic", &["italic", "1"]),
    ("underline", &["underline", "1"]),
    ("strike", &["strike", "1"]),
];

// ══ バルーン定義 ═══════════════════════════════════════════════════════════════════════

/// 読み戻し用バルーンの `descript`（実 `parse_str` を通す ＝ in-code モデルを組み立てない）。
///
/// `writing_mode` は引数で足す（横書きは宣言しない＝既定の `horizontal_tb`）。
fn descript(writing_mode: Option<&str>) -> String {
    let mut s = format!(
        "charset,UTF-8\n\
         font.name,{REAL_FONT}\n\
         font.height,{FONT_HEIGHT}\n\
         font.color.r,0\n\
         font.color.g,0\n\
         font.color.b,0\n\
         validrect.top,5\n\
         validrect.left,5\n\
         validrect.right,-5\n\
         validrect.bottom,-5\n"
    );
    if let Some(mode) = writing_mode {
        s.push_str("writing_mode,");
        s.push_str(mode);
        s.push('\n');
    }
    s
}

/// 実 parser を通したバルーン定義。
pub(crate) fn model(writing_mode: Option<&str>) -> BalloonModel {
    parse_str(&descript(writing_mode), None)
}

// ══ 実フォントの門（本番の判断関数で判定する） ═════════════════════════════════════════

fn factory() -> IDWriteFactory2 {
    dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DirectWrite factory を生成できる")
}

/// 当該のフォント名がこの機械にインストールされているか——**本番の判断関数**
/// `FontCatalog::family_for` で判定する。
pub(crate) fn font_is_installed(name: &str) -> bool {
    let catalog = FontCatalog::new(&factory()).expect("FontCatalog::new");
    catalog.family_for(&[name.to_owned()]).is_some()
}

/// 実フォントが無ければ赤で止める（代替フォントのまま緑にしない）。
pub(crate) fn assert_real_font_present() {
    assert!(
        font_is_installed(REAL_FONT),
        "実フォント {REAL_FONT} がこの機械に無い。本ファイルの期待値は当該フォントの実描画を\
         前提にしているので、代替フォントへ縮退したまま緑にしない"
    );
}

// ══ cue（`\f` は再生時間 0 の汎用キャリア） ════════════════════════════════════════════

fn cue(command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command,
        duration: 0.0,
    }
}

/// `\f[トークン…]` を運ぶ cue（`state.rs` が名前で自己選別して装飾へ届く）。
pub(crate) fn font_cue(tokens: &[&str]) -> TalkCue {
    cue(CueCommand::command_carrier(
        FONT_TAG_CARRIER,
        tokens.iter().map(|t| (*t).to_owned()).collect(),
    ))
}

/// 同じトークン列を**装飾ではない運搬名**で運ぶ cue（較正の対照）。
///
/// `state_decoration.rs::font_tag_tokens` の自己選別は名前が合わない運搬を従来どおり
/// 読み飛ばすので、装飾は 1 つも焼かれない（配置へ渡る番号列は全部 0・描画へ渡る装飾の表は
/// 空＝`render_styled` が既定の呼出列に落ちる）。cue の本数・並び・再生時間は
/// [`font_cue`] と 1 ビットも変わらないため、「画素が違う」という判定が**装飾を焼いたこと**
/// に由来する（cue が 1 本増えたことや実行のばらつきに由来するのではない）ことを示せる。
pub(crate) fn unbaked_cue(tokens: &[&str]) -> TalkCue {
    cue(CueCommand::command_carrier(
        "\\!",
        tokens.iter().map(|t| (*t).to_owned()).collect(),
    ))
}

pub(crate) fn text_cue(body: &str) -> TalkCue {
    cue(CueCommand::Text(body.to_owned()))
}

// ══ 描画環境の用意と 1 フレームの実行 ══════════════════════════════════════════════════

/// 読み戻した供給面（premultiplied BGRA 密配列＋面の寸）。
pub(crate) struct Shot {
    pub(crate) bytes: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Shot {
    fn px(&self, x: u32, y: u32) -> &[u8] {
        let i = ((y * self.width + x) * 4) as usize;
        &self.bytes[i..i + 4]
    }

    pub(crate) fn is_ink(&self, x: u32, y: u32) -> bool {
        self.px(x, y)[3] >= INK_ALPHA_MIN
    }

    /// 面全体の非透明画素数（退化していないことの前提確認用）。
    pub(crate) fn ink_count(&self) -> usize {
        self.bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
    }
}

/// `GraphicsCore` ＋ `WucGraphicsResource` を実資源として載せた wintf World（headless・MTA）。
fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視——テストスレッド毎）。
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

/// emo-present `VisualMount` と同型の予約スロット。
fn spawn_reserved_slot(world: &mut World) -> (bevy_ecs::entity::Entity, bevy_ecs::entity::Entity) {
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((
            Name::new("emo-text-decoration-readback-slot"),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    world.flush();
    (window, slot)
}

/// 台本 1 本を**本番の 1 フレーム経路**（`present_frame`）へ通し、供給面を読み戻す。
///
/// **順序が効く**——装着（`register_actor`）を先に、台本の投入を後にする。装着はバルーン定義から
/// 導いた 2 層（既定・無効表示）を表示状態へ差し込む唯一の点で、現在の見た目が既に既定から
/// 動いていると新しい既定へ追随しない（要件 4.1・task 4.2 の「装着時に 2 層を差し込む口」）。
/// 装着より前に `\f[bold,1]` を流すと、以後その actor の文字は**バルーン定義の 28 px ではなく
/// 素の既定 12 px**で描かれる。本番の起動順（装着 → 発話）はこの向きで、
/// `line_pitch_readback_test.rs` の組み立ても同じである。
pub(crate) fn run(writing_mode: Option<&str>, script: &[TalkCue]) -> Shot {
    let mut world = make_world_with_gpu();
    let (window, slot) = spawn_reserved_slot(&mut world);

    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE),
        ResolvedBalloonText::resolve(&model(writing_mode), IMAGE),
    );
    for c in script {
        rt.apply_cue(c);
    }
    present_frame(&mut rt, &mut world, SETTLED).expect("装着＋描画フレーム");

    let surface = rt.surface(&actor).expect("装着済み actor の供給面");
    let (width, height) = surface.size();
    Shot {
        bytes: surface.read_back().expect("read_back"),
        width,
        height,
    }
}

/// 「素の台本」と「装飾を足した台本」の 2 枚を撮る（同じ書字方向・同じ本文）。
///
/// 装飾ありの側は `\f[トークン…]` を本文の**前**に置く（要件 3.3——装飾は以降の文字に効く）。
pub(crate) fn shoot_pair(writing_mode: Option<&str>, tokens: &[&str]) -> (Shot, Shot) {
    let plain = run(writing_mode, &[text_cue(SAMPLE)]);
    let styled = run(writing_mode, &[font_cue(tokens), text_cue(SAMPLE)]);
    assert_eq!(
        (plain.width, plain.height),
        (styled.width, styled.height),
        "面の寸法は台本によらず同じ（前提）"
    );
    assert!(plain.ink_count() > 0, "素の台本にもインクがある（前提）");
    (plain, styled)
}

// ══ インク判定 ═════════════════════════════════════════════════════════════════════════

/// 2 枚の画素が 1 バイトでも違うか。
pub(crate) fn differs(a: &Shot, b: &Shot) -> bool {
    a.bytes != b.bytes
}

/// 「素の側にインクが無く、装飾した側にインクがある」画素を持つ**行**（y）の一覧。
///
/// 下線・打ち消し線はこの一覧に**連続した帯**として現れる（線は字の隙間も跨いで引かれるので、
/// 字の無い x にもインクが増える）。横書き用——縦書きは列で数える（task 8.2）。
pub(crate) fn added_ink_rows(plain: &Shot, styled: &Shot) -> Vec<u32> {
    (0..plain.height)
        .filter(|&y| (0..plain.width).any(|x| styled.is_ink(x, y) && !plain.is_ink(x, y)))
        .collect()
}

/// [`added_ink_rows`] の縦書き用の対——「素の側にインクが無く、装飾した側にインクがある」
/// 画素を持つ**列**（x）の一覧。
///
/// 縦書きでは行が縦に並び、下線・打ち消し線は列に沿った**縦のインク**として現れるので、
/// 行で数えると字のある行すべてに散らばって帯にならない。数える軸だけが横書きと違う。
pub(crate) fn added_ink_cols(plain: &Shot, styled: &Shot) -> Vec<u32> {
    (0..plain.width)
        .filter(|&x| (0..plain.height).any(|y| styled.is_ink(x, y) && !plain.is_ink(x, y)))
        .collect()
}

/// インクのある列の左右端（縦書きの「字の列」が面のどこにあるか）。
///
/// 線がその列の**どちら側**に出たかを言うための基準。素の側で測る——装飾で足された線を
/// 基準に混ぜると、線が自分自身を基準に判定されて向きの意味が消える。
pub(crate) fn ink_col_range(shot: &Shot) -> (u32, u32) {
    let cols: Vec<u32> = (0..shot.width)
        .filter(|&x| (0..shot.height).any(|y| shot.is_ink(x, y)))
        .collect();
    assert!(!cols.is_empty(), "インクのある列が 1 本も無い");
    (cols[0], cols[cols.len() - 1])
}

/// 一覧が連続した 1 本の帯であることを確かめ、その両端を返す（行にも列にも使う）。
pub(crate) fn single_band(lanes: &[u32], what: &str) -> (u32, u32) {
    assert!(
        !lanes.is_empty(),
        "{what}: インクの増えた行／列が 1 本も無い"
    );
    let (first, last) = (lanes[0], lanes[lanes.len() - 1]);
    assert_eq!(
        lanes.len() as u32,
        last - first + 1,
        "{what}: インクの増えた行／列が連続した 1 本の帯になっていない: {lanes:?}"
    );
    (first, last)
}
