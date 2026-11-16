//! 問い 4（7.2）: q1 の窓に WUC の合成の各段を通し、塗った色を画面の DC から撮る。
//! 段ごとに `[q4] step=<名前> ok|err=<HRESULT>` を 1 行出す。終了コードは README の ExitContract。
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
use windows_core::{Error, HRESULT, Interface, Result};

const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);
const SIZE: i32 = 256;
/// 塗る色（不透明のマゼンタ）。撮った絵の真ん中がこの色かを見る。
const COLOR: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 1.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// 段の結果を 1 行出す。失敗は tracing::error! にも段の名前と HRESULT を出す。
fn step<T>(name: &str, r: Result<T>) -> std::result::Result<T, ExitCode> {
    match r {
        Ok(v) => {
            println!("[q4] step={name} ok");
            Ok(v)
        }
        Err(e) => {
            let hr = e.code().0 as u32;
            println!("[q4] step={name} err=0x{hr:08X}");
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

fn main() -> ExitCode {
    tracing_subscriber::fmt().init();
    run().unwrap_or_else(|code| code)
}

fn run() -> std::result::Result<ExitCode, ExitCode> {
    let exit_ms = std::env::var("AREKA_APP_SMOKE_EXIT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30_000);
    // 認識しない exe は 200% で窓が論理の座標・画面の DC が物理の座標になり、撮りの矩形がずれる（7.2 の 1 回目の走り）。
    step(
        "dpi_awareness",
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }.ok(),
    )?;
    // 置き場の決まり: 書く先は作業木の target\pilot-windows-rs-next\out\q4（exe は …\debug\q4-demo.exe）。
    let out_dir = step(
        "out_dir",
        std::env::current_exe()
            .ok()
            .and_then(|exe| Some(exe.parent()?.parent()?.join("out").join("q4")))
            .ok_or_else(|| Error::new(E_FAIL, "current_exe has no target dir"))
            .and_then(|d| {
                std::fs::create_dir_all(&d).map_err(|e| Error::new(E_FAIL, e.to_string()))?;
                Ok(d)
            }),
    )?;

    // 窓が外から閉じられたかを state に持つ（終了コード 3）。
    let window = step(
        "window",
        Window::new_ex(
            WindowType::TopLevel,
            WS_EX_NOREDIRECTIONBITMAP as u32,
            Cell::new(false),
            |closed, msg| {
                if msg.msg == 0x0010 {
                    // WM_CLOSE
                    closed.set(true);
                }
                None
            },
        )
        .map_err(|_| Error::new(E_FAIL, "Window::new_ex")),
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
            SIZE,
            SIZE,
            (SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOACTIVATE) as u32,
        )
        .ok()
    })?;

    // DQTAT_COM_ASTA: COM も DispatcherQueue に初期化させる。
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
                Width: SIZE as f32,
                Height: SIZE as f32,
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
            visual.SetSize(windows_numerics::Vector2 {
                x: SIZE as f32,
                y: SIZE as f32,
            })?;
            target.SetRoot(&visual)
        })(),
    )?;
    step(
        "paint",
        surface
            .cast::<ICompositionDrawingSurfaceInterop>()
            .and_then(|i| {
                let (dc, _offset) = i.begin_draw(None)?;
                unsafe { dc.Clear(Some(&COLOR)) };
                i.end_draw()
            }),
    )?;

    // DispatcherQueue と DWM に合成させるため、メッセージを回しながら待つ。
    let start = Instant::now();
    let capture_at = Duration::from_millis(exit_ms.min(2_000) / 2);
    let path = out_dir.join("q4-demo.png");
    let mut captured = false;
    while start.elapsed() < Duration::from_millis(exit_ms) {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE as u32) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if window.state().get() {
            println!("[q4] closed from outside");
            return Ok(ExitCode::from(3));
        }
        if !captured && start.elapsed() >= capture_at {
            captured = true;
            step("capture", q4_wuc::capture::capture_window_png(hwnd, &path))?;
            println!("[q4] capture={}", path.display());
            step("check_color", check_color(&path))?;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    println!("[q4] done exit_ms={exit_ms}");
    Ok(ExitCode::SUCCESS)
}

/// ハードウェアの D3D11（BGRA）→ DXGI → D2D の工場 → D2D の装置。
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
    let d3d: ID3D11Device = d3d.ok_or_else(|| Error::new(E_FAIL, "no D3D11 device"))?;
    let dxgi: IDXGIDevice = d3d.cast()?;
    let mut raw = core::ptr::null_mut();
    let factory = unsafe {
        D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            &ID2D1Factory1::IID,
            None,
            &mut raw,
        )
        .ok()?;
        ID2D1Factory1::from_raw(raw)
    };
    unsafe { factory.CreateDevice(&dxgi) }
}

/// 撮った絵の 4 隅（4 px 内側）と真ん中の画素が、どれも塗った色（RGB 各 ±8）かを見る。
fn check_color(path: &std::path::Path) -> Result<()> {
    let io = |e: &dyn std::fmt::Display| Error::new(E_FAIL, format!("png: {e}"));
    let file = std::fs::File::open(path).map_err(|e| io(&e))?;
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .map_err(|e| io(&e))?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buf).map_err(|e| io(&e))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let want = [COLOR.r, COLOR.g, COLOR.b].map(|c| (c * 255.0) as i32);
    let points = [
        (4, 4),
        (w - 5, 4),
        (4, h - 5),
        (w - 5, h - 5),
        (w / 2, h / 2),
    ];
    let mut all = true;
    for (x, y) in points {
        let i = (y * w + x) * 4;
        let px = &buf[i..i + 3];
        let ok = px.iter().zip(want).all(|(&p, w)| (p as i32 - w).abs() <= 8);
        println!(
            "[q4] pixel x={x} y={y} rgb={},{},{} {}",
            px[0],
            px[1],
            px[2],
            if ok { "ok" } else { "mismatch" }
        );
        all &= ok;
    }
    if all {
        Ok(())
    } else {
        Err(Error::new(
            E_FAIL,
            "captured pixels are not the painted color",
        ))
    }
}
