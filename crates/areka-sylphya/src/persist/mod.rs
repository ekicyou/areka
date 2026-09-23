//! 永続層: `PersistScope` / `ScopeRoots` / `PersistKey`（4 key 族 typed）/ 載せ替え orchestration。
//!
//! [`format`]（Task 4.1・TOML スキーマ／寛容読取）と [`io`]（Task 4.2・原子的 IO シーム）の
//! 上に、**typed な 4 key 族モデル**（design「永続層 → Service Interface」）と、その
//! **層別スコープごとのロード／保存編成**（[`load_scope`]／[`save_scope`]）を載せる。
//!
//! ## 責務境界
//!
//! - スコープは 4 層固定 enum（[`PersistScope`]・App/Ghost/Shell/Balloon）。各スコープの保存先
//!   ルートは呼び出し側供給（[`ScopeRoots`]・sylphya はパスを解釈しない・最下層規律）。M1 の本番
//!   key はすべて Ghost スコープに載るが（design「Physical Data Model」）、App/Shell/Balloon も
//!   器として実装しスコープ分離檻で検証する。
//! - key は 4 族の typed enum（[`PersistKey`]）。自由 key の汎用永続は将来シーム（2 例目が要求
//!   してから）。各 key は 2 通りに写像される:
//!   1. **正準 key 文字列**（[`PersistKey::to_canonical_key`]）——鏡像 dotted 区画へ投影する形
//!      （`areka.window.scope(0).x` 等）。これは [`crate::key::parse_dotted`] →
//!      [`crate::PropPath::to_canonical_string`] と往復整合する（読み口 1 本化の要石）。
//!   2. **TOML 写像**（[`FormatDoc`]）——`[window."0"]` x/y 等（Task 4.1 の物理スキーマ）。
//!
//! ## load / save 編成
//!
//! - [`load_scope`]: 起動時一括ロード。root 不在 → 空（不在縮退）。`<root>/sylphya.toml` を
//!   [`PersistIo::read`] で読み、[`format::read_toml_str`] で寛容パースし、実在値のみ
//!   `(PersistKey, String)` へ変換する。read 障害・非数値スコープ ID 等はすべて縮退（panic なし）。
//! - [`save_scope`]: 当該スコープの原子的保存（write-through）。**read-modify-write マージ**で
//!   既存 doc に entries を重ね（無関係な key を温存し）、[`format::to_toml_string`] で直列化し、
//!   [`PersistIo::commit`] で原子的確定する。成功 → [`PersistOutcome::Saved`]、root 不在・commit
//!   失敗 → error!/warn! ＋ [`PersistOutcome::Degraded`]（R6.7・無音失敗なし）。
//!
//! 鏡像／アクターへの結線は行わない（Task 5.x の領分）。本モジュールは注入された [`PersistIo`]
//! の上で typed モデル＋ load/save 編成のみを提供する。

pub mod format;
pub mod io;

pub use format::{FORMAT_VERSION, FormatDoc};
pub use io::{FakePersistIo, FsPersistIo, PersistIo};

use format::AxisPair;
use std::path::{Path, PathBuf};

/// ログ target（steering: areka-log-first-no-silent-failure）。
const LOG_TARGET: &str = "areka_sylphya::persist";

/// 各スコープの永続ファイル名（`<scope root>/sylphya.toml`・design「Physical Data Model」）。
const PERSIST_FILE_NAME: &str = "sylphya.toml";

/// 層別永続スコープ（R6.5・伺か慣行の profile フォルダ準拠）。
///
/// 各層の永続情報は対応する層の profile フォルダ（[`ScopeRoots`] が供給するルート）へ保存する。
/// M1 の本番 key はすべて [`PersistScope::Ghost`] に載るが（design「Physical Data Model」）、
/// 他 3 層も器として実装しスコープ分離を檻で検証する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PersistScope {
    /// areka アプリレベル。
    App,
    /// SHIORI（ゴースト）レベル。M1 の 4 key 族はここに載る。
    Ghost,
    /// シェルレベル。
    Shell,
    /// バルーンレベル。
    Balloon,
}

