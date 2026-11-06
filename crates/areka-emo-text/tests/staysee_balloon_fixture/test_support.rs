//! テーマ間で共有する小ヘルパと、保管フォルダの期待値。
//!
//! 断りの無い要件の番号と design.md の参照は spec `areka-P0-default-balloon-bundle` のもの（別の spec のものはその場で名を書く）。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **2.2**／**5.4**。根の取り方は
//! `areka-P0-default-balloon-nar-fold` 要件 **4.1**〜**4.3**）。
//!
//! 複数のテーマが同じ値・同じ読み方を使う項目はここへ**集約**する（複製すると、片方だけ
//! 直したときに本文の同一性が黙って壊れる）。テーマ 1 つでしか使わない項目はそのテーマの
//! ファイルに置く。

use std::path::PathBuf;
use std::sync::LazyLock;

use areka_emo_text::actor::ResolvedBalloonText;
use areka_parsers::balloon::BalloonModel;
use sample_ghost_kit::SampleRoot;

use crate::support::fixture::{merged_model, read_balloon_file};

/// 既定バルーンの検体。プロセス寿命で保持する（`SampleRoot` は破棄で複製の木を消すため）。
static STAYSEE: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire("StayseeBalloon").expect("StayseeBalloon は登記済みの検体")
});

/// 既定バルーンのフォルダ `<根>/balloon/StayseeBalloon/`。
pub(crate) fn staysee_root() -> PathBuf {
    STAYSEE.folder().to_path_buf()
}

// ── 保管フォルダの期待値（要件 2.2・5.4）──────────────────────────────────────

/// areka が枠として焼き込む 6 枚の原寸（IHDR 由来・image px）。
///
/// `balloons*` が本体側（scope 0）・`balloonk*` が相方側（scope 1）の系列で、
/// 面 0／1 と面 2／3 で高さが違う。この値は文字描画範囲（`validrect` の負値解決）の
/// 基準そのものなので、差し替わったら領域の期待値の意味が変わる。
///
/// 実ファイルの IHDR との一致は [`super::assets`] が固定する。他のテーマはこの表を
/// 「保管した画像の原寸」として引く。
pub(crate) const EXPECTED_FRAME_SIZES: [(&str, u32, u32); 6] = [
    ("balloons0.png", 335, 205),
    ("balloons1.png", 335, 205),
    ("balloons2.png", 335, 395),
    ("balloons3.png", 335, 395),
    ("balloonk0.png", 335, 135),
    ("balloonk1.png", 335, 135),
];

/// [`EXPECTED_FRAME_SIZES`] から 1 枚の原寸を引く。表に無い名前は名指しで落とす。
pub(crate) fn expected_frame_size(file_name: &str) -> (u32, u32) {
    EXPECTED_FRAME_SIZES
        .iter()
        .find(|(name, _, _)| *name == file_name)
        .map(|(_, w, h)| (*w, *h))
        .unwrap_or_else(|| {
            panic!(
                "枠画像 `{file_name}` の原寸が期待値の表に無い（表に在るのは {} 枚: {:?}）",
                EXPECTED_FRAME_SIZES.len(),
                EXPECTED_FRAME_SIZES
                    .iter()
                    .map(|(n, _, _)| *n)
                    .collect::<Vec<_>>()
            )
        })
}

// ── 定義ファイルの読み口（要件 3.3）──────────────────────────────────────────

/// 検体のテキストファイルを本番と同じ規約（既定 Ansi・ファイル内の `charset` 宣言優先）で
/// 復号する。StayseeBalloon は `charset,Shift_JIS` を宣言しているのでその宣言が効く。読みの
/// 失敗と空は共有の部品 [`read_balloon_file`] が名指しで落とす。
pub(crate) fn read_decoded(name: &str) -> String {
    read_balloon_file(&staysee_root(), name)
}

