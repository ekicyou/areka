use super::{BalloonModel, DefaultEncoding, PathBuf, decode, error, parse_str};
use sample_ghost_kit::SampleRoot;
use std::sync::LazyLock;

/// 挨拶の 3 行（fixture スクリプト boot.pasta「起動朝」由来・文字の層の例とタイプライターの例が共に引く）。
///
/// 1 行目（6 グリフ・typewriter 進行の観測対象）。
pub(super) const LINE1: &str = "おっはよー！";
/// 2 行目（10 グリフ・改行観測。横書き 1 行に収まり、縦書きでは複数列へ折返す）。
pub(super) const LINE2: &str = "めっちゃええ朝やん！";
/// 3 行目（8 グリフ・横書き行容量 4 行のうちの 3 行目——旧行送りでは最終行だった）。
pub(super) const LINE3: &str = "今日もいくでー！";

// ---------------------------------------------------------------------------
// fixture パス解決（文字の層の例とタイプライターの例が `#[path]` で共に引く）
// ---------------------------------------------------------------------------

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

/// example ローカル fixture 変種（縦書き観測用）。
fn vertical_variant_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures/emo2-vertical")
}

/// balloon descript ファイルを本番と同じ規約で読む（既定 Ansi・ファイル内の `charset` 宣言優先）。
///
/// 本番 `areka_emo_present::balloon::load_scope_balloon_model` と `tests/support/fixture.rs` の
/// `read_balloon_file` と同じ `decode(&bytes, DefaultEncoding::Ansi)`。1 文字でも違う規約で読むと
/// `charset` の行の無い層で例だけが本番と違う model を観測する。
fn read_decoded(path: &std::path::Path) -> Option<String> {
    match std::fs::read(path) {
        Ok(bytes) => Some(decode(&bytes, DefaultEncoding::Ansi)),
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
