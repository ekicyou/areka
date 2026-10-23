//! `surface-compare <SSP のフォルダ> <areka のフォルダ>`（spec: areka-P0-offline-surface-dump 要件 13）。
//! 判断はすべてライブラリの [`surface_compare::parse_args`]・[`surface_compare::report`] にあり、ここは出すだけ。

fn main() {
    let (ssp, areka) = match surface_compare::parse_args(std::env::args_os().skip(1)) {
        Ok(dirs) => dirs,
        Err(code) => {
            eprintln!("usage: surface-compare <ssp-dir> <areka-dir>");
            std::process::exit(code);
        }
    };
    let (lines, code) = surface_compare::report(surface_compare::compare_dirs(&ssp, &areka));
    for line in &lines {
        if code == 0 {
            println!("{line}");
        } else {
            eprintln!("{line}");
        }
    }
    std::process::exit(code);
}