/// `descript.txt` 単層（面別上書き層なし）から本番と同じ経路でモデルを組む。
///
/// StayseeBalloon は面別上書き層（`balloons0s.txt` 等）を持たないので第 2 引数は `None`。
pub(crate) fn staysee_model() -> BalloonModel {
    merged_model(&staysee_root(), None)
}

// ── 領域と書体の入口（要件 3.3・3.5・設計 C2 の E 行／F 行）────────────────────
//
// 検体の描画範囲・書体・文字の寸法は複数のテーマが引く。ここに集約し、テーマ側で
// 解き直さない（複製すると、片方だけ直したときに前提の同一性が黙って壊れる）。
// 値そのものは `research.md` §8.2 の実測・導出であり、宣言からの計算結果である
// 4 辺は「実測へ合わせて緩める」対象ではない（設計 Error Handling）。

/// 各 scope の面 0 として焼き込まれる枠画像の名前（原寸の引き先・共有の部品の定数）。
///
/// 原寸そのものは [`EXPECTED_FRAME_SIZES`]（実 PNG の IHDR と突合済み）から引くので、
/// ここで二重に綴らない。
pub(crate) use crate::support::fixture::SCOPE_FACE0_FILES;

/// 描画範囲の左辺（`validrect.left,22` の素通し・image px）。
pub(crate) const EXPECTED_LEFT: f32 = 22.0;
/// 描画範囲の上辺（`validrect.top,20` の素通し・image px）。
pub(crate) const EXPECTED_TOP: f32 = 20.0;
/// 描画範囲の右辺（`validrect.right,-26` → 幅 335 − 26・image px）。両 scope とも同値。
pub(crate) const EXPECTED_RIGHT: f32 = 309.0;

/// scope ごとの描画範囲の下辺（`validrect.bottom,-47` → 高さ − 47・image px）。
///
/// 本体側は 205 − 47 ＝ 158・相方側は 135 − 47 ＝ 88。下辺だけが scope で違う。
const EXPECTED_BOTTOM: [(u32, f32); 2] = [(0, 158.0), (1, 88.0)];

/// 解決される文字の高さ（`font.height,12` の素通し・image px）。
pub(crate) const EXPECTED_FONT_HEIGHT: f32 = 12.0;
/// 半角 1 文字の送り幅（`ＭＳ ゴシック` は半角 0.5em ＝ 12 × 0.5・image px）。
pub(crate) const EXPECTED_ADVANCE_HALF: f32 = 6.0;
/// 全角 1 文字の送り幅（`ＭＳ ゴシック` は全角 1em ＝ 12 × 1.0・image px）。
pub(crate) const EXPECTED_ADVANCE_FULL: f32 = 12.0;
/// 行ボックスの丈（`ＭＳ ゴシック` は `ascent + descent` がちょうど 1em・image px）。
pub(crate) const EXPECTED_LINE_BOX: f32 = 12.0;
/// 行送り（正典式 `font.height + 行間 2` ＝ 12 + 2・image px）。
pub(crate) const EXPECTED_LINE_PITCH: f32 = 14.0;

/// scope の面 0 の原寸（image px）。[`EXPECTED_FRAME_SIZES`] から引く。
pub(crate) fn scope_image_size(scope: u32) -> (u32, u32) {
    let (_, file) = SCOPE_FACE0_FILES
        .iter()
        .find(|(s, _)| *s == scope)
        .unwrap_or_else(|| panic!("scope {scope} の面 0 の枠画像が表に無い"));
    expected_frame_size(file)
}

/// scope ごとの描画範囲の下辺を引く。
pub(crate) fn expected_bottom(scope: u32) -> f32 {
    EXPECTED_BOTTOM
        .iter()
        .find(|(s, _)| *s == scope)
        .map(|(_, b)| *b)
        .unwrap_or_else(|| panic!("scope {scope} の下辺の期待値が表に無い"))
}

/// 検体の当該 scope を本番の描画入口と同じ関数で解く。
pub(crate) fn resolve_staysee(scope: u32) -> ResolvedBalloonText {
    ResolvedBalloonText::resolve(&staysee_model(), scope_image_size(scope))
}
