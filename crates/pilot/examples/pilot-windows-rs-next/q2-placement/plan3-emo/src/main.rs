// 問い 2 の案 3 の確かめの bin: wintf 役が作った文脈を、受け渡しの手当て無し（同じ型）で
// emo-text 役へ渡して描き、墨を数える。
use q2_plan3_bind::E_FAIL;
use windows_core::{Error, Result};

const W: u32 = 256;
const H: u32 = 64;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(ink) => {
            println!("[q2-plan3] ink={ink}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[q2-plan3] error={e:?}");
            std::process::ExitCode::from(2)
        }
    }
}

fn run() -> Result<usize> {
    let target = q2_plan3_wintf::create_target(W, H)?;
    q2_plan3_emo::draw_line(&target.context, q2_plan3_emo::TEXT)?;
    let ink = q2_plan3_emo::count_ink(&q2_plan3_wintf::read_back(&target)?);
    if ink == 0 {
        return Err(Error::new(E_FAIL, "no ink pixels"));
    }
    Ok(ink)
}
