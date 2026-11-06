//! # shipped_fixture_region_test — 出荷済み実ゴースト定義の開始点を逐語固定する檻
//!
//! 出典 spec: `areka-P0-balloon-vertical-canon`（要件 **3.10**／**10.7**・設計 **DD5**／**C9**）。
//!
//! ## この檻が塞ぐ穴
//!
//! `検体 emo2 の同梱バルーン emo2-kakukaku` は**実機サインオフと
//! emo-present の実描画に効く実ゴースト定義**でありながら、2026-08-28 の棚卸し時点で
//! **その文字開始点を固定するテストが 1 本も存在しなかった**。すなわち座標解決規則を壊しても
//! ワークスペースは全緑のまま出荷できる状態だった（設計 C9 の Risks「**全緑は十分性の証拠に
//! ならない**」——本リポジトリで 3 度目の構図）。本ファイルはその唯一の穴を塞ぐ。
//!
//! ## 固定するもの
//!
//! 本番と同じ経路（`descript.txt` 基層＋面別上書き層の 2 層マージ →
//! [`TextRegion::resolve`]）で解決した結果を、成分ごとに逐語で固定する:
//!
//! | scope | 面別上書き層 | 画像原寸 | left | top | right | bottom | start | wrap |
//! |---|---|---|---|---|---|---|---|---|
//! | 0（sakura） | `balloons0s.txt` | `balloons0.png` 400×224 | 36 | 46 | 356 | 168 | **(36, 46)** | 351 |
//! | 1（kero） | `balloonk0s.txt` | `balloonk0.png` 288×203 | 24 | 40 | 240 | 133 | **(24, 40)** | 254 |
//!
//! 開始点だけでは壊れ方を見落とすため、`left`／`top`／`right`／`bottom`／`wrap_threshold`／
//! [`WritingMode`] も併せて固定する。複製 fixture `emo2-kakukaku-wplimit` も同値であることを
//! 同じ形で固定する（複製が原本から乖離したら赤くなる）。
//!
//! ## 期待値は origin の宣言にもクランプにも依らない
//!
//! 期待値 (36,46)／(24,40) は
//!
//! - 「宣言 `origin(0,0)` が validrect 外ゆえ**クランプで書字開始角へ寄った**結果」でもあり、
//! - 「宣言が無いので**未宣言の縮退で書字開始角へ落ちた**結果」でもある（要件 3.11 が不変を保証）
//!
//! という**同じ値**である。したがって本檻は宣言の有無にもクランプの有無にも依存しない。
//!
//! **禁則**: 本ファイルで `model.origin().x() == Some(0)` のような**宣言された生値**を assert
//! してはならない。期待値が宣言の有無に依らないことがこの檻の成立条件である。
//!
//! **禁則の対象**: 上の禁則が掛かるのは**原本 `emo2-kakukaku` とその複製 `emo2-kakukaku-wplimit`
//! を見る検査**（檻 1〜4 の 5 本）である。この 2 つは宣言を削除済みで、宣言の有無に依存しないことが
//! 証跡の成立条件だからである。`origin` を今も宣言したままの検体 `emo2-kakukaku-offsetdpi` を
//! 見る検査（ファイル末尾・出典 spec `areka-P0-balloon-origin-outside-validrect`）には掛からない
//! ——そちらは**宣言が在ること自体が前提**であり、宣言が消えたら「宣言されていないときの縮退」を
//! 測るだけの検査に静かにすり替わるので、前提として明示的に確かめる。
//!
//! ## 決定論（要件 10.6）
//!
//! 実 DPI モニタ・実 GPU・実ゴースト・実窓を要さない。ファイル読み込みと純粋層の解決のみで
//! 完結し、同一入力に対して常に同一の結果を返す。
//!
//! ---
//!
//! 対象ファイルの棚卸しの記録の正本は `.kiro/specs/completed/areka-P0-balloon-vertical-canon/tasks.md` の付録 A。

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use sample_ghost_kit::SampleRoot;

