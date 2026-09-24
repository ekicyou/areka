//! 画面取り込み（別スレッド・design §Capture）。
//!
//! OS の Desktop Duplication で画面更新を 1 回ずつ受け取り、提示時刻（QPC）以前に終わった最新の
//! tick の記録と組にして、その窓の矩形を切り出して絵を判別し、`FrameRecord` を共有の記録へ足す。
//! OS を読むだけで World には触らない。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use tracing::{debug, error, info, trace};
use windows::Win32::Foundation::{HMODULE, RECT};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_FLAG, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_MODE_ROTATION_IDENTITY, DXGI_MODE_ROTATION_UNSPECIFIED,
    DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO, IDXGIDevice, IDXGIOutput1,
    IDXGIOutputDuplication, IDXGIResource,
};
use windows_core::Interface;

use crate::observe::{
    Class, FrameRecord, PictureClass, Shared, Signature, TickRecord, Why, classify_picture,
};

/// `AcquireNextFrame` の待ち（ms）。timeout は素通りして停止の旗を見る。
const ACQUIRE_TIMEOUT_MS: u32 = 16;
/// 複製の作り直しに失敗したときの間隔。
const RETRY: Duration = Duration::from_millis(250);
/// 作り直しの失敗を `error!` に出す間隔（回数・250 ms × 20 ＝約 5 秒）。
const RETRY_LOG_EVERY: u32 = 20;

pub struct Capture {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Capture {
    /// 取り込みスレッドを起こす。初期化（device・複製）の失敗はここで `Err` を返す。
    pub fn start(shared: Arc<Mutex<Shared>>, sigs: Arc<[Signature; 2]>) -> Result<Capture, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("pilot-capture".into())
            .spawn(move || {
                let dup = match Dup::new() {
                    Ok(d) => d,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                let _ = tx.send(Ok(()));
                capture_loop(dup, &shared, &sigs, &flag);
            })
            .map_err(|e| format!("取り込みスレッドを起こせない: {e}"))?;
        match rx.recv() {
            Ok(Ok(())) => Ok(Capture {
                stop,
                thread: Some(thread),
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(format!("画面取り込みの初期化に失敗: {e}"))
            }
            Err(_) => {
                let _ = thread.join();
                Err("取り込みスレッドが初期化の結果を返さずに終わった（panic）".into())
            }
        }
    }

    /// 停止の旗を立てて、スレッドの終わりを待つ。
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take()
            && t.join().is_err()
        {
            error!("取り込みスレッドが panic で終わっていた — フレームの記録は途中まで");
        }
    }
}

/// 取り込みスレッドが持つ D3D11 と複製。
struct Dup {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    output: IDXGIOutput1,
    /// 出力の原点と範囲（仮想デスクトップ座標）。
    desktop: RECT,
    /// 喪失の後の作り直しの間だけ `None`。
    dupl: Option<IDXGIOutputDuplication>,
    /// 窓の矩形の大きさの staging（大きさが変わったら作り直す）。
    staging: Option<(ID3D11Texture2D, u32, u32)>,
}

impl Dup {
    fn new() -> Result<Dup, String> {
        let (mut device, mut context) = (None, None);
        // SAFETY: 出力先はこの関数のローカル変数。
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
        }
        .map_err(|e| format!("D3D11CreateDevice: {e}"))?;
        let (Some(device), Some(context)) = (device, context) else {
            return Err("D3D11CreateDevice が device／context を返さない".into());
        };
        // SAFETY: 生きている device からの COM の辿り。
        let output: IDXGIOutput1 = unsafe {
            device
                .cast::<IDXGIDevice>()
                .and_then(|d| d.GetAdapter())
                .and_then(|a| a.EnumOutputs(0))
                .and_then(|o| o.cast())
        }
        .map_err(|e| format!("IDXGIDevice → IDXGIAdapter → EnumOutputs(0) → IDXGIOutput1: {e}"))?;
        // SAFETY: 同上。
        let desc = unsafe { output.GetDesc() }.map_err(|e| format!("出力の GetDesc: {e}"))?;
        let desktop = desc.DesktopCoordinates;
        let dupl = duplicate(&output, &device)?;
        info!(?desktop, "画面取り込みを開始（出力 0）");
        Ok(Dup {
            device,
            context,
            output,
            desktop,
            dupl: Some(dupl),
            staging: None,
        })
    }

