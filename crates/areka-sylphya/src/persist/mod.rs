//! 永続層の型（`PersistScope`・`ScopeRoots`・`PersistKey`・結果）と、型つきの鍵の包み 3 本。
//!
//! [`load_scope`]・[`save_scope`]・[`remove_scope_keys`] は、番人が居ない時点（起動の前・降ろした
//! 直後）に、型つきの鍵（[`PersistKey`]）で 1 つの層を読む・書く・消す。どれも呼んだスレッドの上で
//! 住み場所の核を開き、型つきの鍵を正規名にして読み書きする（層を初めて使うときの
//! `sylphya.toml` の取り込みも核の中で起きる）。錠に当たった書きは置き場の口の時計で 1 秒まで
//! 取り直す。控える範囲（[`withhold`]）の中の書きは [`PersistOutcome::Withheld`]。
//!
//! [`format`] は取り込みのための `sylphya.toml` の読みだけを持つ。

pub mod format;
pub mod withhold;

pub use format::{FORMAT_VERSION, FormatDoc};

use crate::store::{FileWrite, Store, StoreBackend};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const LOG_TARGET: &str = "areka_sylphya::persist";

/// 層別永続スコープ（伺か慣行の profile フォルダ準拠）。
///
/// 各層の永続情報は対応する層の profile フォルダ（[`ScopeRoots`] が供給するルート）へ保存する。
/// 他の層も器として実装しスコープ分離をテストで検証する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PersistScope {
    /// areka アプリレベル。
    App,
    /// SHIORI（ゴースト）レベル。
    Ghost,
    /// シェルレベル。
    Shell,
    /// バルーンレベル。
    Balloon,
}

/// 各スコープの保存先ルート（呼び出し側が供給。`None` ＝当該スコープ利用不可＝不在縮退）。
///
/// 記憶のファイルは `<root>/sylphya.sqlite`（root 自体を `profile/areka/` に取る運用は結線側の契約）。
/// sylphya はパスを解釈しない（最下層規律）——所属実体（ゴースト等）の分離は per-実体 profile
/// ディレクトリの物理分離が担う。
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
    pub(crate) fn root_of(&self, scope: PersistScope) -> Option<&PathBuf> {
        match scope {
            PersistScope::App => self.app.as_ref(),
            PersistScope::Ghost => self.ghost.as_ref(),
            PersistScope::Shell => self.shell.as_ref(),
            PersistScope::Balloon => self.balloon.as_ref(),
        }
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

/// 4 key 族（typed。値ドメインは文字列）。
///
/// 各 variant は正準 key 文字列（[`PersistKey::to_canonical_key`]）と 1 対 1 に対応する
/// （戻すのは [`PersistKey::from_canonical_key`]）。
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
    /// 前回起動したゴーストのフォルダ名 → 正準 key `areka.last.ghost`・TOML `[last]` ghost。
    /// 載せるスコープは呼び手の契約（本番は App）。
    LastGhost,
    /// 前回使ったバルーンのフォルダ名 → 正準 key `areka.last.balloon`・TOML `[last]` balloon。
    /// 載せるスコープは呼び手の契約（本番は Ghost＝ゴーストごとの選択）。
    LastBalloon,
    /// 前回使ったシェルのフォルダ名 → 正準 key `areka.last.shell`・TOML `[last]` shell。
    /// 載せるスコープは呼び手の契約（本番は Ghost）。
    LastShell,
    /// 起動中の印（動いているゴーストの名前）→ 正準 key `areka.last.running`・TOML `[last]` running。
    /// 空文字は「無し」と読む（空文字を書けば消える）。載せるスコープは呼び手の契約（本番は App）。
    LastRunning,
    /// 根の並び → 正準 key `areka.baseware.roots`・TOML `[baseware]` roots。値は 1 本の文字列
    /// （中身の意味は呼び手が持ち、ここは運ぶだけ）。空文字は空文字のまま（「無し」に潰さない）。
    /// 書けば並び全体を 1 回で置き換える。載せるスコープは呼び手の契約（本番は App）。
    BasewareRoots,
    /// 隣り合いの印（キャラクタースコープ別）→ 正準 key `areka.window.scope(ID).adjacent`・
    /// TOML `[window-adjacent]` の `"ID" = "left" | "right" | "none"`。値の意味は呼び手（areka）が持つ。
    WindowAdjacent {
        /// キャラクタースコープ ID。
        scope: u32,
    },
}

