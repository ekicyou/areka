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
//!
//! 本ファイルはサブモジュール宣言・失敗型 [`PlacementError`]（task 1）に加え、
//! 配置準備の合成ルート [`prepare_ghost_windows`]（task 6.1・design「main.rs seam」）
//! を持つ。design File Structure の「mod.rs＝モジュール公開面」に従い、
//! source→config→measure→resolver を束ねる準備関数の自然な置き場として
//! ここ（合成ルート）に実装する（シームの結線自体は task 6.2・main.rs 側）。

pub mod config;
pub mod follow;
pub mod measure;
pub mod persist;
pub mod resolver;
pub mod source;
pub mod spawn;
mod windowposition;
/// `#[cfg(test)]` 限定のテスト共有部品（tracing ログ捕捉ハーネス）。本番バイナリには含まれない。
#[cfg(test)]
mod test_support;

use std::path::{Path, PathBuf};

use areka_emo_compose::ScaleRatio;
use areka_emo_present::balloon::{load_scope_balloon_model, resolve_balloon_faces};
use areka_parsers::balloon::WindowPosition;
use areka_parsers::package::MountError;
use tracing::{error, info, warn};
use wintf::ecs::window::monitor::{Monitor, enumerate_monitors};

use self::config::PlacementConfig;
use self::measure::{MeasureScaling, MeasuredSizes};
use self::resolver::{RectPx, ScopePlacement};
use self::source::GhostTitles;

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
/// # 観測（要件 6.3・design Monitoring 観測点 4）
///
/// scope ごとに `info!` で scope・wp 生値・バルーンの左右・変換後の調整量（物理 px）を記録する。
/// 実機サインオフ（R7.6）は本行を grep して x 方向の符号の向き（`windowposition.rs` の符号表＝
/// 実機確定待ちの仮説）を突合する。
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
        // 未収載 scope には合流先が無いため、side を捏造せずここで見送る。
        let Some(side) = cfg.scopes.get(&scope).map(|sc| sc.balloon_alignment) else {
            warn!(
                scope,
                "placement: 配置表に無い scope の windowposition 供給要求を無視した"
            );
            continue;
        };
        let Some(wp) = scope_windowposition(balloon_root, scope) else {
            continue;
        };
        let (wp_x, wp_y) = (wp.x(), wp.y());
        let adjust = windowposition::to_screen_adjust(wp_x, wp_y, side, k);
        // 観測点 4: 数値指定なし（`adjust=None`）でも 1 行出す——「読んだが指定が無かった」ことと
        // 「そもそも読めなかった」ことを実機ログで区別できるようにするため（`adjusted` で判別）。
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
            "placement: windowposition を初期既定位置の調整量へ変換した（scope 別・要件 3.2/7.6）"
        );
        windowposition::apply_windowposition(cfg, scope, adjust);
    }
}

