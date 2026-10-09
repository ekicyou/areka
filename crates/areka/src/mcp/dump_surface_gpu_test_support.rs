//! 実際の描画を通るテストの土台（spec: areka-P0-mcp-dump-images・design「Integration Tests」・
//! `dump_surface_gpu_tests.rs` と `dump_balloon_gpu_tests.rs` で共用・x64 だけ接続）。
//!
//! 偽の SHIORI の土台（[`SwitchRig`]・emo2 の複製のゴースト A）に、GPU 資源（WARP 可）と、本物の
//! 配置の準備（作業領域だけ合成）で組んだ窓の一式（偽 HWND・`DPI` は呼び手が選ぶ）を据え、本番の
//! `Input`・`Update` の段をそのまま回す。組み方は `shell_balloon_switch_session_lap_tests.rs` の
//! `lap_rig_of`・`spawn_windows` と同じ（2 つ目のシェルは足さない）。台本は A の `OnBoot` の応答で
//! 流し、置き場のゴーストの dispatcher へ合成の Tick を注入して進める。MCP の受け口と後から答える
//! 置き場も据えるので、装着の前に預けた答えはフレームを回すと届く。偽の SHIORI の呼び出しの列を
//! 比べる前に通す関所（kanade へ返事つきの印の NOTIFY）も持つ。ERROR の記録を捕まえる口は
//! 土台に持たず、呼ぶ側のテストが `log_capture_kit`（`capture`・`count_levels`）を直接使う。

use std::panic::Location;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use areka_actor::{ReplyReceiver, reply_channel};
use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::{BindSet, Composer, PatternState};
use areka_emo_present::balloon::{
    build_balloon_target, load_scope_balloon_model, resolve_balloon_faces,
};
use areka_emo_present::shell_target::load_shell_target;
use areka_emo_text::actor::ResolvedBalloonText;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_kanade::{KanadeMsg, MonotonicMs, RaiseOutcome, ShioriMethod};
use areka_mcp::tools::{Answer, Pending, ToolCall, ToolRequest, dump_balloon, dump_surface};
use bevy_ecs::world::World;
use windows::Win32::Foundation::{GENERIC_READ, HINSTANCE, HWND};
use windows::Win32::Graphics::Imaging::{
    GUID_WICPixelFormat32bppRGBA, IWICBitmapSource, WICBitmapDitherTypeNone,
    WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
};
use windows::core::{HSTRING, Interface};
use wintf::com::wic::{
    WICBitmapDecoderExt, WICBitmapSourceExt, WICFormatConverterExt, WICImagingFactoryExt,
    wic_factory,
};
use wintf::ecs::{DPI, FrameTime, GraphicsCore, Input, Update, WindowHandle, WucGraphicsResource};

use crate::emo2_boot::ghost_switch_test_support::{
    BALLOON, FakeShiori, SwitchRig, standard_script,
};
use crate::emo2_boot::spine::{GpuPermit, Progress, RecordedCall, wait_until};
use crate::ghost_session::GhostSlot;
use crate::mcp::resolve;
use crate::placement::follow::MonitorSnapshot;
use crate::placement::resolver::RectPx;
use crate::placement::spawn::spawn_ghost_windows;

/// 合成の作業領域（配置の準備と窓の追従の両方が読む）。
const WORK_AREA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 3840,
    bottom: 2100,
};
/// Tick を注入する実時間の最小間隔（反復ごとに投函して受信箱を溢れさせない）。
const TICK_EVERY: Duration = Duration::from_millis(1);
/// 1 巡で配るメッセージの上限（起こし直しの投函が続いても巡を終わらせる）。
const PUMP_MAX: usize = 1024;
/// 関所の印のイベント（許可表に在り、kanade は Tick が無いと自分では送らない・NOTIFY で送る）。
pub(in crate::mcp) const FLUSH_EVENT: &str = "OnSecondChange";

