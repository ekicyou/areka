//! 窓配置機構のモジュール土台。
//!
//! ゴースト定義（shell dir＋ghost/shell descript の KV）からキャラ窓・バルーン窓の
//! 初期配置を解決し、窓 entity を組み立てる配置パイプラインの器。
//! 配置パイプラインの座標・寸法はすべて物理 px 単一通貨とする（論理 DIP・`BoxStyle` は持ち込まない）。
//!
//! 依存の向き（この約束が名指すのは `config`・`resolver`・`measure`・`spawn`・`follow`・`diag`
//! の 6 つの向きだけで、ほかの 22 のモジュール（`test_support` を含む）は互いに広く引き合う）:
//! `config`（areka-parsers・tracing）← `resolver`（std・tracing・wintf の DPI・`config`）←
//! `measure`（emo-atlas・emo-compose・emo-present のほか、`error`・`descript_window_keys` を引く）←
//! `spawn`／`follow` と同じ段のモジュール（wintf・bevy_ecs ほか・同じ段どうしは引き合ってよい）←
//! 本体のシーム。左のモジュールは右を import しない。
//! [`diag`] は最下流: 配置のモジュールを 1 つも引かない（ほかの crate からは bevy_ecs の
//! `Entity`・wintf の `ZOrderPairStrategy` と `transition_diag::MISSING`・tracing を引く）。
//! 失敗の型 [`PlacementError`] は `error.rs`（配置のモジュールを引かない）に置き、根から出し直す。
//!
//! 同じ段には輪がある:
//! - `chain_finalize`↔`chain_realign`（`chain_finalize` の `on_insert` の hook が `chain_realign` を引く）
//! - `chain_realign`→`spawn`→`chain_finalize`→`chain_realign`
//! - `follow` と同じ段のモジュール（`balloon_limit`・`dpi_sync`・`persist`・`spawn`・`sticky_group_ledger`・
//!   `transition_diag`）との両向き
//!
//! 根を引く子もある: `reset_pos`→根の `primary_monitor`・`square_dpi`・`work_area_px`、
//! `reseed`→根の `scale_then_merge`・
//! `ScopeWindowPositions`・`follow/offset_space`・`follow/move_shift`→根の `scale_signed`、
//! `follow/work_area`・`transition_diag`→根の `work_area_px`、`balloon_limit`・`persist`・`reseed`→`follow` の `rect_at`。
//!
//! 本ファイルはサブモジュール宣言・失敗型 [`PlacementError`] の出し直しに加え、
//! source→config→measure→resolver を束ねる配置準備の合成ルート [`prepare_ghost_windows`]
//! を持つ（シームの結線自体は `areka-session` の `ghost_session::prepare_ghost_windows`）。

pub mod balloon_limit;
/// バルーンの動かせない指定の帳簿と、バルーンの窓の `DragConfig.move_window` の書き手。
pub mod balloon_lock;
pub mod balloon_syncscale;
pub mod chain_finalize;
/// DPI／拡大率の遷移後に連鎖を一度だけ解き直す機構。
pub mod chain_realign;
/// 鎖の規則（起動の確定・隣り合いの印の保存・拡大率の後の詰め直しが同じ規則で組を作る）。
pub mod chain_rule;
pub mod config;
/// descript の既定の面（`*.seriko.defaultsurface`）の読みと戻り先の候補の並び。
pub mod descript_window_keys;
pub use self::descript_window_keys::DefaultSurfaces;
pub mod diag;
mod error;
pub use self::error::PlacementError;
/// 窓ごとの整合ゲート（窓の拡大率とモニタ別拡大率表が揃うまで窓書込を見送る）。
pub mod dpi_sync;
pub mod face_windowposition;
pub mod follow;
pub mod measure;
pub mod persist;
/// シェルの差し替えで、新しいシェルの見た目の値を走っている窓の部品へ入れ直す。
pub mod reseed;
/// バルーン位置初期化。
pub mod reset_balloon_pos;
/// 既定の位置への並べ直し。
pub mod reset_pos;
pub mod resolver;
/// 台本からの吸着の向きの変更。
pub mod script_alignment;
/// 台本のバルーン位置の値の型と引数の読み。
pub mod script_balloon;
pub mod source;
pub mod spawn;
/// 窓の組（`\![set,sticky-window,...]`・descript の `seriko.sticky-window`）の帳簿と World の側の関数。
pub mod sticky_group_ledger;
pub mod transition_diag;
mod windowposition;
/// 台帳のグループと窓の在庫から「望む鎖」を組み立てる純関数（World も Win32 も触らない）と、
/// その鎖を受け口へ置く `publish_chain_plan`（World に触るのはこれだけ）。
pub mod zorder_chain_compose;
/// スコープ窓 Z 順グループの台帳と、タグ／descript 共通のトークン解釈・拒否判定
/// （純関数・Win32／ECS 非依存）。
pub mod zorder_group_ledger;

/// 作者空間の符号付きオフセットを k 倍する唯一の写像（大きさは `ScaleRatio::scale_len`
/// 権威へ委譲し符号のみ保存する）。`windowposition.x/y` と `\![move]` の dx/dy は
/// どちらも「作者基準 px で書かれた画面オフセット」なので同じ写像を通す
/// （片方だけ素通しにすると高 DPI で両者の意味論が割れる）。
pub use windowposition::scale_signed;
/// 見本のゴーストの入口（配置のテストだけ）。
#[cfg(test)]
mod sample_test_support;
/// テストの実行ファイルの落ちの報告とプロセスの寿命の MTA。
#[cfg(test)]
mod test_crash_report;
/// 配置の外のテストも使う支え。本体は dev-dependency でだけ `test-support` を付けるので、
/// 本番のビルドには含まれない。
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
/// Z 順のテストが共に使う道具（台帳・鎖の合成のテストの写しを 1 つにまとめた）。
#[cfg(test)]
pub(crate) mod zorder_test_support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use areka_emo_compose::ScaleRatio;
use areka_emo_present::balloon::{
    ResolvedFace, load_scope_balloon_model, load_scope_balloon_model_quiet,
    resolve_balloon_faces_quiet,
};
use areka_parsers::balloon::{BalloonModel, WindowPosition, WindowPositionRaw};
use tracing::{error, info, warn};
use wintf::ecs::DPI;
use wintf::ecs::window::monitor::{Monitor, enumerate_monitors};