/// 当該 scope のバルーン定義（2 層マージ済み）から `windowposition` を取り出す。
///
/// 系列解決も 2 層マージも権威（`areka-emo-present` の `resolve_balloon_faces` /
/// `load_scope_balloon_model`）の消費のみで行う——採寸・起動時資産構築と同じ規則で同じ面を
/// 見ることが、実機でしか現れない「採寸した枠と表示される枠が違う」欠陥を封じる唯一の手段である
/// （design D2）。接頭辞連鎖も上書きファイル名の導出も本層は持たない。
///
/// 取得できないときは `warn!` のうえ `None`（呼び手が当該 scope の供給を見送る）。
fn scope_windowposition(balloon_root: &Path, scope: usize) -> Option<WindowPosition> {
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
    Some(load_scope_balloon_model(balloon_root, scope_key, face0).windowposition())
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
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use windows::Win32::Foundation::RECT;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
    use wintf::ecs::window::monitor::Monitor;

    use super::resolver::{PointPx, RectPx, SizePx};
    use super::test_support::{LogEvent, capture_logs};
    use super::*;

    /// COM 初期化下でクロージャを実行する（measure が `WicDecoderArm` を要求・
    /// measure.rs テストと同一パターン）。
    fn with_com_initialized<F: FnOnce()>(f: F) {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        f();
        unsafe {
            CoUninitialize();
        }
    }

    /// emo2 fixture ルート（source.rs／measure.rs テストと同一アンカー規約）。
    fn emo2_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2")
    }

    /// emo2 fixture のバルーンルート（task 4.1 テストの規約を踏襲）。
    fn balloon_root() -> PathBuf {
        emo2_root().join("emo2-kakukaku")
    }

    /// 決定論テスト用の合成 work area（物理 px・resolver T-R 群と同流儀）。
    const WA: RectPx = RectPx {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };

    // ── テスト用一時ディレクトリ（measure.rs テストと同じ std-only 最小実装）──

    static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Drop 時に自身を再帰削除する一時ディレクトリ。
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "areka-placement-prepare-{}-{}",
                std::process::id(),
                n
            ));
            std::fs::create_dir_all(&path).expect("一時ディレクトリ作成");
            TempDir { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// emo2 fixture の native 採寸値（`measure.rs` テストの同名定数と同一の実測値）。
    /// k₀ 適用後の期待値はこれらの `scaled_extent` 倍として書く。
    const SCOPE0_W: i32 = 434;
    const SCOPE0_H: i32 = 687;
    const SCOPE1_W: i32 = 336;
    const SCOPE1_H: i32 = 400;
    /// バルーン面 0 は **scope ごとに解決した系列**の実寸である（要件 3.1）——
    /// scope0＝本体側 `balloons0.png`・scope1＝相方側 `balloonk0.png`。
    const BALLOON0_W: i32 = 400;
    const BALLOON0_H: i32 = 224;
    const BALLOON1_W: i32 = 288;
    const BALLOON1_H: i32 = 203;

    /// 既約有理 k のテスト用短縮構築（0 入力はテストの誤りゆえ expect）。
    fn k(num: u32, den: u32) -> ScaleRatio {
        ScaleRatio::new(num, den).expect("テスト入力は非ゼロ")
    }

    /// テスト用合成 Monitor（実 HMONITOR 不要・wintf monitor.rs テストと同流儀）。
    fn make_monitor(handle: isize, work: (i32, i32, i32, i32), is_primary: bool) -> Monitor {
        let (left, top, right, bottom) = work;
        Monitor {
            handle,
            bounds: RECT {
                left,
                top,
                right,
                bottom: bottom + 40, // work_area はタスクバーぶん狭い想定
            },
            work_area: RECT {
                left,
                top,
                right,
                bottom,
            },
            dpi: 96,
            is_primary,
        }
    }

    // ------------------------------------------------------------------
    // 観測可能な完了状態（tasks 6.1）: emo2 fixture 相当のパスで 2 スコープぶんの
    // ScopePlacement を含む PreparedPlacement が返る
    // ------------------------------------------------------------------

    /// emo2 fixture＋合成 work area で `prepare_ghost_windows_with_work_area` が
    /// 2 スコープの `ScopePlacement`（resolver P1〜P5 の厳密値）と titles を返す。
    ///
    /// 期待値の根拠（emo2 実測: alignment=bottom・defaultx=0×2・
    /// balloon.alignment=left/right・寸法 434×687／336×400・balloon は scope 別で
    /// scope0=400×224（`balloons0.png`）／scope1=288×203（`balloonk0.png`）):
    /// - scope0: char=(1920−434, 1040−687)=(1486,353)・基本位置＝左隣=(1486−400)=(1086,353)
    /// - scope1: char=(1486−434, 1040−400)=(1052,640)・基本位置＝右隣=(1052+336)=(1388,640)
    ///
    /// バルーン寸が scope 別になっても**基本位置**が動かないのは、右置き（scope1）の基準 x が
    /// キャラ窓の右端＝バルーン幅に依存しないためである（resolver P5 は無改変）。
    ///
    /// **task 4.3（要件 3.2/7.2）**: 基本位置へ当該 scope の `windowposition` 由来の調整量が
    /// 合流する（k₀=1/1 ゆえ生値そのまま・符号変換のみ）:
    /// - scope0（`balloons0s.txt` の `266,-129`・side=Left ゆえ x 無反転）→ 調整量 (+266,−129)
    ///   → balloon=(1086+266, 353−129)=(1352,224)・offset=(−400+266, −129)=(−134,−129)
    /// - scope1（`balloonk0s.txt` の `-190,-75`・side=Right ゆえ x 反転）→ 調整量 (+190,−75)
    ///   → balloon=(1388+190, 640−75)=(1578,565)・offset=(336+190, −75)=(526,−75)
    ///
    /// 恒等式 `balloon_offset ≡ balloon_pos − char_pos` は両 scope で保たれている
    /// （P5 の加算入力が増えただけ）。x 方向の符号の向きは実機確定待ちの仮説である
    /// （R7.6・task 6.1）——逆と判明した場合の修正は `windowposition.rs` の符号表 1 箇所
    /// と本期待値の x 成分に閉じる。
    #[test]
    fn prepare_emo2_returns_two_scope_placements() {
        with_com_initialized(|| {
            // primary DPI 96＝emo2 の作者基準 DPI（無宣言＝96）ゆえ k₀=1/1（要件 7.2 の錨）。
            let p =
                prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
                    .expect("emo2 fixture の配置準備は成功する");

            assert_eq!(p.placements.len(), 2, "emo2 は 2 スコープ（DD6）");

            let s0 = &p.placements[0];
            assert_eq!(s0.scope, 0);
            assert_eq!(s0.char_pos, PointPx { x: 1486, y: 353 });
            assert_eq!(s0.char_size, SizePx { w: 434, h: 687 });
            assert_eq!(
                s0.balloon_pos,
                PointPx { x: 1352, y: 224 },
                "本体側は balloons0s.txt の windowposition 266,-129 を反映（要件 3.2）"
            );
            assert_eq!(s0.balloon_size, SizePx { w: 400, h: 224 });
            assert_eq!(s0.balloon_offset, PointPx { x: -134, y: -129 });

            let s1 = &p.placements[1];
            assert_eq!(s1.scope, 1);
            assert_eq!(s1.char_pos, PointPx { x: 1052, y: 640 });
            assert_eq!(s1.char_size, SizePx { w: 336, h: 400 });
            assert_eq!(
                s1.balloon_pos,
                PointPx { x: 1578, y: 565 },
                "相方側は balloonk0s.txt の windowposition -190,-75 を反映（本体側の値ではない）"
            );
            assert_eq!(
                s1.balloon_size,
                SizePx {
                    w: BALLOON1_W,
                    h: BALLOON1_H
                },
                "相方側は自分の系列（balloonk0.png）の寸——本体側 400×224 ではない"
            );
            assert_eq!(s1.balloon_offset, PointPx { x: 526, y: -75 });

            // 恒等式（resolver.rs の恒久事後条件）は供給元が増えても保たれる。
            for s in &p.placements {
                assert_eq!(
                    s.balloon_offset,
                    PointPx {
                        x: s.balloon_pos.x - s.char_pos.x,
                        y: s.balloon_pos.y - s.char_pos.y
                    },
                    "scope {}: balloon_offset ≡ balloon_pos − char_pos",
                    s.scope
                );
            }

            // titles は MountModel.names 由来（source T-I5 と同値）
            assert_eq!(p.titles.title(0), "むらさき");
            assert_eq!(p.titles.title(1), "エモ");
        });
    }

    /// 薄いラッパ `prepare_ghost_windows` は実モニタ列挙で primary work area を
    /// 取得し、同じ work area を与えた合成経路と同一の結果を返す。
    /// モニタ 0 台の環境（headless）では `PlacementError::Monitor` が返る
    /// （環境寛容: どちらの分岐でも決定論的に検証する）。
    #[test]
    fn prepare_ghost_windows_uses_primary_monitor() {
        with_com_initialized(|| {
            let monitors = wintf::ecs::window::monitor::enumerate_monitors();
            match prepare_ghost_windows(&emo2_root(), &balloon_root()) {
                Ok(p) => {
                    assert!(
                        !monitors.is_empty(),
                        "モニタ 0 台で Ok は誤成功（work area の出所がない）"
                    );
                    assert_eq!(p.placements.len(), 2);
                    // 同一 work area **と同一 primary DPI** を注入した合成経路と一致
                    // （薄いラッパは委譲のみ・実機の DPI がそのまま k₀ の分子になる）
                    let primary = primary_monitor(&monitors);
                    let wa = work_area_of(primary).expect("primary work area");
                    let q = prepare_ghost_windows_with_work_area(
                        &emo2_root(),
                        &balloon_root(),
                        wa,
                        primary.map(|m| m.dpi),
                    )
                    .expect("合成経路も成功する");
                    assert_eq!(p.placements, q.placements);
                }
                Err(PlacementError::Monitor { .. }) => {
                    assert!(
                        monitors.is_empty(),
                        "モニタが存在するのに Monitor エラーが返った"
                    );
                }
                Err(other) => panic!("Monitor 以外の Err は契約外: {other:?}"),
            }
        });
    }

    // ------------------------------------------------------------------
    // primary_monitor／work_area_of（純粋・合成 Monitor で決定論）
    // ------------------------------------------------------------------

    /// モニタ 0 台は `PlacementError::Monitor`（架空の既定矩形を発明しない）。
    #[test]
    fn primary_work_area_empty_is_monitor_err() {
        let err = work_area_of(primary_monitor(&[])).expect_err("0 台は Err");
        assert!(
            matches!(err, PlacementError::Monitor { .. }),
            "Monitor variant 以外が返った: {err:?}"
        );
    }

    /// `is_primary` のモニタの work area（物理 px）が RectPx へ忠実転写される
    /// （単位変換なし・2.12／U 契約）。
    #[test]
    fn primary_work_area_picks_is_primary() {
        let monitors = [
            make_monitor(1, (-1920, 0, 0, 1040), false),
            make_monitor(2, (0, 0, 2560, 1400), true),
        ];
        let wa = work_area_of(primary_monitor(&monitors)).expect("primary あり");
        assert_eq!(
            wa,
            RectPx {
                left: 0,
                top: 0,
                right: 2560,
                bottom: 1400
            }
        );
    }

    /// primary フラグ無し（列挙異常）は先頭モニタで代替（warn・窓は生やす方針）。
    #[test]
    fn primary_work_area_no_primary_substitutes_first() {
        let monitors = [
            make_monitor(1, (0, 0, 1920, 1040), false),
            make_monitor(2, (1920, 0, 3840, 1040), false),
        ];
        let wa = work_area_of(primary_monitor(&monitors)).expect("非空なら Ok");
        assert_eq!(
            wa,
            RectPx {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040
            }
        );
    }

    // ------------------------------------------------------------------
    // MonitorSnapshot::from_monitors（task 8.1・DD15・実モニタ→snapshot 忠実転写）
    // ------------------------------------------------------------------

    /// `MonitorSnapshot::from_monitors` は全モニタの work area（物理 px）を列挙順の
    /// まま**単位変換なしで忠実転写**する（`primary_work_area` と同じ U 契約）。
    #[test]
    fn monitor_snapshot_from_monitors_transcribes_all_work_areas_in_order() {
        let monitors = [
            make_monitor(1, (-1920, -40, 0, 1000), false),
            make_monitor(2, (0, 0, 2560, 1400), true),
        ];
        let snapshot = follow::MonitorSnapshot::from_monitors(&monitors);
        assert_eq!(
            snapshot.work_areas,
            vec![
                RectPx {
                    left: -1920,
                    top: -40,
                    right: 0,
                    bottom: 1000
                },
                RectPx {
                    left: 0,
                    top: 0,
                    right: 2560,
                    bottom: 1400
                },
            ]
        );
    }

    /// 0 台では空 snapshot（panic しない・消費側 `work_area_for_window` が None 防御）。
    #[test]
    fn monitor_snapshot_from_monitors_empty_is_empty() {
        assert!(
            follow::MonitorSnapshot::from_monitors(&[])
                .work_areas
                .is_empty()
        );
    }

    // ------------------------------------------------------------------
    // Send 契約・失敗経路・永続化なし（2.11）
    // ------------------------------------------------------------------

    /// `PreparedPlacement` は Send（design「main.rs seam」: Send な結果のみ返す）。
    #[test]
    fn prepared_placement_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<PreparedPlacement>();
    }

    /// 準備段階の失敗（ghost_root 不在→Mount）はこの関数内でフォールバックせず
    /// `PlacementError` として呼び手（シーム）へ返る（DD14 の分担）。
    /// 失敗は load（モニタ列挙より前）で起きるため headless でも決定論。
    #[test]
    fn prepare_missing_root_returns_mount_err_to_caller() {
        let root = std::env::temp_dir()
            .join("areka_placement_prepare_missing_root")
            .join("no_such_ghost");
        let err =
            prepare_ghost_windows(&root, &balloon_root()).expect_err("不在 root は Err");
        assert!(
            matches!(err, PlacementError::Mount(_)),
            "Mount variant 以外が返った: {err:?}"
        );
    }

    /// `prepare_ghost_windows` は永続を一切読み書きしない（2.11・A1）:
    /// (a) 実行後どこにも ghost.dat を生成しない（書き込みなし）
    /// (b) 偽の保存位置を持つ ghost.dat を plant しても出力が変わらない（読み込みなし）
    ///
    /// 永続ストアの実体は現在、統一プロパティシステム（sylphya）の別ファイル
    /// `sylphya.toml` にある——これは本 spec（areka-P0-position-persist）が
    /// **消費**する別系統であり、SSP 由来の `ghost.dat` とは無関係。復元の結線は
    /// placement シームの [`persist`] モジュール（と後続タスクの merge シーム）が担い、
    /// `prepare_ghost_windows` 自身は依然として永続を一切読み書きしない（A1＝prepare
    /// 不触は真のまま）。本檻は legacy な ghost.dat probe でその不触性を固定し続ける
    /// （sylphya.toml 系統でも同じく prepare は不触）。
    #[test]
    fn prepare_never_reads_or_writes_ghost_dat() {
        with_com_initialized(|| {
            // 最小の合成ゴースト（emo2 実 PNG を 2 枚だけ複写・descript は emo2 同値キー）
            let root = TempDir::new();
            let ghost_master = root.path().join("ghost").join("master");
            let shell_master = root.path().join("shell").join("master");
            fs::create_dir_all(&ghost_master).expect("create ghost/master");
            fs::create_dir_all(&shell_master).expect("create shell/master");
            fs::write(
                ghost_master.join("descript.txt"),
                "charset,UTF-8\nname,えも\nsakura.name,むらさき\nkero.name,エモ\n",
            )
            .expect("ghost descript");
            fs::write(
                shell_master.join("descript.txt"),
                "charset,UTF-8\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,0\nkero.defaultx,0\nsakura.balloon.alignment,left\nkero.balloon.alignment,right\n",
            )
            .expect("shell descript");
            fs::write(
                shell_master.join("surfaces.txt"),
                "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface10\n{\nelement0,overlay,surface10.png,0,0\n}\n",
            )
            .expect("surfaces.txt");
            fs::copy(
                emo2_root().join("shell/master/surface0.png"),
                shell_master.join("surface0.png"),
            )
            .expect("surface0.png 複写");
            fs::copy(
                emo2_root().join("shell/master/surface10.png"),
                shell_master.join("surface10.png"),
            )
            .expect("surface10.png 複写");

            let before =
                prepare_ghost_windows_with_work_area(root.path(), &balloon_root(), WA, Some(96))
                    .expect("合成ゴーストの準備は成功する");

            // (a) 書き込みなし: ghost.dat がどこにも生成されていない
            for dir in [root.path(), ghost_master.as_path(), shell_master.as_path()] {
                assert!(
                    !dir.join("ghost.dat").exists(),
                    "ghost.dat が生成された: {}",
                    dir.display()
                );
            }

            // (b) 読み込みなし: 偽の保存位置を plant しても出力不変（復元しない）
            let junk = "position.0.x,9999\r\nposition.0.y,9999\r\n";
            fs::write(root.path().join("ghost.dat"), junk).expect("plant root ghost.dat");
            fs::write(ghost_master.join("ghost.dat"), junk).expect("plant ghost ghost.dat");

            let after =
                prepare_ghost_windows_with_work_area(root.path(), &balloon_root(), WA, Some(96))
                    .expect("plant 後も準備は成功する");
            assert_eq!(
                before.placements, after.placements,
                "ghost.dat の有無で配置が変わった（復元経路が存在する疑い・2.11 違反）"
            );
            assert_eq!(before.titles, after.titles);
        });
    }

    // ------------------------------------------------------------------
    // 起動時 k₀ の導出（task 4.3・design D7／Flow 3 手順2・要件 1.4/3.3）
    // ------------------------------------------------------------------

    /// k₀ = primary モニタ DPI ÷ 作者基準 DPI（既約有理・連続 k・D2/D7）。
    ///
    /// 整数倍（192/96=2/1）だけでなく **非整数倍**（120/96=5/4・112/96=7/6）も
    /// 既約有理数として正しく導出されることを固定する（整数段階へ丸めない・要件 2.2）。
    #[test]
    fn build_measure_scaling_derives_k0_from_primary_and_author_dpi() {
        let author = AuthorDpi::DEFAULT; // 96/96（emo2 fixture と同じ無宣言ケース）

        let two = build_measure_scaling(Some(192), author);
        assert_eq!(two.shell, k(2, 1), "192/96 は既約 2/1");
        assert_eq!(two.balloon, k(2, 1));

        let five_quarters = build_measure_scaling(Some(120), author);
        assert_eq!(five_quarters.shell, k(5, 4), "120/96 は既約 5/4（125%）");

        let seven_sixths = build_measure_scaling(Some(112), author);
        assert_eq!(seven_sixths.shell, k(7, 6), "112/96 は既約 7/6（非整数 k）");

        let identity = build_measure_scaling(Some(96), author);
        assert_eq!(
            identity.shell,
            ScaleRatio::ONE,
            "primary DPI＝作者基準 DPI は恒等 k=1/1（要件 1.3/7.2）"
        );
        assert_eq!(identity.balloon, ScaleRatio::ONE);
    }

    /// shell と balloon は**それぞれ自分の**作者基準 DPI で k₀ を得る（2 軸独立・要件 1.1）。
    ///
    /// 取り違え（shell の k をバルーンへ適用する／2 値を入れ替える）は
    /// コンパイルを通ってしまう静かな誤表示ゆえ、両軸に**異なる**宣言値を与えて檻に入れる。
    #[test]
    fn build_measure_scaling_uses_each_axis_own_author_dpi() {
        let scaling = build_measure_scaling(
            Some(192),
            AuthorDpi {
                shell: 96,
                balloon: 144,
            },
        );
        assert_eq!(scaling.shell, k(2, 1), "shell は 192/96＝2/1");
        assert_eq!(
            scaling.balloon,
            k(4, 3),
            "balloon は 192/144＝4/3（shell の k ではない）"
        );

        // 入れ替え検出（同じ 2 値を逆に宣言すると k も逆になる）。
        let swapped = build_measure_scaling(
            Some(192),
            AuthorDpi {
                shell: 144,
                balloon: 96,
            },
        );
        assert_eq!(
            swapped.shell,
            k(4, 3),
            "shell 宣言 144 は shell の k を決める"
        );
        assert_eq!(
            swapped.balloon,
            k(2, 1),
            "balloon 宣言 96 は balloon の k を決める"
        );
    }

    /// primary モニタ DPI 取得不能（`None`／0）は **96 相当**へ縮退する
    /// （design「Error Handling」の boot 行・要件 1.4）。恒等を一律に返すのではなく、
    /// 96 を分子として作者基準 DPI との比を取る（作者が 96 以外を宣言していれば k₀≠1）。
    #[test]
    fn build_measure_scaling_degrades_to_96_equivalent_when_primary_dpi_unobtainable() {
        for missing in [None, Some(0)] {
            let default_author = build_measure_scaling(missing, AuthorDpi::DEFAULT);
            assert_eq!(
                default_author.shell,
                ScaleRatio::ONE,
                "author 96 に対する 96 相当は恒等（従来の native 採寸・{missing:?}）"
            );
            assert_eq!(default_author.balloon, ScaleRatio::ONE);

            let declared = build_measure_scaling(
                missing,
                AuthorDpi {
                    shell: 192,
                    balloon: 48,
                },
            );
            assert_eq!(
                declared.shell,
                k(1, 2),
                "96 相当 ÷ author 192＝1/2（恒等へ丸めない・{missing:?}）"
            );
            assert_eq!(declared.balloon, k(2, 1), "96 相当 ÷ author 48＝2/1");
        }
    }

    /// 要件 3.3／7.2 の錨（実 fixture・偽装境界の end-to-end）: 注入した primary DPI から
    /// 導かれた k₀ 倍後の**物理窓寸**で `ScopePlacement` が組み上がる。
    ///
    /// - primary 96（＝emo2 の作者基準 DPI）: 既存期待値のまま（434×687 等・要件 7.2）
    /// - primary 192: 全寸がちょうど 2 倍
    /// - primary 112（k=7/6・非整数）: 丸めは単一権威 `scaled_extent` に一致する。
    ///   とくに 687×7/6 は **ちょうど 801.5** ゆえ、round half away from zero なら 802・
    ///   切り捨て（`len*num/den`）なら 801 に分かれる（丸め規約の取り違え検出）。
    ///   f32 近道の検出は別檻 [`prepare_k0_rounding_matches_integer_authority_not_f32`]
    ///   が担う（本検体は f32 でも偶然 802 に一致するため判別力を持たない）。
    #[test]
    fn prepare_emo2_scales_window_sizes_by_k0() {
        with_com_initialized(|| {
            let native =
                prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
                    .expect("k₀=1/1 の準備は成功する");
            assert_eq!(
                native.placements[0].char_size,
                SizePx {
                    w: SCOPE0_W,
                    h: SCOPE0_H
                },
                "primary DPI＝作者基準 DPI では既存の native 寸のまま（要件 7.2）"
            );

            let doubled =
                prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(192))
                    .expect("k₀=2/1 の準備は成功する");
            assert_eq!(
                doubled.placements[0].char_size,
                SizePx {
                    w: SCOPE0_W * 2,
                    h: SCOPE0_H * 2
                },
                "primary DPI 192 なら scope0 キャラ窓は物理 2 倍（要件 3.3）"
            );
            assert_eq!(
                doubled.placements[1].char_size,
                SizePx {
                    w: SCOPE1_W * 2,
                    h: SCOPE1_H * 2
                }
            );
            // バルーン窓も自軸の k₀ で追従する。寸は **scope ごとに解決した系列**の
            // 原寸の 2 倍であり、全 placement で同一ではない（要件 3.1）。
            assert_eq!(
                doubled.placements[0].balloon_size,
                SizePx {
                    w: BALLOON0_W * 2,
                    h: BALLOON0_H * 2
                },
                "scope0 は本体側系列（400×224）の 2 倍"
            );
            assert_eq!(
                doubled.placements[1].balloon_size,
                SizePx {
                    w: BALLOON1_W * 2,
                    h: BALLOON1_H * 2
                },
                "scope1 は相方側系列（288×203）の 2 倍"
            );

            let seven_sixths =
                prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(112))
                    .expect("k₀=7/6 の準備は成功する");
            assert_eq!(
                seven_sixths.placements[0].char_size,
                SizePx { w: 506, h: 802 },
                "非整数 k₀ でも丸めは scaled_extent 単一権威（687×7/6＝ちょうど 801.5→802）"
            );
        });
    }

    /// 要件 1.4: primary モニタ DPI が取得できなくても**採寸は成立し窓寸が得られる**
    /// （表示を失わない縮退）。96 相当ゆえ emo2（author 96）では native 寸に一致する。
    #[test]
    fn prepare_emo2_without_primary_dpi_still_measures() {
        with_com_initialized(|| {
            let degraded =
                prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, None)
                    .expect("primary DPI 不明でも準備は成功する（表示を失わない）");
            assert_eq!(
                degraded.placements[0].char_size,
                SizePx {
                    w: SCOPE0_W,
                    h: SCOPE0_H
                },
                "96 相当の k₀ ＝ 従来と同じ native 採寸"
            );
            assert_eq!(degraded.placements.len(), 2);
        });
    }

    /// **宣言された**作者基準 DPI が (a) 採寸 k₀ と (b) attach 搬送値の双方へ効くこと
    /// （design Flow 3 手順1〜3・要件 1.1/3.3）。
    ///
    /// emo2 fixture は shell/balloon とも DPI 無宣言＝96 で、これは正典既定と**同値**ゆえ
    /// 「宣言を読んだ」のか「既定を返した」のかを区別できない。ここでは
    /// shell=120／balloon=144 を**実際に宣言する**合成ゴーストを組み、既定（96）とも
    /// 互いとも異なる 3 値で檻に入れる:
    ///
    /// - 既定へ差し替える誤り → k が 240/96=5/2 になり寸法が落ちる
    /// - shell/balloon の取り違え → char に 5/3・balloon に 2/1 が乗って落ちる
    #[test]
    fn prepare_declared_author_dpi_drives_each_axis_k0_and_attach_value() {
        with_com_initialized(|| {
            let root = TempDir::new();
            // shell=120（125% 原稿）・balloon=144（150% 原稿）——既定 96 とも互いとも異なる 3 値。
            let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(&root, "120", "144", None);

            let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(240))
                .expect("宣言 DPI 付き合成ゴーストの準備は成功する");

            // (a) attach 搬送値＝宣言値そのもの（既定 96 でも入れ替えでもない）。
            assert_eq!(
                p.author_dpi,
                AuthorDpi {
                    shell: 120,
                    balloon: 144
                },
                "宣言された作者基準 DPI が読まれ、そのまま attach 側へ搬送される"
            );

            // (b) 採寸 k₀: char は 240/120＝2/1・balloon は 240/144＝5/3（軸ごとに別）。
            // scope0 の合成寸は emo2 実 surface0 単体＝434×687（native 値と一致する検体）。
            assert_eq!(
                p.placements[0].char_size,
                SizePx {
                    w: SCOPE0_W * 2,
                    h: SCOPE0_H * 2
                },
                "char は shell 宣言 120 由来の k=2/1（balloon の 5/3 ではない）"
            );
            // 本補助の合成バルーンは本体側 `balloons0.png` しか持たないため、全 scope が
            // 本体側系列へ収束する（要件 3.7 の後方互換——`balloonk*` 不在なら全 scope 同一寸）。
            for placement in &p.placements {
                assert_eq!(
                    placement.balloon_size,
                    // 400×5/3＝666.67→667・224×5/3＝373.33→373（scaled_extent 単一権威）
                    SizePx { w: 667, h: 373 },
                    "balloon は balloon 宣言 144 由来の k=5/3（shell の 2/1 ではない）"
                );
            }
        });
    }

    /// f32 近道の混入検出（D4・記憶 areka の丸め単一権威）: `scaled_extent` の整数演算と
    /// `as_f32` 乗算が **1px 食い違う**検体を実経路へ通す。
    ///
    /// 検体: 作者基準 DPI 168・primary DPI 186 → k₀=31/28。emo2 の scope0 幅 434 に対し
    /// - 権威（整数）: (2×434×31+28)/(2×28) = 481（真値ちょうど 480.5 → 0 から遠い側）
    /// - `(434.0 × as_f32(31/28)).round()`: 480（31/28 の f32 表現が真値より小さい）
    ///
    /// k₀ の導出がどこかで f32 を経由していれば 480 になり本檻が落ちる。
    #[test]
    fn prepare_k0_rounding_matches_integer_authority_not_f32() {
        with_com_initialized(|| {
            let root = TempDir::new();
            let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(&root, "168", "96", None);

            let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(186))
                .expect("k₀=31/28 の準備は成功する");

            assert_eq!(
                p.placements[0].char_size,
                // 434→481・687→(2×687×31+28)/56=761
                SizePx { w: 481, h: 761 },
                "丸めは scaled_extent（整数）の値——f32 経由なら幅が 480 になる"
            );

            // 本検体が実際に f32 と食い違うことの自己検証（檻の有効性そのものを固定する）。
            let via_f32 = (SCOPE0_W as f32 * k(31, 28).as_f32()).round() as i32;
            assert_eq!(via_f32, 480, "f32 経路は 480（＝この検体は判別力を持つ）");
        });
    }

    /// 宣言 DPI 付きの最小合成ゴーストを一時ディレクトリへ組む（テスト補助）。
    ///
    /// `prepare_never_reads_or_writes_ghost_dat` の合成ゴーストと同型（emo2 の実 PNG を
    /// 複写した最小 shell）に、`seriko.dpi`／balloon `dpi` の宣言を足したもの。
    /// 返り値は `(ghost_root, balloon_root)`。
    ///
    /// 注意: 合成 shell の `surfaces.txt` は単一 overlay ゆえ、scope0 は emo2 実測と同じ
    /// 434×687 になるが scope1（surface10 単体）は emo2 実 shell の合成寸（336×400）とは
    /// 異なる——本補助を使う檻は scope0 と balloon を期待値の錨に用いる。
    ///
    /// 合成バルーンへ複写するのは本体側 `balloons0.png` の 1 枚のみ（相方側 `balloonk*` を
    /// 置かない）。ゆえに全 scope の連鎖が本体側系列へ収束し、バルーン寸は scope に依らず
    /// 同一になる——本補助を使う檻は同時に要件 3.7（`balloonk*` 不在時の後方互換）の錨でもある。
    ///
    /// `balloon_windowposition`（task 4.3）: `Some((x, y))` ならバルーン既定設定へ
    /// `windowposition.x`/`.y` を宣言する（面別上書き `balloons0s.txt` は置かないので、
    /// 確定値はこの基層のみが供給する）。`None` なら**どの層にも `windowposition` が無い**
    /// ＝要件 3.4 の「数値指定なし」検体になる。
    fn synth_declared_dpi_ghost(
        root: &TempDir,
        shell_dpi: &str,
        balloon_dpi: &str,
        balloon_windowposition: Option<(i32, i32)>,
    ) -> (PathBuf, PathBuf) {
        let ghost_master = root.path().join("ghost").join("master");
        let shell_master = root.path().join("shell").join("master");
        let balloon_dir = root.path().join("balloon-declared");
        fs::create_dir_all(&ghost_master).expect("create ghost/master");
        fs::create_dir_all(&shell_master).expect("create shell/master");
        fs::create_dir_all(&balloon_dir).expect("create balloon dir");
        fs::write(
            ghost_master.join("descript.txt"),
            "charset,UTF-8\nname,えも\nsakura.name,むらさき\nkero.name,エモ\n",
        )
        .expect("ghost descript");
        fs::write(
            shell_master.join("descript.txt"),
            format!(
                "charset,UTF-8\nseriko.dpi,{shell_dpi}\nseriko.alignmenttodesktop,bottom\nsakura.defaultx,0\nkero.defaultx,0\nsakura.balloon.alignment,left\nkero.balloon.alignment,right\n"
            ),
        )
        .expect("shell descript");
        fs::write(
            shell_master.join("surfaces.txt"),
            "surface0\n{\nelement0,overlay,surface0.png,0,0\n}\nsurface10\n{\nelement0,overlay,surface10.png,0,0\n}\n",
        )
        .expect("surfaces.txt");
        for png in ["surface0.png", "surface10.png"] {
            fs::copy(
                emo2_root().join("shell/master").join(png),
                shell_master.join(png),
            )
            .unwrap_or_else(|e| panic!("{png} 複写: {e}"));
        }
        let windowposition = match balloon_windowposition {
            Some((x, y)) => format!("windowposition.x,{x}\nwindowposition.y,{y}\n"),
            None => String::new(),
        };
        fs::write(
            balloon_dir.join("descript.txt"),
            format!("charset,UTF-8\ndpi,{balloon_dpi}\n{windowposition}"),
        )
        .expect("balloon descript");
        fs::copy(
            balloon_root().join("balloons0.png"),
            balloon_dir.join("balloons0.png"),
        )
        .expect("balloons0.png 複写");
        (root.path().to_path_buf(), balloon_dir)
    }

    /// design Flow 3 手順1（**1 度だけ読む**）: 準備が読んだ作者基準 DPI が
    /// `PreparedPlacement` に載って attach 側（`wire_emo2_boot`→`attach_target`）へ渡る。
    ///
    /// 単一読取の証拠は「読取器（task 2.1）を直接呼んだ値と、準備の戻り値が一致すること」。
    /// 本番 fixture（emo2）は 96/96 ゆえ既定との区別が付かない——宣言値との区別は
    /// [`prepare_declared_author_dpi_drives_each_axis_k0_and_attach_value`] が担う。
    #[test]
    fn prepare_emo2_carries_author_dpi_read_once_for_attach() {
        with_com_initialized(|| {
            let src = source::load_descript_source(&emo2_root()).expect("emo2 fixture は読める");
            let expected = AuthorDpi {
                shell: src.shell_author_dpi(),
                balloon: source::load_balloon_author_dpi(&balloon_root()),
            };
            assert_eq!(
                expected,
                AuthorDpi {
                    shell: 96,
                    balloon: 96
                },
                "emo2 は shell/balloon とも DPI 無宣言＝正典既定 96（task 2.1 実測）"
            );

            let p =
                prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(192))
                    .expect("準備は成功する");
            assert_eq!(
                p.author_dpi, expected,
                "採寸 k₀ に使った宣言値がそのまま attach 側へ搬送される（読取は 1 度）"
            );
        });
    }

    // ------------------------------------------------------------------
    // windowposition → 初期既定位置（task 4.3・要件 3.2/3.4/3.5/3.6/6.3/7.6）
    // ------------------------------------------------------------------

    /// 観測点 4（design Monitoring・要件 6.3/7.6）: `prepare_stages` が scope ごとに
    /// **scope・wp 生値・バルーンの左右・変換後の調整量**を info レベルで記録する。
    ///
    /// 実機サインオフ（task 6.1）は本行を `RUST_LOG=info` で grep し、x 方向の符号の
    /// 向き（`windowposition.rs` の符号表＝実機確定待ちの仮説・R7.6）を決定論的に
    /// 突合する。ゆえに**フィールド名と値の形そのものが契約**である。
    #[test]
    fn prepare_windowposition_logs_scope_raw_side_and_converted_adjust() {
        with_com_initialized(|| {
            let (result, events) = capture_logs(|| {
                prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
            });
            result.expect("emo2 fixture の配置準備は成功する");

            let hits: Vec<&LogEvent> = events
                .iter()
                .filter(|e| {
                    e.message()
                        .contains("windowposition を初期既定位置の調整量へ")
                })
                .collect();
            assert_eq!(
                hits.len(),
                2,
                "scope ごとに 1 行（emo2 は 2 スコープ）: {events:?}"
            );

            // scope0（本体側・`balloons0s.txt` の 266,-129・バルーンはキャラ左）。
            assert_eq!(hits[0].level, tracing::Level::INFO);
            assert_eq!(hits[0].field("scope"), "0");
            assert_eq!(hits[0].field("windowposition_x"), "Some(266)");
            assert_eq!(hits[0].field("windowposition_y"), "Some(-129)");
            assert_eq!(hits[0].field("balloon_side"), "Left");
            assert_eq!(hits[0].field("adjust_dx"), "266");
            assert_eq!(hits[0].field("adjust_dy"), "-129");
            assert_eq!(hits[0].field("adjusted"), "true");

            // scope1（相方側・`balloonk0s.txt` の -190,-75・バルーンはキャラ右＝x 反転）。
            assert_eq!(hits[1].field("scope"), "1");
            assert_eq!(hits[1].field("windowposition_x"), "Some(-190)");
            assert_eq!(hits[1].field("windowposition_y"), "Some(-75)");
            assert_eq!(hits[1].field("balloon_side"), "Right");
            assert_eq!(
                hits[1].field("adjust_dx"),
                "190",
                "生値 −190 は Right 側で画面 +x へ反転する（符号表の実機突合対象・R7.6）"
            );
            assert_eq!(hits[1].field("adjust_dy"), "-75");
        });
    }

    /// 要件 3.6: `windowposition` 由来の調整量は**バルーン軸の k**（`scaling.balloon`）で
    /// 拡縮する（`windowposition` はバルーン作者の空間で書かれた値ゆえ・shell の k ではない）。
    ///
    /// 検体（shell=120／balloon=144／primary=240）は k_shell=2/1・k_balloon=5/3 と
    /// **異なる 2 値**ゆえ取り違えが観測できる（既存の
    /// [`prepare_declared_author_dpi_drives_each_axis_k0_and_attach_value`] と同じ流儀）:
    /// - 権威（balloon 軸 5/3）: 266×5/3＝443.33→443・129×5/3＝ちょうど 215
    /// - 誤って shell 軸 2/1 を使うと 532／258 になり本檻が落ちる
    ///
    /// 合成バルーンは `balloons0.png` の 1 枚のみゆえ両 scope が本体側系列へ収束し、
    /// **同一の wp 生値**が side の違い（left／right）だけで逆符号になることも同時に固定する。
    #[test]
    fn prepare_windowposition_adjust_scales_with_balloon_k_not_shell_k() {
        with_com_initialized(|| {
            let root = TempDir::new();
            let (ghost_root, balloon_dir) =
                synth_declared_dpi_ghost(&root, "120", "144", Some((266, -129)));

            let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(240))
                .expect("windowposition 宣言つき合成ゴーストの準備は成功する");

            // balloon 寸は k=5/3 で 667×373（既存檻と同値）。
            let s0 = &p.placements[0];
            assert_eq!(s0.balloon_size, SizePx { w: 667, h: 373 });
            assert_eq!(
                s0.balloon_offset,
                // 基本位置（左隣＝−667）＋調整量 (+443, −215)
                PointPx { x: -224, y: -215 },
                "balloon 軸 k=5/3 で拡縮（shell 軸 2/1 なら (−135, −258) になる）"
            );

            let s1 = &p.placements[1];
            assert_eq!(
                s1.balloon_offset,
                // 基本位置（右隣＝+char_w）＋調整量 (−443, −215)（side=Right ゆえ x 反転）
                PointPx {
                    x: s1.char_size.w - 443,
                    y: -215
                },
                "同一の wp 生値でも side が違えば x の符号が反転する"
            );
        });
    }

    /// 要件 3.4／design Postconditions: `windowposition` の数値指定が**どの層にも無い**
    /// とき（かつ descript の `balloon.offsetx/offsety` も無いとき）`balloon_offset` は
    /// `None` のまま＝resolver 出力は本仕様適用前と bit 同一である。
    ///
    /// 観測面は resolver の出力そのもの: left 置き＝`−balloon_w`・right 置き＝`+char_w`
    /// （P5 の基本位置ちょうど・y はゼロ）。
    #[test]
    fn prepare_without_windowposition_keeps_resolver_output_bit_identical() {
        with_com_initialized(|| {
            let root = TempDir::new();
            // 作者基準 DPI も 96／96（k₀=1/1）＝調整量以外の変化要因を持たない検体。
            let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(&root, "96", "96", None);

            let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(96))
                .expect("windowposition 無宣言の合成ゴーストの準備は成功する");

            let s0 = &p.placements[0];
            assert_eq!(
                s0.balloon_offset,
                PointPx {
                    x: -s0.balloon_size.w,
                    y: 0
                },
                "scope0（left 置き）は基本位置ちょうど＝本仕様適用前と同一"
            );
            let s1 = &p.placements[1];
            assert_eq!(
                s1.balloon_offset,
                PointPx {
                    x: s1.char_size.w,
                    y: 0
                },
                "scope1（right 置き）も基本位置ちょうど＝本仕様適用前と同一"
            );
        });
    }

    /// 要件 3.5: 永続化されたバルーン相対位置があるとき、復元 merge（`persist.rs`・無改変）は
    /// **永続値を優先**し、`windowposition` 由来の初期既定位置は結果に一切残らない。
    ///
    /// 檻の作り: 同じ永続 entries を (a) 本準備の出力（windowposition 反映済み）と
    /// (b) `balloon_offset` を潰した対照へ適用し、**両者の復元結果が一致する**ことを見る。
    /// 一致すれば復元後の値は永続値のみに依存する＝供給元の増加が優先順位を変えていない。
    #[test]
    fn prepare_windowposition_yields_to_persisted_balloon_offset() {
        use areka_sylphya::{Axis, PersistKey};

        with_com_initialized(|| {
            let p =
                prepare_ghost_windows_with_work_area(&emo2_root(), &balloon_root(), WA, Some(96))
                    .expect("emo2 fixture の配置準備は成功する");
            // 前提: windowposition が実際に効いている（対照が空虚一致にならないことの確認）。
            assert_ne!(
                p.placements[0].balloon_offset,
                PointPx {
                    x: -p.placements[0].balloon_size.w,
                    y: 0
                },
                "windowposition 反映前の値と同じでは檻が無意味"
            );

            let entry = |scope: u32, axis: Axis, v: &str| {
                (PersistKey::BalloonOffset { scope, axis }, v.to_string())
            };
            let entries = vec![
                entry(0, Axis::X, "-512"),
                entry(0, Axis::Y, "-48"),
                entry(1, Axis::X, "640"),
                entry(1, Axis::Y, "-16"),
            ];
            let snapshot = follow::MonitorSnapshot {
                work_areas: vec![WA],
            };

            // 対照: windowposition の寄与を取り除いた（基本位置ちょうどの）配置。
            let control: Vec<ScopePlacement> = p
                .placements
                .iter()
                .map(|s| {
                    let balloon_offset = PointPx { x: 0, y: 0 };
                    ScopePlacement {
                        balloon_pos: s.char_pos,
                        balloon_offset,
                        ..*s
                    }
                })
                .collect();

            let restored =
                persist::apply_restored_placements(p.placements.clone(), &entries, &snapshot);
            let restored_control = persist::apply_restored_placements(control, &entries, &snapshot);

            assert_eq!(
                restored, restored_control,
                "永続値がある scope の復元結果は windowposition に依存しない（要件 3.5）"
            );
        });
    }
}