/// 窓・GPU つきのゴースト A。
pub(in crate::mcp) struct GpuRig {
    pub(in crate::mcp) rig: SwitchRig,
    /// 注入した Tick の合成の時刻（単調増加）。
    clock_ms: u64,
    /// GPU の装置の許可（[`GpuPermit`]）。`rig` の World の装置より後に返すので欄の最後。
    _gpu: GpuPermit,
}

impl GpuRig {
    /// A（起動記録あり＝`OnBoot` から始まる）を、`OnBoot` の応答が `on_boot` の偽の SHIORI で、
    /// 窓の `DPI` を `dpi`（96 で拡大率 1）にして起こす。フレームはまだ回さない（装着の前）。
    pub(in crate::mcp) fn new(on_boot: &str, dpi: u16) -> Self {
        let on_boot = on_boot.to_owned();
        let mut rig = SwitchRig::new(vec![(
            "A",
            FakeShiori::Scripted(Box::new(move || {
                // 関所の印の応答（[`GpuRig::flush_to_shiori`] 1 回で 1 件使う・2 回ぶん）。
                standard_script(&on_boot)
                    .notify(FLUSH_EVENT, Ok(()))
                    .notify(FLUSH_EVENT, Ok(()))
            })),
        )]);
        rig.plant_boot_record("A");
        spawn_windows(&mut rig, dpi);
        let gpu = GpuPermit::take();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
        let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
        let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
        rig.world.insert_resource(core);
        rig.world.insert_resource(wuc);
        // 送り手は捨てる（要求は直接 `handle` へ渡す）。置き場は後から答える組を毎フレーム覗かせる。
        let (_, inbox) = mpsc::channel();
        crate::mcp::install(&mut rig.world, inbox);
        rig.boot("A");
        Self {
            rig,
            clock_ms: 0,
            _gpu: gpu,
        }
    }

    /// 本番の `Input`・`Update` の段を `done` が真になるまで有界に回す（期限切れは `false`）。
    /// 巡ごとに置き場のゴーストの dispatcher へ `step_ms` 進めた合成の Tick を注入して台詞を進め
    /// （0 なら台詞の時計は止まったまま）、本番の巡と同じく `FrameTime` を置き、メッセージを配る。
    ///
    /// 打ち切りは足場の進みの目印（`SwitchRig::progress_probe`・SHIORI の呼び出しの数）で決め、打ち切ったら
    /// 呼び出しの場所を添えた文言を標準エラーへ 1 行出して `false`（lap の `LapRig::frames_until` と同じ形・
    /// areka-P0-ghost-session-test-load-flake 要件 2.1）。台詞の再生（dispatcher が Tick を消化する進み）は
    /// テストの側から読めないので、SHIORI を呼ばずに台詞だけが進む間は目印が動かない（30 秒で［止まった］）。
    /// 合成の時刻は 1 巡に高々 `step_ms` しか進まない（時刻が観測を追い越さない）ので、台本の待ちを
    /// `step_ms` 1 で進めると待ちの ms と同じ数の巡が要り、巡が遅い机では届かない。`step_ms` 1（実時間
    /// ほど）で刻むのは字が現れ切るまでに限り、その後の台本の待ちは大きな `step_ms` で進める。
    #[track_caller]
    pub(in crate::mcp) fn frames_until(
        &mut self,
        step_ms: u64,
        mut done: impl FnMut(&World) -> bool,
    ) -> bool {
        let Self { rig, clock_ms, .. } = self;
        let mut last_tick: Option<Instant> = None;
        let probe = rig.progress_probe();
        let at = Location::caller();
        let waited = wait_until(
            &format!("{}:{}", at.file(), at.line()),
            Progress::Count(&probe),
            || {
                if step_ms > 0 && last_tick.is_none_or(|at| at.elapsed() >= TICK_EVERY) {
                    last_tick = Some(Instant::now());
                    *clock_ms += step_ms;
                    if let Some(dispatcher) = rig
                        .world
                        .get_non_send::<GhostSlot>()
                        .and_then(|slot| slot.0.as_ref())
                        .and_then(|session| session.dispatcher())
                    {
                        let _ = dispatcher.send(DispatcherMsg::Tick {
                            now: MonotonicMs(*clock_ms),
                        });
                    }
                }
                rig.world
                    .insert_resource(FrameTime(dola::runtime::clock::now()));
                pump_messages();
                rig.world.run_schedule(Input);
                rig.world.run_schedule(Update);
                done(&rig.world)
            },
        );
        if let Err(failure) = &waited {
            eprintln!("{failure}");
        }
        waited.is_ok()
    }

