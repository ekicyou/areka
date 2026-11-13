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

use crate::decode::MemoryDecoder;
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

/// `use_self_alpha: On`（emo2 の descript は `seriko.use_self_alpha,1`）。
pub(crate) fn params() -> AlphaParams {
    AlphaParams {
        use_self_alpha: UseSelfAlpha::On,
    }
}

/// `params()` の集まり。
pub(crate) fn set<'a>(surfaces: &'a [Surface], base: &'a Path) -> SurfaceSet<'a> {
    SurfaceSet {
        surfaces,
        base_dir: base,
        alpha_params: params(),
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
