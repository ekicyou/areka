//! テストが「空の根」として借りる使い捨ての作業フォルダ（spec: `areka-P0-nar-install`
//! 要件 7.1・7.10）。
//!
//! # なぜ OS の一時フォルダを使わないか
//!
//! 一時パスの窓口（`temp-path-kit`）は OS の一時フォルダの下に置き場を取るが、本仕様の
//! 根は `cargo clean` で消えること・ビルド成果物と取り違えが起きないことの両方を要求
//! されている（要件 7.1）。そこでビルド成果物の置き場の下に**本仕様専用の名前空間**を
//! 掘り、そこだけを使う（要件 7.10 の「OS の一時フォルダを使わない」）。
//!
//! # 置き場の見つけ方
//!
//! `CARGO_TARGET_DIR` が設定されていればそれ、無ければ**実行ファイルの祖先で名前が
//! `target` の最初のフォルダ**。どちらでも決まらなければ、探索の起点を付けて
//! [`SampleError::TargetDirNotFound`] を返す（黙って別の場所へ落ちない）。
//!
//! # 配るもの
//!
//! ```text
//! <置き場>/nar-samples/
//! ├── cache/<検体>-<長さ>-<crc32 8 桁>/  # 読み専用の原本（ベースウェアの根の形）
//! └── work/
//!     ├── <プロセス識別子>-<連番>/       # 配る空の根・展開中の原本
//!     ├── <プロセス識別子>-<連番>.lock   # 生存の札（開いたまま持つ）
//!     └── gc-<プロセス識別子>-<連番>/    # `cache/` から退き上げた古い原本
//! ```
//!
//! # 原本の不変条件——「名前が合う原本は完全」
//!
//! 原本の名前には `.nar` の**丸ごとの長さと CRC-32**（刻印）が入る。原本は必ず作業
//! フォルダで組んでから `rename` で `cache/` へ入れ、消すときは先に `work/gc-…/` へ
//! `rename` してから消す。だから `cache/<検体>-<刻印>/` が見えている間は常に完全で、
//! 半端な木が「使える根」として見えることがない（要件 7.7）。
//!
//! 刻印が変われば名前が変わるので、原本を作り直す判定は名前の有無だけで済む
//! （要件 7.2・7.3）。名前空間ごと消えても次の取得で作り直す（要件 7.8）。
//!
//! 札を**先に**作ってからフォルダを作る。札は削除を共有しない形で開いたままなので、
//! 生きている作業フォルダは他のプロセスの掃除から守られる（後続の掃除は、札を消せたか
//! どうかで持ち主の生死を見分ける）。
//!
//! # 掃除（後続タスク）が較正する 3 つの順序
//!
//! この 3 つは**今の検査では赤にできない**。どれも「生きている作業フォルダが他プロセスの
//! 掃除に消されない」ことを守る仕掛けで、掃除そのものが無い間は観測できないからである。
//! 掃除を建てるタスクは、掃除を書いた後にこの 3 つを壊して赤が出ることを確かめること。
//!
//! 1. 札を作るのがフォルダを作るより**先**であること（順序を入れ替えても今は全 35 本が緑）。
//! 2. 原本の組み上げ中も札が開いたままであること（組み上げの間に札を消しても今は全数が緑）。
//! 3. 札が `FILE_SHARE_READ` だけで開かれていること（これだけは
//!    `the_lease_refuses_deletion_while_it_is_held` が較正済み）。
//!
//! # 使い方
//!
//! 値を束縛している間だけ根が存在する。`let _ = WorkDir::new()` はその場で破棄される。
//!
//! 下の例は**組むところまで**を走らせる（`no_run`）。doctest の実行ファイルだけは
//! ビルド成果物の置き場の外（OS の一時フォルダ）に置かれるので、この窓口は設計どおり
//! [`SampleError::TargetDirNotFound`] を返す。振る舞いの判定は兄弟テストが持ち、この例は
//! **窓口が crate の外から届くこと**（doctest は別 crate として組まれる）を固定する。
//!
//! ```rust,no_run
//! use sample_ghost_kit::WorkDir;
//!
//! let work = WorkDir::new().expect("ビルド成果物の置き場は見つかるはず");
//! assert!(work.path().is_dir());
//! assert!(work.path().is_absolute());
//!
//! let kept = work.path().to_path_buf();
//! drop(work);
//! assert!(!kept.exists());
//! ```

