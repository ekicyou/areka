//! 窓配置機構（areka-P0-window-placement）のモジュール土台。
//!
//! ゴースト定義（shell dir＋ghost/shell descript の KV）からキャラ窓・バルーン窓の
//! 初期配置を解決し、窓 entity を組み立てる配置パイプラインの器。
//! 座標単位契約（design 正本 U1〜U5）に従い、配置パイプラインの座標・寸法は
//! **すべて物理 px 単一通貨**とする（論理 DIP・`BoxStyle` は持ち込まない）。
//!
//! 依存方向（design「Architecture Pattern & Boundary Map」の強制規約）:
//! `resolver`（純粋・std のみ）← `config`（areka-parsers のみ）←
//! `measure`（emo-atlas/compose）← `spawn`／`follow`（wintf/bevy_ecs）← main.rs シーム。
//! 左のモジュールは右へ import しない。
//! [`diag`]（配置観測・areka-P0-dpi-window-vanish design「Allowed Dependencies」）は
//! この鎖の**さらに左**＝最下流に置く: 純データ＋`tracing` のみに依存し、`World`・wintf の
//! 型を一切知らない（wintf `Monitor` からの転写は本 mod.rs・`follow` 側の仕事）。
//!
//! 本ファイルはサブモジュール宣言・失敗型 [`PlacementError`]（task 1）に加え、
//! 配置準備の合成ルート [`prepare_ghost_windows`]（task 6.1・design「main.rs seam」）
//! を持つ。design File Structure の「mod.rs＝モジュール公開面」に従い、
//! source→config→measure→resolver を束ねる準備関数の自然な置き場として
//! ここ（合成ルート）に実装する（シームの結線自体は task 6.2・main.rs 側）。

pub(crate) mod balloon_limit;
pub mod chain_finalize;
/// DPI／拡大率の遷移後に連鎖を一度だけ解き直す機構（設計 C4・要件 6.1〜6.3／6.6）。
pub mod chain_realign;
pub mod config;
pub mod diag;
/// 窓ごとの整合ゲート（窓の拡大率とモニタ別拡大率表が揃うまで窓書込を見送る・設計 C5）。
pub(crate) mod dpi_sync;
pub mod follow;
pub mod measure;
pub mod persist;
pub mod resolver;
pub mod source;
pub mod spawn;
pub mod transition_diag;
/// 遷移観測ログの判定器（純関数・I/O 無し）。
///
/// 消費者は決定論テストと実機サインオフのランナー（`#[ignore]` テスト）だけで、本番の
/// 実行経路からは 1 度も呼ばれない。areka は lib ターゲットを持たない bin crate ゆえ
/// `pub` でも dead_code 免除されず、本番ビルドへ置くと項目ごとに許可属性を貼る羽目に
/// なる（それは以後の真の dead code を隠す）。[`test_support`] と同じ形にして、許可属性を
/// 1 つも置かずに済ませる。
#[cfg(test)]
pub(crate) mod transition_judge;
mod windowposition;
/// 台帳のグループと窓の在庫から「望む鎖」を組み立てる純関数（Win32／ECS 非依存）。
pub(crate) mod zorder_chain_compose;
/// スコープ窓 Z 順グループの台帳と、タグ／descript 共通のトークン解釈・拒否判定
/// （純関数・Win32／ECS 非依存）。
pub(crate) mod zorder_group_ledger;

/// 作者空間の符号付きオフセットを k 倍する唯一の写像（大きさは `ScaleRatio::scale_len`
/// 権威へ委譲し符号のみ保存する）。`windowposition.x/y` と `\![move]` の dx/dy は
/// どちらも「作者基準 px で書かれた画面オフセット」であり、**同じ写像を通す**
/// （片方だけ素通しにすると高 DPI で両者の意味論が割れる）。
// examples の `#[path]` include ビルドでは本再エクスポートの消費者（emo2_boot）が居ないため
// 未使用警告が出る（frame.rs の同型 allow と同じ事情・`areka` は lib target を持たない bin crate）。
#[allow(unused_imports)]
pub(crate) use windowposition::scale_signed;
/// `#[cfg(test)]` 限定のテスト共有部品（tracing ログ捕捉ハーネス）。本番バイナリには含まれない。
///
/// `pub(crate)`: `main.rs` の起動シーム檻（`monitor_snapshot_seam_tests`）も同じ捕捉
/// ハーネスを使う。areka は bin crate ゆえ crate 内可視で足り、公開面は増えない。
#[cfg(test)]
pub(crate) mod test_support;

use std::path::{Path, PathBuf};

