//! # shipped_fixture_region_test — 出荷済み実ゴースト定義の開始点を逐語固定する檻（task 4.2）
//!
//! 出典 spec: `areka-P0-balloon-vertical-canon`（要件 **3.10**／**10.7**・設計 **DD5**／**C9**）。
//!
//! ## この檻が塞ぐ穴
//!
//! `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku` は**実機サインオフと
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
//! ## 2 つの編集を跨いで無改変のまま緑であることが証跡になる
//!
//! 本檻は **origin 宣言がまだ在る状態（クランプ経由）で書かれた**。以後、
//!
//! - task 4.3 = fixture の `origin.x,0`／`origin.y,0` 宣言の削除
//! - task 4.4 = `region.rs` の origin クランプ撤去（要件 3.10）
//!
//! の**いずれを経ても本ファイルは無改変のまま緑であり続けなければならない**。これが両編集の
//! 挙動不変を反証可能にする証跡である。成立の理屈は次のとおり——期待値 (36,46)／(24,40) は
//!
//! - 「宣言 `origin(0,0)` が validrect 外ゆえ**クランプで書字開始角へ寄った**結果」でもあり、
//! - 「宣言が無いので**未宣言の縮退で書字開始角へ落ちた**結果」でもある（要件 3.11 が不変を保証）
//!
//! という**同じ値**である。したがって本檻は宣言の有無にもクランプの有無にも依存しない。
//!
//! **禁則**: 本ファイルで `model.origin().x() == Some(0)` のような**宣言された生値**を assert
//! してはならない。task 4.3 で宣言が消えた瞬間に赤くなり、上の証跡が成立しなくなる。
//!
//! ## 決定論（要件 10.6）
//!
//! 実 DPI モニタ・実 GPU・実ゴースト・実窓を要さない。ファイル読み込みと純粋層の解決のみで
//! 完結し、同一入力に対して常に同一の結果を返す。
//!
//! ---
//!
//! # 付録: task 4.1 意味論の棚卸し記録（2026-08-28 実測）
//!
//! 本檻はこの棚卸しの産物である。監査証跡として恒久記録する（正本は
//! `.kiro/specs/areka-P0-balloon-vertical-canon/tasks.md` 付録 A）。
//!
//! ## 対象ファイル一覧
//!
//! **validrect 外の `origin` 宣言を持つバルーン定義データファイル＝5 本**（いずれも
//! `origin.x,0`／`origin.y,0`）:
//!
//! | # | ファイル | 是正方針（DD5） |
//! |---|---|---|
//! | D1 | `areka-emo-text/examples/fixtures/emo2-vertical/descript.txt` | 宣言削除（開始点 (356,46) 不変） |
//! | D2 | `areka-emo-text/tests/fixtures/emo2-choice/descript-cursor.txt` | 宣言削除＋直前コメントの是正 |
//! | D3 | `areka-emo-text/tests/fixtures/emo2-choice/descript-plain.txt` | 宣言削除（D2 と対） |
//! | D4 | `pilot/…/fixtures/emo2/emo2-kakukaku/descript.txt` | 宣言削除（**本檻の対象**） |
//! | D5 | `pilot/…/fixtures/emo2-kakukaku-wplimit/descript.txt` | 宣言削除（**本檻の対象**・D4 の複製） |
//!
//! **解決後 validrect の外にある in-code モデル＝3 箇所**（＋`region.rs` の fixture 複製 2 箇所）:
//!
//! | # | 場所 | 是正方針（DD5） |
//! |---|---|---|
//! | C1 | `areka-emo-text/tests/draw_readback_test.rs` の `validrect_model` | `Origin::new(None, None)` へ |
//! | C2 | `areka-emo-text/tests/scale_invariance_test.rs` の 3 箇所 | `Origin::new(None, None)` へ |
//! | C3 | `areka-emo-text/src/actor_scale_refresh_tests.rs` | `Origin::new(None, None)` へ |
//! | C4／C5 | `areka-emo-text/src/region.rs` の `fixture_model()`／in-code KV | 意図ごとに個別判断 |
//!
//! ## 内外判定の結果: **5 本すべてが範囲外**
//!
//! 設計 C9 の候補地表は 4 件を挙げていたが、**`emo2-kakukaku-wplimit`（D5）が 5 件目**として
//! 4.1 の棚卸しで判明した。逆向きの食い違い（既知 4 件のいずれかが実は範囲内だった）は無い。
//! `C3` は設計のどの行にも現れていなかった。
//!
//! ## 方法の限界（同じ穴を二度踏まないために）
//!
//! - **語の grep では見つからない類が在る。** 「クランプ」「clamp_origin」「書字開始角」を
//!   検索しても、当該語を 1 度も書いていない定義ファイル（`emo2-kakukaku/descript.txt` 等）は
//!   原理的にヒットしない。2026-08-27 の棚卸しが実ゴースト定義を取りこぼしたのはこの理由。
//!   **語 grep は文言是正の網羅にだけ使い、対象の発見には使わない。**
//! - **基層だけを見て内外を判定してはならない。** `emo2-kakukaku` 系の基層は validrect が全 0
//!   （範囲 [0,0]・両端含む判定で origin 0 は「内」）だが、本番は必ず面別上書き層を重ねるため
//!   実範囲は sakura [36,356]×[46,168]／kero [24,240]×[40,133] で「外」になる。2026-08-27〜28 の
//!   設計判断がこの取り違えで `wplimit` を「不変」と誤判定した。
//! - **全緑は十分性の証拠にならない。** `actor_scale_refresh_tests.rs` の範囲外宣言はクランプ
//!   撤去後も緑のままであり、`emo2-kakukaku` の開始点は当時どのテストも固定していなかった。
//!
//! ## やり直しの手順（棚卸しを再実施するとき）
//!
//! 1. `origin.x`／`origin.y` の行頭アンカー付き全文検索（`target`／`.git`／`vendors`／`.kiro` を
//!    除外。**ファイルシステム側**を見ること——インデックス側だけだと未追跡ファイルを落とす）
//! 2. 網の妥当性を別マーカーで裏取り: `validrect.` を含むファイルを全列挙して
//!    「バルーン定義らしきファイル」を洗い、1 の結果を包含しているか見る
//! 3. in-code は `Origin::` の全構築点を列挙し `Some(..)` のものだけ残す
//! 4. 各宣言について、対応する面別上書き層（`balloons*s.txt`／`balloonk*s.txt`）を後勝ちで
//!    重ね、採用画像の PNG IHDR から原寸を読み、`region.rs` の `resolve_or`／`resolve_coord` の
//!    規則どおりに validrect 4 辺を計算する
//! 5. 解決値が範囲（**両端含む**）に入るかを x・y 独立に判定する
//! 6. 消費側の追随を `.origin()` と `TextRegion::resolve` の 2 本で洗う
//!    （前者が生値を見る檻・後者が解決結果を見る檻）

