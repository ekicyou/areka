use super::{BalloonModel, DefaultEncoding, PathBuf, decode, error, parse_str};
use sample_ghost_kit::SampleRoot;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// 定数・fixture パス解決
// ---------------------------------------------------------------------------

/// \0（sakura）バルーン窓の初期位置（物理 px・スクリーン座標）。
pub(super) const SAKURA_POS: (i32, i32) = (320, 160);
/// \1（kero）バルーン窓の縦間隔（sakura 窓の直下に置く・物理 px）。
pub(super) const KERO_GAP_Y: i32 = 32;
/// シナリオ全体の watchdog（talk 起点相対秒・超過は FAIL）。
pub(super) const WATCHDOG_SECS: f64 = 30.0;

/// emo2 検体。段 ③ で `Drop` が複製を消すため、一時値にせずプロセス寿命で保持する。
static EMO2: LazyLock<SampleRoot> =
    LazyLock::new(|| SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体"));

/// 共有 fixture（emo2 が同時にインストールするバルーン）のディレクトリを窓口から得る
/// （自分でパスを継ぎ足さない・R11.7 共有 fixture 非改変）。
pub(super) fn shared_balloon_dir() -> PathBuf {
    EMO2.balloon("emo2-kakukaku")
        .expect("emo2 の同梱バルーン")
        .to_path_buf()
}

/// example ローカル fixture 変種（縦書き観測用・task 9.1 成果）。
fn vertical_variant_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures/emo2-vertical")
}

/// balloon descript ファイルを読み、charset 宣言に従いデコードする
/// （parser-foundation の decode 経路——tests/vertical_fixture_test.rs と同じ読み込み規約）。
fn read_decoded(path: &std::path::Path) -> Option<String> {
    match std::fs::read(path) {
        Ok(bytes) => Some(decode(&bytes, DefaultEncoding::Utf8)),
        Err(e) => {
            error!(path = %path.display(), error = %e, "balloon descript の読取に失敗");
            None
        }
    }
}

/// balloon model（descript 基層＋`balloons0s.txt` 画像別上書き層の 2 層マージ）を解決する。
///
/// `--vertical` 時は parse 入力だけを fixture 変種へ差し替える（枠画像は共有 fixture 継続）。
pub(super) fn load_balloon_model(vertical: bool) -> Option<BalloonModel> {
    let dir = if vertical {
        vertical_variant_dir()
    } else {
        shared_balloon_dir()
    };
    let base = read_decoded(&dir.join("descript.txt"))?;
    let overlay = read_decoded(&dir.join("balloons0s.txt"))?;
    Some(parse_str(&base, Some(&overlay)))
}
