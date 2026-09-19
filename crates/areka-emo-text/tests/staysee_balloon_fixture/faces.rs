//! 面の系列解決と、その間に出る記録を固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.4**／**3.8**・設計 **C2** の C 行）。

use areka_emo_present::balloon::{ChainTier, ResolvedFace, load_scope_balloon_model};
use log_capture_kit::{CapturedEvent, LineFormat, capture, format_line};

use super::test_support::{
    EXPECTED_FACE_COUNT, EXPECTED_FILE_NAMES, EXPECTED_FRAME_SIZES, resolve_faces, staysee_root,
};

/// 解決の記録の宛先（`resolve_balloon_faces`／`load_scope_balloon_model` の発行元モジュール）。
///
/// 捕捉した記録がこの宛先から出たことまで確かめるのは、別の層が出した無関係な記録を
/// 数え込んで「件数が合っている」ように見えるのを防ぐためである。
const BALLOON_LOG_TARGET: &str = "areka_emo_present::balloon";

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

/// 面ではない資産の全数（保管 29 本 − 枠 6 枚）。
const NON_FACE_ASSET_COUNT: usize = EXPECTED_FILE_NAMES.len() - EXPECTED_FRAME_SIZES.len();

/// 解決の 2 関数を**同じ捕捉窓**で走らせ、採用面列とその間の記録を返す。
///
/// 窓に入れるのは `resolve_balloon_faces` と `load_scope_balloon_model` の 2 つだけである
/// （焼き込み側の記録は本テーマの主張の対象外）。
fn resolve_faces_with_records(scope: u32) -> (Vec<ResolvedFace>, Vec<CapturedEvent>) {
    let root = staysee_root();
    capture(|| {
        let faces = resolve_faces(scope);
        let face0 = faces
            .iter()
            .find(|f| f.surface_id == 0)
            .expect("面 0 の実在は resolve_faces が確かめている")
            .clone();
        let _model = load_scope_balloon_model(&root, scope, &face0);
        faces
    })
}

/// 記録を 1 件 1 行の読める形へ落とす（assert の失敗文言に実測を添えるため）。
fn record_lines(events: &[CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| format_line(e, LineFormat::LevelTargetFields))
        .collect()
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
    assert_eq!(
        non_face.len(),
        NON_FACE_ASSET_COUNT,
        "面ではない資産の本数が期待 {NON_FACE_ASSET_COUNT} に対し実測 {}（保管 {} 本 − 枠 {} 枚）",
        non_face.len(),
        EXPECTED_FILE_NAMES.len(),
        EXPECTED_FRAME_SIZES.len()
    );
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

/// 解決の窓で捕捉した記録を突き合わせる（失敗の記録 0 件・縮退の記録は期待どおりの欄で）。
///
/// 冒頭の 2 件の情報の記録が**対照**である。「失敗の記録が 0 件」は捕捉が空振りしていても
/// 真になるので、同じ窓・同じ宛先で必ず出る記録を先に数え、発行点が生きていたことを示す。
fn assert_resolution_records(
    scope: u32,
    events: &[CapturedEvent],
    expected_fallback: &[(u32, &str)],
) {
    let lines = record_lines(events);

    // 対照 ⑴: 解決の 2 関数の発行点が窓の中で本当に動いた（系列解決の完了＋2 層マージの確定）。
    let infos: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::INFO && e.target == BALLOON_LOG_TARGET)
        .collect();
    assert_eq!(
        infos.len(),
        2,
        "scope {scope}: 宛先 {BALLOON_LOG_TARGET} の情報の記録が期待 2 件に対し実測 {} 件。0 件なら捕捉が空振りしており、以下の「0 件であること」の主張は何も確かめない。捕捉した全 {} 件: {lines:?}",
        infos.len(),
        lines.len()
    );
    // 対照 ⑵: 2 件の内訳が別々の発行点である（欄で見分ける——本文の文字列には依存しない）。
    assert_eq!(
        infos.iter().filter(|e| e.field("chain").is_some()).count(),
        1,
        "scope {scope}: 系列解決の完了の記録（欄 `chain` を持つもの）が 1 件でない: {lines:?}"
    );
    assert_eq!(
        infos
            .iter()
            .filter(|e| e.field("validrect_left").is_some())
            .count(),
        1,
        "scope {scope}: 2 層マージの確定の記録（欄 `validrect_left` を持つもの）が 1 件でない: {lines:?}"
    );

    // 失敗の記録は両 scope とも 0 件（要件 3.8）。
    let errors: Vec<&String> = events
        .iter()
        .zip(&lines)
        .filter(|(e, _)| e.level == tracing::Level::ERROR)
        .map(|(_, l)| l)
        .collect();
    assert!(
        errors.is_empty(),
        "scope {scope}: 失敗の記録は 0 件であること。実測 {} 件: {errors:?}",
        errors.len()
    );

    // 縮退の記録は本体側 0 件・相方側ちょうど 2 件。欄（宛先・scope・面 id・接頭辞）まで固定する。
    let warns: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(
        warns.len(),
        expected_fallback.len(),
        "scope {scope}: 縮退の記録が期待 {} 件に対し実測 {} 件。捕捉した全 {} 件: {lines:?}",
        expected_fallback.len(),
        warns.len(),
        lines.len()
    );
    let actual: Vec<(String, String, String, String)> = warns
        .iter()
        .map(|e| {
            let field = |name: &str| e.field(name).unwrap_or("<欄なし>").to_string();
            (
                e.target.clone(),
                field("scope"),
                field("surface_id"),
                field("prefix"),
            )
        })
        .collect();
    let expected: Vec<(String, String, String, String)> = expected_fallback
        .iter()
        .map(|(surface_id, prefix)| {
            (
                BALLOON_LOG_TARGET.to_string(),
                scope.to_string(),
                surface_id.to_string(),
                (*prefix).to_string(),
            )
        })
        .collect();
    assert_eq!(
        actual, expected,
        "scope {scope}: 縮退の記録の欄（宛先・scope・surface_id・prefix）が期待と違う"
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
    assert_resolution_records(0, &events0, &[]);

    let (faces1, events1) = resolve_faces_with_records(1);
    assert_face_series(1, &faces1, &EXPECTED_SCOPE1_FACES);
    assert_non_face_assets_are_absent(1, &faces1);
    assert_resolution_records(1, &events1, &EXPECTED_SCOPE1_FALLBACK);
}