use std::path::{Path, PathBuf};

use areka_emo_text::region::TextRegion;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};

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

// ── fixture の所在（`CARGO_MANIFEST_DIR` 基準・crates/areka-emo-text から見た相対）───────
//
// アンカー規約は既存の先例に揃える（`areka-emo-present/src/balloon_test_support.rs` の
// `emo2_balloon_root`・`areka/src/emo2_boot/assets_tests.rs` の `emo2_root`）。

/// 原本の実ゴースト用バルーン定義ディレクトリ（`emo2` ツリー内）。
fn shipped_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku")
}

/// 複製 fixture（`windowposition-limit` の実機サインオフ用）。原本の兄弟ディレクトリに在り
/// `emo2` ツリーの外なのでゴーストツリーの列挙には影響しない（同 fixture の `readme.txt` 正典）。
fn wplimit_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-wplimit")
}

// ── 本番と同じ読み込み経路 ───────────────────────────────────────────────

/// バルーン記述ファイルを本番と同じ規約で読む。
///
/// 本番経路 `areka_emo_present::balloon::load_scope_balloon_model` は
/// `decode(&bytes, DefaultEncoding::Ansi)`（**既定 Ansi・ファイル内の `charset` 宣言優先**）で
/// 読む。`emo2-kakukaku/descript.txt` は `charset,UTF-8` を宣言しており、面別上書き層は
/// 宣言を持たないが純 ASCII ゆえどちらの既定でも同一である。ここで `DefaultEncoding::Utf8` を
/// 使わないのは、本番と 1 文字でも違う規約で読むと「檻は緑だが本番は別物」になり得るため。
///
/// **読み込み失敗は明示的に panic する**（パス解決の生存証明）。ファイルを読めなかったときに
/// 「対象 0 件だから緑」になる檻を作ってはならない。
fn read_layer(dir: &Path, name: &str) -> String {
    let path = dir.join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "出荷済みバルーン定義 {} の読取に失敗した（本檻はこのファイルの実在が前提）: {e}",
            path.display()
        )
    });
    assert!(
        !bytes.is_empty(),
        "出荷済みバルーン定義 {} が空である（空ファイルでは解決結果が檻の主張と無関係になる）",
        path.display()
    );
    decode(&bytes, DefaultEncoding::Ansi)
}