/// 各スコープの保存先ルート（呼び出し側が供給。`None` ＝当該スコープ利用不可＝不在縮退）。
///
/// 実ファイルは `<root>/sylphya.toml`（root 自体を `profile/areka/` に取る運用は結線側の契約）。
/// sylphya はパスを解釈しない（最下層規律）——所属実体（ゴースト等）の分離は per-実体 profile
/// ディレクトリの物理分離が担う（R6.5・design「Responsibilities & Constraints」）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScopeRoots {
    /// [`PersistScope::App`] のルート。
    pub app: Option<PathBuf>,
    /// [`PersistScope::Ghost`] のルート。
    pub ghost: Option<PathBuf>,
    /// [`PersistScope::Shell`] のルート。
    pub shell: Option<PathBuf>,
    /// [`PersistScope::Balloon`] のルート。
    pub balloon: Option<PathBuf>,
}

impl ScopeRoots {
    /// 指定スコープのルート（未設定は `None` ＝不在縮退）。
    fn root_of(&self, scope: PersistScope) -> Option<&PathBuf> {
        match scope {
            PersistScope::App => self.app.as_ref(),
            PersistScope::Ghost => self.ghost.as_ref(),
            PersistScope::Shell => self.shell.as_ref(),
            PersistScope::Balloon => self.balloon.as_ref(),
        }
    }

    /// 指定スコープの永続ファイルパス（`<root>/sylphya.toml`）。root 不在なら `None`。
    fn file_of(&self, scope: PersistScope) -> Option<PathBuf> {
        self.root_of(scope).map(|root| root.join(PERSIST_FILE_NAME))
    }
}

/// 軸（窓位置・バルーンオフセットの成分）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    /// X 軸 → 正準 key `.x`・TOML `x`。
    X,
    /// Y 軸 → 正準 key `.y`・TOML `y`。
    Y,
}

impl Axis {
    /// 正準 key／TOML field で使う小文字 1 文字（`x` / `y`）。
    fn as_str(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
        }
    }
}

/// 4 key 族（R6.1・typed——W4 が消費する契約の正本。値ドメインは文字列）。
///
/// 各 variant は正準 key 文字列（[`PersistKey::to_canonical_key`]）と TOML 写像
/// （[`FormatDoc`]）の双方へ決定論的に対応する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PersistKey {
    /// 窓位置（キャラクタースコープ別）→ 正準 key `areka.window.scope(ID).x|y`・TOML `[window."ID"]`。
    WindowPos {
        /// キャラクタースコープ ID。
        scope: u32,
        /// 軸。
        axis: Axis,
    },
    /// バルーン相対オフセット（スコープ別）→ `areka.balloon.offset.scope(ID).x|y`・TOML `[balloon-offset."ID"]`。
    BalloonOffset {
        /// キャラクタースコープ ID。
        scope: u32,
        /// 軸。
        axis: Axis,
    },
    /// 起動記録 → 正準 key `areka.boot.count`・TOML `[boot]` count。
    BootCount,
    /// vanish 回数 → 正準 key `areka.vanish.count`・TOML `[vanish]` count。
    VanishCount,
}

impl PersistKey {
    /// 鏡像 dotted 区画へ投影する正準 key 文字列（design「正準 key 投影」）。
    ///
    /// **不変条件**: `parse_dotted(k).unwrap().to_canonical_string() == k`（[`crate::key`] と往復
    /// 整合——この投影で格納された値は [`crate::reader::SylphyaReader::resolve_dotted`] が同じ
    /// 正準形で引ける）。この整合は檻（`canonical_key_round_trips_with_to_canonical_string`）で保証。
    pub fn to_canonical_key(self) -> String {
        match self {
            PersistKey::WindowPos { scope, axis } => {
                format!("areka.window.scope({scope}).{}", axis.as_str())
            }
            PersistKey::BalloonOffset { scope, axis } => {
                format!("areka.balloon.offset.scope({scope}).{}", axis.as_str())
            }
            PersistKey::BootCount => "areka.boot.count".to_string(),
            PersistKey::VanishCount => "areka.vanish.count".to_string(),
        }
    }
}

