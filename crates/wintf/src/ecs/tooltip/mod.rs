//! ツールチップ（マウスを置いた所に出る短い説明）。

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "OS の境界（os.rs）から使い始める")
)]
mod geometry;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "公開の口（system.rs）から使い始める")
)]
mod ranges;