use self::config::{BalloonXMode, PlacementConfig};
use self::measure::{MeasureScaling, MeasuredSizes};
use self::resolver::{RectPx, ScopePlacement};
use self::source::GhostTitles;
use self::windowposition::{LimitVocab, XVocab, classify_limit_vocab, classify_x_vocab};

/// primary モニタ DPI を取得できないときに採る「96 相当」の DPI。
///
/// この縮退は恒久的な情報損失ではない。窓生成後は窓の実 DPI が正であり、
/// `Changed<DPI>` を観測する `areka_runtime::frame::dpi::run_dpi_phase` と
/// 表示成立点の状態照合が k を自己補正する。
const FALLBACK_PRIMARY_DPI: u32 = 96;

/// `MonitorSnapshot` 構築点（`main.rs`）の呼出点タグ。
///
/// モニタ構成の正典出力点は placement の全判断が読む権威 [`follow::MonitorSnapshot`] の
/// 構築点（列挙の忠実転写点＝以後の判断が実際に見る値そのもの）で、本タグはその出所を名乗る。
pub const MONITOR_SNAPSHOT_CONTEXT: &str = "monitor_snapshot";

mod test_open {
    /// [`prepare_ghost_windows`](crate::prepare_ghost_windows) のモニタ列挙点の呼出点タグ。
    ///
    /// 同一運転内に列挙点は複数ある。同じ共有ヘルパ [`diag::log_monitor_snapshot`](crate::diag::log_monitor_snapshot) を
    /// 呼びつつタグだけを違えることで、ログ上で出所を弁別でき、同じ語彙なので
    /// grep 突合で構成の食い違いを検出できる。
    pub const PREPARE_GHOST_WINDOWS_CONTEXT: &str = "prepare_ghost_windows";
}
// 外へはテストのビルド（`test-support`）でだけ開く。crate の中は列挙点が使う（道と名前は同じ）。
#[cfg(feature = "test-support")]
pub use test_open::PREPARE_GHOST_WINDOWS_CONTEXT;
#[cfg(not(feature = "test-support"))]
pub(crate) use test_open::PREPARE_GHOST_WINDOWS_CONTEXT;

/// 作業領域源の実行時同期（`areka_runtime::frame::work_area_sync`）の呼出点タグ。
///
/// 起動時の構築点（[`MONITOR_SNAPSHOT_CONTEXT`]）と列挙点（`PREPARE_GHOST_WINDOWS_CONTEXT`）に
/// 続く 3 つ目の出所。同じ共有ヘルパ [`diag::log_monitor_snapshot`] を呼びタグだけを違えるので、
/// 起動時の構成と同期後の構成が同じ語彙で並び、どのフレームで何が変わったかを grep 突合で追える。
///
/// 同期の警告（モニタ 0 台）も同じ語を行頭に名乗る（`[work_area_sync] …`）。
pub const WORK_AREA_SYNC_CONTEXT: &str = "work_area_sync";

/// wintf のモニタ列挙結果を診断レコードへ忠実転写する純関数。
///
/// [`diag`] は placement の最下流で `World`・wintf の `Monitor` を知らない。
/// wintf `Monitor` を既に import している本 `lib.rs` が「wintf 型 → 純データ」の唯一の境界になる。
///
/// - 単位変換も丸めも行わない（配置パイプラインの座標は物理 px 単一通貨）
/// - 列挙順を保つ（`index` がログ上の同定子になる）
/// - 実モニタを要さない純関数なので、混在 DPI・負座標・3200 超座標の構成を決定論檻で踏める
pub fn monitor_records(monitors: &[Monitor]) -> Vec<diag::MonitorRecord> {
    monitors
        .iter()
        .map(|m| diag::MonitorRecord {
            handle: m.handle,
            bounds: (m.bounds.left, m.bounds.top, m.bounds.right, m.bounds.bottom),
            work_area: (
                m.work_area.left,
                m.work_area.top,
                m.work_area.right,
                m.work_area.bottom,
            ),
            dpi: m.dpi,
            is_primary: m.is_primary,
        })
        .collect()
}

/// 起動時 k₀ の分母となる作者基準 DPI の対。
///
/// descript 読取は `prepare_stages_for_shell` で 1 度だけ行い（shell＝
/// `DescriptSource::shell_author_dpi()`・balloon＝[`source::load_balloon_author_dpi`]）、
/// その値を本型に束ねて 2 つの消費者へ配る:
///
/// 1. 採寸の k₀（[`build_measure_scaling`] → `measure_scope_sizes`）
/// 2. attach の target 政策（[`PreparedPlacement::author_dpi`] → 本体の `ghost_session` →
///    `areka_runtime::wire_runtime` → `build_boot_assets` → `attach_target`）
///
/// 同じ宣言が 2 経路で食い違わないこと（＝読取が 1 度であること）が本型の存在理由。
///
/// shell と balloon は別々のキー（`seriko.dpi`／`dpi`）で宣言され得るうえ、
/// 素の `u16` 2 引数は取り違えてもコンパイルが通る。名前付きフィールドで束ね、
/// 呼び手に「どちらの `u16` か」を選ばせない（`areka_runtime::frame::dpi` の `AuthorDpis` と同じ防御）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorDpi {
    /// shell descript `seriko.dpi` 由来（縮退梯子適用済みの非ゼロ値）。
    pub shell: u16,
    /// balloon descript `dpi` 由来（同上）。
    pub balloon: u16,
}

impl AuthorDpi {
    /// 正典既定（96/96）。今の使い手はテストの組み立てだけ（準備に失敗した本番の道はこの値を使わない）。
    pub const DEFAULT: AuthorDpi = AuthorDpi {
        shell: source::DEFAULT_AUTHOR_DPI,
        balloon: source::DEFAULT_AUTHOR_DPI,
    };
}

