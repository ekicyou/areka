//! 面 0 の焼き込みと、半透明の保持を固定する。
//!
//! 出典 spec: `default-balloon-selfmade`（要件 **12.3**・**2.4**・**5.9**・設計「Testing Strategy」の
//! 「中身のテスト」の `bake.rs`）。
//!
//! ## なぜ半透明の画素を数えるのか
//!
//! `areka-balloon` は α 付きの PNG だけで縁の柔らかさを出す（`.pna` を持たない＝要件 2.4・
//! `use_self_alpha,1`＝要件 5.9）。焼き込みの途中で α が 2 値へ潰れても「面 0 が引ける」
//! 「原寸が合う」だけでは素通りするので、焼き上がりの画素を走査して半透明が残ることを直接見る。
//!
//! ## 走査が本物であることの示し方
//!
//! `staysee_balloon_fixture/bake.rs` と同じく、割り当ての矩形を元の画像の全体（左上 0,0・原寸）と突き合わせ、
//! 不透明・透明・半透明の 3 種がすべて 1 つ以上あることまで固定する
//! （走査が矩形の一部分に偏ればどれかが 0 になる）。
//!
//! ## 「焼き込みの失敗の記録が 0 件」の判定方法
//!
//! 焼き込みは失敗したときに必ず `error!` を出してから `Err` を返すので、`Ok` が返ったことを
//! もって判定する。面の系列解決の記録（相方側の面 2・3 の縮退の `WARN` 2 件）は `definition.rs`
//! が固定するので、ここでは捕捉しない。

use areka_emo_atlas::WicDecoderArm;
use areka_emo_present::balloon::{
    build_balloon_target_from_faces, load_balloon_use_self_alpha, resolve_balloon_faces,
};

use crate::support::fixture::{assert_three_alpha_kinds, census_face0, init_com_for_this_thread};

use super::test_support::{SCOPE_FACE0_FILES, balloon_root, expected_png_size};

/// 両 scope の面を起動時と同じ経路・透過設定で焼き込み、面 0 に 3 種の α が残ることを固定する。
#[test]
fn baked_face0_keeps_partial_alpha_for_both_scopes() {
    init_com_for_this_thread();
    let root = balloon_root();
    let decoder = WicDecoderArm::new().expect("WIC ファクトリを作れない（COM の初期化を確認）");
    let use_self_alpha = load_balloon_use_self_alpha(&root);

    for (scope, face0_file) in SCOPE_FACE0_FILES {
        let faces = resolve_balloon_faces(&root, scope)
            .unwrap_or_else(|e| panic!("scope {scope} の面の系列解決が失敗した: {e}"));
        let resolved_face0 = faces
            .iter()
            .find(|f| f.surface_id == 0)
            .unwrap_or_else(|| panic!("scope {scope}: 解決した系列に面 0 が無い"));
        assert_eq!(
            resolved_face0.file_name, face0_file,
            "scope {scope}: 面 0 に採られた実ファイル名が期待 `{face0_file}` と違う"
        );

        // 失敗すれば `error!` を出してから `Err` を返す実装なので、`Ok` が「失敗の記録 0 件」の判定である。
        let (_world, table) =
            build_balloon_target_from_faces(&root, &decoder, &faces, use_self_alpha)
                .unwrap_or_else(|e| {
                    panic!("scope {scope}: {} 面の焼き込みが失敗した: {e}", faces.len())
                });

        // `areka-balloon` の面 0 は縁まで割り当てられる（元の画像がそのまま残る）。
        let (w, h) = expected_png_size(face0_file);
        let census = census_face0(&table, scope, face0_file, (w, h), (0, 0, w, h));
        assert_three_alpha_kinds(scope, face0_file, &census);
    }
}