use areka_emo_compose::ScaleRatio;
use areka_emo_present::balloon::{load_scope_balloon_model, resolve_balloon_faces};
use areka_parsers::balloon::{WindowPosition, WindowPositionRaw};
use areka_parsers::package::MountError;
use tracing::{error, info, warn};
use wintf::ecs::window::monitor::{Monitor, enumerate_monitors};

use self::config::{BalloonXMode, PlacementConfig};
use self::measure::{MeasureScaling, MeasuredSizes};
use self::resolver::{RectPx, ScopePlacement};
use self::source::GhostTitles;
use self::windowposition::{LimitVocab, XVocab, classify_limit_vocab, classify_x_vocab};

/// 配置準備パイプライン（resolve→descript 読込→採寸→解決）の観測可能な失敗。
///
/// design「Error Handling」準拠: 安易な panic 禁止・失敗は `error!`＋`Err`。
/// すべて main.rs シームで捕捉され `spawn_dummy_window` フォールバックへ
/// 落ちる（DD14・log-first）。
#[allow(dead_code)] // scaffold（task 1）: 利用側は後続タスクで実装
#[derive(Debug, thiserror::Error)]
pub enum PlacementError {
    /// ゴーストパッケージのマウント解決（`areka_parsers::package::resolve`）失敗。
    ///
    /// `MountError` は `std::error::Error` 未実装のため `#[from]`/`#[source]`
    /// にせず値として保持し `Debug` 表示する。
    #[error("ゴーストのマウント解決に失敗: {0:?}")]
    Mount(MountError),

    /// descript.txt の読み取り失敗（I/O エラー）。
    #[error("descript の読み取りに失敗: {path}")]
    DescriptRead {
        /// 読み取れなかった descript.txt のパス。
        path: PathBuf,
        /// 元の I/O エラー。
        source: std::io::Error,
    },

    /// surface 採寸（emo-atlas/compose による原寸合成）失敗。
    #[error("scope {scope} の surface 採寸に失敗: {reason}")]
    Measure {
        /// 採寸対象のスコープ番号。
        scope: usize,
        /// 失敗理由（下流の詳細を文字列化）。
        reason: String,
    },

    /// モニタ列挙が 0 台で primary work area の出所がない（2.12 の基準を
    /// 満たせない）。架空の既定矩形は発明せず呼び手（シーム）のフォールバックへ
    /// 委ねる（DD14）。task 6.1 で追加（mod.rs＝準備関数の置き場は本タスク境界内）。
    #[error("モニタ列挙に失敗: {reason}")]
    Monitor {
        /// 失敗理由（列挙結果の状況を文字列化）。
        reason: String,
    },
}

/// primary モニタ DPI を取得できないときに採る「96 相当」の DPI
/// （areka-P0-emo-dpi-scaling design「Error Handling」の
/// `primary モニタ DPI 取得不能（boot）` 行・要件 1.4）。
///
/// この縮退は**恒久的な情報損失ではない**——窓生成後は窓の実 DPI が正であり、
/// `Changed<DPI>` を観測する `emo2_boot::frame::run_dpi_phase`（task 4.2）と
/// 表示成立点の状態照合（design Flow 1／Flow 3 手順5）が k を自己補正する（D7）。
const FALLBACK_PRIMARY_DPI: u32 = 96;

/// 作者基準 DPI の既定値（ukadoc 正典・design D1）。
///
/// 縮退梯子（無宣言＝96 debug／不正・0＝96 warn）の単一権威は
/// [`source`] の `parse_author_dpi` であり、ここは placement の準備自体が
/// 成立しなかったときに [`AuthorDpi::DEFAULT`] が採る同値の既定にすぎない
/// （読取器を二重化しない）。
const DEFAULT_AUTHOR_DPI: u16 = 96;

/// `MonitorSnapshot` 構築点（`main.rs`）の**呼出点タグ**
/// （areka-P0-dpi-window-vanish 要件 1.1・design D12）。
///
/// D12 の裁定により、要件 1.1 の**正典出力点**は placement の全判断が読む権威
/// [`follow::MonitorSnapshot`] の構築点である（列挙の忠実転写点＝以後の判断が
/// 実際に見る値そのもの）。本タグはその出所を名乗る。
#[allow(dead_code)] // 消費者は main.rs シーム（`#[path]` include する example には構築点が無い）
pub const MONITOR_SNAPSHOT_CONTEXT: &str = "monitor_snapshot";

/// [`prepare_ghost_windows`] のモニタ列挙点の**呼出点タグ**（要件 1.1・design D12）。
///
/// 同一運転内に列挙点は複数ある。同じ共有ヘルパ [`diag::log_monitor_snapshot`] を
/// 呼びつつタグだけを違えることで、ログ上で出所を弁別でき、かつ**同じ語彙**ゆえ
/// grep 突合で構成の食い違いを検出できる（D12: 専用の突合機構は新設しない）。
pub const PREPARE_GHOST_WINDOWS_CONTEXT: &str = "prepare_ghost_windows";