/// placement 側の同期準備一括の結果。
///
/// I/O は [`prepare_ghost_windows`] までで完結し、Send な素の値のみを運ぶ
/// （`Vec<ScopePlacement>`＝Copy 値の列・`GhostTitles`＝`BTreeMap<usize, String>`。
/// COM/WIC 等のスレッド親和リソースは持たない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlacement {
    /// スコープごとの解決済み配置（`resolve_placement` の出力そのまま・scope 昇順）。
    pub placements: Vec<ScopePlacement>,
    /// 窓タイトルの正本（spawn が消費）。
    pub titles: GhostTitles,
    /// 採寸 k₀ に使った作者基準 DPI。
    ///
    /// 同じ読取結果を attach 側（`attach_target`）へ配るための搬送口（[`AuthorDpi`] の doc）。
    pub author_dpi: AuthorDpi,
    /// shell descript の `seriko.zorder` の生の値（未指定なら `None`）。
    ///
    /// `author_dpi` と同じ搬送の形。descript を読むのは準備の中の 1 度だけなので、
    /// 重なりの基底もそのときの読取結果をここへ載せて本体の `ghost_session` → `wire_runtime` → 台帳へ配る。
    /// 結線の側で読み直すと、配置と重なりが別々の宣言を見る余地が生まれる。
    ///
    /// 値の解釈は placement では行わない（`PlacementConfig::zorder_raw` の生の転記）。
    /// 窓の位置も寸法もこの値では動かない。
    pub zorder_raw: Option<String>,
    /// shell descript の `seriko.sticky-window` の生の値（未指定なら `None`）。`zorder_raw` と同じ
    /// 搬送の形で、窓を作る閉包が窓の組の帳簿へ据える。
    pub sticky_window_raw: Option<String>,
    /// 標準の位置の材料。
    ///
    /// 窓の組を作った後にそのまま置く。読み込んだ descript の値とバルーンの `windowposition`・
    /// 作者の DPI を運ぶ（ファイルを読み直さない）。シェルの切り替えの後始末と同じ形で、spawn は読まない。
    pub default_basis: reset_pos::DefaultPlacementBasis,
    /// 測りに使った descript の既定の面（資産の最初の面と読み直しの戻り先へ同じ値を配る）。
    pub default_surfaces: DefaultSurfaces,
    /// descript の段がドラッグを禁止するスコープ（窓を作る閉包が帳簿へ据える）。
    pub balloon_dontmove: BTreeSet<u32>,
}

/// 準備パイプラインの中間結果（load→config→measure まで・work area 非依存部）。
///
/// [`prepare_ghost_windows`]（実モニタ列挙）と
/// [`prepare_ghost_windows_with_work_area`]（合成 work area 注入・決定論テスト用）
/// が resolve 直前まで同一経路を共有する。
struct PreparedStages {
    cfg: PlacementConfig,
    sizes: MeasuredSizes,
    titles: GhostTitles,
    author_dpi: AuthorDpi,
    /// 標準の位置の材料（読み込んだ値の写し・[`PreparedPlacement::default_basis`]）。
    default_basis: reset_pos::DefaultPlacementBasis,
    /// 採寸に使った表示 DPI（[`build_measure_scaling`] が k₀ の分子へ採った値）。
    ///
    /// 起動時の主モニタ DPI であって、窓がいま載っているモニタの DPI ではない
    /// （意図された挙動）。非主モニタで生まれた窓は最初の `Changed<DPI>` で
    /// 主モニタ空間から実モニタ空間へ引き直される。
    measure_dpi: DPI,
    /// 測りに使った descript の既定の面（[`PreparedPlacement::default_surfaces`]）。
    default_surfaces: DefaultSurfaces,
    /// descript の段の動かせない指定（[`PreparedPlacement::balloon_dontmove`]）。
    balloon_dontmove: BTreeSet<u32>,
}

impl PreparedStages {
    /// work area（物理 px）を与えて配置を確定する（純粋・resolver P1〜P5）。
    fn resolve(self, work_area: RectPx) -> PreparedPlacement {
        let placements =
            resolver::resolve_placement(&self.cfg, work_area, &self.sizes.scopes, self.measure_dpi);
        PreparedPlacement {
            placements,
            titles: self.titles,
            author_dpi: self.author_dpi,
            // 解釈せずそのまま搬送する（重なりの基底の出所はこの読取 1 つ）。
            zorder_raw: self.cfg.zorder_raw,
            sticky_window_raw: self.cfg.sticky_window_raw,
            default_basis: self.default_basis,
            default_surfaces: self.default_surfaces,
            balloon_dontmove: self.balloon_dontmove,
        }
    }
}

