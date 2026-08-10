use super::PathBuf;

// ---------------------------------------------------------------------------
// Constants / fixture paths
// ---------------------------------------------------------------------------

/// 意図的に誤った placeholder 窓寸（donor 必須逸脱 #3）。surface1000 の実合成寸とは必ず異なる値を選び、
/// 最終一致が本番 resize 経路（`resize_window_to`）を通ってしか成立しないことを構図で保証する。
pub(super) const PLACEHOLDER_SIZE: i32 = 100;

/// emo2 `surface1000` の Head 当たり判定矩形（`surfaces.txt`: `collision0,93,62,271,130,Head`・サーフェス px）。
pub(super) const HEAD_RECT: (i64, i64, i64, i64) = (93, 62, 271, 130);
/// emo2 `surface1000` の Bust 当たり判定矩形（`surfaces.txt`: `collision1,133,270,229,326,Bust`・サーフェス px）。
pub(super) const BUST_RECT: (i64, i64, i64, i64) = (133, 270, 229, 326);

/// 有効 bind 実値集合（donor 必須逸脱 #2・`emo-present.rs:170` 相当）。腕組み=1101・口‥‥=1206・
/// 目通常=1302・眉悲しみ=1502・髪飾りリボン=1800。`surface1000` は全パーツ bind 制御ゆえ既定 bind では
/// 全透明になる（狙える絵が出ない）。
pub(super) const REAL_BIND_IDS: [u32; 5] = [1101, 1206, 1302, 1502, 1800];

/// smoke 自動 close の env ゲート名（main.rs／他 example と同名・`AREKA_` 冠規約）。
pub(super) const SMOKE_EXIT_ENV: &str = "AREKA_APP_SMOKE_EXIT_MS";

/// 期待 k ゲートの env 名（`AREKA_` 冠規約・値は `"5/4"` 等の分数か `"2"` 等の整数）。
///
/// **未設定なら assert しない**（実測ログのみ）。開発機で k が何であっても probe をそのまま実行できる
/// ようにするためであり、実機サインオフの水準ごとには必ず設定して「その水準で本当にその k が適用された」
/// ことを probe 自身に hard assert させる（要件 4.1・design Error Handling「probe 期待 k 不一致」）。
pub(super) const EXPECT_K_ENV: &str = "AREKA_COLLISION_PROBE_EXPECT_K";

/// ④ anchor が写像後も矩形内側に確保すべき最小余裕（物理 px）。`scale_len` の丸め差（≤1px）と
/// 無関係に anchor 成立を保証するための下限（design CollisionProbe 節 #2「矩形内側 ≥2px」）。
pub(super) const ANCHOR_MARGIN_PX: u32 = 2;

/// [`ratio_parts`] の分母探索上限。実適用 k は `monitor_dpi / author_dpi` の既約有理であり、正典既定の
/// 作者 DPI 96 では分母は 96 の約数（≤96）にしかならない。桁違いの余裕を持たせた上限であり、
/// 超過時は探索を諦めて `None` を返す（ログ表現の縮退であって判定には一切関与しない）。
pub(super) const RATIO_PARTS_MAX_DEN: u32 = 4096;

/// emo2 fixture のゴーストルート（donor と同一アンカー規約）。
///
/// アンカーはコンパイル時に埋め込まれる `CARGO_MANIFEST_DIR`（＝`crates/areka` の**絶対パス**）であり、
/// 続く `../pilot/...` はそのアンカーからの相対要素にすぎない。ゆえに**プロセスの作業ディレクトリに
/// 依存せず常に絶対解決される**（要件 4.6・ヘッダ「# 起動時のパス（絶対パスの要否）」節）。
pub(super) fn emo2_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
}

/// emo2 fixture のバルーンルート（donor と同一規約）。
pub(super) fn balloon_root() -> PathBuf {
    emo2_root().join("emo2-kakukaku")
}
