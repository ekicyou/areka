//! # emo-text-layer — 注入時刻駆動の観測用専用 example（task 9.2・R11.1–11.5/11.7–11.9）
//!
//! emo2 fixture のバルーン枠（`balloons0.png`）2 窓（\0=sakura／\1=kero）を実表示し、
//! fixture スクリプト（`ghost/master/dic/boot.pasta` の起動朝挨拶）由来のハードコード
//! cue 列を **注入時刻駆動**（`talk_time`＝フレーム時刻−talk 開始・実時間 sleep 不使用）
//! で流して、以下の一連を **読み戻し（readback）述語**で自動判定し、最後に単一の
//! PASS/FAIL を 1 行出力する（exit code 連動: PASS=0／FAIL=1）。
//!
//! 1. **typewriter 進行**（R11.2）: 可視ピクセル数の単調増加。
//! 2. **改行**（R11.1）: `NewLine` cue で行送り軸方向へインク範囲が拡大する。
//! 3. **あふれ→スクロール**（R11.3）: validrect あふれで先頭行が可視窓から消える
//!    （先頭バンドの行内インク範囲が縮む）ことを readback で確認。
//! 4. **全消去**（R11.5）: `Clear` cue でテキスト領域が全透明へ戻る。
//! 5. **複数 actor の振り分け**（R11.8）: \0/\1 が各自のバルーン（別 target・別供給面）へ
//!    独立に描画され、\0 側の更新・Clear が \1 側の供給面バイト列へ波及しない。
//!
//! # 使い方
//! ```text
//! cargo run -p areka-emo-text --example emo-text-layer               # 横書き（共有 fixture・既定 horizontal_tb）
//! cargo run -p areka-emo-text --example emo-text-layer -- --vertical # 縦書き（fixture 変種 emo2-vertical）
//! cargo run -p areka-emo-text --example emo-text-layer -- --hold     # 目視確認（自動クローズせず talk をループ・balloon 上ダブルクリックで終了）
//! ```
//! 既定ではシナリオが自動進行し（約 4 秒）、完了すると窓を閉じて `PASS`／`FAIL` を出力して終了する。
//! `--hold` を付けると自動クローズせず talk をループ再生し、balloon 窓上での左ダブルクリックで終了する
//! （実機での目視確認用・`--vertical` と併用可）。
//!
//! # fixture（R11.4/R11.7）
//!
//! - **枠画像**: 共有 fixture `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/`
//!   （emo-present example と同じ相対パス解決）。**共有 fixture は改変しない**。
//! - **balloon descript**: 通常起動は共有 fixture の `descript.txt`＋`balloons0s.txt`
//!   （2 層マージ・マーカー無し既定 `horizontal_tb` の裏取り）。`--vertical` は parse 入力
//!   だけを example ローカル変種 `examples/fixtures/emo2-vertical/`（`writing_mode,vertical_rl`
//!   ＋有意な `wordwrappoint.y`・`balloons0s.txt` 上書き層）へ差し替える。縦書きでは
//!   折返しが縦書き用閾値（`wordwrappoint.y` 軸）・スクロールが横方向へ切り替わることを
//!   同じ述語群（軸読み替え）で観測する。
//! - 本 example は新規ファイルのみで完結し、既存の `crates/areka` 配下ファイルは一切
//!   変更しない（R11.7）。
//!
//! # 実 DPI（dpi≠96）の手動確認手順（R11.9）
//!
//! 起動時に各バルーン窓の実モニタ DPI と合成スケール k を `info!` でログする。
//! PASS/FAIL の自動判定は readback 述語（物理 1:1・k=1.0 恒常の現行契約）で決定論だが、
//! **バルーン枠とテキストの整合は非 96 DPI 環境で実際に目視確認すること**
//! （emo-present example の task 5.3 手順と同型・記憶 areka-placement-real-ghost-first）:
//!
//! 1. Windows「設定 → システム → ディスプレイ → 拡大縮小」で対象モニタのスケールを
//!    150%（dpi=144）または 200%（dpi=192）に設定する（あるいは既にそのスケールで
//!    動いているモニタへ窓を移動する）。
//! 2. 本 example を起動し、ログの `実モニタ DPI` が非 96 であることを確認する。
//! 3. 観測する:
//!    - (a) バルーン枠が surface 原寸（物理 px・等倍）で描かれ、ぼやけ／膨張が無い。
//!    - (b) テキストが validrect 内（枠の内側余白）に収まり、枠画像とズレない。
//!    - (c) readback 述語群がそのまま PASS する（DPI に依らず物理 px を読み戻すため）。
//! 4. 使用した DPI 値と (a)(b)(c) の結果を記録する。dpi=96 のみの確認は不十分である。
//!
//! **DoD 申し送り**: 実行時 k≠1 経路は上流（emo-present/placement）が k≠1 を供給し
//! 次第、本 example を再実行して検証する（design「観測」DPI 観測の Revalidation Trigger）。

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

use wintf::ecs::layout::HitTest;
use wintf::ecs::{
    FrameFinalize, FrameTime, GraphicsCore, Point, SizeI, Window, WindowHandle, WindowPos,
    WindowStyle, WucGraphicsResource,
};
use wintf::WinApp;

use areka_actor::reply_channel;
use areka_emo_atlas::{AtlasTable, WicDecoderArm};
use areka_emo_compose::{BindSet, Composer, EmoWorld};
use areka_emo_present::{
    build_balloon_target, EmoPresenter, PresentCommand, PresentOutcome, TargetId,
};
use areka_emo_text::actor::{
    present_frame, spawn_emo_text, ResolvedBalloonText, TextLayerRuntime, TextSlotBinding,
};
use areka_emo_text::draw::DWriteMetrics;
use areka_emo_text::layout::{GlyphMetrics, LayoutEngine, WrapPlan};
use areka_emo_text::sink::EmoTextSink;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::viewbox_draw::DrawStats;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{parse_str, BalloonModel};
use areka_parsers::charset::{decode, DefaultEncoding};
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};

// ---------------------------------------------------------------------------
// 定数・fixture パス解決
// ---------------------------------------------------------------------------

/// \0（sakura）バルーン窓の初期位置（物理 px・スクリーン座標）。
const SAKURA_POS: (i32, i32) = (320, 160);
/// \1（kero）バルーン窓の縦間隔（sakura 窓の直下に置く・物理 px）。
const KERO_GAP_Y: i32 = 32;
/// シナリオ全体の watchdog（talk 起点相対秒・超過は FAIL）。
const WATCHDOG_SECS: f64 = 30.0;

/// 共有 fixture（emo2 バルーン）ディレクトリを `CARGO_MANIFEST_DIR` 相対で解決する
/// （emo-present example と同一アンカー規約・R11.7 共有 fixture 非改変）。
fn shared_balloon_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku")
}

/// example ローカル fixture 変種（縦書き観測用・task 9.1 成果）。
fn vertical_variant_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures/emo2-vertical")
}

/// balloon descript ファイルを読み、charset 宣言に従いデコードする
/// （parser-foundation の decode 経路——tests/vertical_fixture_test.rs と同じ読み込み規約）。
fn read_decoded(path: &std::path::Path) -> Option<String> {
    match std::fs::read(path) {
        Ok(bytes) => Some(decode(&bytes, DefaultEncoding::Utf8)),
        Err(e) => {
            error!(path = %path.display(), error = %e, "balloon descript の読取に失敗");
            None
        }
    }
}

