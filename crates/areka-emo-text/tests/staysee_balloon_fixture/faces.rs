//! 面の系列解決と、その間に出る記録を固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.4**／**3.8**・設計 **C2** の C 行）。

use areka_emo_present::balloon::{ChainTier, ResolvedFace};
use log_capture_kit::CapturedEvent;

use crate::support::fixture::{assert_balloon_loading_records, load_face0_with_records};

use super::test_support::{EXPECTED_FACE_COUNT, staysee_root};

/// 本体側（scope 0）で解決される面の並び `(面 id, 採用接頭辞, 実ファイル名, 採用段)`。
///
/// 本体側は連鎖の最終受け皿そのものなので、どの面も縮退ではなく自身の候補
/// （[`ChainTier::Own`]）から採られる。
const EXPECTED_SCOPE0_FACES: [(u32, &str, &str, ChainTier); EXPECTED_FACE_COUNT] = [
    (0, "balloons", "balloons0.png", ChainTier::Own),
    (1, "balloons", "balloons1.png", ChainTier::Own),
    (2, "balloons", "balloons2.png", ChainTier::Own),
    (3, "balloons", "balloons3.png", ChainTier::Own),
];

/// 相方側（scope 1）で解決される面の並び。
///
/// StayseeBalloon は相方側の枠を 2 枚（`balloonk0`／`balloonk1`）しか持たないので、
/// 面 2・3 は面 id 単位で本体側の系列（[`ChainTier::Default`]）へ落ちる。系列全体が
/// 本体側へ切り替わるのではなく、欠けた面だけが落ちることをこの並びが固定する。
const EXPECTED_SCOPE1_FACES: [(u32, &str, &str, ChainTier); EXPECTED_FACE_COUNT] = [
    (0, "balloonk", "balloonk0.png", ChainTier::Own),
    (1, "balloonk", "balloonk1.png", ChainTier::Own),
    (2, "balloons", "balloons2.png", ChainTier::Default),
    (3, "balloons", "balloons3.png", ChainTier::Default),
];

/// 相方側で本体側の系列へ縮退する面（`warn!` の欄 `surface_id`・`prefix`）。
///
/// 本体側は縮退の概念を持たないので空、相方側はちょうどこの 2 件である。件数で固定する
/// のは、保管フォルダの面が増減したらそれ自体が上流の取り直しの合図（要件 2.2）だからである。
const EXPECTED_SCOPE1_FALLBACK: [(u32, &str); 2] = [(2, "balloons"), (3, "balloons")];

/// 解決の 2 関数（`resolve_balloon_faces` と `load_scope_balloon_model`）だけを**同じ捕捉窓**で
/// 走らせ、採用面列とその間の記録を返す（焼き込み側の記録は本テーマの主張の対象外）。
fn resolve_faces_with_records(scope: u32) -> (Vec<ResolvedFace>, Vec<CapturedEvent>) {
    let (faces, _model, events) = load_face0_with_records(&staysee_root(), scope);
    (faces, events)
}

/// 採用面の並び（面 id・接頭辞・ファイル名・採用段）を期待と突き合わせる。
fn assert_face_series(
    scope: u32,
    faces: &[ResolvedFace],
    expected: &[(u32, &str, &str, ChainTier)],
) {
    let actual: Vec<(u32, &str, &str, ChainTier)> = faces
        .iter()
        .map(|f| {
            (
                f.surface_id,
                f.prefix.as_str(),
                f.file_name.as_str(),
                f.tier,
            )
        })
        .collect();
    assert_eq!(
        actual.len(),
        expected.len(),
        "scope {scope}: 解決された面の本数が期待 {} に対し実測 {}（実測の並び: {:?}）",
        expected.len(),
        actual.len(),
        actual
    );
    assert_eq!(
        actual,
        expected.to_vec(),
        "scope {scope}: 面の並び（面 id・採用接頭辞・実ファイル名・採用段）が期待と違う"
    );
}

/// 本体側と相方側で面の系列が解決され、その間の記録が期待どおりである（要件 3.4・3.8）。
///
/// 相方側は枠を 2 枚しか持たないので面 2・3 が本体側の系列へ落ち、その 2 面についてだけ
/// 縮退の記録が出る。本体側は落ちる先を持たないので記録は出ない。この非対称が、記録の
/// 件数の主張が恒真でないこと（＝捕捉が本当に働いていること）の証拠を兼ねている。
#[test]
fn face_series_and_records_are_pinned_for_both_scopes() {
    let (faces0, events0) = resolve_faces_with_records(0);
    assert_face_series(0, &faces0, &EXPECTED_SCOPE0_FACES);
    assert_balloon_loading_records(0, &events0, &[]);

    let (faces1, events1) = resolve_faces_with_records(1);
    assert_face_series(1, &faces1, &EXPECTED_SCOPE1_FACES);
    assert_balloon_loading_records(1, &events1, &EXPECTED_SCOPE1_FALLBACK);
}