/// 起動時の表示スケール k₀ を導出する。
///
/// `k₀ = primary モニタ DPI ÷ 作者基準 DPI` を shell／balloon それぞれの
/// 作者基準 DPI で別々に求める（2 軸は独立）。丸めは行わない。
/// 既約有理数 [`ScaleRatio`] のまま [`MeasureScaling`] に載せ、寸法の乗算と丸めは
/// 単一権威 `ScaleRatio::scaled_extent` だけが行う（`as_f32` を寸法計算に使わない）。
///
/// # 縮退
///
/// `primary_dpi` が `None`（モニタ 0 台）または 0（列挙異常）のとき、`error!` のうえ
/// [`FALLBACK_PRIMARY_DPI`]（96 相当）から k₀ を導く。作者基準 DPI が 96 なら k₀=1/1＝
/// native 採寸、96 以外を宣言していればその宣言に対する 96 相当の比（例: author=192 → k₀=1/2）となる。
/// 0（列挙異常）の腕では窓は生え、窓生成後に窓の実 DPI が
/// `Changed<DPI>` → `refresh_scale`＋窓寸 reconcile で k を自己補正する。
/// この縮退は回復可能で、表示の喪失にはならない。
/// `None` の腕は、本番の [`prepare_ghost_windows_for_shell`] ではモニタ 0 台のときだけ起き、
/// 採寸の直後に [`work_area_of`] が `Err(PlacementError::Monitor)` を返すので窓は生えない
/// （窓が生えるのは作業領域を外から渡すテストの入口だけ）。
///
/// # 戻り値の第 2 要素＝採寸 DPI
///
/// k₀ の分子へ実際に採った表示 DPI（`primary_dpi` が `Some(d)` かつ `d > 0` なら `d`、
/// それ以外は縮退値 [`FALLBACK_PRIMARY_DPI`]）をそのまま返す。内部で決めて捨てると、
/// 追従オフセットの基準 DPI を作る側が同じ判断をもう一度書くことになり、片方だけ直した
/// ときに採寸 k₀ と基準 DPI が静かに食い違う。単一の決定点をここに置く。
fn build_measure_scaling(primary_dpi: Option<u32>, author_dpi: AuthorDpi) -> (MeasureScaling, u32) {
    let dpi = match primary_dpi {
        Some(dpi) if dpi > 0 => dpi,
        unobtainable => {
            error!(
                primary_dpi = ?unobtainable,
                fallback_dpi = FALLBACK_PRIMARY_DPI,
                shell_author_dpi = author_dpi.shell,
                balloon_author_dpi = author_dpi.balloon,
                "placement: primary モニタ DPI を取得できない——96 相当で k₀ を導いて採寸を続行する\
                 （窓が生えれば窓の実 DPI が k を自己補正する。モニタ 0 台なら後で作業領域が無く失敗する・要件 1.4）"
            );
            FALLBACK_PRIMARY_DPI
        }
    };
    // 軸ごとに自分の作者基準 DPI で割る（shell の k を balloon へ漏らさない）。
    let ratio = |author: u16, axis: &'static str| {
        ScaleRatio::new(dpi, u32::from(author)).unwrap_or_else(|| {
            // 到達＝上流の縮退梯子（`parse_author_dpi` は常に非ゼロを返す）が破れた場合のみ。
            // 無言で落とさず観測可能にしたうえで恒等へ縮退する（log-first・表示を失わない）。
            error!(
                axis,
                primary_dpi = dpi,
                author_dpi = author,
                "placement: k₀ を有理数化できない（0 を含む入力）——恒等 k=1/1 へ縮退する"
            );
            ScaleRatio::ONE
        })
    };
    let scaling = MeasureScaling {
        shell: ratio(author_dpi.shell, "shell"),
        balloon: ratio(author_dpi.balloon, "balloon"),
    };
    // 起動時 k₀ の観測点（表示成立点ログと同じ語彙・RUST_LOG grep 対象）。
    // `k_shell`/`k_balloon`（f32）は grep 用の出口ビューで、寸法計算には用いない。
    info!(
        primary_dpi = dpi,
        shell_author_dpi = author_dpi.shell,
        balloon_author_dpi = author_dpi.balloon,
        k_shell = scaling.shell.as_f32(),
        k_balloon = scaling.balloon.as_f32(),
        k_shell_ratio = ?scaling.shell,
        k_balloon_ratio = ?scaling.balloon,
        "placement: 起動時 k₀ を導出（primary モニタ DPI ÷ 作者基準 DPI・D7）"
    );
    (scaling, dpi)
}

/// 準備パイプラインの work area 非依存部を同期実行する:
/// `load_descript_source_for_shell` → `build_placement_config` → `measure_scope_sizes`。
///
/// 失敗は [`PlacementError`] のまま呼び手へ返す（告知と、終了するかどうかは呼び手の分担）。
/// 位置の記憶・復元（`profile/areka` の `sylphya.toml` の読み書き）は一切行わない。
///
/// `primary_dpi` は起動時 k₀ の分子（primary モニタ DPI・物理 DPI 値）。取得不能
/// （モニタ 0 台等）は `None` を渡す。[`build_measure_scaling`] が `error!`＋96 相当へ縮退させる。
///
/// `shell` は [`source::load_descript_source_for_shell`] へそのまま渡す。
fn prepare_stages_for_shell(
    ghost_root: &Path,
    balloon_root: &Path,
    primary_dpi: Option<u32>,
    shell: Option<&str>,
) -> Result<PreparedStages, PlacementError> {
    let src = source::load_descript_source_for_shell(ghost_root, shell)?;
    let mut cfg = config::build_placement_config(&src.ghost_kv, &src.shell_kv);
    let scope_ids: Vec<usize> = cfg.scopes.keys().copied().collect();
    // descript の既定の面と動かせない指定もここで 1 度だけ読む（測りと下流へ同じ値を配る）。
    let default_surfaces = descript_window_keys::read_default_surfaces(&src.ghost_kv);
    let balloon_dontmove =
        descript_window_keys::read_balloon_dontmove(&src.ghost_kv, &src.shell_kv, &scope_ids);
    // 作者基準 DPI はここで 1 度だけ読む。shell は読み込み済みの生 KV から、balloon は別パッケージ
    // なので `load_balloon_author_dpi` の 1 回の寛容読取から得る。
    // 得た値は採寸 k₀ と attach（`PreparedPlacement::author_dpi` 経由）の双方へ配られる。
    let author_dpi = AuthorDpi {
        shell: src.shell_author_dpi(),
        balloon: source::load_balloon_author_dpi(balloon_root),
    };
    // 窓寸の k 倍は採寸の源で吸収され、`spawn.rs` は k 倍済み `SizePx` を consume するのみ。
    let (scaling, measure_dpi) = build_measure_scaling(primary_dpi, author_dpi);
    let sizes = measure::measure_scope_sizes(
        &src.shell_dir,
        balloon_root,
        &scope_ids,
        &default_surfaces,
        &scaling,
    )?;
    // 窓生成へ渡る k₀ 倍後の物理寸を載せる。非 96 環境では k_shell/k_balloon が 1.0 以外になり、
    // scopes の寸が原寸の k₀ 倍であることを RUST_LOG の grep だけで判定できる。
    info!(
        k_shell = scaling.shell.as_f32(),
        k_balloon = scaling.balloon.as_f32(),
        scopes = ?sizes.scopes,
        "placement: k₀ 倍後の物理窓寸で窓を生成する（起動採寸・要件 3.3）"
    );
    // 採寸の後に scope ごとの `windowposition` を権威から取得し、作者空間 → 物理 px の換算の
    // 後で `ScopeConfig.balloon_offset`（P5 が既に加算している欄）へ合流させる。
    // 配置解決の式 P1〜P5 と永続値優先の復元規約（`persist.rs`）には触れない。
    // 読んだ値は標準の位置の材料として返す（読み直さずに運ぶ）。
    // 2 層マージの記録は、同じ定義を組む起動時資産構築が出すので、ここでは出さない。
    let windowpositions =
        load_scope_windowpositions_with(balloon_root, &scope_ids, load_scope_balloon_model_quiet);
    scale_then_merge(
        &mut cfg,
        &windowpositions,
        &scope_ids,
        scaling.shell,
        scaling.balloon,
    );
    let titles = src.titles.clone();
    Ok(PreparedStages {
        cfg,
        sizes,
        titles,
        author_dpi,
        default_basis: reset_pos::DefaultPlacementBasis {
            source: src,
            balloon: reseed::BalloonPlacementInputs {
                author_dpi: author_dpi.balloon,
                windowpositions,
            },
        },
        // 追従オフセットの基準 DPI。k₀ の分子は軸によらず 1 つなので両軸に同じ表示 DPI を置く
        // （軸差は分母＝作者基準 DPI にある）。
        measure_dpi: square_dpi(measure_dpi),
        default_surfaces,
        balloon_dontmove,
    })
}

