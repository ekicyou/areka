//! テーマ間で共有する小ヘルパと、保管フォルダの期待値。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **2.2**／**3.2**／**5.4**・設計 **C2**）。
//!
//! 複数のテーマが同じ値・同じ読み方を使う項目はここへ**集約**する（複製すると、片方だけ
//! 直したときに本文の同一性が黙って壊れる）。テーマ 1 つでしか使わない項目はそのテーマの
//! ファイルに置く。

use std::path::PathBuf;

use areka_emo_present::balloon::{ResolvedFace, resolve_balloon_faces};
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};

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