/// 作業領域源の**実行時同期**（`emo2_boot::frame::work_area_sync`）の呼出点タグ
/// （areka-P0-dpi-transition-atomicity 要件 5.1・設計 C6）。
///
/// 起動時の構築点（[`MONITOR_SNAPSHOT_CONTEXT`]）と列挙点（[`PREPARE_GHOST_WINDOWS_CONTEXT`]）に
/// 続く 3 つ目の出所である。同じ共有ヘルパ [`diag::log_monitor_snapshot`] を呼び、タグだけを
/// 違える（D12: 語彙は共有したまま出所を弁別する）——起動時の構成と同期後の構成が
/// 同じ語彙で並ぶので、どのフレームで何が変わったかを grep 突合で追える。
///
/// 同期の警告（モニタ 0 台）も同じ語を行頭に名乗る（`[work_area_sync] …`）。
pub const WORK_AREA_SYNC_CONTEXT: &str = "work_area_sync";

/// wintf のモニタ列挙結果を診断レコードへ**忠実転写**する純関数（要件 1.1）。
///
/// [`diag`] は placement の最下流であり `World`・wintf の型を知らない（design
/// 「Allowed Dependencies」）。転写は wintf `Monitor` を既に import している本 mod.rs の
/// 仕事であり、ここが「wintf 型 → 純データ」の唯一の境界である。
///
/// - 単位変換も丸めも行わない（U 契約: 配置パイプラインの座標は**物理 px 単一通貨**）
/// - 列挙順を保つ（`index` がログ上の同定子になる）
/// - 実モニタを要さない純関数ゆえ、混在 DPI・負座標・3200 超座標の構成を決定論檻で踏める
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

/// 起動時 k₀ の分母となる作者基準 DPI の対（design D1・Flow 3 手順1）。
///
/// descript 読取は [`prepare_stages`] で **1 度だけ**行い（shell＝
/// `DescriptSource::shell_author_dpi()`・balloon＝[`source::load_balloon_author_dpi`]）、
/// その値を本型に束ねて 2 つの消費者へ配る:
///
/// 1. 採寸の k₀（[`build_measure_scaling`] → `measure_scope_sizes`・要件 3.3）
/// 2. attach の target 政策（[`PreparedPlacement::author_dpi`] → `main` →
///    `emo2_boot::wire_emo2_boot` → `build_boot_assets` → `attach_target`・要件 1.1）
///
/// 同じ宣言が 2 経路で食い違わないこと（＝読取が 1 度であること）が本型の存在理由である。
///
/// shell と balloon は別々のキー（`seriko.dpi`／`dpi`）で宣言され得るうえ、
/// 素の `u16` 2 引数は**取り違えてもコンパイルが通る**。名前付きフィールドで束ね、
/// 呼び手に「どちらの `u16` か」を選ばせない（`emo2_boot::frame` の `AuthorDpis` と同じ防御）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorDpi {
    /// shell descript `seriko.dpi` 由来（縮退梯子適用済みの非ゼロ値）。
    pub shell: u16,
    /// balloon descript `dpi` 由来（同上）。
    pub balloon: u16,
}

impl AuthorDpi {
    /// 正典既定（96/96）——placement の準備自体が失敗して宣言値を読めなかったときの縮退値。
    pub const DEFAULT: AuthorDpi = AuthorDpi {
        shell: DEFAULT_AUTHOR_DPI,
        balloon: DEFAULT_AUTHOR_DPI,
    };
}

impl Default for AuthorDpi {
    fn default() -> Self {
        AuthorDpi::DEFAULT
    }
}

/// placement 側の同期準備一括の結果（design「main.rs seam」正本）。
///
/// I/O は [`prepare_ghost_windows`] までで完結し、**Send な素の値のみ**を運ぶ
/// （`Vec<ScopePlacement>`＝Copy 値の列・`GhostTitles`＝`BTreeMap<usize, String>`。
/// COM/WIC 等のスレッド親和リソースは持たない。Send 契約はテストで固定）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPlacement {
    /// スコープごとの解決済み配置（`resolve_placement` の出力そのまま・scope 昇順）。
    pub placements: Vec<ScopePlacement>,
    /// 窓タイトルの正本（spawn（task 5.1）が消費）。
    pub titles: GhostTitles,
    /// 採寸 k₀ に使った作者基準 DPI（areka-P0-emo-dpi-scaling task 4.3）。
    ///
    /// 同じ読取結果を attach 側（`attach_target`）へ配るための搬送口
    /// （[`AuthorDpi`] の doc・design Flow 3 手順1「1 度だけ読む」）。
    pub author_dpi: AuthorDpi,
    /// shell descript の `seriko.zorder` の生の値（未指定なら `None`）。
    ///
    /// `author_dpi` と**同じ搬送の形**である（areka-P0-scope-zorder-pinning 要件 5.1／5.2）。
    /// descript を読むのは準備の中の 1 度だけなので、重なりの基底もそのときの読取結果を
    /// ここへ載せて `main` → `wire_emo2_boot` → 台帳へ配る。ここで搬送せずに結線の側で
    /// 読み直すと、配置と重なりが**別々の宣言**を見る余地が生まれる（`author_dpi` を搬送に
    /// した理由そのもの）。
    ///
    /// 値の解釈は placement では一切行わない（生の転記＝`PlacementConfig::zorder_raw` の
    /// そのまま）。窓の位置も寸法もこの値では 1 mm も動かない。
    pub zorder_raw: Option<String>,
}

