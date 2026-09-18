//! テーマ間で共有する小ヘルパと、保管フォルダの期待値。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **2.2**／**3.2**／**5.4**・設計 **C2**）。
//!
//! 複数のテーマが同じ値・同じ読み方を使う項目はここへ**集約**する（複製すると、片方だけ
//! 直したときに本文の同一性が黙って壊れる）。テーマ 1 つでしか使わない項目はそのテーマの
//! ファイルに置く。

use std::path::PathBuf;

use areka_emo_present::balloon::{ResolvedFace, resolve_balloon_faces};
use areka_emo_text::actor::ResolvedBalloonText;
use areka_emo_text::draw::DWriteMetrics;
use areka_emo_text::state::TextLayerConfig;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};
use windows::Win32::Graphics::DirectWrite::{DWRITE_FACTORY_TYPE_SHARED, IDWriteFactory2};
use wintf::com::dwrite::dwrite_create_factory;

/// [`crate::STAYSEE_BALLOON_DIR`] を実体化する。
///
/// 検体パスの綴りを持つのは親ファイルの定数だけなので、ここでは `super::` で引くに留める。
pub(crate) fn staysee_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(super::STAYSEE_BALLOON_DIR)
}

// ── 保管フォルダの期待値（要件 2.2・5.4）──────────────────────────────────────

/// 上流 `ponapalt/StayseeBalloon` `fe1b02f3` の配布物に含まれる全 29 ファイル。
///
/// areka が読まないファイル（`thumbnail.pnr`・`online*`・`marker.png`・`sstp.png`・
/// `balloonc*`・`arrow*`・`install.txt`）も削らず、告知ファイルを足しもしない
/// （要件 2.2・5.4）。並びは `verification/provenance.md` §3 のハッシュ一覧と同じ昇順。
pub(crate) const EXPECTED_FILE_NAMES: [&str; 29] = [
    "LICENSE",
    "arrow0.png",
    "arrow1.png",
    "balloonc0.png",
    "balloonc1.png",
    "balloonc2.png",
    "balloonc3.png",
    "balloonc4.png",
    "balloonk0.png",
    "balloonk1.png",
    "balloons0.png",
    "balloons1.png",
    "balloons2.png",
    "balloons3.png",
    "descript.txt",
    "install.txt",
    "marker.png",
    "online0.png",
    "online1.png",
    "online2.png",
    "online3.png",
    "online4.png",
    "online5.png",
    "online6.png",
    "online7.png",
    "online8.png",
    "readme.txt",
    "sstp.png",
    "thumbnail.pnr",
];

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

// ── 面の系列解決（要件 3.4）──────────────────────────────────────────────────

/// どちらの scope でも解決される面の本数（面 0〜3）。
pub(crate) const EXPECTED_FACE_COUNT: usize = 4;

/// 当該 scope の面の系列を本番と同じ公開 API で解決する（記録の捕捉は呼び出し側の仕事）。
///
/// 失敗と面 0 の不在は名指しで落とす（「解けなかったから対象 0 件で緑」を作らない）。
pub(crate) fn resolve_faces(scope: u32) -> Vec<ResolvedFace> {
    let faces = resolve_balloon_faces(&staysee_root(), scope).unwrap_or_else(|e| {
        panic!("scope {scope} の面の系列解決が失敗した（面 0 が解決できたことが前提）: {e}")
    });
    assert!(
        faces.iter().any(|f| f.surface_id == 0),
        "scope {scope}: 解決結果に面 0 が無い（実測 {} 面: {:?}）",
        faces.len(),
        faces.iter().map(|f| f.surface_id).collect::<Vec<_>>()
    );
    faces
}

// ── 定義ファイルの読み口（要件 3.3）──────────────────────────────────────────

