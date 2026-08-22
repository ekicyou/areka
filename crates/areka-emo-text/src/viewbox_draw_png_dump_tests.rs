use super::ViewboxExecutor;
use super::test_support::{Rig, glyph_items};
use crate::canvas::ContentCanvas;
use crate::draw::{DWriteMetrics, DrawExecutor};
use crate::layout::{LayoutEngine, WrapPlan};
use crate::region::ScaleContract;
use crate::state::{TextItem, TextLayerConfig, TextLayerState};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

// ════ 目視診断（PNG ダンプ・#[ignore]・開発者の「出力画像を見せて」要求への回答） ════
//
// 実 fixture（font 28px・validrect 320×122・実 DWriteMetrics・行1「おっはよー！」/行2
// 「めっちゃええ朝やん！」/行3「今日もいくでー！」＋あふれ短行）を typewriter 前進で
// viewbox/oracle 双方へ描き、指定フレームで read_back を白背景へ合成して PNG 保存する。
// byte 等価檻（`diag_line_boundary_dropout_vs_oracle`）は「viewbox==oracle」を保証するが、
// 「その全域再描画自体が正しく見えるか（下端欠け等）」は人（AI vision）が画像を見るしかない。
// 出力先は env `AREKA_DIAG_OUT`（無指定なら CARGO_TARGET_TMPDIR 相当のカレント）。
// 依存を足さないため PNG は自前エンコード（無圧縮 deflate・RGBA8）。

/// CRC-32（PNG チャンク用・多項式 0xEDB88320）。
fn diag_crc32(data: &[u8]) -> u32 {
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
fn diag_adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// PNG チャンク（長さ＋種別＋データ＋CRC）を書き足す。
fn diag_png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut kd = Vec::with_capacity(4 + data.len());
    kd.extend_from_slice(kind);
    kd.extend_from_slice(data);
    out.extend_from_slice(&kd);
    out.extend_from_slice(&diag_crc32(&kd).to_be_bytes());
}

/// RGBA8 を無圧縮 deflate（stored ブロック）で PNG エンコードする（依存追加なし）。
fn diag_encode_png_rgba(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    // フィルタ 0 のスキャンライン列。
    let mut raw = Vec::with_capacity((h * (1 + w * 4)) as usize);
    for y in 0..h {
        raw.push(0u8);
        let row = ((y * w * 4) as usize)..(((y + 1) * w * 4) as usize);
        raw.extend_from_slice(&rgba[row]);
    }
    // zlib: ヘッダ 0x78 0x01 ＋ stored ブロック列 ＋ adler32。
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
    zlib.extend_from_slice(&diag_adler32(&raw).to_be_bytes());

    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8bit・color type 6（RGBA）
    diag_png_chunk(&mut out, b"IHDR", &ihdr);
    diag_png_chunk(&mut out, b"IDAT", &zlib);
    diag_png_chunk(&mut out, b"IEND", &[]);
    out
}