/// 供給時の飽和を記録する語。
///
/// 語彙は綴りごと契約で、実機ログの grep と決定論テストが同じ定数を参照する
/// （`balloon_limit.rs` の `*_TAG` と同じ流儀）。
pub(crate) const BALLOON_OFFSET_SATURATED_TAG: &str = "[balloon-offset] Saturated";

/// `descript` の `balloon.offsetx`／`offsety`（作者空間の生値）を、合流欄の空間である
/// 物理 px へその場で置き換える。
///
/// # 呼び出し位置
///
/// [`scale_then_merge`] が `windowposition` の合流（[`merge_scope_windowpositions`]）の直前に
/// 呼ぶ。合流欄 `ScopeConfig.balloon_offset` は加算欄で、`windowposition` 由来の調整量は既に
/// バルーン軸の k を掛けた物理 px として合流する。本ステップが先に走ることで、加算の
/// 時点で両辺が同じ空間に揃う。逆順にすると windowposition の調整量にもシェル軸の k が
/// 掛かって二重換算になる。
///
/// # 換算軸
///
/// `k` はシェル軸（`MeasureScaling::shell`）。`balloon.offsetx`／`offsety` は
/// ゴースト／シェルの `descript.txt` の語彙＝シェル作者の空間で書かれた値だから。
/// 割り当ての一覧は契約の定義元 `follow/offset_space.rs` のモジュール doc が持つ。
///
/// # 受理規約は変えない
///
/// どの軸を採るか（軸ごとに決まり、書かれない軸は 0）は `config.rs` の `resolve_scope` が持ち、
/// 本ステップは触らない。`None`（どちらの軸も未宣言）はそのまま `None` で通す
/// （非該当であって縮退ではないので記録もしない）。
///
/// # 拡大率 1
///
/// `k` が恒等なら [`follow::scale_author_offset`] は生値を素通しする（丸め権威
/// `ScaleRatio::scale_len` の恒等素通し）。
/// `i32::MIN` も素通しになる（`windowposition` 由来の調整量も同じ。檻
/// `supply_identity_passes_through_the_whole_range` が固定）。
///
/// # 飽和と縮退
///
/// `i32` 域を超えた軸は回り込ませず飽和値（`±i32::MAX`）を採り、scope・軸・生値・
/// 飽和後の値を [`BALLOON_OFFSET_SATURATED_TAG`] の `warn!` で記録する。
/// `k` そのものを解決できない場合は既存の単一縮退点（[`build_measure_scaling`] の
/// `error!`＋恒等縮退）が記録済みなので、重複した警告を新設しない。
fn apply_author_balloon_offset_scale(
    cfg: &mut PlacementConfig,
    scope_ids: &[usize],
    k: ScaleRatio,
) {
    for &scope in scope_ids {
        let Some(sc) = cfg.scopes.get_mut(&scope) else {
            // `scope_ids` は `cfg.scopes` の鍵から作られるため到達しない。それでも
            // 無言で飛ばさない。
            warn!(
                scope,
                "placement: 配置表に無い scope の作者オフセット換算要求を無視した"
            );
            continue;
        };
        // どちらの軸も未宣言なら `None`（受理規約は `config.rs` の所有）。
        // 非該当であって縮退ではないので記録しない。
        let Some(raw) = sc.balloon_offset else {
            continue;
        };
        let (x, y) = follow::scale_author_offset(raw, k);
        warn_if_saturated(scope, "x", raw.0, x);
        warn_if_saturated(scope, "y", raw.1, y);
        sc.balloon_offset = Some((x.value, y.value));
        sc.balloon_offset_descript_x = x.value;
    }
}

/// 飽和した軸を 1 本ぶん記録する（scope・軸・生値・飽和後の値）。
fn warn_if_saturated(scope: usize, axis: &'static str, raw: i32, scaled: follow::ScaledAxis) {
    if !scaled.saturated {
        return;
    }
    warn!(
        scope,
        axis,
        raw,
        saturated_value = scaled.value,
        "{BALLOON_OFFSET_SATURATED_TAG} descript のバルーンオフセットの換算が i32 域を超えた\
         ——回り込ませず飽和値を採る（要件 2.5）"
    );
}

/// 作者の空間のバルーンのずらしをシェル軸で物理 px へ換算し（[`apply_author_balloon_offset_scale`]）、
/// 続けてバルーンの `windowposition` を合流する（[`merge_scope_windowpositions`]）。起動の準備と
/// `reseed` の `resolve_standard_scopes` の 2 か所が呼ぶ（後者はシェルの切り替え・面の `windowposition`
/// の置き直し・バルーン位置初期化が通る。順の決まりを書く所はここだけ）。
///
/// 順を入れ替えると `windowposition` の調整量にもシェル軸の k が掛かって二重に換算される
/// （合流欄 `ScopeConfig.balloon_offset` は加算欄で、加算の時点で両辺が物理 px でなければならない）。
pub(crate) fn scale_then_merge(
    cfg: &mut PlacementConfig,
    windowpositions: &ScopeWindowPositions,
    scope_ids: &[usize],
    k_shell: ScaleRatio,
    k_balloon: ScaleRatio,
) {
    apply_author_balloon_offset_scale(cfg, scope_ids, k_shell);
    merge_scope_windowpositions(cfg, windowpositions, scope_ids, k_balloon);
}

/// scope ごとに読んだバルーンの `windowposition`（数値解釈と生値の対）。読めなかった scope は
/// 載らない（見送りの `warn!` は読んだ側が出している）。
pub(crate) type ScopeWindowPositions = BTreeMap<usize, (WindowPosition, WindowPositionRaw)>;