use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use areka_nar::{InstallRequest, NarArchive};

use crate::SampleError;

/// ビルド成果物の置き場の下に掘る、本仕様専用の名前空間（要件 7.1）。
const NAMESPACE: &str = "nar-samples";

/// 使い捨ての根を並べる棚。
const WORK: &str = "work";

/// 読み専用の原本を並べる棚。名前は `<検体>-<刻印>`。
const CACHE: &str = "cache";

/// 読むことだけを共有する（＝削除は拒む）共有モード `FILE_SHARE_READ`。
const FILE_SHARE_READ: u32 = 1;

/// 同一プロセス内で単調増加する連番。プロセス間の一意性はプロセス識別子が担う。
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(0);

/// ビルド成果物の置き場を決める唯一の式。
///
/// 環境変数の値と実行ファイルの場所を**引数で受ける**ので、環境を書き換えずに
/// （＝並走するテストに影響を与えずに）両方の経路と失敗を判定できる。
fn find_target_dir(configured: Option<&str>, exe: &Path) -> Result<PathBuf, SampleError> {
    if let Some(dir) = configured.map(str::trim).filter(|dir| !dir.is_empty()) {
        return std::path::absolute(dir).map_err(|source| SampleError::Io {
            what: "ビルド成果物の置き場の絶対化",
            path: PathBuf::from(dir),
            source,
        });
    }
    exe.ancestors()
        .find(|ancestor| ancestor.file_name() == Some(OsStr::new("target")))
        .map(Path::to_path_buf)
        .ok_or_else(|| SampleError::TargetDirNotFound {
            started_from: exe.to_path_buf(),
        })
}

/// 本仕様専用の名前空間の絶対パス。
fn namespace_dir() -> Result<PathBuf, SampleError> {
    let exe = std::env::current_exe().map_err(|source| SampleError::Io {
        what: "実行ファイルの場所の取得",
        path: PathBuf::new(),
        source,
    })?;
    let configured = std::env::var("CARGO_TARGET_DIR").ok();
    Ok(find_target_dir(configured.as_deref(), &exe)?.join(NAMESPACE))
}

/// 破棄で木ごと消える、札付きの使い捨ての作業フォルダ。
///
/// `areka-nar` のテストはこれを「空の根」として借りる（OS の一時フォルダを使わずに
/// 済ませるための唯一の窓）。
#[derive(Debug)]
pub struct WorkDir {
    path: PathBuf,
    lock: PathBuf,
    /// 生存の札。[`Drop`] で**先に**閉じる（開いたままでは自分でも消せない）。
    lease: Option<File>,
}

impl WorkDir {
    /// 空の作業フォルダを 1 つ取る。
    ///
    /// 札を `create_new` で作ってからフォルダを作るので、フォルダが見えている間は必ず
    /// 持ち主の札が開いている。
    ///
    /// # Errors
    ///
    /// ビルド成果物の置き場が決まらないとき [`SampleError::TargetDirNotFound`]、
    /// 札やフォルダを作れないとき [`SampleError::Io`]。
    pub fn new() -> Result<WorkDir, SampleError> {
        WorkDir::in_namespace(&namespace_dir()?)
    }