use areka_emo_text::region::TextRegion;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::BalloonModel;

#[path = "support/mod.rs"]
mod support;

use support::fixture::{emo2_balloon_root, png_native_size};

// ── 逐語固定する期待値（本ファイル冒頭の表と 1:1）────────────────────────────
//
// 画像原寸は**ハードコードしつつ実 PNG と突合する**（`native_sizes_match_shipped_png_headers`）。
// 実ファイルから読むだけにしないのは、PNG が差し替わったときに期待値が黙って追従して
// 「檻が自分で期待値を作る」形になるためである。ハードコードした値が実物と食い違えば
// 突合テストが赤くなるので、二重帳簿にはならない。

/// `balloons0.png` の実測原寸（image px・IHDR 由来）。
const SAKURA_IMAGE_SIZE: (u32, u32) = (400, 224);
/// `balloonk0.png` の実測原寸（image px・IHDR 由来）。
const KERO_IMAGE_SIZE: (u32, u32) = (288, 203);

/// scope 0（sakura）の解決後 validrect: `left`／`top`／`right`／`bottom`（image px）。
///
/// 導出: `top,46`（非負素通し）／`bottom,-56` → 224−56＝168／`left,36`／`right,-44` → 400−44＝356。
const SAKURA_EDGES: (f32, f32, f32, f32) = (36.0, 46.0, 356.0, 168.0);
/// scope 0（sakura）の描画開始点（image px）。
const SAKURA_START: (f32, f32) = (36.0, 46.0);
/// scope 0（sakura）の折返し閾値: `balloons0s.txt` の `wordwrappoint.x,-49` → 400−49＝351。
const SAKURA_WRAP: f32 = 351.0;

/// scope 1（kero）の解決後 validrect: `left`／`top`／`right`／`bottom`（image px）。
///
/// 導出: `top,40`／`bottom,-70` → 203−70＝133／`left,24`／`right,-48` → 288−48＝240。
const KERO_EDGES: (f32, f32, f32, f32) = (24.0, 40.0, 240.0, 133.0);
/// scope 1（kero）の描画開始点（image px）。
const KERO_START: (f32, f32) = (24.0, 40.0);
/// scope 1（kero）の折返し閾値: `balloonk0s.txt` に `wordwrappoint` 行が無く基層の
/// `wordwrappoint.x,-34` を継承 → 288−34＝254（2 層マージの継承がここに効く）。
const KERO_WRAP: f32 = 254.0;

/// scope 0（sakura）の面別上書き層。
///
/// scope → 採用面 → 上書きファイル名の対応は emo-present の面解決
/// （`resolve_balloon_faces` → `ResolvedFace::override_file_name`）が決めており、本檻はその
/// 結果を実ファイル名で直接指名する。
const SAKURA_OVERLAY: &str = "balloons0s.txt";
/// scope 1（kero）の面別上書き層。
const KERO_OVERLAY: &str = "balloonk0s.txt";

// ── fixture の所在（窓口 `sample-ghost-kit` が持つ・検体名で引く）───────────────────────
//
// 検体は段 ③ で `Drop` が複製を消すため、一時値にせずプロセス寿命で保持する。原本と複製は
// 別々の検体として登記されているので、保持も 2 つになる（同じ木の複製ではない）。原本
// （実ゴースト `emo2` の同梱バルーン）は共有の部品 `support::fixture::emo2_balloon_root` が持つ。

/// 複製の検体（`windowposition-limit` の実機サインオフ用・単体で登記されたバルーン）。
static WPLIMIT: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire("emo2-kakukaku-wplimit").expect("emo2-kakukaku-wplimit は登記済みの検体")
});

/// 複製 fixture（`windowposition-limit` の実機サインオフ用）。原本とは別の検体として登記されて
/// おり、ゴーストツリーの列挙には影響しない（同 fixture の `readme.txt` 正典）。
fn wplimit_root() -> PathBuf {
    WPLIMIT.folder().to_path_buf()
}