    /// 取得中のフレームから窓の矩形を切り出して絵を判別する。失敗は `Err`（理由）。
    fn classify(
        &mut self,
        res: Option<IDXGIResource>,
        sig: &Signature,
        rect: &RECT,
        b: &D3D11_BOX,
    ) -> Result<(PictureClass, f32, f32, f32, f32), String> {
        let (w, h) = (b.right - b.left, b.bottom - b.top);
        if w == 0 || h == 0 {
            // どの点も矩形の外 → 分母 0 → 測れない（Ambiguous）に落ちる。
            return Ok(classify_picture(sig, rect, &[], 0));
        }
        let tex: ID3D11Texture2D = res
            .ok_or("AcquireNextFrame が画面の資源を返さない")?
            .cast()
            .map_err(|e| format!("画面の資源 → ID3D11Texture2D: {e}"))?;
        if self.staging.as_ref().is_none_or(|s| (s.1, s.2) != (w, h)) {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: w,
                Height: h,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut t = None;
            // SAFETY: desc と出力先はローカル変数。
            unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut t)) }
                .map_err(|e| format!("staging（{w}x{h}）の CreateTexture2D: {e}"))?;
            let t = t.ok_or("CreateTexture2D が staging を返さない")?;
            debug!(w, h, "staging を作り直した");
            self.staging = Some((t, w, h));
        }
        let staging = &self.staging.as_ref().expect("直上で作った").0;
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: 箱は出力の内側（`output_box` で確かめた）。Map は Unmap まで読むだけ。
        unsafe {
            self.context
                .CopySubresourceRegion(staging, 0, 0, 0, 0, &tex, 0, Some(b));
            self.context
                .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut m))
                .map_err(|e| format!("staging の Map: {e}"))?;
        }
        let pitch = m.RowPitch as usize;
        // SAFETY: Map した staging は h 行・各行 pitch バイト（最終行は w*4 まで読む）。
        let bytes = unsafe {
            std::slice::from_raw_parts(
                m.pData as *const u8,
                pitch * (h as usize - 1) + w as usize * 4,
            )
        };
        let r = classify_picture(sig, rect, bytes, pitch);
        // SAFETY: 直上で Map したもの。bytes はここから先で使わない。
        unsafe { self.context.Unmap(staging, 0) };
        Ok(r)
    }
}

/// 複製を作る。BGRA8・回転なし以外は座標や色が合わないので `Err`。
fn duplicate(
    output: &IDXGIOutput1,
    device: &ID3D11Device,
) -> Result<IDXGIOutputDuplication, String> {
    // SAFETY: 生きている出力と device。
    let dupl =
        unsafe { output.DuplicateOutput(device) }.map_err(|e| format!("DuplicateOutput: {e}"))?;
    // SAFETY: 同上。
    let d = unsafe { dupl.GetDesc() };
    if d.ModeDesc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
        return Err(format!(
            "画面の形式が BGRA8 でない: {:?}",
            d.ModeDesc.Format
        ));
    }
    if d.Rotation != DXGI_MODE_ROTATION_IDENTITY && d.Rotation != DXGI_MODE_ROTATION_UNSPECIFIED {
        return Err(format!("出力が回転している: {:?}", d.Rotation));
    }
    Ok(dupl)
}

/// 提示時刻 `present` 以前に終わった最新の tick（`ticks` は `ended_qpc` の昇順）。
fn latest_tick_at(ticks: &[TickRecord], present: i64) -> Option<TickRecord> {
    let i = ticks.partition_point(|t| t.ended_qpc <= present);
    i.checked_sub(1).map(|i| ticks[i])
}

/// 窓の矩形（仮想デスクトップ座標）→ 出力の画像の中の箱。出力からはみ出すなら `None`。
fn output_box(rect: &RECT, desktop: &RECT) -> Option<D3D11_BOX> {
    let inside = rect.left >= desktop.left
        && rect.top >= desktop.top
        && rect.right <= desktop.right
        && rect.bottom <= desktop.bottom
        && rect.left <= rect.right
        && rect.top <= rect.bottom;
    inside.then(|| D3D11_BOX {
        left: (rect.left - desktop.left) as u32,
        top: (rect.top - desktop.top) as u32,
        front: 0,
        right: (rect.right - desktop.left) as u32,
        bottom: (rect.bottom - desktop.top) as u32,
        back: 1,
    })
}