impl PersistKey {
    /// 記憶の鍵の正規名。
    ///
    /// **不変条件**: `parse_dotted(k).unwrap().to_canonical_string() == k`（[`crate::key`] と往復
    /// 整合——この名前で書いた値は名前での読みが同じ正規名で引ける）。
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
            PersistKey::LastGhost => "areka.last.ghost".to_string(),
            PersistKey::LastBalloon => "areka.last.balloon".to_string(),
            PersistKey::LastShell => "areka.last.shell".to_string(),
            PersistKey::LastRunning => "areka.last.running".to_string(),
            PersistKey::BasewareRoots => "areka.baseware.roots".to_string(),
            PersistKey::WindowAdjacent { scope } => format!("areka.window.scope({scope}).adjacent"),
        }
    }

    /// 正規名から型つきの鍵へ戻す（正規名でない綴り・型つきの鍵の無い名前は `None`）。
    pub fn from_canonical_key(key: &str) -> Option<PersistKey> {
        let scoped = |prefix: &str| -> Option<(u32, &str)> {
            let (id, field) = key.strip_prefix(prefix)?.split_once(").")?;
            Some((id.parse().ok()?, field))
        };
        let axis = |field: &str| match field {
            "x" => Some(Axis::X),
            "y" => Some(Axis::Y),
            _ => None,
        };
        let found = if let Some((scope, field)) = scoped("areka.window.scope(") {
            match field {
                "adjacent" => Some(PersistKey::WindowAdjacent { scope }),
                _ => axis(field).map(|axis| PersistKey::WindowPos { scope, axis }),
            }
        } else if let Some((scope, field)) = scoped("areka.balloon.offset.scope(") {
            axis(field).map(|axis| PersistKey::BalloonOffset { scope, axis })
        } else {
            [
                PersistKey::BootCount,
                PersistKey::VanishCount,
                PersistKey::LastGhost,
                PersistKey::LastBalloon,
                PersistKey::LastShell,
                PersistKey::LastRunning,
                PersistKey::BasewareRoots,
            ]
            .into_iter()
            .find(|k| k.to_canonical_key() == key)
        };
        // `01` や `+1` も数には読めるので、正規名に戻るかで綴りを確かめる。
        found.filter(|k| k.to_canonical_key() == key)
    }
}

/// 保存できなかった 1 回の中身。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistFailure {
    /// 書けなかったスコープ。
    pub scope: PersistScope,
    /// 書けなかった置き場（スコープの root＝`…\profile\areka`）。root が無ければ `None`。
    pub dir: Option<PathBuf>,
    /// 覚えられなかった key（投函の entries の key の並び。起動の前の確かめは空）。
    pub keys: Vec<PersistKey>,
    /// 書けなかった理由。
    pub cause: PersistFailureCause,
}

/// [`PersistFailure`] の理由。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistFailureCause {
    /// スコープの置き場が決まっていない（root 不在）。
    NoRoot,
    /// OS の理由（`std::io::Error` の表示。例「アクセスが拒否されました。 (os error 5)」）。
    Io(String),
    /// 既にある記憶のファイル（または取り込む前の `sylphya.toml`）が使えないので書かなかった。
    Unusable(String),
}

/// 永続保存の結果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistOutcome {
    /// 原子的確定に成功（以後のロードで同値復元）。
    Saved,
    /// 保存失敗（root 不在・書けない・使えない記憶のファイル）→ ログ済み・縮退。
    /// 書けなかった 1 回の中身を運ぶ。
    Degraded(PersistFailure),
    /// 読み取りだけの走りで書かずに控えた（中身は走る間だけ読み返せる）。
    Withheld,
}