/// balloon model（descript 基層＋`balloons0s.txt` 画像別上書き層の 2 層マージ）を解決する。
///
/// `--vertical` 時は parse 入力だけを fixture 変種へ差し替える（枠画像は共有 fixture 継続）。
fn load_balloon_model(vertical: bool) -> Option<BalloonModel> {
    let dir = if vertical {
        vertical_variant_dir()
    } else {
        shared_balloon_dir()
    };
    let base = read_decoded(&dir.join("descript.txt"))?;
    let overlay = read_decoded(&dir.join("balloons0s.txt"))?;
    Some(parse_str(&base, Some(&overlay)))
}

// ---------------------------------------------------------------------------
// シナリオ定義（boot.pasta 起動朝 挨拶由来のハードコード cue 列・注入時刻付き）
// ---------------------------------------------------------------------------
//
// 台詞は fixture スクリプト boot.pasta「起動朝」シーン（むらさき=\0／エモ=\1）由来。
// 折返し閾値・行容量の檻が決定論で効くよう、共有 fixture の幾何
// （validrect 320×122 image px・font 28px・line pitch 35px）に合わせて刻んでいる。

/// \0 の 1 行目（6 グリフ・typewriter 進行の観測対象）。
const LINE1: &str = "おっはよー！";
/// \0 の 2 行目（10 グリフ・改行観測。横書き 1 行に収まり、縦書きでは複数列へ折返す）。
const LINE2: &str = "めっちゃええ朝やん！";
/// \0 の 3 行目（8 グリフ・横書き行容量 3 行ちょうどの最終行）。
const LINE3: &str = "今日もいくでー！";
/// \1（kero）の台詞（7 グリフ・複数 actor 振り分けの観測対象）。
const KERO_LINE: &str = "朝から元気だね";
/// あふれ→スクロール誘発用の短行（2 グリフ×9 行・先頭行より行内範囲が確実に短い）。
const SHORT_LINE: &str = "ほな";
/// あふれ誘発の短行数（横書き容量 3 行・縦書き容量 9 列をどちらも確実に超える）。
const OVERFLOW_LINES: usize = 9;

/// 各ステージのチェックポイント注入時刻（talk 起点相対秒・リビール時刻＋丸め余裕）。
const T_CHECK: [f64; 7] = [0.12, 0.35, 1.1, 1.8, 3.0, 3.2, 3.4];

/// R10.3 DrawStats 檻: 1 行/列スクロールフレームの `DrawTextLayout` 増分の tight bound。
/// この共有 fixture（validrect 320×122・28px フォント＝可視 3 行）では、1 行スクロールの
/// ダーティ＝露出帯 ∪ 完成した流入行 ∪ 末尾の空行の 3 枚に、実描画対象は流入行 1 本のみ
/// （空行は skip）＝`draw_text_layout_calls` 増分は 3（`dirty_len 3 × draws 1`）に収まる。
/// talk 全体の行数（あふれで 12 行超）に伸びない小定数であることが要点（reference:
/// tests/viewbox_scroll_test.rs は可視 8 行の別 fixture で `増分 < 可視行数` を厳密に檻化）。
const EXPOSURE_BAND_DRAW_BOUND: u64 = 3;

/// ステージ gate（cue が UI ドレインを経て状態機械へ適用済みであることの決定論条件）。
#[derive(Clone, Copy, Debug)]
enum Gate {
    /// actor 状態の items（グリフ＋改行マーカー）数が一致する。
    Items(usize),
    /// actor 状態が空（Clear 適用済み）。
    Empty,
    /// 条件なし。
    Any,
}

impl Gate {
    fn satisfied(self, rt: &TextLayerRuntime, actor: &ActorKey) -> bool {
        match self {
            Gate::Items(n) => rt
                .state()
                .actor_state(actor)
                .map(|s| s.items().len())
                == Some(n),
            Gate::Empty => rt
                .state()
                .actor_state(actor)
                .is_some_and(|s| s.items().is_empty()),
            Gate::Any => true,
        }
    }
}

/// \0 の gate（ステージ順）。items 数: L1=6 → +1+10=17 → +1+8=26 → +9×(1+2)=53 → Clear。
const GATE_SAKURA: [Gate; 7] = [
    Gate::Items(6),
    Gate::Items(6),
    Gate::Items(17),
    Gate::Items(26),
    Gate::Items(53),
    Gate::Empty,
    Gate::Empty,
];
/// \1 の gate（ステージ順）。KERO_LINE=7 items → Clear。
const GATE_KERO: [Gate; 7] = [
    Gate::Any,
    Gate::Any,
    Gate::Any,
    Gate::Items(7),
    Gate::Items(7),
    Gate::Items(7),
    Gate::Empty,
];

/// cue 生成ヘルパ。
fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

