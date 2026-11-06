//! 面の系列解決と、その間に出る記録を固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.4**／**3.8**・設計 **C2** の C 行）。

use areka_emo_present::balloon::{ChainTier, ResolvedFace};
use log_capture_kit::CapturedEvent;

use crate::support::fixture::{assert_balloon_loading_records, load_face0_with_records};

use super::test_support::{
    EXPECTED_FACE_COUNT, EXPECTED_FILE_NAMES, EXPECTED_FRAME_SIZES, staysee_root,
};

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

/// 保管フォルダに在って areka が面として読まない資産の内訳（名前に含まれる字面 → 本数）。
///
/// 要件 3.8 が名指しする 6 種で、合計 19 本。残る 4 本（`LICENSE` と 3 つの `.txt`）を
/// 足した 23 本が「面ではない資産」の全数＝保管 29 本 − 枠 6 枚である。
const NON_FACE_ASSET_GROUPS: [(&str, usize); 6] = [
    ("balloonc", 5),
    ("arrow", 2),
    ("online", 9),
    ("marker.png", 1),
    ("sstp.png", 1),
    ("thumbnail.pnr", 1),
];

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

/// areka が面として読まない 23 本の資産が解決の列挙に 1 本も載らない（要件 3.8）。
///
/// 「載らない」は列挙が空でも真になるので、**先に列挙の母数を固定**してから主張する。
/// 除外側の 23 本も、要件 3.8 が名指しする 6 種の内訳で数を固定してから使う（保管フォルダ側
/// から資産が消えたら除外集合が黙って縮む形にしない）。
fn assert_non_face_assets_are_absent(scope: u32, faces: &[ResolvedFace]) {
    let frames: Vec<&str> = EXPECTED_FRAME_SIZES.iter().map(|(n, _, _)| *n).collect();
    let non_face: Vec<&str> = EXPECTED_FILE_NAMES
        .iter()
        .copied()
        .filter(|n| !frames.contains(n))
        .collect();
    for (needle, count) in NON_FACE_ASSET_GROUPS {
        let hit = non_face.iter().filter(|n| n.contains(needle)).count();
        assert_eq!(
            hit,
            count,
            "要件 3.8 が名指しする `{needle}` の本数が期待 {count} に対し実測 {hit}（除外対象 {}: {non_face:?}）",
            non_face.len()
        );
    }

    let listed: Vec<&str> = faces.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(
        listed.len(),
        EXPECTED_FACE_COUNT,
        "scope {scope}: 列挙の母数が期待 {EXPECTED_FACE_COUNT} 本に対し実測 {} 本（母数が 0 だと「載らない」の主張は何も確かめない）: {listed:?}",
        listed.len()
    );
    let leaked: Vec<&&str> = non_face.iter().filter(|n| listed.contains(n)).collect();
    assert!(
        leaked.is_empty(),
        "scope {scope}: areka が面として読まない資産が {} 件 {:?} 解決の列挙に載った（列挙 {} 本: {:?}）",
        leaked.len(),
        leaked,
        listed.len(),
        listed
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
    assert_non_face_assets_are_absent(0, &faces0);
    assert_balloon_loading_records(0, &events0, &[]);

    let (faces1, events1) = resolve_faces_with_records(1);
    assert_face_series(1, &faces1, &EXPECTED_SCOPE1_FACES);
    assert_non_face_assets_are_absent(1, &faces1);
    assert_balloon_loading_records(1, &events1, &EXPECTED_SCOPE1_FALLBACK);
}