// ── 本番と同じ読み込み経路 ───────────────────────────────────────────────

/// 指定ディレクトリの 2 層（`descript.txt` 基層＋面別上書き層）をマージした `BalloonModel`。
///
/// 読みは本番と同じ ANSI の規則（`support::fixture::read_balloon_file`）、マージは本番と同じ
/// `areka_parsers::balloon::parse_str(descript, Some(overlay))`。
fn merged_model(dir: &Path, overlay: &str) -> BalloonModel {
    support::fixture::merged_model(dir, Some(overlay))
}

/// 与えたディレクトリの scope 0／1 を解決した `TextRegion` の対を返す。
fn resolve_both(dir: &Path) -> (TextRegion, TextRegion) {
    let sakura = merged_model(dir, SAKURA_OVERLAY);
    let kero = merged_model(dir, KERO_OVERLAY);
    (
        TextRegion::resolve(&sakura, SAKURA_IMAGE_SIZE, WritingMode::resolve(&sakura)),
        TextRegion::resolve(&kero, KERO_IMAGE_SIZE, WritingMode::resolve(&kero)),
    )
}

/// `TextRegion` の全成分を逐語で突合する（どの成分が壊れたかを名指しで出す）。
fn assert_region(
    region: &TextRegion,
    edges: (f32, f32, f32, f32),
    start: (f32, f32),
    wrap: f32,
    what: &str,
) {
    assert_eq!(region.left(), edges.0, "{what}: validrect 左辺");
    assert_eq!(region.top(), edges.1, "{what}: validrect 上辺");
    assert_eq!(region.right(), edges.2, "{what}: validrect 右辺");
    assert_eq!(region.bottom(), edges.3, "{what}: validrect 下辺");
    assert_eq!(region.start(), start, "{what}: 描画開始点");
    assert_eq!(region.wrap_threshold(), wrap, "{what}: 折返し閾値");
}

// ── 檻 1: パス解決とファイル実在の生存証明 ────────────────────────────────

/// ハードコードした画像原寸が出荷 PNG の IHDR と一致する（期待値の二重帳簿を防ぐ）。
#[test]
fn native_sizes_match_shipped_png_headers() {
    for dir in [emo2_balloon_root(), wplimit_root()] {
        assert_eq!(
            png_native_size(&dir.join("balloons0.png")),
            SAKURA_IMAGE_SIZE,
            "{}: balloons0.png の原寸",
            dir.display()
        );
        assert_eq!(
            png_native_size(&dir.join("balloonk0.png")),
            KERO_IMAGE_SIZE,
            "{}: balloonk0.png の原寸",
            dir.display()
        );
    }
}

// ── 檻 2: 書字方向（要件 3.10 の前提）────────────────────────────────────

/// 出荷 fixture は `writing_mode`・`vertical` のいずれも宣言していないため横書きへ解決される。
///
/// 書字方向は書字開始角（横書き＝validrect 左上／`vertical_rl`＝右上）を決めるので、
/// これが動くと開始点の期待値の意味そのものが変わる。開始点と一緒に固定する。
#[test]
fn shipped_fixture_resolves_to_horizontal_tb() {
    for dir in [emo2_balloon_root(), wplimit_root()] {
        for overlay in [SAKURA_OVERLAY, KERO_OVERLAY] {
            assert_eq!(
                WritingMode::resolve(&merged_model(&dir, overlay)),
                WritingMode::HorizontalTb,
                "{} + {overlay}: 縦書き宣言が無いので横書きへ解決される",
                dir.display()
            );
        }
    }
}

// ── 檻 3: 実ゴースト定義の開始点と領域（本ファイルの本題・要件 3.10／10.7）──────