/// 当該スコープの記憶の全部を型つきの鍵で読む（起動の前）。
///
/// root 不在・核を開けない → 空（縮退）。型つきの鍵に戻らない名前は返さない。空文字の起動中の印は
/// 「無し」と読む。返り値は正規名の順。
pub fn load_scope(
    scope: PersistScope,
    roots: &ScopeRoots,
    backend: &Arc<dyn StoreBackend>,
) -> Vec<(PersistKey, String)> {
    if roots.root_of(scope).is_none() {
        tracing::debug!(
            target: LOG_TARGET,
            ?scope,
            "persist scope root absent; loading as empty (tolerant)"
        );
        return Vec::new();
    }
    let Ok(mut store) = Store::open(roots.clone(), backend.clone()) else {
        return Vec::new();
    };
    store
        .read_layer(scope)
        .into_iter()
        .filter_map(|(name, value)| Some((PersistKey::from_canonical_key(&name)?, value)))
        .filter(|(key, value)| !(*key == PersistKey::LastRunning && value.is_empty()))
        .collect()
}

/// 当該スコープへ entries を 1 回のトランザクションで書く（ほかの鍵はそのまま）。
///
/// root 不在 → warn! ＋ `Degraded(NoRoot)`。書けない・使えない記憶のファイル・1 秒待っても錠が
/// 取れない → error! ＋ [`PersistOutcome::Degraded`]。控える範囲の中 → [`PersistOutcome::Withheld`]。
pub fn save_scope(
    scope: PersistScope,
    roots: &ScopeRoots,
    backend: &Arc<dyn StoreBackend>,
    entries: Vec<(PersistKey, String)>,
) -> PersistOutcome {
    let typed = entries.iter().map(|(key, _)| *key).collect();
    let puts = entries
        .into_iter()
        .map(|(key, value)| (key.to_canonical_key(), value))
        .collect();
    write_scope(scope, roots, backend, puts, Vec::new(), typed)
}

/// 当該スコープの記憶から `keys` を消す（[`save_scope`] と同じ道・無い鍵は何も変えない）。
pub fn remove_scope_keys(
    scope: PersistScope,
    roots: &ScopeRoots,
    backend: &Arc<dyn StoreBackend>,
    keys: Vec<PersistKey>,
) -> PersistOutcome {
    let removes = keys.iter().map(|key| key.to_canonical_key()).collect();
    write_scope(scope, roots, backend, Vec::new(), removes, keys)
}

fn write_scope(
    scope: PersistScope,
    roots: &ScopeRoots,
    backend: &Arc<dyn StoreBackend>,
    puts: Vec<(String, String)>,
    removes: Vec<String>,
    typed: Vec<PersistKey>,
) -> PersistOutcome {
    let write = FileWrite {
        layer: scope,
        root: roots.root_of(scope).cloned(),
        puts,
        removes,
        typed,
    };
    match Store::open(roots.clone(), backend.clone()) {
        Ok(mut store) => crate::kvs::write_within_wait(&mut store, backend.as_ref(), &write),
        Err(reason) => crate::guardian::store_not_open(&write, &reason),
    }
}

/// ゴーストの記憶のフォルダを先に作る（控える範囲の中なら作らずに `Ok`）。
pub fn ensure_root_dir(dir: &Path, backend: &Arc<dyn StoreBackend>) -> std::io::Result<()> {
    if backend.withholds(dir) {
        tracing::debug!(
            target: LOG_TARGET,
            event = "persist_root_dir_withheld",
            dir = %dir.display(),
            "persist root dir not created (read-only run)"
        );
        return Ok(());
    }
    backend.ensure_dir(dir)
}

#[cfg(test)]
#[path = "persist_tests.rs"]
mod tests;
