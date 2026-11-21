//! 問い 7:透過の枠なしの窓（q1 の写し）に q4 の合成を通し、描画面へ自前の
//! Direct2D で縦書きの文（カラー絵文字を含む）と選択肢 2 つを描いて撮る。その後、選択肢ごとに中心へ
//! `SendInput` で動かしてクリックし、`WM_LBUTTONDOWN` の位置を `windows-text` の `hit_test_point` に
//! 渡して `[q7] hit choice=<番号> target=<番号>` を出す。カーソルは元の位置へ戻す。
//! 終了コードは README の ExitContract（0＝一致・2＝初期化の失敗・3＝外から閉じられた・4＝食い違い）。
use std::cell::Cell;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use bind_next::Windows::Foundation::Size;
use bind_next::Windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
use bind_next::Windows::UI::Composition::Compositor;
use bind_next::Windows::Win32::*;
use q1::util::{Window, WindowType};
use q4_wuc::wuc::{
    CompositorDesktopInteropExt, CompositorInteropExt, DrawingSurfaceInteropExt,
    create_dispatcher_queue_controller,
};
use q7_vertical::choices::{Choice, byte_to_utf16, choice_at, choice_utf16_range};
use q7_vertical::new_side::{self, Features};
use q7_vertical::old_side::{FONT_HEIGHT, TEXT};
use windows_core::{Error, HRESULT, Interface, Result};
use windows_text::{TextLayout, Vector2};

const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);
/// 本文の 1 列＋選択肢 2 列（各列は 1 行＝右から左へ並ぶ）。
const CHOICE_TEXTS: [&str; 2] = ["🍣を食べる", "やめておく"];
const COLUMNS: i32 = 3;
/// 窓の縁から文字の配置の箱までの余白（px）。
const MARGIN: i32 = 16;
const WIDTH: i32 = COLUMNS * FONT_HEIGHT as i32 + 2 * MARGIN;
const HEIGHT: i32 = 12 * FONT_HEIGHT as i32 + 2 * MARGIN;
const WHITE: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
const BLACK: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};
/// 選択肢の色（撮った絵で選択肢が描かれたかを数える）。
const BLUE: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// 段の結果を 1 行出す。失敗は tracing::error! にも段の名前と HRESULT を出す。
fn step<T>(name: &str, r: Result<T>) -> std::result::Result<T, ExitCode> {
    match r {
        Ok(v) => {
            println!("[q7] step={name} ok");
            Ok(v)
        }
        Err(e) => {
            let hr = e.code().0 as u32;
            println!("[q7] step={name} err=0x{hr:08X}");
            tracing::error!(
                step = name,
                hresult = format!("0x{hr:08X}"),
                "{}",
                e.message()
            );
            Err(ExitCode::from(2))
        }
    }
}

fn fail(what: impl Into<String>) -> Error {
    Error::new(E_FAIL, what.into())
}

/// 窓の状態: 外から閉じられたか・最後の `WM_LBUTTONDOWN` の位置（窓の中の物理の画素）。
type State = (Cell<bool>, Cell<Option<(i32, i32)>>);

fn main() -> ExitCode {
    tracing_subscriber::fmt().init();
    run().unwrap_or_else(|code| code)
}