/// 準備パイプラインの中間結果（load→config→measure まで・work area 非依存部）。
///
/// [`prepare_ghost_windows`]（実モニタ列挙）と
/// [`prepare_ghost_windows_with_work_area`]（合成 work area 注入・決定論テスト用）
/// が resolve 直前まで同一経路を共有するための内部型。
struct PreparedStages {
    cfg: PlacementConfig,
    sizes: MeasuredSizes,
    titles: GhostTitles,
    author_dpi: AuthorDpi,
}

impl PreparedStages {
    /// work area（物理 px）を与えて配置を確定する（純粋・resolver P1〜P5）。
    fn resolve(self, work_area: RectPx) -> PreparedPlacement {
        let placements = resolver::resolve_placement(&self.cfg, work_area, &self.sizes.scopes);
        PreparedPlacement {
            placements,
            titles: self.titles,
            author_dpi: self.author_dpi,
            // 解釈せずそのまま搬送する（重なりの基底の出所は 1 つ＝この読取・要件 5.2）。
            zorder_raw: self.cfg.zorder_raw,
        }
    }
}

/// 起動時の表示スケール k₀ を導出する（design D7・Flow 3 手順2・要件 1.4/3.3）。
///
/// `k₀ = primary モニタ DPI ÷ 作者基準 DPI` を **shell／balloon それぞれの
/// 作者基準 DPI で別々に**求める（2 軸は独立・要件 1.1）。丸めは一切行わない——
/// 既約有理数 [`ScaleRatio`] のまま [`MeasureScaling`] に載せ、寸法の乗算と丸めは
/// 単一権威 `ScaleRatio::scaled_extent`（D4）だけが行う（`as_f32` を寸法計算に使わない）。
///
/// # 縮退（design「Error Handling」・log-first・表示を失わない）
///
/// `primary_dpi` が `None`（モニタ 0 台）または 0（列挙異常）のとき、`error!` のうえ
/// [`FALLBACK_PRIMARY_DPI`]（96 相当）から k₀ を導く。作者基準 DPI が 96 なら k₀=1/1＝
/// 従来どおりの native 採寸であり、96 以外を宣言していればその宣言に対する 96 相当の比
/// （例: author=192 → k₀=1/2）となる。いずれにせよ窓は生え、窓生成後に窓の実 DPI が
/// `Changed<DPI>` → `refresh_scale`＋窓寸 reconcile（task 4.2）で k を自己補正する
/// ——つまりこの縮退は**回復可能**であり、表示の喪失にはならない（要件 1.4/4.1）。
fn build_measure_scaling(primary_dpi: Option<u32>, author_dpi: AuthorDpi) -> MeasureScaling {
    let dpi = match primary_dpi {
        Some(dpi) if dpi > 0 => dpi,
        unobtainable => {
            error!(
                primary_dpi = ?unobtainable,
                fallback_dpi = FALLBACK_PRIMARY_DPI,
                shell_author_dpi = author_dpi.shell,
                balloon_author_dpi = author_dpi.balloon,
                "placement: primary モニタ DPI を取得できない——96 相当で k₀ を導いて採寸を続行する\
                 （窓生成後に窓の実 DPI が k を自己補正するため表示は失われない・要件 1.4）"
            );
            FALLBACK_PRIMARY_DPI
        }
    };
    // 軸ごとに自分の作者基準 DPI で割る（shell の k を balloon へ漏らさない・要件 1.1）。
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
    // 起動時 k₀ の観測点（D10 の表示成立点ログと同じ語彙・要件 6.3 の RUST_LOG grep 対象）。
    // `k_shell`/`k_balloon`（f32）は grep 用の出口ビューであり、寸法計算には用いない（D4）。
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
    scaling
}