/// scope 0（sakura）の解決結果を逐語固定する——**開始点 (36, 46)**。
///
/// この値は「宣言 `origin(0,0)` がクランプで書字開始角へ寄った結果」でもあり
/// 「宣言が無く未宣言縮退で書字開始角へ落ちた結果」でもある同じ値なので、宣言の削除・クランプ撤去の
/// いずれを経ても不変である（モジュール doc「期待値は origin の宣言にもクランプにも依らない」参照）。
#[test]
fn shipped_sakura_scope_region_is_pinned() {
    let (sakura, _) = resolve_both(&emo2_balloon_root());
    assert_region(
        &sakura,
        SAKURA_EDGES,
        SAKURA_START,
        SAKURA_WRAP,
        "emo2-kakukaku scope 0 (sakura)",
    );
}

/// scope 1（kero）の解決結果を逐語固定する——**開始点 (24, 40)**。
///
/// 折返し閾値 254 は基層 `wordwrappoint.x,-34` の**継承**（`balloonk0s.txt` に当該行が無い）に
/// 由来するため、2 層マージの継承が壊れると単独で赤くなる。
#[test]
fn shipped_kero_scope_region_is_pinned() {
    let (_, kero) = resolve_both(&emo2_balloon_root());
    assert_region(
        &kero,
        KERO_EDGES,
        KERO_START,
        KERO_WRAP,
        "emo2-kakukaku scope 1 (kero)",
    );
}

// ── 檻 4: 複製 fixture の乖離検出 ──────────────────

/// 複製 fixture `emo2-kakukaku-wplimit` が原本と**同一の領域解決**を与える。
///
/// 同 fixture の原本との差分は面別上書き層の `windowposition.*` 3 行のみ（`readme.txt` 正典・
/// 実測でも `descript.txt` と 2 枚の PNG は原本と 1 バイト差なし）。`windowposition` は
/// [`TextRegion`] の入力ではないため領域解決は完全に同値になるはずで、複製が原本から乖離
/// したらここで赤くなる。
#[test]
fn wplimit_copy_resolves_identically_to_shipped_original() {
    let (orig_sakura, orig_kero) = resolve_both(&emo2_balloon_root());
    let (copy_sakura, copy_kero) = resolve_both(&wplimit_root());

    // 原本との同値（乖離の検出）。
    assert_eq!(
        copy_sakura, orig_sakura,
        "wplimit 複製の scope 0 は原本 emo2-kakukaku と同一の領域解決を与える"
    );
    assert_eq!(
        copy_kero, orig_kero,
        "wplimit 複製の scope 1 は原本 emo2-kakukaku と同一の領域解決を与える"
    );

    // 逐語固定（原本側と複製側が同時に同じ方向へ壊れても捕まえる）。
    assert_region(
        &copy_sakura,
        SAKURA_EDGES,
        SAKURA_START,
        SAKURA_WRAP,
        "emo2-kakukaku-wplimit scope 0 (sakura)",
    );
    assert_region(
        &copy_kero,
        KERO_EDGES,
        KERO_START,
        KERO_WRAP,
        "emo2-kakukaku-wplimit scope 1 (kero)",
    );
}

// ── 檻 5: 範囲外の `origin` を宣言したままの検体 ───────────────────────────
//
// 出典 spec: `areka-P0-balloon-origin-outside-validrect`（要件 **1.6**／**2.3**／**5.5**）。
// 上の 5 本は宣言を削除済みの原本と複製を見る檻であり、**宣言が範囲外に在るときの規則**は
// どれも固定していない。それを固定するのがここである。

/// 範囲外の `origin` を宣言したままの検体（実機サインオフ用に原本から複製されたバルーン）。
///
/// 原本・`wplimit` が宣言を削除した後も `origin.x,0`／`origin.y,0` を持ち続けている
/// （面別上書き層の `validrect.left` は sakura 36／kero 24 なので、解決後の宣言値は範囲の
/// 外に落ちる）。検体は書き換えない。`Drop` が複製を消すのでプロセス寿命で保持する。
static OFFSETDPI: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire("emo2-kakukaku-offsetdpi")
        .expect("emo2-kakukaku-offsetdpi は登記済みの検体")
});

