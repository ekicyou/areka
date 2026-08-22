use super::{DPI, Entity, GhostWindows, PresentCommand, SpineHarness};

// ===========================================================================
// task 6.2 spine 観測ケース（S1 boot→表示／S3 `\b` 配送／S4 `\b` なし完走）
//
// 6.1 の `SpineHarness` の上に構築する。実 sink 経路の末端（`EmoPresenter::apply` の実描画→
// `read_back`）まで観測境界を延ばす（R8.2）。sleep 不使用・注入 Tick と有界 drain のみ（R8.3）・
// headless GPU（WARP・MTA・R8.4）・x64 完結（R8.6）。
// ===========================================================================

/// BGRA 密配列（`stride=width*4`）のうち α バイト（各 4 バイト画素の index 3）が非 0 の画素数を数える。
///
/// 「非全透明（初期面が実描画された）」の R8.5 述語＝`opaque_count > 0`。`read_back` は
/// premultiplied B8G8R8A8 を密（RowPitch 除去済み）で返すため、単純な 4 バイト刻みで α を見る。
pub(super) fn opaque_count(bgra: &[u8]) -> usize {
    bgra.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// `PresentCommand` の variant 名（`PresentCommand` は `reply` を含み `Debug` 非実装ゆえ診断表示用）。
pub(super) fn variant_name(cmd: &PresentCommand) -> &'static str {
    match cmd {
        PresentCommand::ShowSurface { .. } => "ShowSurface",
        PresentCommand::Hide { .. } => "Hide",
        PresentCommand::InvalidateCache { .. } => "InvalidateCache",
        _ => "<unknown>",
    }
}

// ===========================================================================
// 文字層 k 追従の実経路観測（emo-dpi-scaling task 7.2・D11-3/D11-4・R8.1/8.5/8.6）
//
// `run_text_scale_phase` は「presenter の現適用 k と文字層 binding の k が食い違っていれば
// 組み直す」フェーズである。判定の権威（同値 k／未登録の no-op）は emo-text 側（task 7.1）に
// あり、GPU 不要の判断分岐は frame.rs in-crate が持つ。ここでしか観測できないのは
// **実 attach で `text_slot_view` が `Some` になっている本番経路**——すなわち「実際に文字層が
// 新 k へ組み直されたか」そのものであり、以下 2 ケースがそれを実 GPU・実 fixture で固定する。
// ===========================================================================

/// バルーン窓の `DPI` を**現在値と必ず異なる値**へ差し替え、その新 DPI を返す。
///
/// 窓の `DPI` は wintf の `on_window_add` フックが `GetDpiForSystem()` で事前初期化するため
/// **実行機依存**である（96 の機械もあれば 120/144/192 の機械もある）。固定値を書き込むと
/// 「たまたま同値＝k 不変」で檻が空虚化するため、現在値を読んでから別の値を選ぶ。
pub(super) fn bump_balloon_window_dpi(harness: &mut SpineHarness, scope: usize) -> u16 {
    let ghost_windows = harness
        .world
        .get_resource::<GhostWindows>()
        .expect("spawn_ghost_windows が GhostWindows を挿入済み")
        .clone();
    let window = ghost_windows
        .balloon_window(scope)
        .expect("当該 scope の balloon 窓");
    bump_window_dpi(harness, window)
}

/// キャラ（シェル）窓の `DPI` を**現在値と必ず異なる値**へ差し替え、その新 DPI を返す
/// （[`bump_balloon_window_dpi`] のシェル版・非空虚性の担保は同一）。
///
/// SERIKO ループが載るのは**シェル**の表示スロットゆえ、ループ継続の檻（要件 4.3 の
/// 「進行中挙動の喪失なし」）はシェル窓側の DPI を動かさなければ空虚になる。
pub(super) fn bump_char_window_dpi(harness: &mut SpineHarness, scope: usize) -> u16 {
    let ghost_windows = harness
        .world
        .get_resource::<GhostWindows>()
        .expect("spawn_ghost_windows が GhostWindows を挿入済み")
        .clone();
    let window = ghost_windows
        .char_window(scope)
        .expect("当該 scope の char 窓");
    bump_window_dpi(harness, window)
}

/// 窓 entity の `DPI` を現在値と必ず異なる実機水準（96/192）へ差し替え、その新 DPI を返す。
fn bump_window_dpi(harness: &mut SpineHarness, window: Entity) -> u16 {
    let current = harness.world.get::<DPI>(window).map(|d| d.dpi_x);
    // 実機 DPI 水準のどちらか（96=100% / 192=200%）で、現在値と必ず異なる方を選ぶ。
    let next = if current == Some(192) { 96 } else { 192 };
    harness
        .world
        .entity_mut(window)
        .insert(DPI::from_dpi(next, next));
    next
}