/// 準備パイプラインの work area 非依存部を同期実行する:
/// `load_descript_source` → `build_placement_config` → `measure_scope_sizes`。
///
/// 失敗はフォールバックせず [`PlacementError`] のまま呼び手へ返す（DD14:
/// `spawn_dummy_window` フォールバックは main.rs シームの分担）。
/// 位置の記憶・復元（`ghost.dat` 読み書き）は一切行わない（2.11・テストで固定）。
///
/// `primary_dpi` は起動時 k₀ の分子（primary モニタ DPI・物理 DPI 値）。取得不能
/// （モニタ 0 台等）は `None` を渡す——[`build_measure_scaling`] が `error!`＋96 相当へ
/// 縮退させる（要件 1.4）。
fn prepare_stages(
    ghost_root: &Path,
    balloon_root: &Path,
    primary_dpi: Option<u32>,
) -> Result<PreparedStages, PlacementError> {
    let src = source::load_descript_source(ghost_root)?;
    let mut cfg = config::build_placement_config(&src.ghost_kv, &src.shell_kv);
    let scope_ids: Vec<usize> = cfg.scopes.keys().copied().collect();
    // 作者基準 DPI（design D1・Flow 3 手順1）は**ここで 1 度だけ**読む。shell は既に
    // 読み込み済みの生 KV（`load_descript_source` の戻り）から、balloon は別パッケージ
    // ゆえ `load_balloon_author_dpi` の 1 回の寛容読取から得る（パーサ改造なし・再 I/O なし）。
    // 得た値は採寸 k₀ と attach（`PreparedPlacement::author_dpi` 経由）の双方へ配られる。
    let author_dpi = AuthorDpi {
        shell: src.shell_author_dpi(),
        balloon: source::load_balloon_author_dpi(balloon_root),
    };
    // k₀ 構築（D7）→ 採寸源へ供給（Flow 3 手順2〜3・要件 3.3）。窓寸の k 倍は
    // ここ（採寸の源）で吸収され、`spawn.rs` は k 倍済み `SizePx` を consume するのみ
    // ＝窓生成・窓移動の責務は不変（要件 3.4/7.6）。
    let scaling = build_measure_scaling(primary_dpi, author_dpi);
    let sizes = measure::measure_scope_sizes(&src.shell_dir, balloon_root, &scope_ids, &scaling)?;
    // 起動観測点（D10・要件 6.3）: 窓生成へ渡る**k₀ 倍後の物理寸**そのものを載せる。
    // 非 96 環境では k_shell/k_balloon が 1.0 以外になり、scopes の寸が原寸の k₀ 倍で
    // あることを RUST_LOG の grep だけで決定論的に判定できる。
    info!(
        k_shell = scaling.shell.as_f32(),
        k_balloon = scaling.balloon.as_f32(),
        scopes = ?sizes.scopes,
        "placement: k₀ 倍後の物理窓寸で窓を生成する（起動採寸・要件 3.3）"
    );
    // 表示位置指定（`windowposition` 数値指定）→ 初期既定位置の調整量（areka-P0-kero-balloon
    // 要件 3.2/3.3/3.4/3.6・design D1'）。**採寸の後**に scope ごとの定義を権威から取得し、
    // `ScopeConfig.balloon_offset`（P5 が既に加算している欄）へ合流させる供給のみを行う
    // ——配置解決の式 P1〜P5 は無改変であり、永続値優先の復元規約（`persist.rs`）にも触れない。
    apply_scope_windowpositions(&mut cfg, balloon_root, &scope_ids, scaling.balloon);
    Ok(PreparedStages {
        cfg,
        sizes,
        titles: src.titles,
        author_dpi,
    })
}