fn run() -> std::result::Result<ExitCode, ExitCode> {
    let exit_ms = std::env::var("AREKA_APP_SMOKE_EXIT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30_000);
    let start = Instant::now();
    let deadline = Duration::from_millis(exit_ms);
    // 認識しないと 200% の画面で窓は論理の座標・撮りと SendInput は物理の画素になり、当たりもずれる。
    step(
        "dpi_awareness",
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }.ok(),
    )?;
    // 置き場の決まり: 書く先は作業木の target\pilot-windows-rs-next\out\q7（exe は …\debug\vt-balloon.exe）。
    let out_dir = step(
        "out_dir",
        std::env::current_exe()
            .ok()
            .and_then(|exe| Some(exe.parent()?.parent()?.join("out").join("q7")))
            .ok_or_else(|| fail("current_exe has no target dir"))
            .and_then(|d| {
                std::fs::create_dir_all(&d).map_err(|e| fail(e.to_string()))?;
                Ok(d)
            }),
    )?;

    // 文と選択肢（選択肢の範囲は UTF-8 のバイト位置・choices の換算で UTF-16 へ）。
    let text = format!("{TEXT}\n{}\n{}", CHOICE_TEXTS[0], CHOICE_TEXTS[1]);
    let choices: Vec<Choice> = CHOICE_TEXTS
        .iter()
        .zip(1..)
        .map(|(c, number)| {
            let start = text.rfind(c).unwrap_or(0);
            Choice {
                number,
                bytes: start..start + c.len(),
            }
        })
        .collect();
    let layout = step(
        "layout",
        new_side::layout(&text, Features::Areka)
            .map(|mut l| {
                l.set_max_size(
                    (COLUMNS * FONT_HEIGHT as i32) as f32,
                    (HEIGHT - 2 * MARGIN) as f32,
                );
                l
            })
            .map_err(fail),
    )?;

    let window = step(
        "window",
        Window::new_ex(
            WindowType::TopLevel,
            (WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST) as u32,
            State::default(),
            |state, msg| {
                match msg.msg {
                    0x0010 => state.0.set(true), // WM_CLOSE
                    0x0201 => {
                        // WM_LBUTTONDOWN: lParam の下 16 bit が x・上 16 bit が y（符号つき・窓の中）。
                        let (x, y) = (msg.lparam as i16 as i32, (msg.lparam >> 16) as i16 as i32);
                        state.1.set(Some((x, y)));
                    }
                    _ => {}
                }
                None
            },
        )
        .map_err(|_| fail("Window::new_ex")),
    )?;
    // q1 の束縛と bind-next の束縛は HWND__ が別の型なので、ポインタの型だけ移す。
    let hwnd: HWND = window.hwnd().cast();
    step("popup", unsafe {
        SetWindowLongPtrW(hwnd, GWL_STYLE, WS_POPUP as isize);
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            100,
            100,
            WIDTH,
            HEIGHT,
            (SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOACTIVATE) as u32,
        )
        .ok()
    })?;

    let _dq = step(
        "dispatcher_queue",
        create_dispatcher_queue_controller(DQTAT_COM_ASTA),
    )?;
    let compositor = step("compositor", Compositor::new())?;
    let target = step(
        "desktop_window_target",
        compositor
            .cast::<ICompositorDesktopInterop>()
            .and_then(|i| i.create_desktop_window_target(hwnd, false)),
    )?;
    let d2d_device = step("d2d_device", create_d2d_device())?;
    let graphics = step(
        "graphics_device",
        compositor
            .cast::<ICompositorInterop>()
            .and_then(|i| i.create_graphics_device(&d2d_device)),
    )?;
    let surface = step(
        "drawing_surface",
        graphics.CreateDrawingSurface(
            Size {
                Width: WIDTH as f32,
                Height: HEIGHT as f32,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        ),
    )?;
    step(
        "sprite_visual",
        (|| {
            let brush = compositor.CreateSurfaceBrushWithSurface(&surface)?;
            let visual = compositor.CreateSpriteVisual()?;
            visual.SetBrush(&brush)?;
            visual.SetSize(Vector2::new(WIDTH as f32, HEIGHT as f32))?;
            target.SetRoot(&visual)
        })(),
    )?;
    step(
        "paint",
        surface
            .cast::<ICompositionDrawingSurfaceInterop>()
            .and_then(|i| paint(&i, &layout, &text, &choices)),
    )?;

    // DispatcherQueue と DWM に合成させるため、メッセージを回しながら待つ。
    let pump = |until: Duration| -> std::result::Result<(), ExitCode> {
        while start.elapsed() < until.min(deadline) {
            let mut msg = MSG::default();
            while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE as u32) }.as_bool() {
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            if window.state().0.get() {
                println!("[q7] closed from outside");
                return Err(ExitCode::from(3));
            }
            if window.state().1.get().is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        Ok(())
    };
    if let Err(code) = pump(Duration::from_millis(exit_ms.min(2_000) / 2)) {
        return Ok(code);
    }
    let path = out_dir.join("vt-balloon.png");
    step("capture", q4_wuc::capture::capture_window_png(hwnd, &path))?;
    println!("[q7] capture={}", path.display());
    step("check_pixels", check_pixels(&path))?;

    let mut window_rect = RECT::default();
    step(
        "window_rect",
        unsafe { GetWindowRect(hwnd, &mut window_rect) }
            .as_bool()
            .then_some(())
            .ok_or_else(|| fail("GetWindowRect")),
    )?;
    let mut original = POINT::default();
    step(
        "cursor_pos",
        unsafe { GetCursorPos(&mut original) }
            .as_bool()
            .then_some(())
            .ok_or_else(|| fail("GetCursorPos")),
    )?;
    // カーソルを取った後はどの終わり方でも 1 か所で戻すため、クリックの回しを閉包に入れる。
    let clicks = || -> std::result::Result<bool, ExitCode> {
        let mut all_match = true;
        for c in &choices {
            // 狙いの選択肢の中心（配置の箱の座標 → 窓の中 → 画面）。
            let r = layout.range_bounds(c.bytes.clone());
            let Some(r) = r.first() else {
                tracing::error!(choice = c.number, "range_bounds is empty");
                return Err(ExitCode::from(2));
            };
            let (cx, cy) = (
                MARGIN + ((r.left + r.right) / 2.0) as i32,
                MARGIN + ((r.top + r.bottom) / 2.0) as i32,
            );
            window.state().1.set(None);
            step(
                "send_input",
                click_at(window_rect.left + cx, window_rect.top + cy),
            )?;
            let wait = start.elapsed() + Duration::from_millis(2_000);
            pump(wait)?;
            let hit = window.state().1.get().and_then(|(x, y)| {
                let h =
                    layout.hit_test_point(Vector2::new((x - MARGIN) as f32, (y - MARGIN) as f32));
                println!(
                    "[q7] click x={x} y={y} aim x={cx} y={cy} range={:?} inside={}",
                    h.text_range, h.is_inside
                );
                h.is_inside
                    .then(|| choice_at(&text, &choices, byte_to_utf16(&text, h.text_range.start)))
                    .flatten()
            });
            let shown = hit.map_or("none".to_owned(), |n| n.to_string());
            println!("[q7] hit choice={shown} target={}", c.number);
            all_match &= hit == Some(c.number);
        }
        Ok(all_match)
    };
    let result = clicks();
    restore_cursor(original);
    Ok(match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(4),
        Err(code) => code,
    })
}

