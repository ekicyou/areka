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
//! `staysee_balloon_fixture/bake.rs` と同じく、走査した画素の総数を矩形の面積と突き合わせ、
//! 内訳の合計が総数と一致し、不透明・透明・半透明の 3 種がすべて 1 つ以上あることまで固定する
//! （走査が矩形の一部分に偏ればどれかが 0 になる）。
//!
//! ## 「焼き込みの失敗の記録が 0 件」の判定方法
//!
//! 焼き込みは失敗したときに必ず `error!` を出してから `Err` を返すので、`Ok` が返ったことを
//! もって判定する。面の系列解決の記録（相方側の面 2・3 の縮退の `WARN` 2 件）は `definition.rs`
//! が固定するので、ここでは捕捉しない。

use areka_emo_atlas::{AtlasPage, AtlasTable, Rect, SetId, Size, WicDecoderArm};
use areka_emo_present::balloon::{
    build_balloon_target_from_faces, load_balloon_use_self_alpha, resolve_balloon_faces,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::test_support::{EXPECTED_PNG_SIZES, balloon_root};

/// 各 scope で面 0 として焼き込まれる絵の名前（要件 2.1・2.2）。
const EXPECTED_FACE0_FILES: [(u32, &str); 2] = [(0, "balloons0.png"), (1, "balloonk0.png")];

/// 走査した画素の α の内訳。
#[derive(Debug, Default)]
struct AlphaCensus {
    /// 走査した画素の総数。
    total: u64,
    /// α ＝ 0（完全な透明）。
    transparent: u64,
    /// 0 < α < 255（半透明）。
    partial: u64,
    /// α ＝ 255（完全な不透明）。
    opaque: u64,
}

/// COM を当スレッドで MTA として初期化する（既に初期化済みの戻り値は無視してよい）。
fn init_com_for_this_thread() {
    // SAFETY: COM の MTA 初期化。`S_FALSE`／`RPC_E_CHANGED_MODE` でも COM は使える。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// 期待値の表から絵の寸法を引く。
fn expected_png_size(file_name: &str) -> (u32, u32) {
    EXPECTED_PNG_SIZES
        .iter()
        .find(|(name, _, _)| *name == file_name)
        .map(|&(_, w, h)| (w, h))
        .unwrap_or_else(|| panic!("`{file_name}` が期待値の表 EXPECTED_PNG_SIZES に無い"))
}

/// アトラス頁の矩形 `uv` を 1 画素ずつ走査して α の内訳を数える（premultiplied BGRA・行は `stride` 送り）。
fn census_alpha(page: &AtlasPage, uv: Rect) -> AlphaCensus {
    assert!(
        uv.x + uv.w <= page.width && uv.y + uv.h <= page.height,
        "走査する矩形 {uv:?} が頁 {}×{} の外へ出ている",
        page.width,
        page.height
    );
    let stride = page.stride as usize;
    let mut census = AlphaCensus::default();
    for row in 0..uv.h {
        let row_head = (uv.y + row) as usize * stride;
        for col in 0..uv.w {
            census.total += 1;
            match page.bytes[row_head + (uv.x + col) as usize * 4 + 3] {
                0 => census.transparent += 1,
                255 => census.opaque += 1,
                _ => census.partial += 1,
            }
        }
    }
    census
}

/// 焼き上がった面 0 のエントリから、原寸を確かめて頁の矩形の α を数える。
fn census_face0(table: &AtlasTable, scope: u32, file_name: &str) -> AlphaCensus {
    let id = table.resolve(SetId(0), file_name).unwrap_or_else(|| {
        panic!(
            "scope {scope}: 焼き上がりの索引に面 0 の `{file_name}` が無い（索引の項目は {} 件）",
            table.len()
        )
    });
    let entry = table.entry(id);
    let (w, h) = expected_png_size(file_name);
    assert_eq!(
        entry.original,
        Size { w, h },
        "scope {scope}: 面 0 `{file_name}` の原寸が期待 {w}×{h} と違う"
    );
    let placement = entry.placement.as_ref().unwrap_or_else(|| {
        panic!("scope {scope}: 面 0 `{file_name}` に割り当て結果が無い（全画素が透明と判断された）")
    });
    let page = table.page(placement.page).unwrap_or_else(|| {
        panic!(
            "scope {scope}: 面 0 `{file_name}` の割り当て先の頁 {} が無い（頁は {} 枚）",
            placement.page,
            table.pages().len()
        )
    });

    let uv = placement.uv_rect;
    let census = census_alpha(page, uv);
    assert_eq!(
        census.total,
        u64::from(uv.w) * u64::from(uv.h),
        "scope {scope}: 面 0 `{file_name}` で走査した画素の総数が矩形 {}×{} の面積と違う（実測 {}）",
        uv.w,
        uv.h,
        census.total
    );
    assert_eq!(
        census.total,
        u64::from(w) * u64::from(h),
        "scope {scope}: 面 0 `{file_name}` の走査総数が面の面積 {w}×{h} と違う（実測 {}）",
        census.total
    );
    assert_eq!(
        census.transparent + census.partial + census.opaque,
        census.total,
        "scope {scope}: 面 0 `{file_name}` の内訳の合計が走査総数と合わない（実測 {census:?}）"
    );
    census
}

/// 両 scope の面を起動時と同じ経路・透過設定で焼き込み、面 0 に 3 種の α が残ることを固定する。
#[test]
fn baked_face0_keeps_partial_alpha_for_both_scopes() {
    init_com_for_this_thread();
    let root = balloon_root();
    let decoder = WicDecoderArm::new().expect("WIC ファクトリを作れない（COM の初期化を確認）");
    let use_self_alpha = load_balloon_use_self_alpha(&root);

    for (scope, face0_file) in EXPECTED_FACE0_FILES {
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

        let census = census_face0(&table, scope, face0_file);
        assert!(
            census.partial > 0,
            "scope {scope}: 面 0 `{face0_file}` に半透明の画素が 1 つも無い（α が 2 値へ潰れている）。内訳 {census:?}"
        );
        assert!(
            census.opaque > 0,
            "scope {scope}: 面 0 `{face0_file}` に完全な不透明の画素が 1 つも無い。内訳 {census:?}"
        );
        assert!(
            census.transparent > 0,
            "scope {scope}: 面 0 `{face0_file}` に完全な透明の画素が 1 つも無い。内訳 {census:?}"
        );
    }
}
