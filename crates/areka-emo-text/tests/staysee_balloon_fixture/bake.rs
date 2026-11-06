//! 枠画像の焼き込みと、半透明の保持を固定する。
//!
//! 出典 spec: `areka-P0-default-balloon-bundle`（要件 **3.4**・設計 **C2** の D 行）。
//!
//! ## なぜ半透明の画素を数えるのか
//!
//! StayseeBalloon は**半透明を前提にした枠**であり、縁の柔らかさは α が 0 でも 255 でもない
//! 画素で表現されている。焼き込みの途中で α が 2 値へ潰れると、枠の縁が硬いギザギザになる
//! ——けれども「面 0 が引ける」「原寸が合う」だけを見ていると、その崩れは素通りする。
//! そこで焼き上がりの画素を実際に走査し、完全な不透明でも完全な透明でもない画素が
//! 残っていることを直接固定する。
//!
//! ## 走査が本物であることの示し方
//!
//! 「半透明の画素が 1 つ以上ある」は、走査範囲が空だったり行送りを間違えて同じ画素を
//! 何度も数えていても真になりうる。そこで
//!
//! 1. 走査した画素の**総数**を矩形の面積と突き合わせてから、
//! 2. 完全な不透明・完全な透明・半透明の 3 つに分類した内訳の合計が総数と一致し、
//! 3. **3 種すべてが 1 つ以上実在する**
//!
//! ことまで固定する。⑶ は、走査が矩形の一部分（たとえば縁だけ・中身だけ）に偏っていたら
//! どれかが 0 になって赤くなる、という意味での対照になっている。
//!
//! ## 「焼き込みの失敗の記録が 0 件」の判定方法
//!
//! 記録の捕捉窓は面の系列解決の 2 関数に限る（設計 DD3）ので、ここでは窓を張らない。
//! 焼き込みは失敗したときに必ず `error!` を出してから `Err` を返す実装なので、
//! **`Ok` が返ったこと**をもって「失敗の記録を出す経路を踏んでいない」と判定する
//! （設計 System Flows の定め）。

use areka_emo_atlas::WicDecoderArm;
use areka_emo_present::balloon::{build_balloon_target_from_faces, load_balloon_use_self_alpha};

use crate::support::fixture::{
    SCOPE_FACE0_FILES, assert_three_alpha_kinds, census_face0, init_com_for_this_thread,
};

use super::test_support::{EXPECTED_FACE_COUNT, expected_frame_size, resolve_faces, staysee_root};

/// 両 scope の面を起動時と同じ透過設定で焼き込み、面 0 の原寸・割り当て・半透明を固定する。
///
/// 焼き込みは `resolve_balloon_faces` が返した系列をそのまま渡す経路（起動時の資産構築が
/// 使うのと同じ公開 API）で行う。透過の宣言も起動時と同じ入口 `load_balloon_use_self_alpha` で
/// スコープのループの前に 1 回読んで渡す（検体の descript.txt の宣言を本物の経路で読む）。
#[test]
fn baked_face0_keeps_partial_alpha_for_both_scopes() {
    init_com_for_this_thread();
    let root = staysee_root();
    let decoder = WicDecoderArm::new().expect("WIC ファクトリを作れない（COM の初期化を確認）");

    let use_self_alpha = load_balloon_use_self_alpha(&root);
    for (scope, face0_file) in SCOPE_FACE0_FILES {
        let faces = resolve_faces(scope);
        assert_eq!(
            faces.len(),
            EXPECTED_FACE_COUNT,
            "scope {scope}: 焼き込みに渡す面の本数が期待 {EXPECTED_FACE_COUNT} に対し実測 {}",
            faces.len()
        );
        let resolved_face0 = faces
            .iter()
            .find(|f| f.surface_id == 0)
            .expect("面 0 の実在は resolve_faces が確かめている");
        assert_eq!(
            resolved_face0.file_name, face0_file,
            "scope {scope}: 面 0 に採られた実ファイル名が期待 `{face0_file}` と違う（焼き上がりを引く鍵そのものなので先に固定する）"
        );

        // 失敗すれば `error!` を出してから `Err` を返す実装なので、`Ok` が「失敗の記録 0 件」の判定である。
        let (_world, table) = build_balloon_target_from_faces(
            &root,
            &decoder,
            &faces,
            use_self_alpha,
        )
        .unwrap_or_else(|e| {
            panic!(
                "scope {scope}: {} 面の焼き込みが失敗した（失敗の記録が出る経路を踏んだ）: {e}",
                faces.len()
            )
        });
        assert_eq!(
            table.len(),
            faces.len(),
            "scope {scope}: 焼き上がりの索引の項目数が渡した面の本数 {} と違う（実測 {}）",
            faces.len(),
            table.len()
        );

        // 原寸は保管した PNG の IHDR と突合済みの期待値から引く。
        let census = census_face0(&table, scope, face0_file, expected_frame_size(face0_file));
        assert_three_alpha_kinds(scope, face0_file, &census);
    }
}