/// 白の背景（角丸）・黒の文・青の選択肢を描画面へ描く。DPI は 96 に固定し、配置の座標＝窓の画素にする。
fn paint(
    i: &ICompositionDrawingSurfaceInterop,
    layout: &TextLayout,
    text: &str,
    choices: &[Choice],
) -> Result<()> {
    let (dc, offset) = i.begin_draw(None)?;
    let (ox, oy) = (offset.x as f32, offset.y as f32);
    let raw: &IDWriteTextLayout = layout.as_interface();
    unsafe {
        dc.SetDpi(96.0, 96.0);
        dc.Clear(None);
        let white = dc.CreateSolidColorBrush(&WHITE, None)?;
        let black = dc.CreateSolidColorBrush(&BLACK, None)?;
        let blue = dc.CreateSolidColorBrush(&BLUE, None)?;
        for c in choices {
            let r = choice_utf16_range(text, c);
            raw.SetDrawingEffect(
                &blue,
                DWRITE_TEXT_RANGE {
                    startPosition: r.start as u32,
                    length: (r.end - r.start) as u32,
                },
            )
            .ok()?;
        }
        dc.FillRoundedRectangle(
            &D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: ox,
                    top: oy,
                    right: ox + WIDTH as f32,
                    bottom: oy + HEIGHT as f32,
                },
                radiusX: 6.0,
                radiusY: 6.0,
            },
            &white,
        );
        dc.DrawTextLayout(
            Vector2::new(ox + MARGIN as f32, oy + MARGIN as f32),
            raw,
            &black,
            D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
        );
    }
    i.end_draw()
}