/// 永続保存の結果（design「永続層 → Service Interface」）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistOutcome {
    /// 原子的確定に成功（以後のロードで同値復元・R6.6）。
    Saved,
    /// 保存失敗（root 不在・commit 失敗）→ ログ済み・縮退（鏡像反映はアクターの領分・R6.7）。
    Degraded,
}

/// 当該スコープの永続状態を一括ロードする（起動時・R6.1/R6.3/R6.5）。
///
/// - root 不在（[`ScopeRoots`] の該当が `None`）→ 空（当該スコープ不在縮退・debug）。
/// - `<root>/sylphya.toml` を [`PersistIo::read`] で読む。不在（`Ok(None)`）→ 空。read 障害
///   （`Err`）→ warn! ＋ 空（縮退・panic なし・R6.7）。
/// - 読めた content は [`format::read_toml_str`] で寛容パースし、実在値のみ `(PersistKey, String)`
///   へ変換する。非数値スコープ ID（typed [`u32`] に載らない）は debug ＋ 当該エントリskip（寛容）。
///
/// 返り値は決定論的順序（[`FormatDoc`] は [`std::collections::BTreeMap`] ゆえ scope ID 昇順、
/// window → balloon-offset → boot → vanish、各 pair は x → y）。
pub fn load_scope(
    scope: PersistScope,
    roots: &ScopeRoots,
    io: &dyn PersistIo,
) -> Vec<(PersistKey, String)> {
    let Some(path) = roots.file_of(scope) else {
        tracing::debug!(
            target: LOG_TARGET,
            ?scope,
            "persist scope root absent; loading as empty (tolerant)"
        );
        return Vec::new();
    };

    let content = match io.read(&path) {
        Ok(Some(content)) => content,
        Ok(None) => {
            tracing::debug!(target: LOG_TARGET, ?scope, path = %path.display(), "persist file absent; loading as empty");
            return Vec::new();
        }
        Err(e) => {
            // 真の IO 障害（権限等）→ warn ＋ 不在縮退（無音失敗なし・R6.7）。
            tracing::warn!(
                target: LOG_TARGET,
                ?scope,
                path = %path.display(),
                error = %e,
                "persist read failed; degrading to absent (tolerant)"
            );
            return Vec::new();
        }
    };

    let doc = format::read_toml_str(&content);
    doc_to_entries(scope, &doc)
}

/// 当該スコープへ entries を原子的に保存する（write-through・R6.1/R6.2/R6.6）。
///
/// **read-modify-write マージ**: 既存 `<root>/sylphya.toml` を読んで [`FormatDoc`] へ寛容パース
/// し、entries を重ねてから直列化・確定する。これにより例えば `boot.count` だけを保存しても
/// `window.*` 等の無関係な key が消えない（マージ檻で保証）。
///
/// - root 不在 → warn! ＋ [`PersistOutcome::Degraded`]（保存先が無い・panic なし・R6.7）。
/// - 既存読取が `Err` → warn! ＋ 既存を空とみなして続行（tolerant・design「read 失敗→不在」）。
/// - [`PersistIo::commit`] 失敗 → error! ＋ [`PersistOutcome::Degraded`]（既存ファイルは temp→rename
///   ゆえ無傷・R6.2。鏡像更新はアクターの領分——ここは結果のみ報告）。
/// - 成功 → [`PersistOutcome::Saved`]。
pub fn save_scope(
    scope: PersistScope,
    roots: &ScopeRoots,
    io: &dyn PersistIo,
    entries: Vec<(PersistKey, String)>,
) -> PersistOutcome {
    let Some(path) = roots.file_of(scope) else {
        tracing::warn!(
            target: LOG_TARGET,
            ?scope,
            "persist scope root absent; save degraded (no destination)"
        );
        return PersistOutcome::Degraded;
    };

    // read-modify-write: 既存 doc を土台にマージ（無関係 key を温存）。
    let mut doc = read_existing(scope, io, &path);
    for (key, value) in entries {
        apply_entry(&mut doc, key, value);
    }

    let content = format::to_toml_string(&doc);
    match io.commit(&path, &content) {
        Ok(()) => PersistOutcome::Saved,
        Err(e) => {
            tracing::error!(
                target: LOG_TARGET,
                ?scope,
                path = %path.display(),
                error = %e,
                "persist commit failed; existing file intact (temp→rename), reporting Degraded"
            );
            PersistOutcome::Degraded
        }
    }
}