    /// `dump_surface` を要求として作り、本番の `handle` へ渡す（答えは返事を受ける側から覗く）。
    pub(in crate::mcp) fn dump_surface(
        &mut self,
        scope: Option<i64>,
        surface: Option<i64>,
    ) -> Pending {
        let args = dump_surface::Args {
            scope,
            surface,
            ghost_name: None,
        };
        let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(args.clone()));
        let ghost = resolve::active(&self.rig.world).expect("A は起動中");
        super::handle(&mut self.rig.world, &ghost, args, req.reply);
        pending
    }

    /// `dump_balloon` を要求として作り、本番の `handle` へ渡す。
    pub(in crate::mcp) fn dump_balloon(&mut self, scope: Option<i64>) -> Pending {
        let args = dump_balloon::Args {
            scope,
            ghost_name: None,
        };
        let (req, pending) = ToolRequest::new(ToolCall::DumpBalloon(args.clone()));
        let ghost = resolve::active(&self.rig.world).expect("A は起動中");
        crate::mcp::dump_balloon::handle(&mut self.rig.world, &ghost, args, req.reply);
        pending
    }

    /// 台詞の時計を止めたまま本番の段を有界に回し、`pending` の答えを待つ（答えが後の巡で届いても、
    /// その場で届いていても同じに使える）。期限切れ・答えずに手放したら `None`。
    #[track_caller]
    pub(in crate::mcp) fn answer_of(&mut self, pending: &Pending) -> Option<Answer> {
        let mut got = None;
        self.frames_until(0, |_| match pending.try_answer() {
            Ok(Some(answer)) => {
                got = Some(answer);
                true
            }
            Ok(None) => false,
            Err(_) => true,
        });
        got
    }

    /// 台詞の時計を止めたまま、本番の段を `n` 巡だけ回す（読み戻しの後に画面へ遅れて届く変化が
    /// 無いことを見るため）。
    #[track_caller]
    pub(in crate::mcp) fn frames(&mut self, n: usize) {
        let mut left = n;
        self.frames_until(0, |_| {
            left = left.saturating_sub(1);
            left == 0
        });
    }

    /// 呼び出しの列を比べる前の関所: A の kanade へ印の NOTIFY（[`FLUSH_EVENT`]）を返事つきで送り、
    /// 返事が届くまで巡を回す。定常でないと断られたら送り直す。kanade は依頼を届いた順に 1 つずつ
    /// 済ませるので、送れた（`NoReply`）返事が届いた時点で、それより前に届いた依頼はみな SHIORI まで
    /// 済み、印も呼び出しの列に載っている。返るのは最後の返事（期限切れ・送れなければ `None`）。
    #[track_caller]
    pub(in crate::mcp) fn flush_to_shiori(&mut self) -> Option<RaiseOutcome> {
        let mut waiting: Option<ReplyReceiver<RaiseOutcome>> = None;
        let mut last = None;
        self.frames_until(0, |world| {
            if let Some(rx) = &waiting {
                match rx.try_recv() {
                    Ok(Some(RaiseOutcome::NotSteady)) => waiting = None,
                    Ok(Some(outcome)) => {
                        last = Some(outcome);
                        return true;
                    }
                    _ => return false,
                }
            }
            let (tx, rx) = reply_channel();
            let sent = world
                .get_non_send::<GhostSlot>()
                .and_then(|slot| slot.0.as_ref())
                .and_then(|session| session.kanade())
                .is_some_and(|kanade| {
                    kanade
                        .send(KanadeMsg::RaiseEvent {
                            id: FLUSH_EVENT.to_owned(),
                            references: Vec::new(),
                            method: ShioriMethod::Notify,
                            reply: Some(tx),
                        })
                        .is_ok()
                });
            waiting = sent.then_some(rx);
            false
        });
        last
    }

    /// A の偽の SHIORI の呼び出しの列（状態の問い合わせと `homeurl` の照会を除く）。
    pub(in crate::mcp) fn calls(&self) -> Vec<RecordedCall> {
        self.rig.calls("A").into_iter().next().unwrap_or_default()
    }

    /// A のシェル（`shell/master`）を表示の経路と同じ読み込みで組み、`surface_id` を着せ替えなし・
    /// アニメーションなしで合成した絵の、乗算を戻した 8 ビット RGBA と（幅, 高さ）。
    pub(in crate::mcp) fn composed_rgba(&self, surface_id: u32) -> (u32, u32, Vec<u8>) {
        let shell_dir = self.rig.root.ghost_dir("A").join("shell").join("master");
        let decoder = WicDecoderArm::new().expect("WIC の復号器");
        let target = load_shell_target(&shell_dir, &decoder).expect("A のシェルを読める");
        let pic = Composer::new()
            .compose(
                &target.build_world(),
                target.atlas(),
                surface_id,
                &BindSet::default(),
                &PatternState::default(),
            )
            .expect("合成できる");
        (pic.width(), pic.height(), unpremultiply(pic.bytes()))
    }

    /// 装着の相と同じ読み込みで `scope` のバルーンを組み、面 0（装着で表示を確立する面）を
    /// 合成した絵の、乗算済み BGRA のままの画素と（幅, 高さ）。
    pub(in crate::mcp) fn balloon_composed(&self, scope: u32) -> (u32, u32, Vec<u8>) {
        let dir = self.rig.root.balloon_dir(BALLOON);
        let decoder = WicDecoderArm::new().expect("WIC の復号器");
        let (world, atlas) =
            build_balloon_target(&dir, &decoder, scope).expect("A のバルーンを読める");
        let pic = Composer::new()
            .compose(
                &world,
                &atlas,
                0,
                &BindSet::default(),
                &PatternState::default(),
            )
            .expect("合成できる");
        (pic.width(), pic.height(), pic.bytes().to_vec())
    }

    /// `scope` のバルーンの定義（面 0 の上書き込み）から解いた、文字の領域の原点（原寸の px）。
    pub(in crate::mcp) fn text_area_origin(
        &self,
        scope: u32,
        image_size: (u32, u32),
    ) -> (f32, f32) {
        let dir = self.rig.root.balloon_dir(BALLOON);
        let faces = resolve_balloon_faces(&dir, scope).expect("面を解ける");
        let model = load_scope_balloon_model(&dir, scope, &faces[0]);
        let region = ResolvedBalloonText::resolve(&model, image_size).region;
        (region.left(), region.top())
    }

    /// 置き場のゴーストを有界に降ろす（成功で `true`）。
    pub(in crate::mcp) fn shutdown(&mut self) -> bool {
        self.rig.shutdown()
    }
}

