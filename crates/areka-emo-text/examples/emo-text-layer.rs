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
//! ```
//! シナリオは自動進行し（約 4 秒）、完了すると窓を閉じて `PASS`／`FAIL` を出力して終了する。
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
use windows::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
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
use areka_emo_text::layout::{GlyphMetrics, LayoutEngine};
use areka_emo_text::sink::EmoTextSink;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{parse_str, BalloonModel};
use areka_parsers::charset::{decode, DefaultEncoding};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use areka_sakura::TextSink;

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

    let mgr = WinApp::new()?;
    let world = mgr.world();

    world
        .borrow_mut()
        .world_mut()
        .insert_resource(Verdict::default());

    // UI スレッドで「アセット構築＋窓生成＋Demo 挿入」を行う（WIC は COM 初期化済みスレッド）。
    world.borrow().spawn(move |tx| async move {
        let _ = tx.send(Box::new(move |world: &mut World| {
            build_and_spawn(world, vertical);
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
    println!("  判定: readback 述語で自動判定し、最後に PASS/FAIL を 1 行出力（exit code 連動）");
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
fn build_and_spawn(world: &mut World, vertical: bool) {
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
    });
    info!(w, h, vertical, "emo-text-layer: 窓生成とアセット構築を完了（GPU 資源到達で装着）");
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
        info!("emo-text-layer: 全チェックポイント通過 — PASS で終了する");
        finish(demo, world);
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
}
