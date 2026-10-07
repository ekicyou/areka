//! バルーンの透過の宣言のテスト（spec: areka-P0-self-alpha-declaration 要件 2.1・2.3・2.4・2.7・2.8・5.7）。
//!
//! 宣言を読むのは基層の descript.txt だけで、面ごとの設定ファイルに上書きされないこと、シェルの
//! キーを読まないこと、フォルダ直下の `.pna` の数を 1 行だけ記録すること、焼く関数が受けた値で
//! 描くことを固定する。

use super::*;

use super::test_support::{CapturedEvent, FieldUnquoted, TempDir, capture_events};
use areka_emo_atlas::MemoryDecoder;

/// `.pna` の記録（`balloon:` で始まり `ignored_pna` の欄を持つ `warn!`）だけを拾う。
fn pna_warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::WARN
                && e.message().starts_with("balloon:")
                && e.field("ignored_pna").is_some()
        })
        .collect()
}

/// descript.txt の `0` は面ごとの設定ファイルの `1` に上書きされない（要件 2.1・2.4・2.7）。
#[test]
fn descript_declaration_is_not_overridden_by_face_settings() {
    let dir = TempDir::new();
    dir.write("descript.txt", "charset,UTF-8\nuse_self_alpha,0\n");
    dir.write("balloons0s.txt", "charset,UTF-8\nuse_self_alpha,1\n");
    dir.touch("balloons0.png");

    assert_eq!(load_balloon_use_self_alpha(dir.path()), UseSelfAlpha::Off);
}

/// シェルのキー（`seriko.use_self_alpha`）だけを書いたバルーンは宣言なし（要件 2.8）。
#[test]
fn shell_key_alone_is_undeclared() {
    let dir = TempDir::new();
    dir.write(
        "descript.txt",
        "charset,UTF-8\nseriko.use_self_alpha,full\n",
    );

    assert_eq!(
        load_balloon_use_self_alpha(dir.path()),
        UseSelfAlpha::Undeclared
    );
}

/// フォルダ直下の `.pna` を拡張子の大小を問わず数え、`warn!` を 1 行だけ出す（要件 5.7）。
#[test]
fn pna_files_are_counted_once_in_one_warn() {
    let dir = TempDir::new();
    dir.write("descript.txt", "charset,UTF-8\nuse_self_alpha,1\n");
    dir.touch("balloons0.png");
    dir.touch("balloons0.pna");
    dir.touch("ARROW0.PNA");

    let (got, events) = capture_events(|| load_balloon_use_self_alpha(dir.path()));
    assert_eq!(got, UseSelfAlpha::On);
    let warns = pna_warns(&events);
    assert_eq!(warns.len(), 1, ".pna の warn! は 1 行: {events:?}");
    assert_eq!(
        warns[0].field_unquoted("ignored_pna"),
        Some("2"),
        "{:?}",
        warns[0]
    );
}

/// `.pna` が 1 つも無ければ記録しない（要件 5.7）。
#[test]
fn no_pna_no_warn() {
    let dir = TempDir::new();
    dir.write("descript.txt", "charset,UTF-8\nuse_self_alpha,1\n");
    dir.touch("balloons0.png");

    let (_, events) = capture_events(|| load_balloon_use_self_alpha(dir.path()));
    assert!(pna_warns(&events).is_empty(), "{events:?}");
}

/// 焼く関数は受けた宣言で描く: `full` なら α なしの面の左上の色を抜かない（要件 2.3）。
/// 対照として `1` では同じ絵が左上の色で抜かれて全部透明になる。
#[test]
fn full_keeps_the_top_left_color_of_an_alphaless_face() {
    let dir = Path::new("balloon_dir");
    let px = [10u8, 20, 30, 255];
    let mut dec = MemoryDecoder::new();
    dec.insert(dir.join("balloons0.png"), 2, 1, 8, [px, px].concat(), false);
    let faces = [ResolvedFace {
        surface_id: 0,
        prefix: "balloons".to_string(),
        tier: ChainTier::Own,
        file_name: "balloons0.png".to_string(),
    }];

    let placement_of = |use_self_alpha| {
        let Ok((_, table)) = build_balloon_target_from_faces(dir, &dec, &faces, use_self_alpha)
        else {
            panic!("{use_self_alpha:?} で組み立てに失敗");
        };
        let id = table
            .resolve(SetId(0), "balloons0.png")
            .expect("面がアトラスに解決される");
        table.entry(id).placement.clone().map(|p| {
            let page = table.page(p.page).expect("page exists").clone();
            (p, page)
        })
    };

    let (p, page) = placement_of(UseSelfAlpha::Full).expect("full では抜かれず placement を持つ");
    let at = (p.uv_rect.y * page.stride + p.uv_rect.x * 4) as usize;
    assert_eq!(page.bytes[at..at + 4], px, "左上の画素がそのまま残る");
    assert!(
        placement_of(UseSelfAlpha::On).is_none(),
        "対照: 1 では左上の色で抜かれて全部透明になる"
    );
}