/// premultiplied BGRA read_back を白背景へ合成し、pitch グリッド線（薄青・破線的に）と
/// 面境界（マゼンタ）を重ねた RGBA8 を返す（下端欠け・行境界の欠落を目視しやすくする）。
fn diag_composite_rgba(bgra: &[u8], w: u32, h: u32, pitch: u32) -> Vec<u8> {
    let bg = [255u8, 255, 255];
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for (idx, px) in bgra.chunks_exact(4).enumerate() {
        let (b, g, r, a) = (px[0] as u32, px[1] as u32, px[2] as u32, px[3] as u32);
        let inv = 255 - a;
        let out_r = (r + bg[0] as u32 * inv / 255).min(255) as u8;
        let out_g = (g + bg[1] as u32 * inv / 255).min(255) as u8;
        let out_b = (b + bg[2] as u32 * inv / 255).min(255) as u8;
        let o = idx * 4;
        rgba[o] = out_r;
        rgba[o + 1] = out_g;
        rgba[o + 2] = out_b;
        rgba[o + 3] = 255;
    }
    // pitch グリッド線（各行の上端 y=0,pitch,2pitch,…）を薄青で、境界をマゼンタで。
    let put = |rgba: &mut [u8], x: u32, y: u32, c: [u8; 3]| {
        if x < w && y < h {
            let o = ((y * w + x) * 4) as usize;
            rgba[o] = c[0];
            rgba[o + 1] = c[1];
            rgba[o + 2] = c[2];
        }
    };
    if pitch > 0 {
        let mut gy = 0u32;
        while gy < h {
            for x in (0..w).step_by(3) {
                put(&mut rgba, x, gy, [170, 200, 255]);
            }
            gy += pitch;
        }
    }
    for x in 0..w {
        put(&mut rgba, x, 0, [255, 0, 255]);
        put(&mut rgba, x, h - 1, [255, 0, 255]);
    }
    for y in 0..h {
        put(&mut rgba, 0, y, [255, 0, 255]);
        put(&mut rgba, w - 1, y, [255, 0, 255]);
    }
    rgba
}

/// 最下端の非透明インク行 y（画面下端欠けの定量指標・インクなしは None）。
fn diag_bottom_ink_row(bgra: &[u8], w: u32, h: u32) -> Option<u32> {
    for y in (0..h).rev() {
        for x in 0..w {
            if bgra[((y * w + x) * 4 + 3) as usize] != 0 {
                return Some(y);
            }
        }
    }
    None
}