    /// 名前空間を指定して 1 つ取る。
    ///
    /// 原本の組み上げは `rename` で `cache/` へ入れるので、作業フォルダは**原本と同じ
    /// 名前空間**（＝同じボリューム）に居なければならない。テストも私有の名前空間を
    /// 渡して、走っている他のテストと混ざらずに全体を作り直せる。
    fn in_namespace(namespace: &Path) -> Result<WorkDir, SampleError> {
        let work = namespace.join(WORK);
        std::fs::create_dir_all(&work).map_err(|source| SampleError::Io {
            what: "作業フォルダの棚の作成",
            path: work.clone(),
            source,
        })?;

        let stem = format!(
            "{}-{}",
            std::process::id(),
            NEXT_SERIAL.fetch_add(1, Ordering::Relaxed)
        );
        let lock = work.join(format!("{stem}.lock"));
        let lease = OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(FILE_SHARE_READ)
            .open(&lock)
            .map_err(|source| SampleError::Io {
                what: "生存の札の作成",
                path: lock.clone(),
                source,
            })?;

        let path = work.join(&stem);
        if let Err(source) = std::fs::create_dir(&path) {
            // 札だけが残らないよう、作れなかったときはその場で畳む。
            drop(lease);
            report_cleanup("生存の札の後始末", std::fs::remove_file(&lock), &lock);
            return Err(SampleError::Io {
                what: "作業フォルダの作成",
                path,
                source,
            });
        }

        Ok(WorkDir {
            path,
            lock,
            lease: Some(lease),
        })
    }

    /// 配られた作業フォルダそのもの（絶対パス）。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 生存の札のパス。掃除の担当（後続タスク）と、札が守られていることを確かめる
    /// テストが読む。
    ///
    /// 借り手が使うのは [`WorkDir::path`] だけなので `pub(crate)` に絞りたいところだが、
    /// 今日の呼び手はテストだけなので絞ると本体のビルドで「使われていない」の警告が出る。
    /// 掃除（後続タスク）が最初の非テストの呼び手になるので、絞るのはそのときでよい。
    pub fn lock_path(&self) -> &Path {
        &self.lock
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        // 札を開いたままでは削除できない（削除を共有していないため）。先に閉じる。
        drop(self.lease.take());
        // 原本として `cache/` へ移した後は木が既に無い（`NotFound`）。それは後始末の
        // 目的が達成された状態なので、札だけを閉じて消す。
        report_cleanup(
            "作業フォルダの後始末",
            std::fs::remove_dir_all(&self.path),
            &self.path,
        );
        report_cleanup(
            "生存の札の後始末",
            std::fs::remove_file(&self.lock),
            &self.lock,
        );
    }
}

/// 検体の `.nar` の置き場。本 crate だけがこの綴りを持つ。
fn nar_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendors/sample_ghost"
    ))
}

/// `.nar` 1 本の刻印＝**丸ごとの長さ**と **CRC-32**（要件 7.3）。
///
/// 長さだけでは同じ大きさの差し替えを見逃し、CRC だけでは長さの違いを 1/2^32 で
/// 取り違える。両方を名前に入れるので、原本を作り直す判定は名前の有無だけで済む。
fn stamp(bytes: &[u8]) -> String {
    format!("{}-{:08x}", bytes.len(), areka_nar::crc32(bytes))
}

/// `cache/` の 1 件の名前を `(検体名, 刻印)` に割る。割れなければ `None`。
///
/// 検体名自身が `-` を含む（`emo2-kakukaku-wplimit`）ので、**前から**数えると
/// `emo2` の原本を探しているつもりで `emo2-kakukaku-wplimit` の原本に当たってしまう。
/// 刻印は必ず末尾の `-<10 進の長さ>-<16 進 8 桁>` なので、後ろから 2 つだけ剥がして
/// 残り全部を名前とする。
fn split_stamp(entry: &str) -> Option<(&str, &str)> {
    let (head, crc) = entry.rsplit_once('-')?;
    let (name, len) = head.rsplit_once('-')?;
    let shaped = crc.len() == 8
        && crc.bytes().all(|b| b.is_ascii_hexdigit())
        && !len.is_empty()
        && len.bytes().all(|b| b.is_ascii_digit())
        && !name.is_empty();
    shaped.then(|| (name, &entry[name.len() + 1..]))
}

