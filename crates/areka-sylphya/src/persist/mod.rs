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

pub use format::{FormatDoc, FORMAT_VERSION};
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
            set_axis(doc.window.entry(scope.to_string()).or_default(), axis, value);
        }
        PersistKey::BalloonOffset { scope, axis } => {
            set_axis(doc.balloon_offset.entry(scope.to_string()).or_default(), axis, value);
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
        PersistKey::WindowPos { scope: scope_id, axis }
    });
    push_axis_family(scope, &mut out, &doc.balloon_offset, |scope_id, axis| {
        PersistKey::BalloonOffset { scope: scope_id, axis }
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
mod tests {
    use super::*;
    use crate::key::parse_dotted;

    fn all_families() -> Vec<PersistKey> {
        vec![
            PersistKey::WindowPos { scope: 0, axis: Axis::X },
            PersistKey::WindowPos { scope: 2, axis: Axis::Y },
            PersistKey::BalloonOffset { scope: 0, axis: Axis::X },
            PersistKey::BalloonOffset { scope: 1, axis: Axis::Y },
            PersistKey::BootCount,
            PersistKey::VanishCount,
        ]
    }

    // --- 正準 key 往復（Task 3.2 の申し送り: to_canonical_string と一致・読み口 1 本化の要石）---

    #[test]
    fn canonical_key_round_trips_with_to_canonical_string() {
        for key in all_families() {
            let s = key.to_canonical_key();
            let back = parse_dotted(&s).unwrap().to_canonical_string();
            assert_eq!(back, s, "canonical mismatch for {key:?}");
        }
    }

    #[test]
    fn canonical_key_exact_strings() {
        assert_eq!(
            PersistKey::WindowPos { scope: 0, axis: Axis::X }.to_canonical_key(),
            "areka.window.scope(0).x"
        );
        assert_eq!(
            PersistKey::WindowPos { scope: 3, axis: Axis::Y }.to_canonical_key(),
            "areka.window.scope(3).y"
        );
        assert_eq!(
            PersistKey::BalloonOffset { scope: 0, axis: Axis::X }.to_canonical_key(),
            "areka.balloon.offset.scope(0).x"
        );
        assert_eq!(PersistKey::BootCount.to_canonical_key(), "areka.boot.count");
        assert_eq!(PersistKey::VanishCount.to_canonical_key(), "areka.vanish.count");
    }

    // --- 4 key 族 put→load 往復（R6.6・完了条件）---

    #[test]
    fn four_family_put_load_value_round_trip() {
        let io = FakePersistIo::new();
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/ghost")), ..ScopeRoots::default() };
        let entries = vec![
            (PersistKey::WindowPos { scope: 0, axis: Axis::X }, "1024".to_string()),
            (PersistKey::WindowPos { scope: 0, axis: Axis::Y }, "512".to_string()),
            (PersistKey::BalloonOffset { scope: 0, axis: Axis::X }, "30".to_string()),
            (PersistKey::BalloonOffset { scope: 0, axis: Axis::Y }, "-10".to_string()),
            (PersistKey::BootCount, "3".to_string()),
            (PersistKey::VanishCount, "0".to_string()),
        ];
        assert_eq!(
            save_scope(PersistScope::Ghost, &roots, &io, entries.clone()),
            PersistOutcome::Saved
        );
        let loaded = load_scope(PersistScope::Ghost, &roots, &io);
        for (k, v) in &entries {
            let found = loaded.iter().find(|(lk, _)| lk == k).map(|(_, lv)| lv.clone());
            assert_eq!(found.as_deref(), Some(v.as_str()), "family {k:?} did not round-trip");
        }
    }

    #[test]
    fn round_trip_preserves_negative_and_multi_scope() {
        let io = FakePersistIo::new();
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
        let entries = vec![
            (PersistKey::WindowPos { scope: 0, axis: Axis::X }, "-5".to_string()),
            (PersistKey::WindowPos { scope: 1, axis: Axis::Y }, "200".to_string()),
        ];
        save_scope(PersistScope::Ghost, &roots, &io, entries.clone());
        let loaded = load_scope(PersistScope::Ghost, &roots, &io);
        for (k, v) in &entries {
            assert!(loaded.contains(&(*k, v.clone())), "missing {k:?}={v}");
        }
    }

    // --- TOML 写像の正しさ（保存 doc の配置）---

    #[test]
    fn toml_mapping_places_families_under_expected_tables() {
        let io = FakePersistIo::new();
        let path = PathBuf::from("/g/sylphya.toml");
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
        save_scope(
            PersistScope::Ghost,
            &roots,
            &io,
            vec![
                (PersistKey::WindowPos { scope: 0, axis: Axis::X }, "1024".into()),
                (PersistKey::BalloonOffset { scope: 0, axis: Axis::Y }, "-10".into()),
                (PersistKey::BootCount, "3".into()),
                (PersistKey::VanishCount, "0".into()),
            ],
        );
        let serialized = io.read(&path).unwrap().unwrap();
        // toml は数字のみの key を bare key として出力する（"0" → window.0）。
        assert!(serialized.contains("[window.0]"), "serialized=\n{serialized}");
        assert!(serialized.contains("[balloon-offset.0]"), "serialized=\n{serialized}");
        assert!(serialized.contains("[boot]"), "serialized=\n{serialized}");
        assert!(serialized.contains("[vanish]"), "serialized=\n{serialized}");
        assert!(serialized.contains("format-version = 1"), "serialized=\n{serialized}");
        // 値の配置も確認（x は window 表下・count は boot/vanish 表下）。
        assert!(serialized.contains("x = \"1024\""), "serialized=\n{serialized}");
    }

    // --- マージが無関係 key を温存（read-modify-write・clobber なし）---

    #[test]
    fn merge_preserves_unrelated_keys() {
        let io = FakePersistIo::new();
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
        save_scope(
            PersistScope::Ghost,
            &roots,
            &io,
            vec![(PersistKey::WindowPos { scope: 0, axis: Axis::X }, "7".into())],
        );
        save_scope(PersistScope::Ghost, &roots, &io, vec![(PersistKey::BootCount, "2".into())]);
        let loaded = load_scope(PersistScope::Ghost, &roots, &io);
        assert!(
            loaded.contains(&(PersistKey::WindowPos { scope: 0, axis: Axis::X }, "7".into())),
            "window.* が boot.count 保存で消えた: {loaded:?}"
        );
        assert!(
            loaded.contains(&(PersistKey::BootCount, "2".into())),
            "boot.count 未反映: {loaded:?}"
        );
    }

    #[test]
    fn overwrite_same_key_updates_value() {
        let io = FakePersistIo::new();
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
        save_scope(PersistScope::Ghost, &roots, &io, vec![(PersistKey::BootCount, "1".into())]);
        save_scope(PersistScope::Ghost, &roots, &io, vec![(PersistKey::BootCount, "5".into())]);
        let loaded = load_scope(PersistScope::Ghost, &roots, &io);
        assert!(loaded.contains(&(PersistKey::BootCount, "5".into())));
        assert_eq!(loaded.iter().filter(|(k, _)| *k == PersistKey::BootCount).count(), 1);
    }

    // --- スコープ分離（R6.5・別 root は非混同）---

    #[test]
    fn scope_isolation_distinct_ghost_roots_do_not_cross() {
        let io = FakePersistIo::new();
        let roots_a = ScopeRoots { ghost: Some(PathBuf::from("/a")), ..ScopeRoots::default() };
        let roots_b = ScopeRoots { ghost: Some(PathBuf::from("/b")), ..ScopeRoots::default() };
        save_scope(PersistScope::Ghost, &roots_a, &io, vec![(PersistKey::BootCount, "9".into())]);
        assert!(
            load_scope(PersistScope::Ghost, &roots_b, &io).is_empty(),
            "root B が root A の値を見た（混同）"
        );
        assert!(
            load_scope(PersistScope::Ghost, &roots_a, &io)
                .contains(&(PersistKey::BootCount, "9".into())),
            "root A 自身は自値を見られる"
        );
    }

    #[test]
    fn scope_isolation_different_scopes_use_different_files() {
        // App と Ghost が別ルート → 別ファイル → 非混同。
        let io = FakePersistIo::new();
        let roots = ScopeRoots {
            app: Some(PathBuf::from("/app")),
            ghost: Some(PathBuf::from("/ghost")),
            ..ScopeRoots::default()
        };
        save_scope(PersistScope::App, &roots, &io, vec![(PersistKey::BootCount, "11".into())]);
        assert!(load_scope(PersistScope::Ghost, &roots, &io).is_empty());
        assert!(
            load_scope(PersistScope::App, &roots, &io).contains(&(PersistKey::BootCount, "11".into()))
        );
    }

    // --- root None → 寛容（R6.7・panic なし）---

    #[test]
    fn none_root_load_is_empty() {
        let io = FakePersistIo::new();
        assert!(load_scope(PersistScope::Ghost, &ScopeRoots::default(), &io).is_empty());
    }

    #[test]
    fn none_root_save_is_degraded_no_panic() {
        let io = FakePersistIo::new();
        assert_eq!(
            save_scope(
                PersistScope::Ghost,
                &ScopeRoots::default(),
                &io,
                vec![(PersistKey::BootCount, "1".into())]
            ),
            PersistOutcome::Degraded
        );
    }

    // --- IO 障害の寛容縮退（read/commit 故障注入）---

    #[test]
    fn load_tolerates_read_failure() {
        let io = FakePersistIo::new();
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
        save_scope(PersistScope::Ghost, &roots, &io, vec![(PersistKey::BootCount, "1".into())]);
        io.fail_next_read();
        assert!(
            load_scope(PersistScope::Ghost, &roots, &io).is_empty(),
            "read 障害は空縮退（panic なし）"
        );
    }

    #[test]
    fn save_commit_failure_is_degraded_and_leaves_prior_intact() {
        let io = FakePersistIo::new();
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
        save_scope(PersistScope::Ghost, &roots, &io, vec![(PersistKey::BootCount, "1".into())]);
        io.fail_next_commit();
        assert_eq!(
            save_scope(PersistScope::Ghost, &roots, &io, vec![(PersistKey::BootCount, "2".into())]),
            PersistOutcome::Degraded
        );
        // 既存内容は無傷（原子的確定・R6.2）——1 のまま。
        assert!(
            load_scope(PersistScope::Ghost, &roots, &io).contains(&(PersistKey::BootCount, "1".into()))
        );
    }

    // --- 非数値スコープ ID の寛容 skip（format 層は不透明文字列を許す）---

    #[test]
    fn non_numeric_scope_id_in_file_is_skipped() {
        let io = FakePersistIo::new();
        let path = PathBuf::from("/g/sylphya.toml");
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
        // 手書き TOML: window の ID が非数値 "main" ＋ 正常な boot。
        io.commit(
            &path,
            "format-version = 1\n[window.\"main\"]\nx = \"5\"\n[boot]\ncount = \"1\"\n",
        )
        .unwrap();
        let loaded = load_scope(PersistScope::Ghost, &roots, &io);
        // 非数値 window は skip、boot は載る（panic なし）。
        assert!(loaded.contains(&(PersistKey::BootCount, "1".into())));
        assert!(loaded.iter().all(|(k, _)| !matches!(k, PersistKey::WindowPos { .. })));
    }

    // --- 決定論（同一入力→同一結果）---

    #[test]
    fn load_is_deterministic() {
        let io = FakePersistIo::new();
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/g")), ..ScopeRoots::default() };
        save_scope(
            PersistScope::Ghost,
            &roots,
            &io,
            vec![
                (PersistKey::WindowPos { scope: 2, axis: Axis::X }, "9".into()),
                (PersistKey::WindowPos { scope: 0, axis: Axis::X }, "1".into()),
                (PersistKey::VanishCount, "4".into()),
            ],
        );
        assert_eq!(
            load_scope(PersistScope::Ghost, &roots, &io),
            load_scope(PersistScope::Ghost, &roots, &io)
        );
    }

    #[test]
    fn absent_file_loads_empty() {
        let io = FakePersistIo::new();
        let roots = ScopeRoots { ghost: Some(PathBuf::from("/never-written")), ..ScopeRoots::default() };
        assert!(load_scope(PersistScope::Ghost, &roots, &io).is_empty());
    }
}