/// save のマージ土台となる既存 doc を読む（read 障害・不在は空 doc へ寛容縮退）。
fn read_existing(scope: PersistScope, io: &dyn PersistIo, path: &Path) -> FormatDoc {
    match io.read(path) {
        Ok(Some(content)) => format::read_toml_str(&content),
        Ok(None) => FormatDoc::default(),
        Err(e) => {
            tracing::warn!(
                target: LOG_TARGET,
                ?scope,
                path = %path.display(),
                error = %e,
                "persist read-before-write failed; merging onto empty base (tolerant)"
            );
            FormatDoc::default()
        }
    }
}

/// 1 entry を [`FormatDoc`] へ適用する（TOML 写像・design「TOML mapping」）。
fn apply_entry(doc: &mut FormatDoc, key: PersistKey, value: String) {
    match key {
        PersistKey::WindowPos { scope, axis } => {
            set_axis(
                doc.window.entry(scope.to_string()).or_default(),
                axis,
                value,
            );
        }
        PersistKey::BalloonOffset { scope, axis } => {
            set_axis(
                doc.balloon_offset.entry(scope.to_string()).or_default(),
                axis,
                value,
            );
        }
        PersistKey::BootCount => doc.boot_count = Some(value),
        PersistKey::VanishCount => doc.vanish_count = Some(value),
    }
}

/// [`AxisPair`] の指定軸へ値を書く。
fn set_axis(pair: &mut AxisPair, axis: Axis, value: String) {
    match axis {
        Axis::X => pair.x = Some(value),
        Axis::Y => pair.y = Some(value),
    }
}

/// [`FormatDoc`] を `(PersistKey, String)` エントリ列へ変換する（実在値のみ・決定論順）。
fn doc_to_entries(scope: PersistScope, doc: &FormatDoc) -> Vec<(PersistKey, String)> {
    let mut out = Vec::new();

    push_axis_family(scope, &mut out, &doc.window, |scope_id, axis| {
        PersistKey::WindowPos {
            scope: scope_id,
            axis,
        }
    });
    push_axis_family(scope, &mut out, &doc.balloon_offset, |scope_id, axis| {
        PersistKey::BalloonOffset {
            scope: scope_id,
            axis,
        }
    });

    if let Some(v) = &doc.boot_count {
        out.push((PersistKey::BootCount, v.clone()));
    }
    if let Some(v) = &doc.vanish_count {
        out.push((PersistKey::VanishCount, v.clone()));
    }

    out
}

/// window / balloon-offset の軸族 1 つをエントリ列へ展開する（非数値 ID は寛容 skip）。
fn push_axis_family(
    scope: PersistScope,
    out: &mut Vec<(PersistKey, String)>,
    map: &std::collections::BTreeMap<String, AxisPair>,
    make: impl Fn(u32, Axis) -> PersistKey,
) {
    for (id, pair) in map {
        // typed スコープは u32。非数値 ID（format 層は不透明文字列を許す）は載せられない——
        // debug ＋ skip（寛容・panic なし・R6.7）。
        let Ok(scope_id) = id.parse::<u32>() else {
            tracing::debug!(
                target: LOG_TARGET,
                ?scope,
                raw_id = %id,
                "persist scope id is not a u32; skipping entry (tolerant)"
            );
            continue;
        };
        if let Some(x) = &pair.x {
            out.push((make(scope_id, Axis::X), x.clone()));
        }
        if let Some(y) = &pair.y {
            out.push((make(scope_id, Axis::Y), y.clone()));
        }
    }
}

#[cfg(test)]
#[path = "persist_tests.rs"]
mod tests;