/// 検体 1 つの原本（`cache/<検体>-<刻印>/`）を用意して、その絶対パスを返す
/// （要件 7.2・7.3・7.6・7.7・7.8）。
///
/// 原本は**誰にも配らない**。配るのはここから取る複製で、複製を作るのは後続タスクの
/// 仕事（この入口がその唯一の呼び手になる）。
///
/// `pub` なのは、今日の呼び手がテストだけで `pub(crate)` にすると本体のビルドで
/// 「使われていない」の警告が出るからである（[`WorkDir::lock_path`] と同じ事情）。
/// 窓口 `SampleRoot::acquire` がこれを内側から呼ぶタスクで `pub(crate)` へ絞ること——
/// 設計の窓口の一覧にこの関数は無く、公開したままでは「検体を得る唯一の入口は
/// `SampleRoot`」（要件 1.1）が嘘になる。
///
/// # Errors
///
/// `.nar` が読めない・原本を据え付けられないとき [`SampleError::Io`]、`.nar` が
/// 受理されないとき [`SampleError::Nar`]。古い原本の回収に失敗しても**失敗にはしない**
/// （標準エラーへ出して取得を続ける＝要件 7.3）。
pub fn cached_root(name: &str) -> Result<PathBuf, SampleError> {
    cached_root_in(
        &namespace_dir()?,
        name,
        &nar_dir().join(format!("{name}.nar")),
    )
}

/// 名前空間と `.nar` を明示して原本を用意する。
///
/// テストは私有の名前空間と自前で組んだ `.nar` を渡す（並走する他のテストを巻き込まずに
/// 「原本が無い」「刻印が変わった」「名前空間ごと消えた」を作れる）。
fn cached_root_in(namespace: &Path, name: &str, nar: &Path) -> Result<PathBuf, SampleError> {
    let bytes = std::fs::read(nar).map_err(|source| SampleError::Io {
        what: "検体の .nar の読み取り",
        path: nar.to_path_buf(),
        source,
    })?;
    let cache = namespace.join(CACHE);
    let stamp = stamp(&bytes);
    let master = cache.join(format!("{name}-{stamp}"));

    // 名前が合う原本は完全（不変条件）。だから在るかどうかだけを見る。
    if !master.is_dir() {
        stage_into_cache(namespace, &cache, nar, &master)?;
    }
    reclaim_stale(namespace, &cache, name, &stamp);
    Ok(master)
}

/// 空の根へ展開してから、`rename` 1 回で `cache/` へ据え付ける。
///
/// 展開の間ずっと作業フォルダの札が開いているので、並走する別プロセスの掃除には
/// 消されない。`rename` が成功した後の作業フォルダの [`Drop`] は木が無いことを許容する。
fn stage_into_cache(
    namespace: &Path,
    cache: &Path,
    nar: &Path,
    master: &Path,
) -> Result<(), SampleError> {
    std::fs::create_dir_all(cache).map_err(|source| SampleError::Io {
        what: "原本の棚の作成",
        path: cache.to_path_buf(),
        source,
    })?;
    let work = WorkDir::in_namespace(namespace)?;
    let archive = NarArchive::open(nar)?;
    archive.install(&InstallRequest {
        root: work.path(),
        target_ghost: None,
    })?;
    match std::fs::rename(work.path(), master) {
        Ok(()) => Ok(()),
        // 別のプロセスが先に据え付けた（要件 7.6）。原本は常に空でない木で、Windows の
        // `rename` は空でないフォルダを宛先にすると必ず失敗するので、この形で勝敗が
        // 決まる（兄弟テストが規則そのものを実測している）。負けた側は自分の作業を
        // 捨てて（`Drop` が消す）勝者の木を使う。
        Err(_) if master.is_dir() => Ok(()),
        Err(source) => Err(SampleError::Io {
            what: "原本の据え付け",
            path: master.to_path_buf(),
            source,
        }),
    }
}