/// scope ごとの表示位置指定（`windowposition` 数値指定）を初期既定位置の調整量へ変換し
/// `cfg.scopes[scope].balloon_offset` へ合流させる（要件 3.2/3.3/3.4/3.6・design D1'）。
///
/// **供給元を増やすだけ**の層である——`resolver.rs` の配置式 P1〜P5 は無改変で、P5 が既に
/// 加算している `balloon_offset.unwrap_or((0, 0))` の入力が増える。ゆえに恒等式
/// `balloon_offset ≡ balloon_pos − char_pos` も、キャラ窓の基準原点も、位置の保存・復元の
/// 基準（`persist.rs`・永続値優先）も変わらない（要件 3.5/3.8）。
///
/// `k` は**バルーン軸**の表示スケール（`MeasureScaling::balloon`）である——`windowposition` は
/// バルーン作者の空間で書かれた値ゆえ、シェル軸の k を掛けてはならない（要件 3.6・2 軸独立）。
/// 大きさの丸めは既存権威 `ScaleRatio::scale_len` へ委譲される（新しい丸め規約を導入しない）。
///
/// # 語彙の解決（windowposition-limit 要件 1.3/1.4/4.6・design C3「取得経路の拡張」）
///
/// 同じ 2 層マージ済み定義から生値（[`WindowPositionRaw`]）も取り出し、[`classify_limit_vocab`]／
/// [`classify_x_vocab`] の分類結果を `ScopeConfig.balloon_limit`／`balloon_x_mode` へ反映する。
/// 分類器は警告を持たない——**不正値の警告は scope 文脈を持つ本層の所有**であり
/// （design C1 Invariants）、警告の無い縮退経路を 1 本も残さない（要件 6.3）。
///
/// # 観測（要件 6.3・design Monitoring 観測点 4）
///
/// scope ごとに `info!` で scope・wp 生値・バルーンの左右・変換後の調整量（物理 px）と、
/// **解決済みの limit 値・水平配置モードの実値**（windowposition-limit 要件 6.2）を記録する。
/// 実機サインオフ（R7.6・task 6.1）はこの行を grep して SSP 実測と突合し、x 方向の符号規約を
/// **確定させた**——確定形は「左右で反転しない素の画面座標オフセット」（`windowposition.rs` の
/// `to_screen_adjust`「符号規約」節）。`balloon_side` は調整量の計算には効かなくなったが、
/// 基本位置（Left＝`char_x − balloon_w`／Right＝`char_x + char_w`）がどちらだったかを実機ログ
/// だけで再構成できるようにするため記録し続ける。
///
/// # 失敗（log-first・表示を失わない）
///
/// 系列解決・面 0 の不在はいずれも `warn!` のうえ当該 scope の供給を見送る（配置自体は基本位置で
/// 成立する）。同じ事象は直前の採寸（`measure_scope_sizes`）が既に `error!`＋`Err` で弾いており
/// ——採寸が成功した後に本関数だけが失敗する経路は実在しない——ここは無言化を防ぐ防波堤である。
fn apply_scope_windowpositions(
    cfg: &mut PlacementConfig,
    balloon_root: &Path,
    scope_ids: &[usize],
    k: ScaleRatio,
) {
    for &scope in scope_ids {
        // バルーンの左右は配置構成の解決済み値（`balloon.alignment`・cascade 済み）。
        // 調整量の計算には効かない（符号は置き側に依らない・R7.6 確定形）が、観測点 4 の
        // 記録項目である。未収載 scope には合流先が無いため、ここで供給を見送る判定も兼ねる。
        //
        // 語彙の解決値（limit／x_mode）の合流先も同じ `ScopeConfig` ゆえ、可変で 1 度だけ引く。
        let Some(sc) = cfg.scopes.get_mut(&scope) else {
            warn!(
                scope,
                "placement: 配置表に無い scope の windowposition 供給要求を無視した"
            );
            continue;
        };
        let side = sc.balloon_alignment;
        let Some((wp, wp_raw)) = scope_windowposition(balloon_root, scope) else {
            // 定義を読めなかった scope は `ScopeConfig` の正典既定（limit=1・`Side`）のまま。
            // 見送りの warn は `scope_windowposition` が既に出している（無言の縮退なし）。
            continue;
        };
        // 語彙分類（C1）→ 不正値の警告付き縮退（要件 1.3/4.6/6.3）。
        let (balloon_limit, x_mode, wp_x) = resolve_windowposition_vocab(scope, &wp_raw, wp.x());
        let wp_y = wp.y();
        // scope 別構成へ反映（要件 1.4——limit も x も「その scope が採用した面の
        // 2 層マージ結果」という同一単位で解決される）。
        sc.balloon_limit = balloon_limit;
        sc.balloon_x_mode = x_mode;
        // キーワード指定のとき `wp_x` は `None`（C1 の不変量）ゆえ、既存の
        // `to_screen_adjust(None, wp_y)` がそのまま `(0, dy)` を供給する（要件 4.4）。
        let adjust = windowposition::to_screen_adjust(wp_x, wp_y, k);
        // 観測点 4: 数値指定なし（`adjust=None`）でも 1 行出す——「読んだが指定が無かった」ことと
        // 「そもそも読めなかった」ことを実機ログで区別できるようにするため（`adjusted` で判別）。
        // `limit`／`x_mode` は**解決済みの実値**（縮退後の値）である（要件 6.2）。
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
        windowposition::apply_windowposition(cfg, scope, adjust);
    }
}