/// 検体のテキストファイルを本番と同じ規約で復号する。
///
/// 本番経路 `areka_emo_present::balloon::load_scope_balloon_model` は
/// `decode(&bytes, DefaultEncoding::Ansi)`（**既定 Ansi・ファイル内の `charset` 宣言優先**）で
/// 読む。StayseeBalloon は `charset,Shift_JIS` を宣言しているのでその宣言が効く。
///
/// **読み込み失敗は明示的に panic する**。読めなかったときに「対象 0 件だから緑」になる形を
/// 作ってはならない。
pub(crate) fn read_decoded(name: &str) -> String {
    let path = staysee_root().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "既定バルーンの定義 {} の読取に失敗した（本テストはこのファイルの実在が前提）: {e}",
            path.display()
        )
    });
    assert!(
        !bytes.is_empty(),
        "既定バルーンの定義 {} が空である（空ファイルでは読み取り結果が主張と無関係になる）",
        path.display()
    );
    decode(&bytes, DefaultEncoding::Ansi)
}

/// `descript.txt` 単層（面別上書き層なし）から本番と同じ経路でモデルを組む。
///
/// StayseeBalloon は面別上書き層（`balloons0s.txt` 等）を持たないので第 2 引数は `None`。
pub(crate) fn staysee_model() -> BalloonModel {
    parse_str(&read_decoded("descript.txt"), None)
}

// ── 文字の計測の入口（要件 1.3・3.5）────────────────────────────────────────

/// DirectWrite の factory（文字の寸法を測るのに要るのはこれだけ——実 GPU も実窓も要らない）。
///
/// 同 crate の既存の統合テスト（`line_pitch_readback_test.rs`・`kero_menu_capacity_test.rs`）と
/// 同じ作り方である。生成失敗は明示的に落とす。
pub(crate) fn dwrite_factory() -> IDWriteFactory2 {
    dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)
        .expect("DirectWrite factory を生成できる（文字の寸法の観測はこれが前提）")
}

// ── 領域と書体の入口（要件 3.3・3.5・設計 C2 の E 行／F 行）────────────────────
//
// 検体の描画範囲・書体・文字の寸法は複数のテーマが引く。ここに集約し、テーマ側で
// 解き直さない（複製すると、片方だけ直したときに前提の同一性が黙って壊れる）。
// 値そのものは `research.md` §8.2 の実測・導出であり、宣言からの計算結果である
// 4 辺は「実測へ合わせて緩める」対象ではない（設計 Error Handling）。

/// 各 scope の面 0 として焼き込まれる枠画像の名前（原寸の引き先）。
///
/// 原寸そのものは [`EXPECTED_FRAME_SIZES`]（実 PNG の IHDR と突合済み）から引くので、
/// ここで二重に綴らない。
pub(crate) const SCOPE_FACE0_FILES: [(u32, &str); 2] = [(0, "balloons0.png"), (1, "balloonk0.png")];

/// 描画範囲の左辺（`validrect.left,22` の素通し・image px）。
pub(crate) const EXPECTED_LEFT: f32 = 22.0;
/// 描画範囲の上辺（`validrect.top,20` の素通し・image px）。
pub(crate) const EXPECTED_TOP: f32 = 20.0;
/// 描画範囲の右辺（`validrect.right,-26` → 幅 335 − 26・image px）。両 scope とも同値。
pub(crate) const EXPECTED_RIGHT: f32 = 309.0;

/// scope ごとの描画範囲の下辺（`validrect.bottom,-47` → 高さ − 47・image px）。
///
/// 本体側は 205 − 47 ＝ 158・相方側は 135 − 47 ＝ 88。下辺だけが scope で違う。
pub(crate) const EXPECTED_BOTTOM: [(u32, f32); 2] = [(0, 158.0), (1, 88.0)];

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

/// 検体の書体で実測の文字の寸法を組む（GPU 不要——計測に要るのは factory だけ）。
pub(crate) fn staysee_metrics(resolved: &ResolvedBalloonText) -> DWriteMetrics {
    DWriteMetrics::new(
        &dwrite_factory(),
        &resolved.font,
        resolved.mode,
        &TextLayerConfig::default(),
    )
    .expect("既定バルーンの書体で文字の寸法を組める")
}