/// 範囲外宣言を持つ検体のバルーン定義ディレクトリ（自分でパスを継ぎ足さない）。
///
/// 窓口の登記ではバルーン種別なので、根は `balloon()` ではなく `folder()` で引く
/// （既存の `wplimit_root()` と同形）。
fn offsetdpi_root() -> PathBuf {
    OFFSETDPI.folder().to_path_buf()
}

/// 範囲外に宣言された `origin` を持つ実物の検体が、書字開始角から書き始める。
///
/// 期待値は原本と同じ定数をそのまま使う——この検体の 2 枚の PNG は原本と同寸で、面別上書き層の
/// `validrect`／`wordwrappoint` も原本と同じだからである（原本との差分は `windowposition` と
/// `origin` 宣言の有無のみ）。原本の領域とまるごと比べる形にはせず成分ごとに突合する。
///
/// 前提を 3 つ同じ検査の中で確かめる。いずれかが崩れると、この検査は規則ではなく別の何かを
/// 測るものに静かにすり替わる: ⑴ マージ後のモデルが `origin` を**宣言している**こと
/// （冒頭 doc「禁則の対象」参照）⑵ 2 枚の PNG の原寸が定数と一致すること（原寸が違えば範囲の
/// 4 辺が別の値になる）⑶ 横書きへ解決されること（書字開始角がどの隅かは書字方向が決める）
#[test]
fn offsetdpi_fixture_with_out_of_range_origin_starts_at_writing_corner() {
    let dir = offsetdpi_root();

    let sakura_model = merged_model(&dir, SAKURA_OVERLAY);
    let kero_model = merged_model(&dir, KERO_OVERLAY);

    // 前提 1: 範囲外の宣言が在ること。
    for (model, what) in [
        (&sakura_model, "scope 0 (sakura)"),
        (&kero_model, "scope 1 (kero)"),
    ] {
        assert_eq!(
            model.origin().x(),
            Some(0),
            "emo2-kakukaku-offsetdpi {what}: origin.x の宣言が在ることが本検査の前提"
        );
        assert_eq!(
            model.origin().y(),
            Some(0),
            "emo2-kakukaku-offsetdpi {what}: origin.y の宣言が在ることが本検査の前提"
        );
    }

    // 前提 2: 画像原寸が期待値の定数と一致すること。
    assert_eq!(
        png_native_size(&dir.join("balloons0.png")),
        SAKURA_IMAGE_SIZE,
        "emo2-kakukaku-offsetdpi: balloons0.png の原寸"
    );
    assert_eq!(
        png_native_size(&dir.join("balloonk0.png")),
        KERO_IMAGE_SIZE,
        "emo2-kakukaku-offsetdpi: balloonk0.png の原寸"
    );

    // 前提 3: 横書きへ解決されること。
    let sakura_writing = WritingMode::resolve(&sakura_model);
    let kero_writing = WritingMode::resolve(&kero_model);
    assert_eq!(
        sakura_writing,
        WritingMode::HorizontalTb,
        "emo2-kakukaku-offsetdpi scope 0 (sakura): 縦書き宣言が無いので横書きへ解決される"
    );
    assert_eq!(
        kero_writing,
        WritingMode::HorizontalTb,
        "emo2-kakukaku-offsetdpi scope 1 (kero): 縦書き宣言が無いので横書きへ解決される"
    );

    // 本題: 範囲外の成分は宣言なしと同じ扱いになり、開始点は書字開始角（横書きなので
    // validrect の左上）へ落ちる。
    assert_region(
        &TextRegion::resolve(&sakura_model, SAKURA_IMAGE_SIZE, sakura_writing),
        SAKURA_EDGES,
        SAKURA_START,
        SAKURA_WRAP,
        "emo2-kakukaku-offsetdpi scope 0 (sakura)",
    );
    assert_region(
        &TextRegion::resolve(&kero_model, KERO_IMAGE_SIZE, kero_writing),
        KERO_EDGES,
        KERO_START,
        KERO_WRAP,
        "emo2-kakukaku-offsetdpi scope 1 (kero)",
    );
}