/// `windowposition` の語彙（limit／x）を解決し、不正値を **scope 文脈つきの警告を出したうえで**
/// 正典既定へ縮退させる（要件 1.3/4.6/6.3・design C1 Invariants／Error Handling 表）。
///
/// 戻り値は `(limit 解決値, 水平配置モード, 調整量計算へ渡す数値 x)`。
///
/// - limit: `0`/`1` は受理・未指定は正典既定 `true`・それ以外は **warn（scope・生値）→ `true`**。
/// - x: 数値・未指定は現行どおり（`Side` ＋ 生値を見ない＝要件 5.1 の回帰境界）。
///   キーワードは対応する [`BalloonXMode`]（数値 x は存在しないので `None`）。
///   それ以外は **warn（scope・生値）→ 未指定扱い（`Side`・`None`）**（要件 4.6）。
///
/// 警告を出さずに縮退する腕は 1 本も無い——`Invalid` の 2 腕がどちらも warn を通る
/// ことが本関数の存在理由である（分類器側は scope を知らないので警告を持てない）。
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
        // 数値・未指定は現行と bit 同一（生値を一切見ない・要件 5.1）。
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
/// **数値解釈と生値の対**を取り出す（design C3「取得経路の拡張」）。
///
/// 生値（[`WindowPositionRaw`]）を同じ 1 回の解決から一緒に返すのは、数値と生値が
/// **同じ面・同じマージ結果**に由来することを構造で保証するためである（別々に読むと
/// 面の選択がずれて「数値は面 0・生値は別の面」という食い違いを作り得る・要件 1.4）。
///
/// 系列解決も 2 層マージも権威（`areka-emo-present` の `resolve_balloon_faces` /
/// `load_scope_balloon_model`）の消費のみで行う——採寸・起動時資産構築と同じ規則で同じ面を
/// 見ることが、実機でしか現れない「採寸した枠と表示される枠が違う」欠陥を封じる唯一の手段である
/// （design D2）。接頭辞連鎖も上書きファイル名の導出も本層は持たない。
///
/// 取得できないときは `warn!` のうえ `None`（呼び手が当該 scope の供給を見送る）。
fn scope_windowposition(
    balloon_root: &Path,
    scope: usize,
) -> Option<(WindowPosition, WindowPositionRaw)> {
    // 権威の scope 通貨は u32（placement の通貨は usize）。表現できない scope を無言で
    // 切り詰めると別 scope の系列を採ってしまうため、変換失敗は供給の見送りとして報告する。
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
    let faces = match resolve_balloon_faces(balloon_root, scope_key) {
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
    // 定義の上書き層は**採用した面 0**に対応するもの（要件 2.2/2.3）。面 0 の必在は権威側の
    // 単一施行点（要件 1.7）ゆえここで再判定はしないが、空列を無言で通さない。
    let Some(face0) = faces.iter().find(|face| face.surface_id == 0) else {
        warn!(
            balloon_root = %balloon_root.display(),
            scope,
            "placement: 解決結果に面 0 が無い（windowposition の供給を見送る）"
        );
        return None;
    };
    let model = load_scope_balloon_model(balloon_root, scope_key, face0);
    Some((model.windowposition(), model.windowposition_raw().clone()))
}

/// 窓配置の準備処理（design「main.rs seam」・task 6.1）。
///
/// `load_descript_source` → `build_placement_config` → `measure_scope_sizes` →
/// `enumerate_monitors()` の `is_primary` モニタ work area 取得（2.12）→
/// `resolve_placement` の順に**同期実行**し、Send な結果のみの
/// [`PreparedPlacement`] を返す。
///
/// - 準備段階の失敗は `Err(PlacementError)` のまま返す（本関数はフォールバック
///   しない・DD14: フォールバックは呼び手＝main.rs シームの分担）
/// - 位置の記憶・復元（`ghost.dat` 読み書き）は一切行わない（2.11）
/// - 呼び出しスレッドは COM 初期化済みであること（measure の `WicDecoderArm`
///   前提・本番は MTA UI スレッド）
pub fn prepare_ghost_windows(
    ghost_root: &Path,
    balloon_root: &Path,
) -> Result<PreparedPlacement, PlacementError> {
    let monitors = enumerate_monitors();
    // 観測（areka-P0-dpi-window-vanish 要件 1.1）: 列挙の**直後**＝準備段の失敗より手前で
    // 出す。fixture 不在等でダミー窓へフォールバックした運転のログからも、その運転が
    // どのモニタ構成を見ていたかを再構成できることが事後診断の条件である。
    // 呼出点タグで正典出力点（main.rs 構築点）と弁別する（D12）。
    diag::log_monitor_snapshot(&monitor_records(&monitors), PREPARE_GHOST_WINDOWS_CONTEXT);
    // primary モニタは **work area（2.12）と 起動 k₀ の DPI（D7）の同一の出所**である。
    // 選択（`is_primary`／先頭代替の `warn!`）を 1 回だけ行い、両者へ配る
    // （2 度選ぶと代替の警告も二重に出て、しかも別のモニタを指し得る）。
    let primary = primary_monitor(&monitors);
    let primary_dpi = primary.map(|m| m.dpi);
    // 準備段（load→config→measure）を work area の検査より**先**に走らせる:
    // 準備段の失敗（Mount・DescriptRead・Measure）はモニタ列挙異常より手前の事象として
    // 報告される（既存の失敗順序＝headless でも Mount が返る契約を保つ）。
    let stages = prepare_stages(ghost_root, balloon_root, primary_dpi)?;
    let work_area = work_area_of(primary)?;
    Ok(stages.resolve(work_area))
}

/// [`prepare_ghost_windows`] の**実モニタ注入版**（決定論テスト用の偽装境界）。
///
/// 実モニタ列挙（`enumerate_monitors`）に由来する 2 つの値——work area（配置解決の
/// 基準矩形・2.12）と primary モニタ DPI（起動 k₀ の分子・D7）——だけを合成値で
/// 置き換え、それ以外（load→config→measure→resolve）は本番と同一経路を通す
/// （記憶 prefer-x64-fake-boundary-tests の流儀。headless 環境でも emo2 fixture
/// の観測可能な完了状態を決定論的に検証できる）。
///
/// `primary_dpi` を偽装境界に含めるのは、**テスト機の実 DPI が制御できない**ためである
/// （実 DPI に依存すると採寸期待値が機械ごとに変わり決定論が壊れる）。既存の work area
/// 注入と同じ「実列挙由来の値を引数で差し替える」形に揃え、新しい機構は導入しない。
/// `None` は「primary モニタ DPI 取得不能」の縮退分岐（要件 1.4）を檻に入れるための入力。
#[allow(dead_code)] // scaffold（task 6.1）: テスト専用の偽装境界（本番は prepare_ghost_windows）
pub fn prepare_ghost_windows_with_work_area(
    ghost_root: &Path,
    balloon_root: &Path,
    work_area: RectPx,
    primary_dpi: Option<u32>,
) -> Result<PreparedPlacement, PlacementError> {
    Ok(prepare_stages(ghost_root, balloon_root, primary_dpi)?.resolve(work_area))
}

/// モニタ列挙結果から primary モニタを選ぶ（2.12・work area と k₀ DPI の共通の出所）。
///
/// - `is_primary` のモニタをそのまま返す
/// - primary フラグ無し（列挙異常）: `warn!` の上で先頭モニタを代替に用いる
///   （窓は生やす方針・design「Error Handling」）
/// - 0 台: `None`（架空の既定モニタは発明しない。work area 側の致命判定は
///   [`work_area_of`]・DPI 側の縮退は [`build_measure_scaling`] がそれぞれ担う）
fn primary_monitor(monitors: &[Monitor]) -> Option<&Monitor> {
    if let Some(primary) = monitors.iter().find(|m| m.is_primary) {
        return Some(primary);
    }
    match monitors.first() {
        Some(first) => {
            warn!(
                monitor_count = monitors.len(),
                "primary フラグを持つモニタが見つからない（列挙異常）——先頭モニタで代替する"
            );
            Some(first)
        }
        None => None,
    }
}

/// [`primary_monitor`] の work area（物理 px）を [`RectPx`] へ取り出す（2.12）。
///
/// - `work_area`（`RECT`・物理 px）を **単位変換なしで忠実転写**する
///   （U 契約: どちらも物理 px 通貨）
/// - `None`（モニタ 0 台）: `error!`＋`Err(PlacementError::Monitor)`（架空の既定矩形は
///   発明しない・フォールバックはシームの分担）
fn work_area_of(monitor: Option<&Monitor>) -> Result<RectPx, PlacementError> {
    let Some(monitor) = monitor else {
        error!("モニタが 1 台も列挙されない——primary work area の出所がない");
        return Err(PlacementError::Monitor {
            reason: "enumerate_monitors() が 0 台を返した".to_string(),
        });
    };
    let wa = monitor.work_area;
    Ok(RectPx {
        left: wa.left,
        top: wa.top,
        right: wa.right,
        bottom: wa.bottom,
    })
}

#[cfg(test)]
#[path = "placement_monitor_tests.rs"]
mod monitor_tests;
#[cfg(test)]
#[path = "placement_prepare_tests.rs"]
mod prepare_tests;
#[cfg(test)]
#[path = "placement_shared_test_support.rs"]
mod shared_test_support;
#[cfg(test)]
#[path = "placement_windowposition_tests.rs"]
mod windowposition_tests;
#[cfg(test)]
#[path = "placement_windowposition_vocab_tests.rs"]
mod windowposition_vocab_tests;