/// バルーンの `windowposition` を scope ごとに読む（ファイルを読む）。2 層マージの記録も出す。
/// 合流は [`scale_then_merge`]。
///
/// シェルの差し替えは、これを背景のスレッドで呼んで結果を UI へ運び、UI では読む半分を通さずに
/// [`scale_then_merge`] だけを通す。
///
/// # 失敗
///
/// 系列が解決できない scope（フォルダが読めない・面 0 が無い）は `warn!` のうえ供給を見送る
/// （配置自体は基本位置で成立する）。起動の準備では、同じ事象を直前の採寸（`measure_scope_sizes`）が
/// 既に `error!`＋`Err` で弾いている。シェルの差し替えはバルーンを測り直さないので採寸の記録が無く、
/// フォルダが読めないときに scope 付きで残るのはこの `warn!` だけになる（面 0 が無いときは系列の
/// 解決が scope 付きの `error!` を別に出す）。
pub fn load_scope_windowpositions(
    balloon_root: &Path,
    scope_ids: &[usize],
) -> ScopeWindowPositions {
    load_scope_windowpositions_with(balloon_root, scope_ids, load_scope_balloon_model)
}

fn load_scope_windowpositions_with(
    balloon_root: &Path,
    scope_ids: &[usize],
    merge: fn(&Path, u32, &ResolvedFace) -> BalloonModel,
) -> ScopeWindowPositions {
    scope_ids
        .iter()
        .filter_map(|&scope| scope_windowposition(balloon_root, scope, merge).map(|wp| (scope, wp)))
        .collect()
}

/// scope ごとの表示位置指定（`windowposition` 数値指定）を初期既定位置の調整量へ変換し
/// `cfg.scopes[scope].balloon_offset` へ合流させる。
///
/// 供給元を増やすだけの層で、`resolver.rs` の配置式 P1〜P5 は変えず、P5 が既に
/// 加算している `balloon_offset.unwrap_or((0, 0))` の入力が増える。ゆえに恒等式
/// `balloon_offset ≡ balloon_pos − char_pos` も、キャラ窓の基準原点も、位置の保存・復元の
/// 基準（`persist.rs`・永続値優先）も変わらない。
///
/// `k` はバルーン軸の表示スケール（`MeasureScaling::balloon`）。`windowposition` は
/// バルーン作者の空間で書かれた値なので、シェル軸の k を掛けてはならない（2 軸独立）。
/// 大きさの丸めは既存権威 `ScaleRatio::scale_len` へ委譲される。
///
/// # 語彙の解決
///
/// 同じ 2 層マージ済み定義から生値（[`WindowPositionRaw`]）も取り出し、[`classify_limit_vocab`]／
/// [`classify_x_vocab`] の分類結果を `ScopeConfig.balloon_limit`／`balloon_x_mode` へ反映する。
/// 分類器は警告を持たない。不正値の警告は scope 文脈を持つ本層の所有で、
/// 警告の無い縮退経路を 1 本も残さない。
///
/// # 観測
///
/// scope ごとに `info!` で scope・wp 生値・バルーンの左右・変換後の調整量（物理 px）と、
/// 解決済みの limit 値・水平配置モードの実値を記録する。
/// x 方向の符号規約は SSP 実測と突合して確定済みで、確定形は「左右で反転しない素の画面座標オフセット」
/// （`windowposition.rs` の `to_screen_adjust`「符号規約」節）。`balloon_side` は調整量の計算には
/// 効かないが、基本位置（Left＝`char_x − balloon_w`／Right＝`char_x + char_w`）がどちらだったかを
/// 実機ログだけで再構成できるよう記録し続ける。
///
/// 純関数（ファイルは読まない。読むのは [`load_scope_windowpositions`]）。
fn merge_scope_windowpositions(
    cfg: &mut PlacementConfig,
    windowpositions: &ScopeWindowPositions,
    scope_ids: &[usize],
    k: ScaleRatio,
) {
    for &scope in scope_ids {
        // 語彙の解決値（limit／x_mode）の合流先も同じ `ScopeConfig` なので、可変で 1 度だけ引く。
        // 未収載 scope には合流先が無いので、ここで供給を見送る判定も兼ねる。
        let Some(sc) = cfg.scopes.get_mut(&scope) else {
            warn!(
                scope,
                "placement: 配置表に無い scope の windowposition 供給要求を無視した"
            );
            continue;
        };
        // バルーンの左右は配置構成の解決済み値（`balloon.alignment`・cascade 済み）。
        // 調整量の計算には効かない（符号は置き側に依らない）が、観測の記録項目。
        let side = sc.balloon_alignment;
        let Some((wp, wp_raw)) = windowpositions.get(&scope) else {
            // 定義を読めなかった scope は `ScopeConfig` の正典既定（limit=1・`Side`）のまま。
            // 見送りの warn は `scope_windowposition` が出している。
            continue;
        };
        let wp = *wp;
        let (balloon_limit, x_mode, wp_x) = resolve_windowposition_vocab(scope, wp_raw, wp.x());
        let wp_y = wp.y();
        // limit も x も「その scope が採用した面の 2 層マージ結果」という同一単位で解決される。
        sc.balloon_limit = balloon_limit;
        sc.balloon_x_mode = x_mode;
        // キーワード指定のとき `wp_x` は `None` なので、`to_screen_adjust(None, wp_y)` がそのまま `(0, dy)` を供給する。
        let adjust = windowposition::to_screen_adjust(wp_x, wp_y, k);
        // 数値指定なし（`adjust=None`）でも 1 行出す。「読んだが指定が無かった」ことと
        // 「そもそも読めなかった」ことを `adjusted` で区別できる。
        // `limit`／`x_mode` は縮退後の解決済みの値。
        let (adjust_dx, adjust_dy) = adjust.unwrap_or((0, 0));
        info!(
            scope,
            windowposition_x = ?wp_x,
            windowposition_y = ?wp_y,
            balloon_side = ?side,
            adjust_dx,
            adjust_dy,
            adjusted = adjust.is_some(),
            k = k.as_f32(),
            limit = balloon_limit,
            x_mode = ?x_mode,
            "placement: windowposition を初期既定位置の調整量へ変換した（scope 別・要件 3.2/7.6）"
        );
        windowposition::apply_windowposition(sc, adjust);
    }
}