/// 目視診断: 実 fixture を typewriter 前進で描き、指定 visible 数のフレームで viewbox/oracle の
/// read_back を PNG 保存する（`cargo test -p areka-emo-text --lib -- --ignored --nocapture
/// diag_dump_horizontal_pngs`・env `AREKA_DIAG_OUT` で出力先指定）。
#[test]
#[ignore = "PNG ダンプ（ファイル副作用・目視診断用・明示実行のみ）"]
fn diag_dump_horizontal_pngs() {
    let out_dir = std::env::var("AREKA_DIAG_OUT").unwrap_or_else(|_| ".".to_string());
    let mut rig = Rig::new();
    // 実 fixture（emo2-kakukaku）の balloon model を実ファイルからロードする（Yu Gothic UI・
    // validrect [36,356]×[46,168]＝320×122・wordwrappoint.x=-49）——example の load_balloon_model
    // と同一経路。フォント/折返し/validrect を実機と完全一致させる（既定 ＭＳ ゴシックでない）。
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku");
    let read_dec = |name: &str| -> String {
        let bytes = std::fs::read(fixture_dir.join(name)).expect("fixture 読取");
        areka_parsers::charset::decode(&bytes, areka_parsers::charset::DefaultEncoding::Utf8)
    };
    let model = areka_parsers::balloon::parse_str(
        &read_dec("descript.txt"),
        Some(&read_dec("balloons0s.txt")),
    );
    // balloon 画像原寸（surface0＝400×224）で region を解決する（validrect は image 相対）。
    let balloon_image = (400u32, 224u32);
    let resolved = crate::actor::ResolvedBalloonText::resolve(&model, balloon_image);
    let mode = resolved.mode;
    let font = &resolved.font;
    let region = &resolved.region;
    // 供給面＝validrect 物理寸（ceil(validrect × k)・k=1）。
    let image = (
        (region.right() - region.left()).ceil() as u32,
        (region.bottom() - region.top()).ceil() as u32,
    );
    let mut oracle_surface = rig.attach(image, 1.0);
    let mut viewbox_surface = rig.attach(image, 1.0);
    let contract = ScaleContract::new(1.0, None);
    let config = TextLayerConfig::default();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let metrics = DWriteMetrics::new(&factory, font, mode, &config).expect("DWriteMetrics");
    let pitch = {
        use crate::layout::GlyphMetrics;
        metrics.line_pitch(font.height) as u32
    };
    eprintln!(
        "[diag] model: font={} height={} mode={mode:?} validrect=[{},{}]x[{},{}] surface={image:?} wrap={}",
        font.name,
        font.height,
        region.left(),
        region.right(),
        region.top(),
        region.bottom(),
        region.wrap_threshold()
    );
    let mut oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor");
    let mut viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor");
    let actor = ActorKey::from("0");
    let mut state = TextLayerState::default();
    let mk = |cmd| TalkCue {
        at: 0.0,
        actor: actor.clone(),
        command: cmd,
        duration: 0.0,
    };
    // 実 example と同一の cue `at` スケジュール（LINE1 at 0.0・LINE2 at 0.5・LINE3 at 1.2・
    // あふれ短行 at 2.0）。reveal は配送 duration 由来（interval = duration / N）。Text へ
    // N×0.05 を焼き込むと各 chunk 内が旧 char_wait=0.05 と同一ペースで per-glyph 進行する
    // （chunk 境界は at で gate）。
    // 【newline-defer】かつて（即時意味論では）at を分散させると未リビール NewLine が即座に
    // 「幽霊空行」を出し人工的なスクロールを誘発した——遅延化（deferred newline）で保留改行は
    // 次の可視グリフが reveal されるまで行を開かないため、幽霊空行はもはや生じず、あふれ発火は
    // 実体化時刻（改行より後ろのグリフの reveal 時）へ後退する。at 分散は実機の reveal タイミングを
    // 模す診断ダンプの時間対応としてのみ残す（幽霊空行の再現目的ではない）。
    let cue_at = |at: f64, cmd: CueCommand| TalkCue {
        at,
        actor: actor.clone(),
        duration: match &cmd {
            CueCommand::Text(t) => t.chars().count() as f64 * 0.05,
            _ => 0.0,
        },
        command: cmd,
    };
    state.apply_cue(&cue_at(0.0, CueCommand::Text("おっはよー！".into())));
    state.apply_cue(&cue_at(0.5, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue_at(
        0.5,
        CueCommand::Text("めっちゃええ朝やん！".into()),
    ));
    state.apply_cue(&cue_at(1.2, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue_at(1.2, CueCommand::Text("今日もいくでー！".into())));
    for _ in 0..9 {
        state.apply_cue(&cue_at(2.0, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue_at(2.0, CueCommand::Text("ほな".into())));
    }
    let _ = mk;

    // 密な時間格子で前進提示（viewbox は状態依存ゆえ昇順に全フレーム回す）。dump は目標時刻通過時。
    let dt = 0.02f64;
    let mut dump_times: Vec<f64> = vec![0.35, 1.10, 1.90, 2.02, 2.10, 2.30, 2.60, 3.00];
    eprintln!(
        "[diag] out_dir={out_dir} pitch={pitch} region_h={} reveal_interval={}",
        region.bottom() - region.top(),
        0.05
    );
    let mut t = 0.0f64;
    while t <= 3.05 {
        let visible = state.visible_glyphs(&actor, t);
        let items: Vec<TextItem> = state
            .actor_state(&actor)
            .map(|s| s.items().to_vec())
            .unwrap_or_default();
        let lines = LayoutEngine::layout(
            &items,
            visible,
            &region,
            mode,
            font.height,
            &metrics,
            WrapPlan::CharByChar,
        );
        let window = LayoutEngine::visible_window(&lines, &region, mode);
        let canvas = ContentCanvas::from_layout(&lines, &region, mode);
        oracle
            .render(&canvas, &window, font, mode, &contract, &mut oracle_surface)
            .expect("oracle render");
        viewbox
            .render(
                &canvas,
                &window,
                font,
                mode,
                &contract,
                &mut viewbox_surface,
            )
            .expect("viewbox render");

        if dump_times.first().is_some_and(|&t_target| t >= t_target) {
            let target = dump_times.remove(0);
            let ob = oracle_surface.read_back().expect("oracle read_back");
            let vb = viewbox_surface.read_back().expect("viewbox read_back");
            let (w, h) = viewbox_surface.size();
            let diff_rows = (0..h)
                .filter(|&y| {
                    let r = ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
                    ob[r.clone()] != vb[r]
                })
                .count();
            let vb_bottom = diag_bottom_ink_row(&vb, w, h);
            // 差分行の内訳: 各 y で「oracle インク有り・viewbox 空」= 欠け画素数。
            let mut miss_rows: Vec<(u32, u32)> = Vec::new();
            for y in 0..h {
                let mut miss = 0u32;
                for x in 0..w {
                    let o = ((y * w + x) * 4) as usize;
                    if ob[o + 3] != 0 && vb[o + 3] == 0 {
                        miss += 1;
                    }
                }
                if miss > 0 {
                    miss_rows.push((y, miss));
                }
            }
            eprintln!(
                "[diag] t={t:.2}(target {target:.2}) visible={visible} lines={} fvl={} \
                 block_off={:.1} diff_rows={diff_rows} viewbox_bottom_ink_y={vb_bottom:?} (面高={h})",
                lines.len(),
                window.first_visible_line,
                window.block_offset
            );
            eprintln!("[diag]   欠け行(oracle有/viewbox空) y,画素数 = {miss_rows:?}");
            let ms = (target * 100.0).round() as u32;
            for (tag, bytes) in [("viewbox", &vb), ("oracle", &ob)] {
                let rgba = diag_composite_rgba(bytes, w, h, pitch);
                let png = diag_encode_png_rgba(&rgba, w, h);
                let path = format!("{out_dir}/diag_h_{tag}_t{ms:03}.png");
                std::fs::write(&path, &png).expect("PNG 書き込み");
                eprintln!("[diag]   saved {path}");
            }
            // diff 画像: oracle を薄く敷き、viewbox≠oracle の画素を赤で強調（欠け位置の可視化）。
            {
                let mut rgba = diag_composite_rgba(&ob, w, h, pitch);
                // oracle をグレーアウト（インク位置の文脈だけ残す）。
                for px in rgba.chunks_exact_mut(4) {
                    px[0] = 200 + (px[0] as u16 * 55 / 255) as u8;
                    px[1] = 200 + (px[1] as u16 * 55 / 255) as u8;
                    px[2] = 200 + (px[2] as u16 * 55 / 255) as u8;
                }
                for y in 0..h {
                    for x in 0..w {
                        let o = ((y * w + x) * 4) as usize;
                        let (oa, va) = (ob[o + 3], vb[o + 3]);
                        let diff = ob[o] != vb[o]
                            || ob[o + 1] != vb[o + 1]
                            || ob[o + 2] != vb[o + 2]
                            || oa != va;
                        if diff {
                            // oracle にインクがあり viewbox に無い＝欠け（赤）。逆＝過剰（青）。
                            if oa != 0 && va == 0 {
                                rgba[o] = 255;
                                rgba[o + 1] = 0;
                                rgba[o + 2] = 0;
                            } else {
                                rgba[o] = 0;
                                rgba[o + 1] = 80;
                                rgba[o + 2] = 255;
                            }
                        }
                    }
                }
                let png = diag_encode_png_rgba(&rgba, w, h);
                let path = format!("{out_dir}/diag_h_DIFF_t{ms:03}.png");
                std::fs::write(&path, &png).expect("PNG 書き込み");
                eprintln!("[diag]   saved {path} (赤=viewbox欠け・青=過剰)");
            }
        }
        t += dt;
    }
}

/// 目視診断（budoux ワードラップ・R9.2/9.3）: 実 fixture（emo2-kakukaku・Yu Gothic UI 28px・
/// 狭い validrect 320×122・`wordwrappoint.x=-49`）へ、(1) 通常文を **OFF（文字単位）** と
/// **ON（分かち書き）** の両方、(2) budoux 境界を持たない長大塊を **ON** で描き、fully-revealed
/// （全グリフ可視）で PNG を保存する。
///
/// - **通常ケース（OFF/ON 並置）**: 狭いバルーンでは OFF が文節を行末で途中分割するが、ON は
///   塊を丸ごと次行へ送る（R9.1）。`diag_budoux_normal_off.png` と `diag_budoux_normal_on.png`
///   を並べて改善を目視できる。
/// - **長大塊ケース（ON）**: 行頭からでも 1 行に収まらない塊（budoux 境界のない連続カナ）は
///   当該塊に限って文字単位縮退し、バルーン（validrect＝供給面寸）からはみ出さない（R9.2）。
///   `diag_budoux_longseg_on.png` を出す。最右インク列が供給面右端で張り付いていない（＝
///   はみ出しクリップでない）ことを `eprintln!` の数値でも補助確認する。
///
/// byte 等価檻（`diag_line_boundary_dropout_vs_oracle` 等）は「viewbox==oracle」を保証するが、
/// 「分かち書き折返しが読みやすく見えるか・長大塊がはみ出さないか」は人（AI vision）が画像を
/// 見るしかない（記憶 emo-text-byte-equiv-default-font-blindspot）。全域再描画オラクル
/// [`DrawExecutor`] を使い fully-revealed の完成画像を 1 フレームで得る。
/// `cargo test -p areka-emo-text --lib -- --ignored --nocapture diag_dump_budoux_wordwrap_pngs`
/// （出力先は env `AREKA_DIAG_OUT`・無指定はカレント）。
#[test]
#[ignore = "PNG ダンプ（ファイル副作用・目視診断用・明示実行のみ）"]
fn diag_dump_budoux_wordwrap_pngs() {
    let out_dir = std::env::var("AREKA_DIAG_OUT").unwrap_or_else(|_| ".".to_string());
    let mut rig = Rig::new();
    // 実 fixture（emo2-kakukaku）を実ファイルからロード（Yu Gothic UI・validrect 320×122・
    // wordwrappoint.x=-49）——diag_dump_horizontal_pngs と同一経路。狭いバルーンが char-by-char
    // の途中分割を誘発する（＝ON の改善が見える台）。
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku");
    let read_dec = |name: &str| -> String {
        let bytes = std::fs::read(fixture_dir.join(name)).expect("fixture 読取");
        areka_parsers::charset::decode(&bytes, areka_parsers::charset::DefaultEncoding::Utf8)
    };
    let model = areka_parsers::balloon::parse_str(
        &read_dec("descript.txt"),
        Some(&read_dec("balloons0s.txt")),
    );
    // balloon 画像原寸（surface0＝400×224）で region を解決する（validrect は image 相対）。
    let balloon_image = (400u32, 224u32);
    let resolved = crate::actor::ResolvedBalloonText::resolve(&model, balloon_image);
    let mode = resolved.mode;
    let font = &resolved.font;
    let region = &resolved.region;
    // 供給面＝validrect 物理寸（ceil(validrect × k)・k=1）＝バルーンの折返し閾/はみ出し境界。
    let image = (
        (region.right() - region.left()).ceil() as u32,
        (region.bottom() - region.top()).ceil() as u32,
    );
    let contract = ScaleContract::new(1.0, None);
    let config = TextLayerConfig::default();
    let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
    let metrics = DWriteMetrics::new(&factory, font, mode, &config).expect("DWriteMetrics");
    let pitch = {
        use crate::layout::GlyphMetrics;
        metrics.line_pitch(font.height) as u32
    };
    eprintln!(
        "[diag-budoux] model: font={} height={} mode={mode:?} validrect=[{},{}]x[{},{}] surface={image:?} wrap_threshold={}",
        font.name,
        font.height,
        region.left(),
        region.right(),
        region.top(),
        region.bottom(),
        region.wrap_threshold()
    );

    // 1 ケース（sentence を fully-revealed で描画）を PNG 保存するヘルパ。ON（use_budoux=true）は
    // actor.rs と同型の遅延初期化で segment_plan を計算し WrapPlan::Segmented を供給する（OFF は
    // segment_plan を呼ばず CharByChar）。戻り値＝(行数, 最右インク列 x)。
    let dump_case = |rig: &mut Rig,
                     tag: &str,
                     sentence: &str,
                     use_budoux: bool|
     -> (usize, Option<u32>) {
        let items = glyph_items(sentence);
        let visible = items
            .iter()
            .filter(|i| matches!(i, TextItem::Glyph { .. }))
            .count();
        let plan; // ON アームでのみ束縛（借用 &plan が layout 呼出まで生存）。
        let wrap = if use_budoux {
            plan = crate::segment::segment_plan(&items);
            let segs: Vec<(usize, usize)> =
                plan.segments().iter().map(|s| (s.start, s.len)).collect();
            eprintln!("[diag-budoux]   segments(start,len)={segs:?}");
            WrapPlan::Segmented(&plan)
        } else {
            WrapPlan::CharByChar
        };
        let lines =
            LayoutEngine::layout(&items, visible, region, mode, font.height, &metrics, wrap);
        let window = LayoutEngine::visible_window(&lines, region, mode);
        let canvas = ContentCanvas::from_layout(&lines, region, mode);
        let mut surface = rig.attach(image, 1.0);
        let mut exec = DrawExecutor::new(&rig.core).expect("DrawExecutor");
        exec.render(&canvas, &window, font, mode, &contract, &mut surface)
            .expect("render");
        let bytes = surface.read_back().expect("read_back");
        let (w, h) = surface.size();
        // 最右インク列（inline はみ出しの定量指標・供給面は validrect 寸ゆえ描画は自動クリップ＝
        // w-1 に張り付くならはみ出しをクリップで隠している疑い）。
        let rightmost = {
            let mut r: Option<u32> = None;
            for x in (0..w).rev() {
                let hit = (0..h).any(|y| bytes[((y * w + x) * 4 + 3) as usize] != 0);
                if hit {
                    r = Some(x);
                    break;
                }
            }
            r
        };
        let rgba = diag_composite_rgba(&bytes, w, h, pitch);
        let png = diag_encode_png_rgba(&rgba, w, h);
        let path = format!("{out_dir}/diag_budoux_{tag}.png");
        std::fs::write(&path, &png).expect("PNG 書き込み");
        eprintln!(
            "[diag-budoux]   saved {path}  budoux={use_budoux} sentence=「{sentence}」 \
                 visible={visible} lines={} rightmost_ink_x={rightmost:?} (surface_w={w} pitch={pitch})",
            lines.len()
        );
        (lines.len(), rightmost)
    };

    // ── 通常ケース: 狭いバルーンで char-by-char が文節を途中分割する和文（OFF/ON 並置）。 ──
    let normal = "今日はとても良い天気ですね一緒に近くの公園へ遊びに行きましょう";
    eprintln!("[diag-budoux] === 通常ケース（OFF: 文字単位） ===");
    dump_case(&mut rig, "normal_off", normal, false);
    eprintln!("[diag-budoux] === 通常ケース（ON: 分かち書き） ===");
    dump_case(&mut rig, "normal_on", normal, true);

    // ── 長大塊ケース: budoux 境界を持たない連続カナ（行頭からでも 1 行に収まらない）を ON で。 ──
    // 前後に通常の和文を置き「長大塊のみ縮退し前後は分かち書き継続」を目視できる形にする（R3.3）。
    let longseg =
        "むかしむかしバアアアアアアアアアアアアアアアアアアアアアア山という所に住んでいました";
    eprintln!("[diag-budoux] === 長大塊ケース（ON: 縮退＋はみ出しなし） ===");
    dump_case(&mut rig, "longseg_on", longseg, true);
}
