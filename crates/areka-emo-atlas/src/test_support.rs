//! クレートのテストが共有する部品の置き場（spec: audit-emo-atlas 指摘 4・10）。
//!
//! COM に入って出る手伝い、面と集まりの組み立て、偽の読み手へ登録する絵、索引表の読み出し、
//! emo2 のシェルの読み込みを 1 か所に置く。`decode/` のテストもここから引く。
//! 検体の在処の窓口は `sample_test_support.rs` のまま。

use std::path::{Path, PathBuf};

use areka_parsers::shell::{
    Animation, AppendTarget, DrawMethod, Element, ElementPath, Interval, Pattern, Surface,
};
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use crate::decode::{AnimationInfo, DecodedImage, MemoryDecoder};
use crate::{AlphaParams, AtlasTable, ElementId, SurfaceSet, UseSelfAlpha, WicDecoderArm};

// ---- COM ----

/// COM を張った下で `f` を回す（WIC は COM が要る）。
///
/// 初期化に失敗したら落とす。別の型の apartment に入っているスレッド（`RPC_E_CHANGED_MODE`）は
/// そのまま使い、入っていない apartment を `CoUninitialize` で壊さない。
pub(crate) fn with_com<F: FnOnce()>(f: F) {
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let entered = hr.is_ok();
    assert!(
        entered || hr == RPC_E_CHANGED_MODE,
        "CoInitializeEx が失敗した: {hr:?}"
    );
    f();
    if entered {
        unsafe { CoUninitialize() };
    }
}

/// COM を張った下で本番の読み手を組んで渡す（読み手は `CoUninitialize` の前に捨てる）。
pub(crate) fn with_arm<F: FnOnce(&WicDecoderArm)>(f: F) {
    with_com(|| {
        let arm = WicDecoderArm::new().expect("WIC factory creates under COM init");
        f(&arm);
    });
}

// ---- 面と集まりの組み立て ----

/// 原点 (0, 0) の element。
pub(crate) fn elem(layer: u32, path: &str) -> Element {
    Element {
        layer,
        path: ElementPath::new(path.to_string()),
        x: 0,
        y: 0,
    }
}

/// element だけを持つ面（collision/animation は空）。
pub(crate) fn surface(id: u32, elements: Vec<Element>) -> Surface {
    Surface {
        targets: vec![AppendTarget::Single(id)],
        elements,
        collisions: Vec::new(),
        animations: Vec::new(),
    }
}

/// 宣言 `use_self_alpha` の α の扱い。
pub(crate) fn params(use_self_alpha: UseSelfAlpha) -> AlphaParams {
    AlphaParams { use_self_alpha }
}

/// `params(On)` の集まり（emo2 の descript は `seriko.use_self_alpha,1`）。
pub(crate) fn set<'a>(surfaces: &'a [Surface], base: &'a Path) -> SurfaceSet<'a> {
    SurfaceSet {
        surfaces,
        base_dir: base,
        alpha_params: params(UseSelfAlpha::On),
    }
}

/// 着せ替えのシェル: 面 `base` の `interval` のアニメーションが `overlay` で面 3000 を参照し、
/// 面 3000 は `rel` だけを持つ。
pub(crate) fn dressed_shell(mut base: Surface, interval: Interval, rel: &str) -> Vec<Surface> {
    base.animations = vec![Animation {
        id: 0,
        interval,
        patterns: vec![Pattern {
            index: 0,
            method: DrawMethod::new("overlay".to_string()),
            surface_id: 3000,
            target: "3000".into(),
            wait: 0,
            x: 0,
            y: 0,
        }],
    }];
    vec![base, surface(3000, vec![elem(0, rel)])]
}

// ---- 偽の読み手へ登録する絵 ----

/// 偽の読み手へ渡す絵の形（幅・高さ・stride・乗算済み BGRA・`has_alpha`）。
pub(crate) type ImageSpec = (u32, u32, u32, Vec<u8>, bool);

/// 詰めた 2×2 の不透明（α=255）の絵。
pub(crate) fn opaque_2x2() -> ImageSpec {
    let bgra = vec![
        10, 20, 30, 255, //
        40, 50, 60, 255, //
        70, 80, 90, 255, //
        100, 110, 120, 255,
    ];
    (2, 2, 8, bgra, true)
}

/// 詰めた 2×2 の全透明（α=0）の絵。
pub(crate) fn transparent_2x2() -> ImageSpec {
    (2, 2, 8, vec![0; 16], true)
}

/// 元画像が 0 寸の絵。
pub(crate) fn zero_sized() -> ImageSpec {
    (0, 0, 0, Vec::new(), true)
}

/// 偽の読み手へ `base.join(rel)` の実パスで絵を登録する。
pub(crate) fn register(dec: &mut MemoryDecoder, base: &Path, rel: &str, spec: ImageSpec) {
    let (w, h, stride, bgra, has_alpha) = spec;
    dec.insert(base.join(rel), w, h, stride, bgra, has_alpha);
}

// ---- 索引表の読み出し ----

/// エントリを頁から原寸の絵へ戻す（切り詰めの外は透明）。
pub(crate) fn full(table: &AtlasTable, id: ElementId) -> Vec<[u8; 4]> {
    let e = table.entry(id);
    let w = e.original.w;
    let mut out = vec![[0; 4]; (w * e.original.h) as usize];
    if let Some(p) = &e.placement {
        let page = table.page(p.page).expect("page exists");
        for y in 0..p.uv_rect.h {
            for x in 0..p.uv_rect.w {
                let src = ((p.uv_rect.y + y) * page.stride + (p.uv_rect.x + x) * 4) as usize;
                let dst = ((p.trim_offset.y as u32 + y) * w + p.trim_offset.x as u32 + x) as usize;
                out[dst] = page.bytes[src..src + 4].try_into().unwrap();
            }
        }
    }
    out
}