/// 画面の点 (x, y)（物理の画素）へ `SendInput` で動かし、左ボタンを押して離す。
fn click_at(x: i32, y: i32) -> Result<()> {
    let (vx, vy, vw, vh) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    // 絶対の位置は仮想画面を 0..65535 に正規化する（切り上げで狙いの画素に落とす）。
    let norm = |p: i32, o: i32, len: i32| {
        ((((p - o) as i64) * 65536 + len as i64 - 1) / len as i64) as i32
    };
    let mouse = |flags: u32, dx: i32, dy: i32| INPUT {
        r#type: INPUT_MOUSE as u32,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let (nx, ny) = (norm(x, vx, vw), norm(y, vy, vh));
    let abs = (MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK) as u32;
    let mut inputs = [
        mouse(abs, nx, ny),
        mouse(MOUSEEVENTF_LEFTDOWN as u32 | abs, nx, ny),
        mouse(MOUSEEVENTF_LEFTUP as u32 | abs, nx, ny),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            core::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(Error::from_thread())
    }
}

/// カーソルを元の位置へ戻す（失敗は記録だけ・終了コードは当たりで決める）。
fn restore_cursor(p: POINT) {
    if !unsafe { SetCursorPos(p.x, p.y) }.as_bool() {
        tracing::error!(x = p.x, y = p.y, "SetCursorPos failed");
    } else {
        println!("[q7] cursor restored x={} y={}", p.x, p.y);
    }
}

/// ハードウェアの D3D11（BGRA）→ DXGI → D2D の工場（今の側と同じ multi-threaded）→ D2D の装置。
fn create_d2d_device() -> Result<ID2D1Device> {
    let mut d3d = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            core::ptr::null_mut(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT as u32,
            None,
            0,
            D3D11_SDK_VERSION as u32,
            Some(&mut d3d),
            None,
            None,
        )
        .ok()?
    };
    let d3d: ID3D11Device = d3d.ok_or_else(|| fail("no D3D11 device"))?;
    let dxgi: IDXGIDevice = d3d.cast()?;
    let mut raw = core::ptr::null_mut();
    let factory = unsafe {
        D2D1CreateFactory(
            D2D1_FACTORY_TYPE_MULTI_THREADED,
            &ID2D1Factory1::IID,
            None,
            &mut raw,
        )
        .ok()?;
        ID2D1Factory1::from_raw(raw)
    };
    unsafe { factory.CreateDevice(&dxgi) }
}

/// 撮った絵の 4 隅（角丸の内側＝8 px 内側）が白の背景で、絵文字の暖色と
/// 選択肢の青（B が高く R・G が低い）の画素が在るかを見る。
fn check_pixels(path: &std::path::Path) -> Result<()> {
    let io = |e: &dyn std::fmt::Display| fail(format!("png: {e}"));
    let file = std::fs::File::open(path).map_err(|e| io(&e))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| io(&e))?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buf).map_err(|e| io(&e))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let px = |x: usize, y: usize| -> [u8; 3] {
        let i = (y * w + x) * 4;
        [buf[i], buf[i + 1], buf[i + 2]]
    };
    let mut ok = true;
    for (x, y) in [(8, 8), (w - 9, 8), (8, h - 9), (w - 9, h - 9)] {
        let p = px(x, y);
        let white = p.iter().all(|&c| c >= 247);
        println!(
            "[q7] pixel x={x} y={y} rgb={},{},{} {}",
            p[0],
            p[1],
            p[2],
            if white { "ok" } else { "mismatch" }
        );
        ok &= white;
    }
    let all = (0..h).flat_map(|y| (0..w).map(move |x| (x, y)));
    let blue = all
        .clone()
        .filter(|&(x, y)| {
            let p = px(x, y);
            p[2] > 128 && p[0] < 96 && p[1] < 96
        })
        .count();
    // 寿司の絵文字の暖色（R が B より十分高い）。黒の字と青の選択肢のふちには出ない。
    let warm = all
        .filter(|&(x, y)| {
            let p = px(x, y);
            p[0] as i32 > p[2] as i32 + 48
        })
        .count();
    println!("[q7] pixels blue={blue} warm={warm}");
    ok &= blue > 0 && warm > 0;
    if ok {
        Ok(())
    } else {
        Err(fail("captured pixels do not show the balloon"))
    }
}