/// `windowposition` の語彙（limit／x）を解決し、不正値を scope 文脈つきの警告を出したうえで
/// 正典既定へ縮退させる。
///
/// 戻り値は `(limit 解決値, 水平配置モード, 調整量計算へ渡す数値 x)`。
///
/// - limit: `0`/`1` は受理・未指定は正典既定 `true`・それ以外は warn（scope・生値）→ `true`。
/// - x: 数値・未指定は `Side` ＋ 生値を見ない。
///   キーワードは対応する [`BalloonXMode`]（数値 x は存在しないので `None`）。
///   それ以外は warn（scope・生値）→ 未指定扱い（`Side`・`None`）。
///
/// 警告を出さずに縮退する腕は 1 本も無い。`Invalid` の 2 腕がどちらも warn を通ることが
/// 本関数の存在理由（分類器側は scope を知らないので警告を持てない）。
fn resolve_windowposition_vocab(
    scope: usize,
    wp_raw: &WindowPositionRaw,
    x_num: Option<i32>,
) -> (bool, BalloonXMode, Option<i32>) {
    let balloon_limit = match classify_limit_vocab(wp_raw.limit_raw()) {
        LimitVocab::Value(v) => v,
        LimitVocab::Invalid => {
            warn!(
                scope,
                limit_raw = ?wp_raw.limit_raw(),
                "placement: windowposition.limit が 0/1 以外——正典既定 1（画面内へ維持）へ縮退する（要件 1.3）"
            );
            true
        }
    };
    let (x_mode, wp_x) = match classify_x_vocab(x_num, wp_raw.x_raw()) {
        // 数値・未指定は生値を一切見ない。
        XVocab::Numeric(x) => (BalloonXMode::Side, x),
        // キーワード指定は基本位置を変える（幾何は resolver P5 の所有）。数値 x は存在しない。
        XVocab::Keyword(mode) => (mode, None),
        XVocab::Invalid => {
            warn!(
                scope,
                x_raw = ?wp_raw.x_raw(),
                "placement: windowposition.x が数値でもキーワード（center/top/bottom）でもない\
                 ——未指定（調整量なし）へ縮退する（要件 4.6）"
            );
            (BalloonXMode::Side, None)
        }
    };
    (balloon_limit, x_mode, wp_x)
}

/// 当該 scope のバルーン定義（2 層マージ済み）から `windowposition` の
/// 数値解釈と生値の対を取り出す。
///
/// 生値（[`WindowPositionRaw`]）を同じ 1 回の解決から一緒に返すのは、数値と生値が
/// 同じ面・同じマージ結果に由来することを構造で保証するため（別々に読むと
/// 面の選択がずれて「数値は面 0・生値は別の面」という食い違いを作り得る）。
///
/// 系列解決も 2 層マージも権威（`areka-emo-present` の `resolve_balloon_faces` と、呼び手が選んだ
/// 2 層マージの入口 `merge`＝`load_scope_balloon_model` か記録を出さない `load_scope_balloon_model_quiet`）
/// の消費のみで行う。採寸・起動時資産構築と同じ規則で同じ面を
/// 見ることが、実機でしか現れない「採寸した枠と表示される枠が違う」欠陥を封じる唯一の手段。
/// 接頭辞連鎖も上書きファイル名の導出も本層は持たない。
///
/// 取得できないときは `warn!` のうえ `None`（呼び手が当該 scope の供給を見送る）。
fn scope_windowposition(
    balloon_root: &Path,
    scope: usize,
    merge: fn(&Path, u32, &ResolvedFace) -> BalloonModel,
) -> Option<(WindowPosition, WindowPositionRaw)> {
    // 権威の scope 通貨は u32（placement の通貨は usize）。表現できない scope を無言で
    // 切り詰めると別 scope の系列を採ってしまうので、変換失敗は供給の見送りとして報告する。
    let scope_key = match u32::try_from(scope) {
        Ok(scope_key) => scope_key,
        Err(_) => {
            warn!(
                scope,
                "placement: scope 番号が u32 に収まらない（windowposition の供給を見送る）"
            );
            return None;
        }
    };
    let faces = match resolve_balloon_faces_quiet(balloon_root, scope_key) {
        Ok(faces) => faces,
        Err(err) => {
            warn!(
                balloon_root = %balloon_root.display(),
                scope,
                error = %err,
                "placement: バルーン系列の解決に失敗（windowposition の供給を見送る）"
            );
            return None;
        }
    };
    // 定義の上書き層は採用した面 0 に対応するもの。面 0 の必在は権威側の
    // 単一施行点が保証するのでここで再判定はしないが、空列を無言で通さない。
    let Some(face0) = faces.iter().find(|face| face.surface_id == 0) else {
        warn!(
            balloon_root = %balloon_root.display(),
            scope,
            "placement: 解決結果に面 0 が無い（windowposition の供給を見送る）"
        );
        return None;
    };
    let model = merge(balloon_root, scope_key, face0);
    Some((model.windowposition(), model.windowposition_raw().clone()))
}

/// 窓配置の準備処理。
///
/// `enumerate_monitors()` と `is_primary` のモニタの選択 → `load_descript_source_for_shell` →
/// `build_placement_config` → `measure_scope_sizes` → そのモニタの work area の取り出し
/// （0 台ならここで `Err`）→ `resolve_placement` の順に同期実行し、Send な結果のみの
/// [`PreparedPlacement`] を返す。
///
/// - 準備段階の失敗は `Err(PlacementError)` のまま返す（告知と、終了するかどうかは呼び手の
///   分担）
/// - 位置の記憶・復元（`profile/areka` の `sylphya.toml` の読み書き）は一切行わない
/// - 呼び出しスレッドは COM 初期化済みであること（measure の `WicDecoderArm`
///   前提・本番は MTA UI スレッド）
pub fn prepare_ghost_windows(
    ghost_root: &Path,
    balloon_root: &Path,
) -> Result<PreparedPlacement, PlacementError> {
    prepare_ghost_windows_for_shell(ghost_root, balloon_root, None)
}