fn capture_loop(mut d: Dup, shared: &Mutex<Shared>, sigs: &[Signature; 2], stop: &AtomicBool) {
    // 複製を失ってから最初に取れたフレームまで「測れない（CaptureLost）」。
    let mut lost = false;
    let mut last_picture = None;
    // 作り直しの連続失敗の回数（最初と約 5 秒ごとだけ error! に出す）。
    let mut failures = 0u32;
    while !stop.load(Ordering::Relaxed) {
        let Some(dupl) = d.dupl.clone() else {
            match duplicate(&d.output, &d.device) {
                Ok(n) => {
                    info!(failures, "画面の複製を作り直した");
                    d.dupl = Some(n);
                    failures = 0;
                }
                Err(e) => {
                    failures += 1;
                    if failures % RETRY_LOG_EVERY == 1 {
                        error!(error = %e, failures, "画面の複製を作り直せない — 作り直すまで取り込めない");
                    }
                    std::thread::sleep(RETRY);
                }
            }
            continue;
        };
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut res = None;
        // SAFETY: 出力先はローカル変数。成功したら必ず ReleaseFrame する。
        match unsafe { dupl.AcquireNextFrame(ACQUIRE_TIMEOUT_MS, &mut info, &mut res) } {
            Ok(()) => {}
            Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => continue,
            Err(e) => {
                // DXGI_ERROR_ACCESS_LOST（モード変更・セキュアデスクトップ等）とその他の失敗。
                error!(error = %e, "AcquireNextFrame が失敗 — 複製を捨てて作り直す（直後のフレームは CaptureLost）");
                lost = true;
                d.dupl = None;
                continue;
            }
        }
        let frame = on_frame(&mut d, res, &info, shared, sigs, &mut lost);
        // SAFETY: 直上の AcquireNextFrame が成功している。
        if let Err(e) = unsafe { dupl.ReleaseFrame() } {
            error!(error = %e, "ReleaseFrame が失敗");
        }
        let Some(frame) = frame else { continue };
        let n = {
            let mut s = shared.lock().unwrap_or_else(PoisonError::into_inner);
            s.frames.push(frame);
            s.frames.len()
        };
        trace!(n, ?frame, "フレームの記録");
        if n % 30 == 1 || last_picture != Some(frame.picture) {
            debug!(
                n,
                tick = ?frame.tick,
                present_qpc = frame.present_qpc,
                accumulated = frame.accumulated,
                picture = ?frame.picture,
                a = frame.a,
                b = frame.b,
                ab_p = frame.ab_p,
                ab_q = frame.ab_q,
                "取り込んだフレーム（30 件ごとと絵が変わったとき）"
            );
        }
        last_picture = Some(frame.picture);
    }
    debug!("取り込みスレッドを止めた");
}

/// 取得中の 1 フレームを記録にする。記録しない（マウスだけ・突き合わせる tick が無い）なら `None`。
fn on_frame(
    d: &mut Dup,
    res: Option<IDXGIResource>,
    info: &DXGI_OUTDUPL_FRAME_INFO,
    shared: &Mutex<Shared>,
    sigs: &[Signature; 2],
    lost: &mut bool,
) -> Option<FrameRecord> {
    let present = info.LastPresentTime;
    if present == 0 {
        trace!("絵の更新が無い（マウスだけ）— 飛ばす");
        return None;
    }
    let tick = {
        let s = shared.lock().unwrap_or_else(PoisonError::into_inner);
        latest_tick_at(&s.ticks, present)
    };
    let Some(tick) = tick else {
        debug!(present, "提示時刻以前に終わった tick が無い — 捨てる");
        return None;
    };
    let nan = f32::NAN;
    let unmeasurable = |why| (Class::Unmeasurable(why), nan, nan, nan, nan);
    let (picture, a, b, ab_p, ab_q) = if std::mem::take(lost) {
        unmeasurable(Why::CaptureLost)
    } else {
        match output_box(&tick.rect, &d.desktop) {
            None => {
                debug!(rect = ?tick.rect, desktop = ?d.desktop, "窓の矩形が出力の外");
                unmeasurable(Why::OutsideOutput)
            }
            Some(bx) => d
                .classify(res, &sigs[tick.pair], &tick.rect, &bx)
                .unwrap_or_else(|e| {
                    error!(error = %e, tick = tick.tick, "フレームの切り出しに失敗 — 測れない（CaptureLost）");
                    unmeasurable(Why::CaptureLost)
                }),
        }
    };
    Some(FrameRecord {
        present_qpc: present,
        accumulated: info.AccumulatedFrames,
        tick: Some(tick.tick),
        picture,
        a,
        b,
        ab_p,
        ab_q,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn frame_pairs_with_latest_tick_ended_at_or_before_present() {
        let tick = |tick, ended_qpc| TickRecord {
            tick,
            pair: 0,
            ended_qpc,
            rect: RECT::default(),
            hit: Class::P,
            h_p: 0.0,
            h_q: 0.0,
            h_both: 0.0,
            covered: false,
            k_ok: true,
            slot_hit: false,
        };
        let ticks = [tick(1, 100), tick(2, 200), tick(3, 300)];
        let at = |p| latest_tick_at(&ticks, p).map(|t| t.tick);
        assert_eq!(at(99), None);
        assert_eq!(at(100), Some(1));
        assert_eq!(at(299), Some(2));
        assert_eq!(at(300), Some(3));
        assert_eq!(at(10_000), Some(3));
        assert_eq!(latest_tick_at(&[], 100).map(|t| t.tick), None);
    }

    #[test]
    fn window_rect_maps_to_output_box_or_outside() {
        let desk = r(-1920, 0, 0, 1080); // 主画面の左にある出力
        let b = output_box(&r(-1760, 160, -1425, 365), &desk).unwrap();
        assert_eq!(
            (b.left, b.top, b.right, b.bottom, b.front, b.back),
            (160, 160, 495, 365, 0, 1)
        );
        assert!(output_box(&r(-10, 0, 10, 20), &desk).is_none()); // 右へはみ出す
        assert!(output_box(&r(-1921, 0, -1900, 20), &desk).is_none()); // 左へはみ出す
        assert!(output_box(&r(-100, 1070, -50, 1081), &desk).is_none()); // 下へはみ出す
        assert!(output_box(&r(-1920, 0, 0, 1080), &desk).is_some()); // 出力いっぱい
    }
}