/// ステージごとに sink へ流す cue 列（`at` は注入時刻＝リビール開始の下限）。
fn stage_cues(stage: usize) -> Vec<TalkCue> {
    match stage {
        0 => vec![cue("0", 0.0, CueCommand::Text(LINE1.into()))],
        1 => Vec::new(),
        2 => vec![
            cue("0", 0.5, CueCommand::NewLine { ratio: 1.0 }),
            cue("0", 0.5, CueCommand::Text(LINE2.into())),
        ],
        3 => vec![
            cue("0", 1.2, CueCommand::NewLine { ratio: 1.0 }),
            cue("0", 1.2, CueCommand::Text(LINE3.into())),
            cue("1", 1.2, CueCommand::Text(KERO_LINE.into())),
        ],
        4 => {
            let mut cues = Vec::with_capacity(OVERFLOW_LINES * 2);
            for _ in 0..OVERFLOW_LINES {
                cues.push(cue("0", 2.0, CueCommand::NewLine { ratio: 1.0 }));
                cues.push(cue("0", 2.0, CueCommand::Text(SHORT_LINE.into())));
            }
            cues
        }
        5 => vec![cue("0", 3.1, CueCommand::Clear)],
        6 => vec![cue("1", 3.3, CueCommand::Clear)],
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// 判定（Verdict）と readback 述語ヘルパ
// ---------------------------------------------------------------------------

/// 単一 pass/fail の集計（World リソース・main が run() 復帰後に読む）。
#[derive(Resource, Default)]
struct Verdict {
    /// 通過した readback/構造 assert の件数。
    checks: usize,
    /// 失敗記録（空なら PASS 候補）。
    failures: Vec<String>,
    /// シナリオが最後（Clear 検証）まで到達したか。
    done: bool,
}

impl Verdict {
    /// 述語を検証し、失敗は log-first で記録する（panic しない・FAIL へ集計）。
    fn check(&mut self, cond: bool, label: &str) {
        if cond {
            self.checks += 1;
            info!(check = label, "readback 検証 OK");
        } else {
            error!(check = label, "readback 検証 FAIL");
            self.failures.push(label.to_string());
        }
    }
}

/// 非透明ピクセル数（BGRA 密配列の α ≠ 0・attach_wiring_test と同じ述語）。
fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// ピクセル (x, y) が非透明か。
fn is_opaque(bytes: &[u8], w: u32, x: u32, y: u32) -> bool {
    bytes[((y * w + x) * 4 + 3) as usize] != 0
}

/// 行送り軸（block 軸）方向のインク範囲（validrect-local 物理 px）。
///
/// 軸読み替え正準表: horizontal_tb＝最下インク行（+y）・vertical_rl＝右端から最左インク列
/// までの距離（−x 方向）。改行・折返しで行/列が増えると単調に伸びる。
fn block_extent(bytes: &[u8], w: u32, h: u32, mode: WritingMode) -> u32 {
    let mut extent = 0u32;
    for y in 0..h {
        for x in 0..w {
            if is_opaque(bytes, w, x, y) {
                let e = match mode {
                    WritingMode::HorizontalTb => y + 1,
                    WritingMode::VerticalRl => w - x,
                    WritingMode::VerticalLr => x + 1,
                };
                extent = extent.max(e);
            }
        }
    }
    extent
}

/// 先頭バンド（可視窓先頭の 1 行/列分・厚み `pitch`）内の行内軸インク範囲
/// （validrect-local 物理 px）。スクロールで先頭行が消える（短い行に入れ替わる）と縮む。
fn inline_extent_first_band(bytes: &[u8], w: u32, h: u32, mode: WritingMode, pitch: u32) -> u32 {
    let mut extent = 0u32;
    match mode {
        WritingMode::HorizontalTb => {
            for y in 0..pitch.min(h) {
                for x in 0..w {
                    if is_opaque(bytes, w, x, y) {
                        extent = extent.max(x + 1);
                    }
                }
            }
        }
        WritingMode::VerticalRl => {
            for x in w.saturating_sub(pitch)..w {
                for y in 0..h {
                    if is_opaque(bytes, w, x, y) {
                        extent = extent.max(y + 1);
                    }
                }
            }
        }
        WritingMode::VerticalLr => {
            for x in 0..pitch.min(w) {
                for y in 0..h {
                    if is_opaque(bytes, w, x, y) {
                        extent = extent.max(y + 1);
                    }
                }
            }
        }
    }
    extent
}

/// 行送り軸バンド `[b0, b1)`（validrect-local）内の非透明ピクセル数。
fn band_ink(bytes: &[u8], w: u32, h: u32, mode: WritingMode, b0: f32, b1: f32) -> usize {
    let clamp = |v: f32, max: u32| -> u32 { (v.max(0.0) as u32).min(max) };
    let mut count = 0usize;
    match mode {
        WritingMode::HorizontalTb => {
            for y in clamp(b0, h)..clamp(b1, h) {
                for x in 0..w {
                    if is_opaque(bytes, w, x, y) {
                        count += 1;
                    }
                }
            }
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            for x in clamp(b0, w)..clamp(b1, w) {
                for y in 0..h {
                    if is_opaque(bytes, w, x, y) {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

// ---------------------------------------------------------------------------
// 観測記録（チェックポイント間で持ち回る readback 実測値）
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Observations {
    /// C1: typewriter 途中の非透明ピクセル数。
    ink_c1: usize,
    /// C2: 改行前の block 軸インク範囲。
    block_extent_c2: u32,
    /// C2: 改行前の行数（純粋 layout・折返し観測の基準）。
    lines_c2: usize,
    /// C4: スクロール前の先頭バンド行内インク範囲。
    inline_extent_c4: u32,
    /// C4: \1（kero）供給面のバイト列スナップショット（独立性検証用）。
    kero_bytes_c4: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Demo（NonSend・EmoPresenter/TextLayerRuntime を内包する集約ルート）
// ---------------------------------------------------------------------------

/// 起動〜シナリオ駆動の全状態（NonSend リソース・UI スレッド所有）。
struct Demo {
    presenter: EmoPresenter,
    /// \0（sakura）バルーン窓。
    win0: Entity,
    /// \1（kero）バルーン窓。
    win1: Entity,
    /// attach_target で move 消費するアセット（装着後 None）。
    assets0: Option<(EmoWorld, AtlasTable)>,
    assets1: Option<(EmoWorld, AtlasTable)>,
    /// 2 層マージ済み balloon model（両 actor 共通・fixture 由来）。
    model: BalloonModel,
    /// 装着＋結線完了フラグ。
    attached: bool,
    /// UI スレッド所有の集約ルート（sink ドレインと present_frame が共有）。
    runtime: Rc<RefCell<TextLayerRuntime>>,
    /// cue 受信口（UI ドレインへ配送）。
    sink: Option<EmoTextSink>,
    /// drain の join ハンドル（生存保持のみ）。
    _drain: Option<wintf_winmsg_executor::JoinHandle<()>>,
    /// \0/\1 共通の解決済み layout 入力（writing_mode/region/font）。
    resolved: Option<ResolvedBalloonText>,
    /// チェックポイントの純粋 layout 再導出に使う実測 metrics（描画と同一 probe 経路）。
    metrics: Option<DWriteMetrics>,
    /// 供給面（validrect）の物理寸。
    dims: (u32, u32),
    /// talk 開始のフレーム時刻（秒）。
    talk_start: f64,
    /// 現在のステージ（0..7）。
    stage: usize,
    /// 現ステージの cue を sink へ流し終えたか。
    fed: bool,
    obs: Observations,
    finished: bool,
    /// `--hold`: シナリオ完了後も窓を閉じず talk をループ再生し、balloon 上でのダブルクリックで終了する
    /// （自動クローズしない目視確認モード・PASS/FAIL 自動判定は行わない）。
    hold: bool,
    /// ダブルクリック検出用: 直前フレームの左ボタン押下状態（rising edge 抽出）。
    prev_lbutton: bool,
    /// ダブルクリック検出用: 直近クリック（rising edge）のフレーム時刻（連続 2 クリックの間隔判定）。
    last_click_time: f64,
}

/// sakura/kero の ActorKey（結線側が所有する actor→target 対応の鍵・R9.5）。
fn actor0() -> ActorKey {
    ActorKey::from("0")
}
fn actor1() -> ActorKey {
    ActorKey::from("1")
}

// ---------------------------------------------------------------------------
// エントリポイント
// ---------------------------------------------------------------------------

fn main() -> windows::core::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let vertical = std::env::args().any(|a| a == "--vertical");
    let hold = std::env::args().any(|a| a == "--hold");

    let mgr = WinApp::new()?;
    let world = mgr.world();

    world
        .borrow_mut()
        .world_mut()
        .insert_resource(Verdict::default());

    // UI スレッドで「アセット構築＋窓生成＋Demo 挿入」を行う（WIC は COM 初期化済みスレッド）。
    world.borrow().spawn(move |tx| async move {
        let _ = tx.send(Box::new(move |world: &mut World| {
            build_and_spawn(world, vertical, hold);
        }));
    });

    // 装着（GPU 資源到達で 1 回）＋シナリオ駆動（毎フレーム・注入時刻）を担う排他 system。
    world.borrow_mut().add_systems(FrameFinalize, drive_demo_system);

    println!();
    println!("areka emo-text-layer 観測 example（task 9.2）");
    println!("================================================");
    println!(
        "  モード: {}（--vertical で縦書き fixture 変種へ切替）",
        if vertical { "縦書き vertical_rl" } else { "横書き horizontal_tb（既定）" }
    );
    println!("  シナリオ: typewriter 進行 → 改行 → あふれスクロール → Clear（自動・約 4 秒）");
    if hold {
        println!("  モード: --hold（目視確認）— talk をループ再生し、balloon 上でダブルクリックすると終了");
    } else {
        println!("  判定: readback 述語で自動判定し、最後に PASS/FAIL を 1 行出力（exit code 連動）");
    }
    println!();

    mgr.run()?;

    // run() 復帰後に判定を回収して単一 pass/fail を出力する。
    let verdict = world.borrow_mut().world_mut().remove_resource::<Verdict>();
    match verdict {
        Some(v) if v.done && v.failures.is_empty() => {
            println!("[emo-text-layer] PASS ({} checks)", v.checks);
            Ok(())
        }
        Some(v) => {
            if !v.done {
                println!(
                    "[emo-text-layer] FAIL (シナリオ完了前に終了・{} checks 通過)",
                    v.checks
                );
            } else {
                println!(
                    "[emo-text-layer] FAIL ({} failures / {} checks)",
                    v.failures.len(),
                    v.checks
                );
            }
            for f in &v.failures {
                println!("  - FAIL: {f}");
            }
            std::process::exit(1);
        }
        None => {
            println!("[emo-text-layer] FAIL (判定リソース不在)");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// セットアップ（UI スレッド・COM 初期化済み）
// ---------------------------------------------------------------------------

/// 失敗時は log-first で真因を出し、観測不能として loud に終了する（誤 PASS を作らない）。
fn setup_abort(msg: &str) -> ! {
    error!("{msg}");
    println!("[emo-text-layer] FAIL (セットアップ失敗: {msg})");
    std::process::exit(1);
}

/// アセット構築・窓生成・`Demo` 挿入を一括で行う（UI スレッド・emo-present example と同型）。
fn build_and_spawn(world: &mut World, vertical: bool, hold: bool) {
    let Ok(decoder) = WicDecoderArm::new() else {
        setup_abort("WicDecoderArm 生成に失敗（COM 未初期化？）");
    };

    // balloon descript（2 層マージ）: --vertical は parse 入力だけを変種へ差し替える。
    let Some(model) = load_balloon_model(vertical) else {
        setup_abort("balloon descript の読取/解釈に失敗");
    };

    // バルーン枠アセット×2（\0/\1 target）: 共有 fixture をシェルと同一経路で構築。
    let balloon_dir = shared_balloon_dir();
    let (Ok(assets0), Ok(assets1)) = (
        build_balloon_target(&balloon_dir, &decoder),
        build_balloon_target(&balloon_dir, &decoder),
    ) else {
        setup_abort("バルーン枠アセットの構築に失敗（共有 fixture の配置を確認）");
    };

    // 窓寸 ≔ balloon surface0 の合成原寸（物理 px・DPI 表示契約＝等倍）。
    let (w, h) = match Composer::new().compose(&assets0.0, &assets0.1, 0, &BindSet::default()) {
        Ok(cs) => (cs.width(), cs.height()),
        Err(e) => {
            setup_abort(&format!("balloon surface0 の採寸合成に失敗: {e}"));
        }
    };
    if w == 0 || h == 0 {
        setup_abort("balloon surface0 の合成外形が 0 寸");
    }

    let win0 = create_balloon_window(world, "sakura", SAKURA_POS.0, SAKURA_POS.1, w, h);
    let win1 = create_balloon_window(
        world,
        "kero",
        SAKURA_POS.0,
        SAKURA_POS.1 + h as i32 + KERO_GAP_Y,
        w,
        h,
    );

    world.insert_non_send_resource(Demo {
        presenter: EmoPresenter::new(),
        win0,
        win1,
        assets0: Some(assets0),
        assets1: Some(assets1),
        model,
        attached: false,
        runtime: Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
        sink: None,
        _drain: None,
        resolved: None,
        metrics: None,
        dims: (0, 0),
        talk_start: 0.0,
        stage: 0,
        fed: false,
        obs: Observations::default(),
        finished: false,
        hold,
        prev_lbutton: false,
        last_click_time: -10.0,
    });
    info!(w, h, vertical, hold, "emo-text-layer: 窓生成とアセット構築を完了（GPU 資源到達で装着）");
}

/// バルーン窓 Entity を構築する（emo-present example の balloon 窓と同型・物理 px 採寸）。
fn create_balloon_window(
    world: &mut World,
    label: &str,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Entity {
    world
        .spawn((
            Name::new(format!("EmoText-Balloon-{label}")),
            Window {
                title: format!("areka emo-text balloon ({label})"),
                ..Default::default()
            },
            WindowStyle {
                style: WS_POPUP | WS_VISIBLE,
                ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            },
            WindowPos {
                position: Some(Point { x, y }),
                size: Some(SizeI {
                    width: w as i32,
                    height: h as i32,
                }),
                ..Default::default()
            },
            HitTest::none(),
        ))
        .id()
}

// ---------------------------------------------------------------------------
// 駆動 system（装着 → シナリオ・排他 &mut World・UI スレッド）
// ---------------------------------------------------------------------------

/// `--hold`: balloon 窓上での左ダブルクリック（連続 2 クリック ≤0.4s）を検出したら終了要求（true）。
///
/// `GetAsyncKeyState` の自前ポーリングでフレーム差分から rising edge を拾う——ECS ポインタの
/// transient クリアや system 実行順序に依存せず、click-through 窓（`HitTest::none()`＋
/// `WS_EX_TRANSPARENT`）でも物理ボタン押下を検出できる。カーソルが balloon 窓の矩形内にある
/// ときのみ有効化し、別アプリ上のダブルクリックで閉じないようにする。
fn poll_double_click_quit(demo: &mut Demo, world: &World) -> bool {
    let now = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);
    // SAFETY: GetAsyncKeyState は現在のボタン状態（high bit＝押下中）を読むだけ（副作用なし）。
    let down = ((unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) }) as u16 & 0x8000) != 0;
    let rising = down && !demo.prev_lbutton;
    demo.prev_lbutton = down;
    if !rising {
        return false;
    }
    if !cursor_over_demo_window(demo, world) {
        demo.last_click_time = -10.0; // 窓外クリックは double 判定の起点にしない。
        return false;
    }
    let is_double = (now - demo.last_click_time) <= 0.4;
    demo.last_click_time = now;
    is_double
}

/// カーソル（スクリーン座標）が \0/\1 いずれかの balloon 窓の矩形内にあるか。
fn cursor_over_demo_window(demo: &Demo, world: &World) -> bool {
    let mut p = POINT::default();
    // SAFETY: GetCursorPos は現在のカーソル位置を `p` へ書き込むだけ。
    if unsafe { GetCursorPos(&mut p) }.is_err() {
        return false;
    }
    for win in [demo.win0, demo.win1] {
        if let Some(wp) = world.get::<WindowPos>(win) {
            if let (Some(pos), Some(size)) = (wp.position, wp.size) {
                if p.x >= pos.x
                    && p.x < pos.x + size.width
                    && p.y >= pos.y
                    && p.y < pos.y + size.height
                {
                    return true;
                }
            }
        }
    }
    false
}

fn drive_demo_system(world: &mut World) {
    // 未挿入 or 完了済みなら何もしない。
    match world.get_non_send_resource::<Demo>() {
        Some(d) if !d.finished => {}
        _ => return,
    }
    let mut demo = world
        .remove_non_send_resource::<Demo>()
        .expect("直上で存在確認済み");

    if !demo.attached {
        try_attach(&mut demo, world);
    } else if demo.hold && poll_double_click_quit(&mut demo, world) {
        // hold モード: balloon 上ダブルクリックで終了（窓を despawn → run() 復帰）。
        info!("emo-text-layer: balloon 上でダブルクリックを検出 — hold デモを終了する");
        finish(&mut demo, world);
    } else {
        drive_scenario(&mut demo, world);
    }

    world.insert_non_send_resource(demo);
}

/// GPU 資源到達フレームで attach→ShowSurface→結線（text_slot_view→spawn_emo_text→routing 登録）
/// を 1 回だけ駆動する（emo-present example の boot_present_system と同型）。
fn try_attach(demo: &mut Demo, world: &mut World) {
    let ready = world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !ready {
        return;
    }

    // ── attach_target ＋ ShowSurface（reply で成立を要求・R11.1 の枠表示） ──
    for (target, window, assets) in [
        (TargetId(0), demo.win0, demo.assets0.take()),
        (TargetId(1), demo.win1, demo.assets1.take()),
    ] {
        let Some((emo_world, atlas)) = assets else {
            fail_attach(demo, world, format!("{target:?} のアセットが二重消費された（構造不変の破れ）"));
            return;
        };
        if let Err(e) = demo
            .presenter
            .attach_target(world, target, window, emo_world, atlas)
        {
            fail_attach(demo, world, format!("{target:?} の attach_target に失敗: {e}"));
            return;
        }
        let (tx, rx) = reply_channel::<PresentOutcome>();
        demo.presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target,
                surface_id: 0,
                binds: BindSet::default(),
                reply: Some(tx),
            },
        );
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => {}
            other => {
                fail_attach(demo, world, format!("{target:?} の ShowSurface が成立しない: {other:?}"));
                return;
            }
        }
    }

    // ── 予約スロットへの結線（R9.1/R9.2/R9.5 経路・task 8 と同じ組み方） ──
    let (Some(view0), Some(view1)) = (
        demo.presenter.text_slot_view(TargetId(0)),
        demo.presenter.text_slot_view(TargetId(1)),
    ) else {
        fail_attach(demo, world, "表示確立後の text_slot_view が None".to_string());
        return;
    };
    if view0.slot() == view1.slot() {
        fail_attach(demo, world, "2 target の予約スロットが同一（振り分け不能）".to_string());
        return;
    }

    // image_size は binding の一点導出値（`image_size = round(surface_size / k)`・k=1.0 恒常）
    // ——検証用 layout も描画（register_actor_view 内部）と同じ入力で組む（2 空間モデル遵守）。
    let binding0 = TextSlotBinding::from_view(&view0);
    let resolved = ResolvedBalloonText::resolve(&demo.model, binding0.image_size);
    info!(
        mode = ?resolved.mode,
        region = ?(resolved.region.left(), resolved.region.top(), resolved.region.right(), resolved.region.bottom()),
        wrap_threshold = resolved.region.wrap_threshold(),
        font = %resolved.font.name,
        font_height = resolved.font.height,
        "emo-text-layer: writing_mode/領域/フォントの解決結果（起動時 1 回）"
    );

    // ── 実モニタ DPI ログ（R11.9・非 96 DPI 手動確認手順はヘッダ doc 参照） ──
    for (label, win) in [("sakura", demo.win0), ("kero", demo.win1)] {
        match world.get::<WindowHandle>(win) {
            Some(handle) => info!(
                balloon = label,
                dpi = handle.get_dpi(),
                scale_k = view0.scale(),
                "emo-text-layer: 実モニタ DPI（物理 1:1 表示契約・k=1.0 恒常）"
            ),
            None => warn!(balloon = label, "WindowHandle 未付与のため DPI を読めない"),
        }
    }

    // ── UI ドレイン起動＋actor routing 登録（結線側が actor→target 対応を所有） ──
    let (sink, drain) = match spawn_emo_text(Rc::clone(&demo.runtime)) {
        Ok(pair) => pair,
        Err(e) => {
            fail_attach(demo, world, format!("spawn_emo_text に失敗: {e}"));
            return;
        }
    };
    demo.runtime
        .borrow_mut()
        .register_actor_view(actor0(), &view0, &demo.model);
    demo.runtime
        .borrow_mut()
        .register_actor_view(actor1(), &view1, &demo.model);

    // チェックポイントの純粋 layout 再導出用 metrics（描画と同一の probe 経路で組む）。
    let metrics = {
        let Some(factory) = world
            .get_resource::<GraphicsCore>()
            .and_then(|core| core.dwrite_factory().cloned())
        else {
            fail_attach(demo, world, "dwrite_factory 不在（metrics を構築できない）".to_string());
            return;
        };
        match DWriteMetrics::new(&factory, &resolved.font, resolved.mode, &TextLayerConfig::default())
        {
            Ok(m) => m,
            Err(e) => {
                fail_attach(demo, world, format!("DWriteMetrics 構築に失敗: {e}"));
                return;
            }
        }
    };

    demo.dims = (
        (resolved.region.right() - resolved.region.left()).ceil() as u32,
        (resolved.region.bottom() - resolved.region.top()).ceil() as u32,
    );
    demo.resolved = Some(resolved);
    demo.metrics = Some(metrics);
    demo.sink = Some(sink);
    demo._drain = Some(drain);
    demo.talk_start = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);
    demo.attached = true;
    info!("emo-text-layer: 装着・結線完了 — 注入時刻駆動のシナリオを開始する");
}

/// 装着フェーズの失敗集約（log-first・FAIL 記録→シナリオ打切り）。
fn fail_attach(demo: &mut Demo, world: &mut World, msg: String) {
    error!("{msg}");
    world.resource_mut::<Verdict>().failures.push(msg);
    finish(demo, world);
}

/// シナリオ終了処理: sink close → 窓 despawn（registry 空遷移で run() が復帰する）。
fn finish(demo: &mut Demo, world: &mut World) {
    if let Some(sink) = demo.sink.as_ref() {
        sink.close();
    }
    for win in [demo.win0, demo.win1] {
        world.despawn(win);
    }
    demo.finished = true;
}

/// 注入時刻駆動のシナリオ本体（毎フレーム）。
///
/// ステージごとに (1) cue を sink へ流す → (2) UI ドレイン適用を gate で待つ →
/// (3) フレーム時刻がチェックポイント時刻へ達するのを待つ →
/// (4) **正確な注入時刻**で `present_frame` → readback 述語を検証、の順で進める。
/// (4) の時刻は固定値（`T_CHECK`）を注入するため、フレームレートに依らず
/// 可視グリフ数・レイアウト・readback が決定論になる。
fn drive_scenario(demo: &mut Demo, world: &mut World) {
    let now = world
        .get_resource::<FrameTime>()
        .map(|ft| ft.0)
        .unwrap_or(0.0);
    let t = now - demo.talk_start;

    // watchdog: 進行不能（drain 停止等）を FAIL として観測する。
    if t > WATCHDOG_SECS {
        let msg = format!("watchdog 超過（stage={} t={t:.1}s）——シナリオが進行しない", demo.stage);
        error!("{msg}");
        world.resource_mut::<Verdict>().failures.push(msg);
        finish(demo, world);
        return;
    }

    let stage = demo.stage;
    if stage >= T_CHECK.len() {
        return; // 全ステージ完了（finish 済みのはず・防御）。
    }

    // (1) 現ステージの cue を一度だけ sink へ流す（`at` がリビール下限を規定する）。
    if !demo.fed {
        if let Some(sink) = demo.sink.as_mut() {
            for c in stage_cues(stage) {
                sink.emit(c);
            }
        }
        demo.fed = true;
        return; // 次フレーム以降で UI ドレインが適用する。
    }

    // (2) gate: cue が状態機械へ適用済みであること（UI ドレインは posted メッセージで進む）。
    {
        let rt = demo.runtime.borrow();
        if !GATE_SAKURA[stage].satisfied(&rt, &actor0())
            || !GATE_KERO[stage].satisfied(&rt, &actor1())
        {
            return; // 未適用——次フレーム再試行（typewriter の表示は継続提示で流れる）。
        }
    }

    // (3) チェックポイント時刻まではフレーム時刻で継続提示（typewriter の目視観測）。
    let t_check = T_CHECK[stage];
    if t < t_check {
        let mut rt = demo.runtime.borrow_mut();
        if let Err(e) = present_frame(&mut rt, world, t) {
            warn!(error = %e, "継続提示フレームで失敗（次フレーム再試行）");
        }
        return;
    }

    // (4) 正確な注入時刻で提示し、readback 述語を検証する。
    let failures_before = world.resource::<Verdict>().failures.len();
    run_checkpoint(demo, world, stage, t_check);
    let failed_now = world.resource::<Verdict>().failures.len() > failures_before;

    if failed_now {
        finish(demo, world); // 早期打切り（失敗は記録済み・FAIL 出力へ）。
        return;
    }

    demo.stage += 1;
    demo.fed = false;
    if demo.stage == T_CHECK.len() {
        world.resource_mut::<Verdict>().done = true;
        if demo.hold {
            // ループ再生: Clear 済み（状態は空）ゆえ stage 0 から talk を再度流す。窓は閉じない。
            // 注入時刻起点 talk_start を現在フレーム時刻へ戻し、typewriter を最初から見せる。
            let now = world
                .get_resource::<FrameTime>()
                .map(|ft| ft.0)
                .unwrap_or(demo.talk_start);
            demo.stage = 0;
            demo.fed = false;
            demo.talk_start = now;
            info!("emo-text-layer: hold — talk をループ再生（balloon 上ダブルクリックで終了）");
        } else {
            info!("emo-text-layer: 全チェックポイント通過 — PASS で終了する");
            finish(demo, world);
        }
    }
}

/// チェックポイント本体: `present_frame(t_check)` → readback → ステージ別述語。
fn run_checkpoint(demo: &mut Demo, world: &mut World, stage: usize, t_check: f64) {
    let (w, h) = demo.dims;
    let resolved = demo.resolved.as_ref().expect("attach 完了後は Some");
    let mode = resolved.mode;
    let font_height = resolved.font.height;
    let pitch = demo
        .metrics
        .as_ref()
        .expect("attach 完了後は Some")
        .line_pitch(font_height) as u32;

    // 正確な注入時刻で提示（決定論・実時間 sleep 不使用・R11.1）。
    {
        let mut rt = demo.runtime.borrow_mut();
        if let Err(e) = present_frame(&mut rt, world, t_check) {
            world
                .resource_mut::<Verdict>()
                .failures
                .push(format!("stage {stage}: present_frame({t_check}) が失敗: {e}"));
            return;
        }
    }

    let rt = demo.runtime.borrow();
    let read = |actor: &ActorKey| -> Option<Vec<u8>> {
        rt.surface(actor).and_then(|s| s.read_back().ok())
    };

    // 純粋 layout の再導出（可視窓・行構造の構造述語用——描画と同一入力・同一 metrics）。
    let layout_of = |actor: &ActorKey| {
        let state = rt.state().actor_state(actor)?;
        let visible = rt.state().visible_glyphs(actor, t_check);
        let lines = LayoutEngine::layout(
            state.items(),
            visible,
            &resolved.region,
            mode,
            font_height,
            demo.metrics.as_ref().expect("attach 完了後は Some"),
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &resolved.region, mode);
        Some((lines, window))
    };

    let mut verdict = std::mem::take(&mut *world.resource_mut::<Verdict>());

    match stage {
        // C1: typewriter 進行の途中観測（R11.2）——インクが出始めている。
        0 => {
            let Some(bytes) = read(&actor0()) else {
                verdict.failures.push("C1: \\0 供給面の readback 不能".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            demo.obs.ink_c1 = opaque_count(&bytes);
            verdict.check(demo.obs.ink_c1 > 0, "C1: typewriter 途中でインクが可視");
        }
        // C2: typewriter 単調増加（R11.2）＋折返し構造（横=1 行のまま／縦=縦書き閾値で折返し・R11.4）。
        1 => {
            let Some(bytes) = read(&actor0()) else {
                verdict.failures.push("C2: \\0 供給面の readback 不能".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            let ink = opaque_count(&bytes);
            verdict.check(
                ink > demo.obs.ink_c1,
                "C2: 可視ピクセルが単調増加（typewriter 進行）",
            );
            if let Some((lines, _)) = layout_of(&actor0()) {
                match mode {
                    WritingMode::HorizontalTb => verdict.check(
                        lines.len() == 1,
                        "C2: 横書きは 1 行目が折返しなしで収まる（横書き閾値）",
                    ),
                    WritingMode::VerticalRl | WritingMode::VerticalLr => verdict.check(
                        lines.len() >= 2,
                        "C2: 縦書きは縦書き用閾値（wordwrappoint.y 軸）で折返す",
                    ),
                }
                demo.obs.lines_c2 = lines.len();
            }
            demo.obs.block_extent_c2 = block_extent(&bytes, w, h, mode);
            verdict.check(demo.obs.block_extent_c2 > 0, "C2: 行送り軸のインク範囲が非零");
        }
        // C3: 改行（NewLine）で行送り軸方向へインク範囲が拡大する（R11.1/改行）。
        2 => {
            let Some(bytes) = read(&actor0()) else {
                verdict.failures.push("C3: \\0 供給面の readback 不能".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            let extent = block_extent(&bytes, w, h, mode);
            verdict.check(
                extent > demo.obs.block_extent_c2,
                "C3: NewLine 後に行送り軸のインク範囲が拡大（改行の readback 観測）",
            );
            if let Some((lines, window)) = layout_of(&actor0()) {
                verdict.check(lines.len() > demo.obs.lines_c2, "C3: 行数が増加（改行の構造観測）");
                verdict.check(window.first_visible_line == 0, "C3: あふれ前はスクロールしない");
            }
        }
        // C4: あふれ直前の基準採取＋複数 actor 振り分け（R11.8）。
        3 => {
            let Some(bytes0) = read(&actor0()) else {
                verdict.failures.push("C4: \\0 供給面の readback 不能".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            let Some(bytes1) = read(&actor1()) else {
                verdict.failures.push("C4: \\1 供給面の readback 不能（振り分け不成立）".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            if let Some((_, window)) = layout_of(&actor0()) {
                verdict.check(window.first_visible_line == 0, "C4: 3 行時点ではスクロールしない");
            }
            demo.obs.inline_extent_c4 = inline_extent_first_band(&bytes0, w, h, mode, pitch);
            verdict.check(
                demo.obs.inline_extent_c4 > 0,
                "C4: 先頭バンド（1 行目/1 列目）にインクがある",
            );
            verdict.check(
                opaque_count(&bytes1) > 0,
                "C4: \\1（kero）のテキストが自分のバルーンへ描画される（振り分け）",
            );
            verdict.check(
                bytes1.len() == (w * h * 4) as usize,
                "C4: \\1 供給面が自 target のバルーン原寸（validrect 全域）",
            );
            demo.obs.kero_bytes_c4 = bytes1;
        }
        // C5: あふれ→スクロール（R11.3）＋ \0 更新が \1 へ波及しない独立性（R11.8）。
        4 => {
            let Some(bytes0) = read(&actor0()) else {
                verdict.failures.push("C5: \\0 供給面の readback 不能".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            let Some((lines, window)) = layout_of(&actor0()) else {
                verdict.failures.push("C5: \\0 の layout 再導出に失敗".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            verdict.check(
                window.first_visible_line >= 1,
                "C5: validrect あふれで行単位スクロールが発火（可視窓が進む）",
            );
            // 先頭行の消失: 先頭バンドが「長い 1 行目」から「短い後続行」へ入れ替わり、
            // 行内軸のインク範囲が確実に縮む（readback 述語・R11.3）。
            let extent_after = inline_extent_first_band(&bytes0, w, h, mode, pitch);
            verdict.check(
                (extent_after as f32) + font_height / 2.0 <= demo.obs.inline_extent_c4 as f32,
                "C5: 先頭バンドの行内インク範囲が縮む（先頭行の消失を readback で確認）",
            );
            // 最新行は常に可視（visible_window の飽和契約）——最新行バンドにインクがある。
            if let Some(last) = lines.last() {
                let (b0, b1) = match mode {
                    WritingMode::HorizontalTb => {
                        let y0 = last.rect.top - resolved.region.top() + window.block_offset;
                        (y0, y0 + font_height)
                    }
                    WritingMode::VerticalRl | WritingMode::VerticalLr => {
                        let x0 = last.rect.left - resolved.region.left() + window.block_offset;
                        (x0, x0 + font_height)
                    }
                };
                verdict.check(
                    band_ink(&bytes0, w, h, mode, b0, b1) > 0,
                    "C5: スクロール後も最新行が可視域内に描画される",
                );
            }
            // 独立性: \0 のあふれ更新は \1 の供給面へ一切波及しない（バイト同一・R11.8）。
            match read(&actor1()) {
                Some(bytes1) => verdict.check(
                    bytes1 == demo.obs.kero_bytes_c4,
                    "C5: \\0 のスクロール更新が \\1 の供給面へ波及しない（バイト同一）",
                ),
                None => verdict
                    .failures
                    .push("C5: \\1 供給面の readback 不能".into()),
            }
        }
        // C6: \0 の Clear で全透明へ（R11.5）・\1 は残存（独立性）。
        5 => {
            let Some(bytes0) = read(&actor0()) else {
                verdict.failures.push("C6: \\0 供給面の readback 不能".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            verdict.check(
                opaque_count(&bytes0) == 0,
                "C6: Clear 後の \\0 テキスト領域が全透明へ戻る",
            );
            match read(&actor1()) {
                Some(bytes1) => verdict.check(
                    opaque_count(&bytes1) > 0,
                    "C6: \\0 の Clear は \\1 のテキストを消さない（独立性）",
                ),
                None => verdict
                    .failures
                    .push("C6: \\1 供給面の readback 不能".into()),
            }
        }
        // C7: \1 の Clear で全透明へ（R11.5・両バルーンの全消去完了）。
        6 => {
            let Some(bytes1) = read(&actor1()) else {
                verdict.failures.push("C7: \\1 供給面の readback 不能".into());
                *world.resource_mut::<Verdict>() = verdict;
                return;
            };
            verdict.check(
                opaque_count(&bytes1) == 0,
                "C7: Clear 後の \\1 テキスト領域が全透明へ戻る",
            );
        }
        _ => {}
    }

    drop(rt);
    *world.resource_mut::<Verdict>() = verdict;

    // ── R10.3: あふれ→スクロール区間の DrawStats 檻（stage 4=C5 のみ・単一 pass/fail へ追加） ──
    // 既存の readback 述語（C5）で「あふれ→スクロールが発火した」ことを確認した直後に、
    // 描画統計（DrawStats）のフレーム間増分で「可視窓のみ移動フレームは確定行を再描画しない
    // （露出帯限定）」「内容・可視窓とも不変フレームは全増分 0」を決定論に檻化する。
    if stage == 4 {
        observe_redraw_less_stats(demo, world);
    }
}

/// R10.3: あふれ→スクロール区間の再描画レス（可視窓のみ移動＝ダーティ限定描画）と
/// 内容・可視窓とも不変フレームの全増分 0 を、`draw_stats` のフレーム間増分で単一 pass/fail
/// へ檻化する（reference: tests/viewbox_scroll_test.rs の実 pump 檻を観測 example へ写像）。
///
/// 決定論の要: `present_frame` へ渡す注入時刻を本関数が固定制御し（実フレーム時刻に依存しない）、
/// 可視窓は純粋 layout（[`window_probe`]）から導出するため、フレームレートに依らず同一結果になる。
/// cue 列・注入時刻シナリオ自体は一切変えず、観測用の追加提示のみを行う（既存 C1–C7 は不変）。
fn observe_redraw_less_stats(demo: &mut Demo, world: &mut World) {
    let actor = actor0();

    // ── 「可視窓のみ移動（＋1 完成行の流入）」フレームの端点となる注入時刻対を決定論に選ぶ ──
    // draw_text_layout_calls は「ダーティ矩形数 × 描画対象行数」の積で計上される。末尾行が
    // typewriter 途中（部分リビール）だとその変化行がダーティを 1 枚増やして積を膨らませるため、
    // 端点は**リビールのプラトー**（可視グリフ数が一定＝行が完成し次行が未リビールの区間）の
    // 中心に採る。こうすると変化行は「新規に流入した完成行」だけ・ダーティは（露出帯 ∪ その行）に
    // 限られ、確定行は面内 blit で保持されて再描画されない（横書き/縦書き共通・軸非依存・決定論）。
    let plateaus = enumerate_reveal_plateaus(demo, &actor);
    // 完成プラトー＝**可視窓が直前プラトーから +1 したプラトー**（流入した行が完成した瞬間・
    // 末尾実行行はちょうど完成しており部分リビール行がない＝ダーティが露出帯＋その 1 行に限られる）。
    let complete: Vec<&RevealPlateau> = (0..plateaus.len())
        .filter(|&i| i > 0 && plateaus[i].fvl >= 1 && plateaus[i].fvl == plateaus[i - 1].fvl + 1)
        .map(|i| &plateaus[i])
        .collect();
    // 完成プラトーは可視窓 fvl=1,2,3,… と単調に 1 ずつ進む。深い側で **2 段連続の 1 行スクロール**
    // （P → P+1 → P+2）を観測して「スクロールフレームの描画統計が可視窓の深さに依らず一定」
    // ＝確定行がスクロール量に比例して蓄積再描画されないことを檻化する（再描画レスの核）。
    if complete.len() < 3 {
        world.resource_mut::<Verdict>().failures.push(
            "C8: あふれ短行リビール区間で 1 行スクロールの完成プラトーが 3 つ未満（観測不能）".into(),
        );
        return;
    }
    let i2 = complete.len() - 1; // 最深スクロール先（可視窓 P+2）
    let i1 = i2 - 1; // 中間（可視窓 P+1）
    let i0 = i1 - 1; // ステップ始点（可視窓 P・基準フレーム）
    let visible_line_count = complete[i0].vlc;

    // ── 前方リプレイ: 完成プラトーを昇順に提示し、committed／prev_lines／行キャッシュを
    // 「1 完成行ずつ流入する自然な前方スクロール」状態へ確定する（直前の stage-4 チェックポイント
    // ＝末尾状態への後方ジャンプ汚染を吸収する）。到達点 complete[i0] が測定の基準フレーム。 ──
    for pl in &complete[..=i0] {
        present_at(demo, world, pl.t_mid);
    }
    let s_base = stats_of(demo, &actor);

    // ── 条件 1（NoChange gating・R10.3/R3.5）: 同一注入時刻の再提示で全統計増分 0 ──
    // 全域再描画方式ならここでも描き直すため増分が出る＝これが再描画レスの一次判別。
    present_at(demo, world, complete[i0].t_mid);
    let s_idle = stats_of(demo, &actor);
    {
        let mut v = std::mem::take(&mut *world.resource_mut::<Verdict>());
        v.check(
            s_idle.draw_text_layout_calls == s_base.draw_text_layout_calls,
            "C8: 内容・可視窓とも不変のフレームは DrawTextLayout 増分 0（NoChange gating）",
        );
        v.check(
            s_idle.line_layout_creations == s_base.line_layout_creations,
            "C8: 不変フレームは行レイアウト生成 増分 0",
        );
        v.check(s_idle.blits == s_base.blits, "C8: 不変フレームは面内 blit 増分 0");
        v.check(
            s_idle.full_clears == s_base.full_clears,
            "C8: 不変フレームは FullClear 増分 0",
        );
        *world.resource_mut::<Verdict>() = v;
    }

    // ── 条件 2: 連続する 2 段の 1 行スクロールフレーム（P→P+1, P+1→P+2）の描画統計を測る ──
    present_at(demo, world, complete[i1].t_mid);
    let s1 = stats_of(demo, &actor);
    present_at(demo, world, complete[i2].t_mid);
    let s2 = stats_of(demo, &actor);
    let (fvl1, _) = window_probe(demo, &actor, complete[i1].t_mid);
    let (fvl2, _) = window_probe(demo, &actor, complete[i2].t_mid);

    let draw1 = s1.draw_text_layout_calls - s_idle.draw_text_layout_calls;
    let draw2 = s2.draw_text_layout_calls - s1.draw_text_layout_calls;
    let create1 = s1.line_layout_creations - s_idle.line_layout_creations;
    let create2 = s2.line_layout_creations - s1.line_layout_creations;
    let blit1 = s1.blits - s_idle.blits;
    let blit2 = s2.blits - s1.blits;
    info!(
        p = complete[i0].fvl,
        fvl1, fvl2, visible_line_count,
        draw1, draw2, create1, create2, blit1, blit2,
        "C8: スクロールフレームの DrawStats 増分（R10.3 観測・可視窓のみ移動・2 段）"
    );

    let mut v = std::mem::take(&mut *world.resource_mut::<Verdict>());
    // 各段で可視窓がちょうど 1 行/列だけ進む（あふれ→行単位スクロールの発火）。
    v.check(
        fvl1 == complete[i0].fvl + 1 && fvl2 == fvl1 + 1,
        "C8: 連続 2 段で可視窓が 1 行/列ずつ進む（あふれ→行単位スクロール）",
    );
    // 可視窓が動いた決定論的証拠＝各段で面内 blit がちょうど 1 回（確定ピクセルを複製して保持）。
    // 全域再描画方式は保持せず描き直す＝blit 0——ここが本 fixture での再描画レスの決定的判別。
    v.check(
        blit1 == 1 && blit2 == 1,
        "C8: 各スクロール段で面内 blit がちょうど 1 回（保持ピクセルを複製＝全域再描画は blit 0）",
    );
    // 再描画レスの核: スクロールフレームの DrawTextLayout 増分は可視窓の深さに依らず一定
    // （確定行は面内 blit で保持され、描画は露出帯＋境界の変化行に限られる）。全域再描画／確定行を
    // 蓄積再描画する退行では深い段ほど増分が増える＝この不変が破れる。
    v.check(
        draw1 == draw2,
        "C8: スクロール描画増分がスクロール深さに依らず一定（確定行を蓄積再描画しない）",
    );
    // 描画は露出帯＋境界の変化行のダーティ矩形に限定され、talk 全体の行数（あふれで 12 行超）まで
    // 伸びない小定数に収まる（tight bound・reference: tests/viewbox_scroll_test.rs）。
    v.check(
        draw1 <= EXPOSURE_BAND_DRAW_BOUND && draw2 <= EXPOSURE_BAND_DRAW_BOUND,
        "C8: スクロール描画増分が露出帯＋境界行の tight bound 以下（ダーティ限定描画）",
    );
    // 確定行は行 TextLayout を再生成しない（面ピクセル＋blit＋行キャッシュが保持機構）＝
    // 各段の生成増分は流入した高々 1 行分に収まる（可視行数 vlc より小さい）。
    v.check(
        create1 <= 1 && create2 <= 1 && (create1 as usize) < visible_line_count,
        "C8: 確定行は行レイアウトを再生成しない（生成増分は流入 1 行分以下）",
    );
    *world.resource_mut::<Verdict>() = v;
}

/// リビールのプラトー（可視グリフ数が一定＝typewriter 非進行の区間）の代表点。
struct RevealPlateau {
    /// プラトー中心の注入時刻（端点で行が完成している安全点）。
    t_mid: f64,
    /// このプラトーでの先頭可視行 index。
    fvl: usize,
    /// このプラトーでの可視行数（`lines.len() - first_visible_line`）。
    vlc: usize,
}

/// あふれ短行リビール区間 `[1.95, 2.95]` を細かく走査し、可視グリフ数が一定の各プラトー
/// （typewriter 非進行区間）の中心時刻と可視窓を列挙する（純粋 layout・GPU present 不使用・決定論）。
fn enumerate_reveal_plateaus(demo: &Demo, actor: &ActorKey) -> Vec<RevealPlateau> {
    const STEP: f64 = 0.005;
    const START: f64 = 1.95;
    const END: f64 = 2.95;
    let mut out = Vec::new();
    let mut t = START;
    let (mut cur_vc, mut cur_fvl, mut cur_vlc) = reveal_probe(demo, actor, t);
    let mut plateau_start = t;
    let mut prev = t;
    while t < END {
        t += STEP;
        let (vc, fvl, vlc) = reveal_probe(demo, actor, t);
        if vc != cur_vc {
            out.push(RevealPlateau {
                t_mid: (plateau_start + prev) / 2.0,
                fvl: cur_fvl,
                vlc: cur_vlc,
            });
            plateau_start = t;
            cur_vc = vc;
            cur_fvl = fvl;
            cur_vlc = vlc;
        }
        prev = t;
    }
    out.push(RevealPlateau {
        t_mid: (plateau_start + prev) / 2.0,
        fvl: cur_fvl,
        vlc: cur_vlc,
    });
    out
}

/// 純粋 layout から `(visible_glyphs, first_visible_line, visible_line_count)` を導出する
/// （プラトー検出用・GPU present 不使用・決定論）。
fn reveal_probe(demo: &Demo, actor: &ActorKey, t: f64) -> (usize, usize, usize) {
    let rt = demo.runtime.borrow();
    let resolved = demo.resolved.as_ref().expect("attach 完了後は Some");
    let Some(state) = rt.state().actor_state(actor) else {
        return (0, 0, 0);
    };
    let visible = rt.state().visible_glyphs(actor, t);
    let lines = LayoutEngine::layout(
        state.items(),
        visible,
        &resolved.region,
        resolved.mode,
        resolved.font.height,
        demo.metrics.as_ref().expect("attach 完了後は Some"),
        WrapPlan::CharByChar,
    );
    let window = LayoutEngine::visible_window(&lines, &resolved.region, resolved.mode);
    (visible, window.first_visible_line, lines.len() - window.first_visible_line)
}

/// 純粋 layout から `(first_visible_line, visible_line_count)` を導出する
/// （GPU present 不使用・描画と同一 metrics ゆえ供給面の可視窓と一致・決定論）。
fn window_probe(demo: &Demo, actor: &ActorKey, t: f64) -> (usize, usize) {
    let (_, fvl, vlc) = reveal_probe(demo, actor, t);
    (fvl, vlc)
}

/// 指定注入時刻で `present_frame` を回す（失敗は log-first で Verdict へ記録して継続）。
fn present_at(demo: &mut Demo, world: &mut World, t: f64) {
    let mut rt = demo.runtime.borrow_mut();
    if let Err(e) = present_frame(&mut rt, world, t) {
        drop(rt);
        world
            .resource_mut::<Verdict>()
            .failures
            .push(format!("C8: present_frame({t}) が失敗: {e}"));
    }
}

/// 装着済み actor の `DrawStats`（`Copy`・借用を残さず値で返す）。
fn stats_of(demo: &Demo, actor: &ActorKey) -> DrawStats {
    demo.runtime
        .borrow()
        .draw_stats(actor)
        .expect("装着済み actor の draw_stats は Some")
}
