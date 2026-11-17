// 画面の撮り（7.1）: 窓を作らず、デスクトップの窓の矩形を撮って PNG の大きさが矩形と合うかを見る。
use bind_next::Windows::Win32::{GetDesktopWindow, GetWindowRect, RECT};

#[test]
fn desktop_rect_to_png() {
    let hwnd = unsafe { GetDesktopWindow() };
    let mut r = RECT::default();
    assert!(unsafe { GetWindowRect(hwnd, &mut r) }.as_bool());
    // 置き場の決まり: 一時のファイルは作業木の target の下（CARGO_TARGET_TMPDIR）だけ。
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("q4-capture-test.png");
    q4_wuc::capture::capture_window_png(hwnd, &path).expect("capture_window_png");
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()));
    let info = decoder.read_info().unwrap().info().clone();
    // 画面の中身を残さない。
    std::fs::remove_file(&path).unwrap();
    assert_eq!(info.width as i32, r.right - r.left);
    assert_eq!(info.height as i32, r.bottom - r.top);
    assert_eq!(info.color_type, png::ColorType::Rgba);
}
