//! 問い 7 の比べ: 同じ過程の中で今の側（`areka-emo-text`・`windows` 0.62）の 1 枚と
//! 新しい側（`windows-text`・0.101）の 2 枚（字形の指定なし・areka の 10 個）を別々の装置で描き、
//! 「同じ原点のまま」と「最も合う平行移動」の 2 通りで比べる。
//! 合格の閾値は置かない。窓は出さない（画面の外のハードウェアの GPU だけ）。
//! 終了コード（ExitContract）: 0 ＝ 出し終えた・2 ＝ 初期化（描く・書く）の失敗。

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use q7_vertical::diff::{Shot, compare, write_diff_png};
use q7_vertical::new_side::{self, Features};
use q7_vertical::old_side::{self, TEXT};

/// 段の失敗を段の名前つきで出し、終了コード 2 にする。
fn step<T, E: std::fmt::Display>(name: &str, r: Result<T, E>) -> Result<T, ExitCode> {
    r.map_err(|e| {
        eprintln!("[vt-compare] {name}: failed: {e}");
        ExitCode::from(2)
    })
}

/// premultiplied BGRA を白の上に重ねた RGB の PNG に書く（見る用・α は落とす）。
fn write_shot_png(s: &Shot, path: &Path) -> std::io::Result<()> {
    let rgb: Vec<u8> = s
        .bgra
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let bg = 255 - p[3];
            [p[2], p[1], p[0]].map(|c| c.saturating_add(bg))
        })
        .collect();
    let mut enc = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(path)?),
        s.width,
        s.height,
    );
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(std::io::Error::other)?;
    w.write_image_data(&rgb).map_err(std::io::Error::other)
}

fn main() -> ExitCode {
    run().unwrap_or_else(|code| code)
}

fn run() -> Result<ExitCode, ExitCode> {
    // 置き場の決まり: 書く先は作業木の target\pilot-windows-rs-next\out\q7（exe は …\debug\vt-compare.exe）。
    let out_dir: PathBuf = step(
        "out_dir",
        std::env::current_exe()
            .ok()
            .and_then(|exe| Some(exe.parent()?.parent()?.join("out").join("q7")))
            .ok_or("current_exe has no target dir")
            .and_then(|d| {
                std::fs::create_dir_all(&d).map_err(|_| "create_dir_all")?;
                Ok(d)
            }),
    )?;

    // 同じ過程・同じスレッドで MTA を始める（両側の draw も同じ呼び出しをし、S_FALSE は見ない）。
    // SAFETY: 予約の引数は None・COINIT_MULTITHREADED（0）。
    let _ = unsafe { bind_next::Windows::Win32::CoInitializeEx(None, 0) };

    // 両側とも呼ぶたびに自分の装置を作る（新旧の COM の型はつながない）。
    let old = step("old_side::draw", old_side::draw(TEXT))?;
    step(
        "write old",
        write_shot_png(&old, &out_dir.join("q7-old.png")),
    )?;

    let mut table = String::from(
        "| 新しい側の字形の指定 | 同じ原点: 違う画素 | 同じ原点: 最大の差 | 最良のずれ (dx, dy) | 最良のずれ: 違う画素 | 大きさの違い |\n\
         |---|---|---|---|---|---|\n",
    );
    let mut cells_lines = String::new();
    for (features, tag) in [(Features::None, "none"), (Features::Areka, "areka")] {
        let new = step(
            &format!("new_side::draw({features:?})"),
            new_side::draw(TEXT, features),
        )?;
        step(
            "write new",
            write_shot_png(&new, &out_dir.join(format!("q7-new-{tag}.png"))),
        )?;
        let r = compare(&old, &new);
        step(
            "write diff (same origin)",
            write_diff_png(
                &old,
                &new,
                (0, 0),
                &out_dir.join(format!("q7-diff-{tag}-origin.png")),
            ),
        )?;
        step(
            "write diff (best shift)",
            write_diff_png(
                &old,
                &new,
                r.best_shift,
                &out_dir.join(format!("q7-diff-{tag}-best.png")),
            ),
        )?;
        println!("[vt-compare] {tag}: {r:?}");
        // 同じ原点で違う画素を字の枠（縦に 20 px ずつ）ごとに数え、違いがどの字に在るかを見る。
        let cells: Vec<usize> = old
            .bgra
            .as_chunks::<4>()
            .0
            .chunks(old.width as usize)
            .zip(new.bgra.as_chunks::<4>().0.chunks(new.width as usize))
            .map(|(a, b)| a.iter().zip(b).filter(|(p, q)| p != q).count())
            .collect::<Vec<_>>()
            .chunks(old_side::FONT_HEIGHT as usize)
            .map(|c| c.iter().sum())
            .collect();
        cells_lines += &format!("- {features:?}: 字の枠ごとの違う画素（上から）{cells:?}\n");
        table += &format!(
            "| {features:?} | {} | {} | {:?} | {} | {} |\n",
            r.differing_pixels, r.max_delta, r.best_shift, r.differing_at_best, r.size_mismatch
        );
    }
    table += &format!("\n{cells_lines}");
    step(
        "write table",
        std::fs::write(out_dir.join("q7-compare.md"), &table),
    )?;
    print!("{table}");
    println!("[vt-compare] out={}", out_dir.display());
    Ok(ExitCode::SUCCESS)
}
