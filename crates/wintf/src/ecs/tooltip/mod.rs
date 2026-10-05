//! ツールチップ（マウスを置いた所に出る短い説明）。

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "OS の境界（os.rs）が使うが、os.rs は公開の口（system.rs）から使い始める"
    )
)]
mod geometry;
#[expect(
    dead_code,
    reason = "公開の口（system.rs）から使い始める。テストも純粋な部分しか使わない"
)]
mod os;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "公開の口（system.rs）から使い始める")
)]
mod ranges;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "画面更新ごとの判定（system.rs）から使い始める")
)]
mod turn;