/// A の起動の配置（作業領域だけ合成・作者 DPI 96）で 2 スコープの窓を生やし、偽 HWND と
/// `DPI::from_dpi(dpi, dpi)` を持たせる（`WindowPos` への書込の口が通る・実窓には触れない）。
fn spawn_windows(rig: &mut SwitchRig, dpi: u16) {
    let prepared = crate::placement::prepare_ghost_windows_with_work_area(
        &rig.root.ghost_dir("A"),
        &rig.root.balloon_dir(BALLOON),
        WORK_AREA,
        Some(96),
    )
    .expect("A の配置の準備");
    let world = &mut rig.world;
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![WORK_AREA],
    });
    let windows = spawn_ghost_windows(world, &prepared.placements, &prepared.titles);
    let mut raw = 0x100usize;
    let all: Vec<_> = windows
        .scopes()
        .flat_map(|s| [windows.char_window(s), windows.balloon_window(s)])
        .map(Option::unwrap)
        .collect();
    for &window in &all {
        world.entity_mut(window).insert(WindowHandle {
            hwnd: HWND(raw as *mut _),
            instance: HINSTANCE::default(),
        });
        raw += 0x10;
    }
    // `WindowHandle` の付与の hook が偽 HWND の DPI を遅延で書くので、流してから置く。
    world.flush();
    for window in all {
        world.entity_mut(window).insert(DPI::from_dpi(dpi, dpi));
    }
}

