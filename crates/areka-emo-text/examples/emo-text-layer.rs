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

// 責務単位のサブモジュール。`examples/` 直下に置くと Cargo が別のサンプルターゲットとして
// 拾ってしまうため、サブディレクトリへ置いてパス属性で接続する（当該ディレクトリに `main.rs`
// は作らない——作るとそれ自体が別ターゲットになる）。子は上の `use` 束縛を `use super::{…};`
// で引くので、非 test ビルドでも全 import が消費され未使用インポートの抑止指示は要らない。
#[path = "emo-text-layer/demo.rs"]
mod demo;
#[path = "emo-text-layer/drive.rs"]
mod drive;
#[path = "emo-text-layer/fixture.rs"]
mod fixture;
#[path = "emo-text-layer/scenario.rs"]
mod scenario;
#[path = "emo-text-layer/setup.rs"]
mod setup;
#[path = "emo-text-layer/verdict.rs"]
mod verdict;

// 子モジュールの項目もここで束ね直す（子同士は `use super::{…};` でこの束縛を引く）。
// 新しい公開モジュールパスは生えない——子はすべて私有 `mod` である。
use self::demo::{Demo, actor0, actor1};
use self::drive::drive_demo_system;
use self::fixture::{
    KERO_GAP_Y, SAKURA_POS, WATCHDOG_SECS, load_balloon_model, shared_balloon_dir,
};
use self::scenario::{EXPOSURE_BAND_DRAW_BOUND, GATE_KERO, GATE_SAKURA, T_CHECK, stage_cues};
use self::setup::build_and_spawn;
use self::verdict::{
    Observations, Verdict, band_ink, block_extent, inline_extent_first_band, opaque_count,
};

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