/// [`prepare_ghost_windows`] のシェル名つきの兄弟。
///
/// `shell` が `Some(名)` なら配置の値（シェルの `descript.txt`）と採寸のシェルのフォルダを
/// `ghost_root/shell/<名>` から取る。名前の先が無ければ既定のシェルへ
/// 黙って戻らず `Err(PlacementError::Mount)` を返す。それ以外は [`prepare_ghost_windows`] と同じ。
pub fn prepare_ghost_windows_for_shell(
    ghost_root: &Path,
    balloon_root: &Path,
    shell: Option<&str>,
) -> Result<PreparedPlacement, PlacementError> {
    let monitors = enumerate_monitors();
    // 列挙の直後＝準備段の失敗より手前で出す。準備が倒れて起動窓を開けずに終わった運転の
    // ログからも、どのモニタ構成を見ていたかを再構成できるようにするため。
    // 呼出点タグで正典出力点（main.rs 構築点）と弁別する。
    diag::log_monitor_snapshot(&monitor_records(&monitors), PREPARE_GHOST_WINDOWS_CONTEXT);
    // primary モニタは work area と起動 k₀ の DPI の同一の出所。
    // 選択（`is_primary`／先頭代替の `warn!`）を 1 回だけ行い、両者へ配る
    // （2 度選ぶと代替の警告も二重に出て、しかも別のモニタを指し得る）。
    let primary = primary_monitor(&monitors);
    let primary_dpi = primary.map(|m| m.dpi);
    // 準備段（load→config→measure）を work area の検査より先に走らせる。
    // 準備段の失敗（Mount・DescriptRead・Measure）はモニタ列挙異常より手前の事象として
    // 報告される（headless でも Mount が返る契約）。
    let stages = prepare_stages_for_shell(ghost_root, balloon_root, primary_dpi, shell)?;
    let work_area = work_area_of(primary)?;
    Ok(stages.resolve(work_area))
}

/// [`prepare_ghost_windows`] の実モニタ注入版（決定論テスト用の偽装境界）。
///
/// 実モニタ列挙（`enumerate_monitors`）に由来する 2 つの値——work area（配置解決の
/// 基準矩形）と primary モニタ DPI（起動 k₀ の分子）——だけを合成値で
/// 置き換え、それ以外（load→config→measure→resolve）は本番と同一経路を通す
/// （headless 環境でも emo2 fixture の観測可能な完了状態を x64 の決定論テストで検証できる）。
///
/// `primary_dpi` を偽装境界に含めるのは、テスト機の実 DPI が制御できないため
/// （実 DPI に依存すると採寸期待値が機械ごとに変わり決定論が壊れる）。
/// `None` は「primary モニタ DPI 取得不能」の縮退分岐を檻に入れるための入力。
#[cfg(any(test, feature = "test-support"))]
pub fn prepare_ghost_windows_with_work_area(
    ghost_root: &Path,
    balloon_root: &Path,
    work_area: RectPx,
    primary_dpi: Option<u32>,
) -> Result<PreparedPlacement, PlacementError> {
    Ok(prepare_stages_for_shell(ghost_root, balloon_root, primary_dpi, None)?.resolve(work_area))
}

/// モニタ列挙結果から primary モニタを選ぶ（work area と k₀ DPI の共通の出所）。
///
/// - `is_primary` のモニタをそのまま返す
/// - primary フラグ無し（列挙異常）: `warn!` の上で先頭モニタを代替に用いる
///   （窓は生やす方針）
/// - 0 台: `None`（架空の既定モニタは発明しない。work area 側の致命判定は
///   [`work_area_of`]・DPI 側の縮退は [`build_measure_scaling`] がそれぞれ担う）
///
/// 使い手は crate の中の並べ直し [`reset_pos`] と、本体の `\![move]` の `primaryscreen`
/// （`crates/areka-runtime/src/move_cue_place.rs`）。
pub fn primary_monitor(monitors: &[Monitor]) -> Option<&Monitor> {
    if let Some(primary) = monitors.iter().find(|m| m.is_primary) {
        return Some(primary);
    }
    let first = monitors.first()?;
    warn!(
        monitor_count = monitors.len(),
        "primary フラグを持つモニタが見つからない（列挙異常）——先頭モニタで代替する"
    );
    Some(first)
}

/// [`primary_monitor`] の work area（物理 px）を [`RectPx`] へ取り出す。
///
/// - `work_area`（`RECT`・物理 px）を単位変換なしで転写する
/// - `None`（モニタ 0 台）: `error!`＋`Err(PlacementError::Monitor)`（架空の既定矩形は
///   発明しない・告知と、終了するかどうかは呼び手の分担）
fn work_area_of(monitor: Option<&Monitor>) -> Result<RectPx, PlacementError> {
    let Some(monitor) = monitor else {
        error!("モニタが 1 台も列挙されない——primary work area の出所がない");
        return Err(PlacementError::Monitor {
            reason: "enumerate_monitors() が 0 台を返した".to_string(),
        });
    };
    Ok(work_area_px(monitor))
}

/// モニタの work area（`RECT`・物理 px）を [`RectPx`] へ単位変換なしで写す。
pub fn work_area_px(monitor: &Monitor) -> RectPx {
    let wa = monitor.work_area;
    RectPx {
        left: wa.left,
        top: wa.top,
        right: wa.right,
        bottom: wa.bottom,
    }
}

/// 表示 DPI を両軸へ写す（`u16` に収まらない値は切り詰めず `u16::MAX` へ飽和する）。
pub(crate) fn square_dpi(dpi: u32) -> DPI {
    let dpi = u16::try_from(dpi).unwrap_or(u16::MAX);
    DPI::from_dpi(dpi, dpi)
}

#[cfg(test)]
#[path = "balloon_offset_supply_tests.rs"]
mod balloon_offset_supply_tests;
#[cfg(test)]
#[path = "placement_char_clamp_tests.rs"]
mod char_clamp_tests;
#[cfg(test)]
#[path = "placement_monitor_tests.rs"]
mod monitor_tests;
#[cfg(test)]
#[path = "placement_prepare_descript_keys_tests.rs"]
mod prepare_descript_keys_tests;
#[cfg(test)]
#[path = "placement_prepare_tests.rs"]
mod prepare_tests;
#[cfg(test)]
#[path = "placement_shared_test_support.rs"]
pub(crate) mod shared_test_support;
#[cfg(test)]
#[path = "placement_windowposition_tests.rs"]
mod windowposition_tests;
#[cfg(test)]
#[path = "placement_windowposition_vocab_tests.rs"]
mod windowposition_vocab_tests;
