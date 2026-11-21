use super::{
    DPI, Entity, GhostWindows, PresentCommand, RecordedCall, SpineHarness, settle_bounded,
};

// 観測の補助（不透明の数・variant 名・DPI の差し替え）。

/// BGRA 密配列（`stride=width*4`）のうち α バイト（各 4 バイト画素の index 3）が非 0 の画素数を数える。
///
/// 「非全透明（初期面が実描画された）」の述語＝`opaque_count > 0`。`read_back` は
/// premultiplied B8G8R8A8 を密（RowPitch 除去済み）で返すため、単純な 4 バイト刻みで α を見る。
pub(super) fn opaque_count(bgra: &[u8]) -> usize {
    bgra.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// 起動の系列が終わった（最後の basewareversion が記録された）。OnBoot が台詞を返すと 6 呼び出し・返さないと 5 呼び出し。
pub(super) fn boot_finished(calls: &[RecordedCall]) -> bool {
    calls
        .iter()
        .any(|c| matches!(c, RecordedCall::Notify { id, .. } if id == "basewareversion"))
}

/// 受信済みの表示指令の残りを [`settle_bounded`] で汲み切って `buf` へ足す。
///
/// 期待の件数に届いた所で汲むのを止めると、その後に届く余分な指令を見ずに「ちょうど N 件」が空振りする。
/// 打ち切りは反復回数ではなく壁時計の最小持続と連続空観測で決まり、時刻は進めない（回収だけ）。
pub(super) fn drain_settled(harness: &mut SpineHarness, buf: &mut Vec<PresentCommand>) {
    settle_bounded(|| {
        let got = harness.wiring.drain_received();
        let n = got.len();
        buf.extend(got);
        n
    });
}

/// `PresentCommand` の variant 名（`PresentCommand` は `reply` を含み `Debug` 非実装ゆえ診断表示用）。
pub(super) fn variant_name(cmd: &PresentCommand) -> &'static str {
    match cmd {
        PresentCommand::ShowSurface { .. } => "ShowSurface",
        PresentCommand::Hide { .. } => "Hide",
        PresentCommand::InvalidateCache { .. } => "InvalidateCache",
        PresentCommand::ReplaceTarget { .. } => "ReplaceTarget",
        PresentCommand::StageAck { .. } => "StageAck",
        PresentCommand::MoveShift { .. } => "MoveShift",
        _ => "<unknown>",
    }
}

/// バルーン窓の `DPI` を**現在値と必ず異なる値**へ差し替え、その新 DPI を返す。
///
/// 窓の `DPI` は wintf の `on_window_add` フックが `GetDpiForSystem()` で事前初期化するため
/// **実行機依存**である（96 の機械もあれば 120/144/192 の機械もある）。固定値を書き込むと
/// 「たまたま同値＝k 不変」で檻が空虚化するため、現在値を読んでから別の値を選ぶ。
pub(super) fn bump_balloon_window_dpi(harness: &mut SpineHarness, scope: usize) -> u16 {
    let window = harness
        .world
        .resource::<GhostWindows>()
        .balloon_window(scope)
        .expect("当該 scope の balloon 窓");
    bump_window_dpi(harness, window)
}

/// キャラ（シェル）窓の `DPI` を**現在値と必ず異なる値**へ差し替え、その新 DPI を返す
/// （[`bump_balloon_window_dpi`] のシェル版・非空虚性の担保は同一）。
///
/// SERIKO ループが載るのは**シェル**の表示スロットゆえ、ループ継続の檻はシェル窓側の DPI を動かさなければ
/// 空虚になる。
pub(super) fn bump_char_window_dpi(harness: &mut SpineHarness, scope: usize) -> u16 {
    let window = harness
        .world
        .resource::<GhostWindows>()
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