/// 指定ディレクトリの 2 層（`descript.txt` 基層＋面別上書き層）をマージした `BalloonModel`。
///
/// マージは本番と同じ `areka_parsers::balloon::parse_str(descript, Some(overlay))`
/// （権威経路の呼び方は `areka/src/emo2_boot/assets_tests.rs` と
/// `areka-emo-present/src/balloon.rs` の `load_scope_balloon_model` が固定している）。
fn merged_model(dir: &Path, overlay: &str) -> BalloonModel {
    parse_str(
        &read_layer(dir, "descript.txt"),
        Some(&read_layer(dir, overlay)),
    )
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

/// PNG の IHDR から原寸 `(width, height)` を読む（署名 8B ＋ 長さ 4B ＋ `IHDR` 4B の直後）。
///
/// 画像デコーダ（WIC）を持ち込まないのは、本檻を純粋層・非 GPU・非 COM に保つため
/// （要件 10.6）。IHDR はストリーム先頭に在ることが PNG 仕様で決まっているので固定
/// オフセットで足りる。
fn png_native_size(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("バルーン枠画像 {} の読取に失敗した: {e}", path.display()));
    assert!(
        bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR",
        "{} が PNG（先頭 IHDR チャンク付き）として読めない",
        path.display()
    );
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    (w, h)
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

/// 本檻が読む出荷ファイルが 2 ディレクトリとも実在し、非空であることを先に固定する。
///
/// これが無いと、パス解決が壊れた（あるいは fixture が移動した）ときに他の檻が
/// 「読めないから panic」ではなく静かに別の何かを測る形へ滑り込む余地が残る。
#[test]
fn shipped_fixture_files_exist_and_are_non_empty() {
    for dir in [shipped_root(), wplimit_root()] {
        for name in ["descript.txt", SAKURA_OVERLAY, KERO_OVERLAY] {
            let text = read_layer(&dir, name);
            assert!(
                !text.trim().is_empty(),
                "{} は空でないバルーン定義であること",
                dir.join(name).display()
            );
        }
        for name in ["balloons0.png", "balloonk0.png"] {
            let path = dir.join(name);
            assert!(path.is_file(), "{} が実在すること", path.display());
        }
    }
}

/// ハードコードした画像原寸が出荷 PNG の IHDR と一致する（期待値の二重帳簿を防ぐ）。
#[test]
fn native_sizes_match_shipped_png_headers() {
    for dir in [shipped_root(), wplimit_root()] {
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
    for dir in [shipped_root(), wplimit_root()] {
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
/// 「宣言が無く未宣言縮退で書字開始角へ落ちた結果」でもある同じ値なので、task 4.3／4.4 の
/// いずれを経ても不変である（モジュール doc「2 つの編集を跨いで…」参照）。
#[test]
fn shipped_sakura_scope_region_is_pinned() {
    let (sakura, _) = resolve_both(&shipped_root());
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
    let (_, kero) = resolve_both(&shipped_root());
    assert_region(
        &kero,
        KERO_EDGES,
        KERO_START,
        KERO_WRAP,
        "emo2-kakukaku scope 1 (kero)",
    );
}

// ── 檻 4: 複製 fixture の乖離検出（4.1 で判明した 5 件目）──────────────────

/// 複製 fixture `emo2-kakukaku-wplimit` が原本と**同一の領域解決**を与える。
///
/// 同 fixture の原本との差分は面別上書き層の `windowposition.*` 3 行のみ（`readme.txt` 正典・
/// 実測でも `descript.txt` と 2 枚の PNG は原本と 1 バイト差なし）。`windowposition` は
/// [`TextRegion`] の入力ではないため領域解決は完全に同値になるはずで、複製が原本から乖離
/// したらここで赤くなる。
#[test]
fn wplimit_copy_resolves_identically_to_shipped_original() {
    let (orig_sakura, orig_kero) = resolve_both(&shipped_root());
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
