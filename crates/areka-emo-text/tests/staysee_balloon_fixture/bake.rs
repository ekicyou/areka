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

use areka_emo_atlas::{AtlasPage, AtlasTable, Rect, SetId, Size, WicDecoderArm};
use areka_emo_present::balloon::build_balloon_target_from_faces;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::test_support::{EXPECTED_FACE_COUNT, expected_frame_size, resolve_faces, staysee_root};

/// 各 scope で面 0 として焼き込まれる枠画像の名前。
///
/// 原寸は [`expected_frame_size`] が保管フォルダ側の期待値（実 PNG の IHDR と突合済み）から
/// 引くので、ここで二重に綴らない。
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

/// COM を当スレッドで MTA として初期化する。
///
/// 既に初期化済みのとき（テストは既定で並列に走る）は `S_FALSE`／`RPC_E_CHANGED_MODE` が
/// 返るが、どちらも「このスレッドで COM が使える」ことに変わりはないので無視する。
fn init_com_for_this_thread() {
    // SAFETY: COM の MTA 初期化。戻り値は上記のとおり無視してよい。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// アトラス頁の矩形 `uv` を 1 画素ずつ走査して α の内訳を数える。
///
/// 頁は premultiplied BGRA（1 画素 4 バイト・α は 4 バイト目）で、行の先頭は `stride`
/// バイトごとに並ぶ（頁の幅 × 4 とは限らない）。
fn census_alpha(page: &AtlasPage, uv: Rect) -> AlphaCensus {
    assert!(
        uv.x + uv.w <= page.width && uv.y + uv.h <= page.height,
        "走査する矩形 {uv:?} が頁 {}×{} の外へ出ている（この矩形の外は焼き込みの対象外で常に透明なので、はみ出したまま数えると内訳が意味を失う）",
        page.width,
        page.height
    );
    let stride = page.stride as usize;
    let mut census = AlphaCensus::default();
    for row in 0..uv.h {
        let row_head = (uv.y + row) as usize * stride;
        for col in 0..uv.w {
            let alpha = page.bytes[row_head + (uv.x + col) as usize * 4 + 3];
            census.total += 1;
            match alpha {
                0 => census.transparent += 1,
                255 => census.opaque += 1,
                _ => census.partial += 1,
            }
        }
    }
    census
}

/// 焼き上がった面 0 のエントリから、頁の矩形を取り出して α を数える。
fn census_face0(table: &AtlasTable, scope: u32, file_name: &str) -> AlphaCensus {
    let id = table.resolve(SetId(0), file_name).unwrap_or_else(|| {
        panic!(
            "scope {scope}: 焼き上がりの索引に面 0 の `{file_name}` が無い（索引の項目は {} 件）",
            table.len()
        )
    });
    let entry = table.entry(id);
    let (width, height) = expected_frame_size(file_name);
    assert_eq!(
        entry.original,
        Size {
            w: width,
            h: height
        },
        "scope {scope}: 面 0 `{file_name}` の原寸が期待 {width}×{height} と違う（出所: 保管した PNG の IHDR）"
    );

    let placement = entry.placement.as_ref().unwrap_or_else(|| {
        panic!(
            "scope {scope}: 面 0 `{file_name}` に割り当て結果が無い（全画素が透明だと判断されて転写が省かれた合図。原寸は {width}×{height}）"
        )
    });
    let page = table.page(placement.page).unwrap_or_else(|| {
        panic!(
            "scope {scope}: 面 0 `{file_name}` の割り当て先の頁 {} が無い（頁は {} 枚）",
            placement.page,
            table.pages().len()
        )
    });

    let census = census_alpha(page, placement.uv_rect);
    let uv = placement.uv_rect;
    assert_eq!(
        census.total,
        u64::from(uv.w) * u64::from(uv.h),
        "scope {scope}: 面 0 `{file_name}` で走査した画素の総数が矩形 {}×{} の面積と違う（実測 {}）",
        uv.w,
        uv.h,
        census.total
    );
    assert!(
        census.total > 0,
        "scope {scope}: 面 0 `{file_name}` の走査範囲が空である（母数 0 では以下の内訳の主張は何も確かめない）"
    );
    assert_eq!(
        census.transparent + census.partial + census.opaque,
        census.total,
        "scope {scope}: 面 0 `{file_name}` の内訳の合計が走査総数 {} と合わない（実測 {census:?}）",
        census.total
    );
    census
}

/// 両 scope の面を起動時と同じ透過設定で焼き込み、面 0 の原寸・割り当て・半透明を固定する。
///
/// 焼き込みは `resolve_balloon_faces` が返した系列をそのまま渡す経路（起動時の資産構築が
/// 使うのと同じ公開 API）で行う。透過設定は本番側が `UseSelfAlpha::On` に固定しているので、
/// テスト側で切り替えることはできない＝起動時と同じ扱いであることが構造から保証される。
#[test]
fn baked_face0_keeps_partial_alpha_for_both_scopes() {
    init_com_for_this_thread();
    let root = staysee_root();
    let decoder = WicDecoderArm::new().expect("WIC ファクトリを作れない（COM の初期化を確認）");

    assert_eq!(
        EXPECTED_FACE0_FILES.len(),
        2,
        "焼き込む scope は本体側と相方側の 2 つであること"
    );
    for (scope, face0_file) in EXPECTED_FACE0_FILES {
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
        let (_world, table) = build_balloon_target_from_faces(&root, &decoder, &faces)
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

        let census = census_face0(&table, scope, face0_file);
        assert!(
            census.partial > 0,
            "scope {scope}: 面 0 `{face0_file}` に半透明（0 より大きく 255 未満の α）の画素が 1 つも無い（α が 2 値へ潰れている＝枠の縁が硬くなる）。内訳 {census:?}"
        );
        // 走査が矩形の一部分に偏っていないことの対照——3 種がすべて実在する。
        assert!(
            census.opaque > 0,
            "scope {scope}: 面 0 `{face0_file}` に完全な不透明の画素が 1 つも無い（枠の中身が焼けていないか走査が偏っている）。内訳 {census:?}"
        );
        assert!(
            census.transparent > 0,
            "scope {scope}: 面 0 `{face0_file}` に完全な透明の画素が 1 つも無い（角の抜けが焼けていないか走査が偏っている）。内訳 {census:?}"
        );
    }
}