/// 同じ検体の刻印違いの原本を回収する（要件 7.3・7.9）。
///
/// 先に `work/gc-…/` へ `rename` してから消すので、途中で落ちても `cache/` の下に
/// 半端な木は残らない（残るのは `work/` の下の消し残しで、掃除の担当が拾う）。
///
/// 失敗しても**取得は続ける**。別のプロセスが複写中・ウイルス対策が掴んでいる、は
/// どれも次の取得で消せるようになる一時的な事情で、これで検体が得られないのは損だから。
///
/// 実測: Windows は**子孫のファイルが開かれているフォルダの `rename` を拒む**
/// （共有モードに書き込みと削除を足しても同じ）。掴まれている古い原本は最初の一歩で
/// 失敗するので、`cache/` に無傷のまま残り、次の取得で改めて回収される（兄弟テスト
/// `a_reclaim_that_cannot_move_the_stale_copy_keeps_going_and_retries_on_the_next_acquire`）。
fn reclaim_stale(namespace: &Path, cache: &Path, name: &str, keep: &str) {
    let entries = match std::fs::read_dir(cache) {
        Ok(entries) => entries,
        Err(err) => return report_cleanup("古い原本の走査", Err(err), cache),
    };
    let shelf = namespace.join(WORK);
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report_cleanup("古い原本の走査", Err(err), cache);
                continue;
            }
        };
        let raw = entry.file_name();
        // 名前を読めない・刻印の形をしていないものは、この仕様が置いた原本ではない。
        // 黙って消さずに残す（消すかどうかを決めるのは掃除の担当）。
        let Some((sample, stamp)) = raw.to_str().and_then(split_stamp) else {
            continue;
        };
        if sample != name || stamp == keep {
            continue;
        }
        let stale = entry.path();
        if let Err(err) = std::fs::create_dir_all(&shelf) {
            report_cleanup("回収の棚の作成", Err(err), &shelf);
            continue;
        }
        let gc = shelf.join(format!(
            "gc-{}-{}",
            std::process::id(),
            NEXT_SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        // `cache/` の外へ出すのが先。「名前が合う原本は完全」を一瞬も破らない。
        if let Err(err) = std::fs::rename(&stale, &gc) {
            report_cleanup("古い原本の退避", Err(err), &stale);
            continue;
        }
        report_cleanup("退避した原本の削除", std::fs::remove_dir_all(&gc), &gc);
    }
}

/// 後始末の失敗を必ず人の読める形で残す。
///
/// [`Drop`] と古い原本の回収は失敗を返せない（回収の失敗で取得を止めない＝要件 7.3）ので、
/// 呼び手へ伝える道がここしかない。本 crate はテスト専用で記録層に依存しない
/// （依存を足さない）ため、宛先は標準エラー出力にする。既に無いのは後始末の目的が
/// 達成された状態なので黙って通す。残った木と札は次の走行が回収する。
fn report_cleanup(what: &str, result: std::io::Result<()>, path: &Path) {
    if let Err(err) = result {
        if err.kind() == std::io::ErrorKind::NotFound {
            return;
        }
        eprintln!(
            "sample-ghost-kit: {what}に失敗した（次の走行で回収する）: {} ({err})",
            path.display()
        );
    }
}

#[cfg(test)]
#[path = "devroot_tests.rs"]
mod tests;

/// 原本（`cache/`）の兄弟テスト。1 ファイル 1,000 行の上限に収めるために分けただけで、
/// 見ている物は同じ [`WorkDir`] と同じ名前空間。
#[cfg(test)]
#[path = "devroot_cache_tests.rs"]
mod cache_tests;