/// エントリの見え方（寸法・切り詰めの位置と大きさ・原寸へ戻した画素）。頁の上の位置は含めない。
pub(crate) fn view(table: &AtlasTable, id: ElementId) -> String {
    let e = table.entry(id);
    let trim = e
        .placement
        .as_ref()
        .map(|p| (p.trim_offset, p.uv_rect.w, p.uv_rect.h));
    format!("{:?} {:?} {:?}", e.original, trim, full(table, id))
}

/// 表の中身（鍵・エントリ・動く絵・頁の画素）を順序の決まった文字列にする。
pub(crate) fn dump(table: &AtlasTable) -> String {
    let mut s = String::new();
    for i in 0..table.len() {
        let id = ElementId(i as u32);
        s += &format!(
            "{i}: {:?} {:?} {:?}\n",
            table.key(id),
            table.entry(id),
            table.animation(id)
        );
    }
    for page in table.pages() {
        s += &format!(
            "page {}x{} {:?}\n",
            page.width,
            page.height,
            &page.bytes[..]
        );
    }
    s
}

// ---- 読み手のテスト ----

/// `src/testdata/<folder>`（`animated`・`palette` の検体のフォルダ）。
pub(crate) fn testdata(folder: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/testdata")
        .join(folder)
}

/// `testdata/animated/` の検体。
pub(crate) fn animated_sample(name: &str) -> PathBuf {
    testdata("animated").join(name)
}

/// `testdata/palette/` の検体。
pub(crate) fn palette_sample(name: &str) -> PathBuf {
    testdata("palette").join(name)
}

/// 動く絵の検体の見出し（どれも 8×8）。
pub(crate) fn info(frame_count: u32) -> AnimationInfo {
    AnimationInfo {
        width: 8,
        height: 8,
        frame_count,
    }
}

/// 乗算済み BGRA の (x, y) の 1 画素（BGRA の順のまま）。
pub(crate) fn pixel(bgra: &[u8], stride: u32, x: u32, y: u32) -> [u8; 4] {
    let i = (y * stride + x * 4) as usize;
    bgra[i..i + 4].try_into().unwrap()
}

// ---- 正規化のテスト ----

/// 画素の並び（4 バイト単位）と寸法から `DecodedImage` を組む。`stride` は `width * 4`。
/// `image(0, 0, false, &[])` が空の絵。
pub(crate) fn image(width: u32, height: u32, has_alpha: bool, pixels: &[[u8; 4]]) -> DecodedImage {
    assert_eq!(
        pixels.len(),
        (width * height) as usize,
        "画素数と寸法が合わない"
    );
    DecodedImage {
        width,
        height,
        stride: width * 4,
        bgra: pixels.concat(),
        has_alpha,
    }
}

// ---- 記録の文言（2 つ以上のテストファイルが比べる・本番の側には置かない） ----

/// 動く絵が上限を超えて 1 枚へ縮むときの `warn!`。
pub(crate) const OVER: &str = "bake: 動く絵が上限を超えたので 1 枚だけ読みます";
/// 動く絵として読めずに 1 枚へ縮むときの `warn!`。
pub(crate) const BROKEN: &str = "bake: 動く絵として読めなかったので 1 枚だけ読みます";
/// 動く絵の 1 枚目も読めずに今までの読み方へ落ちるときの `warn!`。
pub(crate) const FELL_BACK: &str =
    "bake: 動く絵の 1 枚目を読めなかったので、今までの読み方で 1 枚だけ読みます";
/// 抜き色で透過した絵の `debug!`。
pub(crate) const KEYED: &str = "bake: element を抜き色（左上の 1 画素と同じ色）で透過しました";
/// 元画像が 0 寸の絵の `warn!`。
pub(crate) const ZERO_WARN: &str = "元画像が 0 寸です";
/// 着せ替えのアニメーションからだけ参照される面の全透明の絵の `debug!`。
pub(crate) const BIND_DEBUG: &str = "着せ替え（bind）のアニメーションからだけ参照される面の絵です";
/// 全透明でトリム後 0 寸の絵の `warn!`。
pub(crate) const CLEAR_WARN: &str = "全透明（α=0）でトリム後 0 寸です";

// ---- emo2 のシェル ----

/// emo2 の `shell/master`（`surfaces.txt` の element の相対パスの起点）。
pub(crate) fn emo2_shell_dir() -> PathBuf {
    crate::sample_test_support::emo2_root().join("shell/master")
}

/// emo2 の `surfaces.txt` を読んで解いた面の一式と、その起点のフォルダ。
pub(crate) fn emo2_shell() -> (Vec<Surface>, PathBuf) {
    let dir = emo2_shell_dir();
    let content = std::fs::read_to_string(dir.join("surfaces.txt"))
        .expect("emo2 surfaces.txt must be readable (UTF-8)");
    let surfaces = areka_parsers::shell::parse(&content).surfaces;
    assert!(
        !surfaces.is_empty(),
        "parse produced surfaces from surfaces.txt"
    );
    (surfaces, dir)
}