/// 今のスレッドのキューに溜まったメッセージを配る（本番のメッセージループの 1 巡ぶん）。文字の層の
/// cue の適用は executor の窓へのメッセージで走るので、配らないと台詞の文字が載らない。
fn pump_messages() {
    let mut msg = MSG::default();
    for _ in 0..PUMP_MAX {
        // SAFETY: 今のスレッドのキューから 1 件取り出して配るだけ（待たない）。
        let got = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) };
        if !got.as_bool() {
            break;
        }
        // SAFETY: いま取り出したメッセージを同じスレッドで配る。
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// 乗算済み BGRA → 乗算していない RGBA（要件 5.2 の式・検査の側で別に組む）。
pub(in crate::mcp) fn unpremultiply(premultiplied_bgra: &[u8]) -> Vec<u8> {
    premultiplied_bgra
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[b, g, r, a]| {
            if a == 0 {
                return [0, 0, 0, 0];
            }
            let a32 = u32::from(a);
            let un = |c: u8| ((u32::from(c) * 255 + a32 / 2) / a32).min(255) as u8;
            [un(r), un(g), un(b), a]
        })
        .collect()
}

/// 画像の content の base64 の PNG を、ワークツリーの `target\` の下の一時ファイルを経て WIC で
/// 復号し、乗算していない RGBA の（幅, 高さ, 画素）を返す（別の実装で読み戻す）。
pub(in crate::mcp) fn decode_png(png_base64: &str) -> (u32, u32, Vec<u8>) {
    // 呼び出しごとに別のフォルダ（並走するテストが互いのフォルダを消さない）。
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-tmp")
        .join(format!(
            "areka-dump-gpu-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&dir).expect("target の下に一時ディレクトリを作れる");
    let file = dir.join("dump.png");
    std::fs::write(&file, base64_decode(png_base64)).expect("PNG を書ける");
    let decoded = wic_decode_rgba(&file);
    let _ = std::fs::remove_dir_all(&dir);
    decoded
}

fn wic_decode_rgba(path: &Path) -> (u32, u32, Vec<u8>) {
    let factory = wic_factory().expect("WIC の工場");
    let decoder = factory
        .create_decoder_from_filename(
            &HSTRING::from(path.as_os_str()),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )
        .expect("WIC が PNG を開ける");
    let frame = decoder.frame(0).expect("先頭のフレーム");
    let converter = factory.create_format_converter().expect("変換器");
    converter
        .init(
            &frame.cast::<IWICBitmapSource>().expect("フレームは絵の源"),
            &GUID_WICPixelFormat32bppRGBA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )
        .expect("乗算していない RGBA へ変換できる");
    let source: IWICBitmapSource = converter.cast().expect("変換器は絵の源");
    let (w, h) = source.get_size().expect("大きさ");
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    source
        .copy_pixels(None, w * 4, &mut pixels)
        .expect("画素を写せる");
    (w, h, pixels)
}

/// 標準の文字集合・`=` の詰めありの base64 を戻す（改行なし前提）。
fn base64_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    for group in text.as_bytes().chunks(4) {
        let digits: Vec<u32> = group
            .iter()
            .take_while(|&&c| c != b'=')
            .map(|c| ALPHABET.iter().position(|a| a == c).expect("base64 の字") as u32)
            .collect();
        let n = digits
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, d)| n | d << (18 - 6 * i));
        out.extend_from_slice(&n.to_be_bytes()[1..digits.len()]);
    }
    out
}
