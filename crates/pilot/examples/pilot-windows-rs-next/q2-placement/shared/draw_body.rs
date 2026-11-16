// 3 案が #[path] で読む同じ描く本体（DirectWrite の文字を 1 行 Direct2D で描き、墨を数える）。
// 型の出どころは読み込む側（super）の `use` だけが決める。ここは案ごとに変えない。
use super::*;

/// 3 案で同じに描く 1 行
pub const TEXT: &str = "q2 placement 墨";

/// 受け取った文脈（描く先は wintf 役が付けた物）へ文字を 1 行描く
pub fn draw_line(context: &ID2D1DeviceContext, text: &str) -> Result<()> {
    let factory: IDWriteFactory =
        unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory::IID)?.cast()? };
    let format = unsafe {
        factory.CreateTextFormat(
            w!("Segoe UI").as_ptr(),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            20.0,
            w!("ja-jp").as_ptr(),
        )?
    };
    let wide: Vec<u16> = text.encode_utf16().collect();
    let size = unsafe { context.GetSize() };
    let layout = unsafe {
        factory.CreateTextLayout(
            wide.as_ptr(),
            wide.len() as u32,
            &format,
            size.width,
            size.height,
        )?
    };
    let black = D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let brush = unsafe { context.CreateSolidColorBrush(&black, None)? };
    unsafe {
        context.BeginDraw();
        context.Clear(Some(&D2D1_COLOR_F::default()));
        context.DrawTextLayout(
            windows_numerics::Vector2 { x: 4.0, y: 4.0 },
            &layout,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_NONE,
        );
        context.EndDraw(None, None).ok()
    }
}

/// premultiplied BGRA の墨（α>0）の画素を数える
pub fn count_ink(bgra: &[u8]) -> usize {
    bgra.as_chunks::<4>().0.iter().filter(|p| p[3] > 0).count()
}

#[cfg(test)]
mod tests {
    #[test]
    fn count_ink_counts_alpha_only() {
        let bgra = [0, 0, 0, 0, 0, 0, 0, 1, 255, 255, 255, 0, 0, 0, 0, 255];
        assert_eq!(super::count_ink(&bgra), 2);
    }
}
