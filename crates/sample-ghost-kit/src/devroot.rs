//! テストが「空の根」として借りる使い捨ての作業フォルダ（spec: `areka-P0-nar-install`
//! 要件 7.1・7.10）。
//!
//! # なぜ OS の一時フォルダを使わないか
//!
//! 一時パスの窓口（`temp-path-kit`）もビルド成果物の置き場の下（`target\test-roots\`）に
//! 置き場を取るが、本仕様の根はそれとは別の**本仕様専用の名前空間**（`nar-samples`）に
//! 置く。根は `cargo clean` で消えること・ビルド成果物と取り違えが起きないことの両方を
//! 要求されている（要件 7.1）ので、ビルド成果物の置き場の下にその名前空間を掘り、そこ
//! だけを使う（要件 7.10 の「OS の一時フォルダを使わない」）。
//!
//! # 置き場の見つけ方
//!
//! `CARGO_TARGET_DIR` が設定されていればそれ（相対の値は、実行ファイルの祖先のうちその値で
//! 終わる最も近いもの・無ければ作業フォルダの上で絶対にしたもの）、無ければ**実行ファイルの
//! 祖先で名前が `target` の最初のフォルダ**。どちらでも決まらなければ、探索の起点を付けて
//! [`SampleError::TargetDirNotFound`] を返す（黙って別の場所へ落ちない）。
//!
//! # 配るもの
//!
//! ```text
//! <置き場>/nar-samples/
//! ├── cache-2/<検体>-<長さ>-<crc32 8 桁>/  # 読み専用の原本（ベースウェアの根の形）
//! └── work/
//!     ├── <プロセス識別子>-<連番>/       # 配る空の根・展開中の原本
//!     ├── <プロセス識別子>-<連番>.lock   # 生存の札（開いたまま持つ）
//!     └── gc-<プロセス識別子>-<連番>/    # `cache-2/` から退き上げた古い原本
//! ```
//!
//! 棚の名前の数を上げる前の世代の棚（`cache/` など）は、原本を作るときに退ける。
//!
//! # 原本の不変条件——「名前が合う原本は完全」
//!
//! 原本の名前には `.nar` の**丸ごとの長さと CRC-32**（刻印）が入る。原本は必ず作業
//! フォルダで組んでから `rename` で `cache-2/` へ入れ、消すときは先に `work/gc-…/` へ
//! `rename` してから消す。だから `cache-2/<検体>-<刻印>/` が見えている間は常に完全で、
//! 半端な木が「使える根」として見えることがない（要件 7.7）。
//!
//! 刻印が変われば名前が変わるので、原本を作り直す判定は名前の有無だけで済む
//! （要件 7.2・7.3）。名前空間ごと消えても次の取得で作り直す（要件 7.8）。
//!
//! 札を**先に**作ってからフォルダを作る。札は削除を共有しない形で開いたままなので、
//! 生きている作業フォルダは他のプロセスの掃除から守られる（後続の掃除は、札を消せたか
//! どうかで持ち主の生死を見分ける）。
//!
//! # 掃除——札が生死を分ける
//!
//! 取得のたびに作業の棚を 1 度走査し、札を消してみる。**消せた札は持ち主が居ない**ので
//! 相方の木ごと片付ける。消せない札は触らない——共有違反は生きている利用者の物であり、
//! 既に無いのは別のプロセスが同時に回収している最中で、どちらも「持ち主が居ない」とは
//! 判定しない。札の無い木（札を作る前に落ちた残骸・回収の途中で落ちた `gc-` の木）は
//! 片付ける（要件 7.7・7.9）。
//!
//! だから札を**フォルダより先に**作り、原本の組み上げの**間ずっと**開いたままにする。
//! どちらを崩しても、札の無い木が一瞬だけ棚に見える窓が開き、並走する掃除に退けられる
//! （兄弟テスト `a_sweeper_running_alongside_never_disturbs_a_staging_tree_or_a_live_copy`
//! がその窓を檻にしている）。札が `FILE_SHARE_READ` だけで開かれていることは
//! `the_lease_refuses_deletion_while_it_is_held` が較正する。
//!
//! # 配るのは複製
//!
//! 原本は誰にも配らない。取得のたびに札を先に開いてから原本の木をそっくり複写し、その
//! 複製を配る。`.nar` を展開し直さない（要件 6.5）ので、複製は「原本の複写」であって
//! 再インストールの経路を通らない。複製は札を開いている間だけ生き、破棄で木と札の両方が
//! 消える（要件 7.4・7.5・7.9）。
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
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use areka_nar::{InstallRequest, NarArchive, SingleRoot};

use crate::{Sample, SampleError, SampleSource};

/// ビルド成果物の置き場の下に掘る、本仕様専用の名前空間（要件 7.1）。
const NAMESPACE: &str = "nar-samples";

/// 使い捨ての根を並べる棚。
const WORK: &str = "work";

/// 読み専用の原本を並べる棚。名前は `<検体>-<刻印>`。
///
/// 棚の名前に展開器の置き方の世代を持つ。刻印は `.nar` だけから決まるので、置き方が
/// 変わると同じ刻印の古い原本が残ってしまう（spec: `areka-P0-ghost-inner-balloon` で
/// 同梱バルーンが根の `balloon/` から本体の中へ移った）。置き方を変えたら数を上げる。
pub(crate) const CACHE: &str = "cache-2";

/// 手で使う根を並べる棚。名前は検体名そのもの（刻印は入れない——人が読んで打つ）。
///
/// [`sweep`] が走査するのは [`WORK`] の棚だけなので、この棚は掃除の対象外である
/// （要件 1.9。持ち主の札が無い木なので、掃除が届けば印字した直後に消えてしまう）。
const MANUAL: &str = "manual";

/// 読むことだけを共有する（＝削除は拒む）共有モード `FILE_SHARE_READ`。
const FILE_SHARE_READ: u32 = 1;

/// 生存の札の綴り。木の名前にこれを付けた隣が、その木の札になる。
const LOCK_SUFFIX: &str = ".lock";

/// 同一プロセス内で単調増加する連番。プロセス間の一意性はプロセス識別子が担う。
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(0);

/// ビルド成果物の置き場を決める唯一の式。
///
/// 環境変数の値と実行ファイルの場所を**引数で受ける**ので、環境を書き換えずに
/// （＝並走するテストに影響を与えずに）両方の経路と失敗を判定できる。
///
/// 相対の値は、実行ファイルの祖先のうちその値で終わる最も近いものを使う（audit-test-kits 決め D4）。
/// cargo は起こした場所の上で解くが、テストの実行体はパッケージの場所で走るので、作業
/// フォルダの上で解くと別の場所を指す。どちらで起こしても実行体は `<target>\debug\…` の
/// 下にあるので祖先に現れる。照らすのは頭の `.`・`..` を外した名前の段だけ（`..\tgt` の
/// `..` は起こした場所の上の印で、祖先の名前には現れない）。名前の段が無いか、祖先に
/// 無ければ作業フォルダの上で絶対にする。
fn find_target_dir(configured: Option<&str>, exe: &Path) -> Result<PathBuf, SampleError> {
    if let Some(dir) = configured.map(str::trim).filter(|dir| !dir.is_empty()) {
        let named: PathBuf = Path::new(dir)
            .components()
            .skip_while(|part| matches!(part, Component::CurDir | Component::ParentDir))
            .collect();
        if Path::new(dir).is_relative()
            && !named.as_os_str().is_empty()
            && let Some(found) = exe.ancestors().find(|ancestor| ancestor.ends_with(&named))
        {
            return Ok(found.to_path_buf());
        }
        return std::path::absolute(dir).map_err(|source| SampleError::Io {
            what: "make the build output directory absolute",
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
pub(crate) fn namespace_dir() -> Result<PathBuf, SampleError> {
    let exe = std::env::current_exe().map_err(|source| SampleError::Io {
        what: "find the running executable",
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
    /// 生存の札。[`Drop`] で木を消し終えてから閉じて消す（開いたままでは自分でも消せない）。
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
    /// 原本の組み上げは `rename` で [`CACHE`] へ入れるので、作業フォルダは**原本と同じ
    /// 名前空間**（＝同じボリューム）に居なければならない。テストも私有の名前空間を
    /// 渡して、走っている他のテストと混ざらずに全体を作り直せる。
    fn in_namespace(namespace: &Path) -> Result<WorkDir, SampleError> {
        WorkDir::in_namespace_with(namespace, || NEXT_SERIAL.fetch_add(1, Ordering::Relaxed))
    }

    /// [`WorkDir::in_namespace`] の本体。連番の出どころを引数で受けるので、同じ名前の残りが
    /// ある場合を決定論で試せる。
    ///
    /// 死んだプロセスの札やフォルダが同じ名前で残っていたら（プロセス識別子の使い回し）、
    /// 次の連番で取り直す。残りの札は他人の物かもしれないので消さない（掃除が拾う）。
    fn in_namespace_with(
        namespace: &Path,
        mut next_serial: impl FnMut() -> u32,
    ) -> Result<WorkDir, SampleError> {
        let work = namespace.join(WORK);
        std::fs::create_dir_all(&work).map_err(|source| SampleError::Io {
            what: "create the work shelf",
            path: work.clone(),
            source,
        })?;

        loop {
            let stem = format!("{}-{}", std::process::id(), next_serial());
            let lock = work.join(format!("{stem}{LOCK_SUFFIX}"));
            let lease = match open_lease(&lock) {
                Ok(lease) => lease,
                Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => {
                    return Err(SampleError::Io {
                        what: "create the lease file",
                        path: lock,
                        source,
                    });
                }
            };

            let path = work.join(&stem);
            match std::fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(WorkDir {
                        path,
                        lock,
                        lease: Some(lease),
                    });
                }
                Err(source) => {
                    // 札だけが残らないよう、作れなかったときはその場で畳む（自分の札）。
                    retire_lease(Some(lease), &lock);
                    if source.kind() != std::io::ErrorKind::AlreadyExists {
                        return Err(SampleError::Io {
                            what: "create the work folder",
                            path,
                            source,
                        });
                    }
                }
            }
        }
    }

    /// 配られた作業フォルダそのもの（絶対パス）。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 生存の札のパス。札が守られていることを確かめるテストだけが読む。
    #[cfg(test)]
    pub(crate) fn lock_path(&self) -> &Path {
        &self.lock
    }
}

impl WorkDir {
    /// 破棄の手順。木を消す関数を受け取るのは、消している最中の棚を兄弟テストが
    /// 覗くため（本番は [`Drop`] が `remove_dir_all` を渡す）。
    ///
    /// 順は「木を消す → 札を閉じる → 札を消す」。札を先に閉じると、消している間の木が
    /// 持ち主の居ない組に見え、並走する掃除が札を消して消しかけの木を退けようとする
    /// （spec: `areka-P0-ghost-session-test-load-flake` 要件 5.2・檻
    /// `a_work_dir_keeps_its_lease_while_its_tree_is_being_removed`）。
    fn release_with(&mut self, remove_tree: impl FnOnce(&Path) -> std::io::Result<()>) {
        // 原本として `CACHE` へ移した後は木が既に無い（`NotFound`）。それは後始末の
        // 目的が達成された状態なので、札だけを閉じて消す。
        report_cleanup(
            "remove the work folder",
            remove_tree(&self.path),
            &self.path,
        );
        retire_lease(self.lease.take(), &self.lock);
    }
}

/// 生存の札を開く。読むことだけを共有する（＝持ち主が居る間は誰も消せない）。
fn open_lease(lock: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_READ)
        .open(lock)
}

/// 札を閉じてから消す（開いたままでは削除を共有していないので自分でも消せない）。
fn retire_lease(lease: Option<File>, lock: &Path) {
    drop(lease);
    report_cleanup("remove the lease file", std::fs::remove_file(lock), lock);
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        self.release_with(|tree| std::fs::remove_dir_all(tree));
    }
}

/// 検体の `.nar` の置き場。本 crate だけがこの綴りを持つ。
///
/// 登記の往復（[`crate::SAMPLES`] の各 `.nar` を空の根へ展開して要素を登記と突き合わせる）
/// は窓口の兄弟テストに住むので、crate の中でだけ見えるようにしてある。狙いは第 2 の
/// 綴りを作らせないことなので、crate の外へは出さない。
pub(crate) fn nar_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendors/sample_ghost"
    ))
}

/// 登記の行から、展開に使う書庫のパスを決める唯一の式（spec: `areka-P0-sample-url-fetch`
/// 要件 1.4・3.1・3.2・3.4・3.8・3.9）。
///
/// リポジトリに置く行は今と同じ `vendors/sample_ghost/<name>.nar`（置き場を見ない）。URL の行は
/// 置き場のハッシュのファイルと SHA-256 が合った置き場の書庫（[`crate::fetched::verified_nar_in`]）。
/// 窓口の 3 つの入口は取得のたびにここを通るので、照合も取得のたびに行われ、落ちた書庫は
/// 原本の作り（[`cached_root_in`]）へ渡らない。
///
/// 返すのは書庫のパスと**読んだバイト**（audit-test-kits 決め D13）。原本の刻印と据え付けはこのバイトから
/// 作り、パスのファイルを読み直さない（読み直すと、確かめていないバイトで原本を作りうる）。
///
/// # Errors
///
/// 未登録の名前なら [`SampleError::UnknownSample`]。リポジトリの `.nar` が読めなければ
/// [`SampleError::Io`]。URL の行の照合の失敗は [`crate::fetched::verified_nar_in`] と同じ。
#[cfg(test)]
pub(crate) fn sample_nar(name: &str) -> Result<(PathBuf, Vec<u8>), SampleError> {
    sample_nar_for(crate::registered(name)?, namespace_dir)
}

/// [`sample_nar`] の本体。名前空間は URL の行のときだけ求める（テストは私有の名前空間と
/// 固定入力の行を渡す）。
fn sample_nar_for(
    sample: &'static Sample,
    namespace: impl FnOnce() -> Result<PathBuf, SampleError>,
) -> Result<(PathBuf, Vec<u8>), SampleError> {
    match sample.source {
        SampleSource::Vendored => {
            let nar = nar_dir().join(format!("{}.nar", sample.name));
            let bytes = std::fs::read(&nar).map_err(|source| SampleError::Io {
                what: "read the sample .nar",
                path: nar.clone(),
                source,
            })?;
            Ok((nar, bytes))
        }
        SampleSource::Fetched { .. } => crate::fetched::verified_nar_in(&namespace()?, sample.name),
    }
}

/// 窓口の入口へ渡す、検体 1 つの確かめたバイトの出どころ（呼ぶたびに読み直して確かめる）。
fn verified_bytes<'a>(
    sample: &'static Sample,
    namespace: &'a Path,
) -> impl Fn() -> Result<Vec<u8>, SampleError> + 'a {
    move || sample_nar_for(sample, || Ok(namespace.to_path_buf())).map(|(_, bytes)| bytes)
}

/// `.nar` 1 本の刻印＝**丸ごとの長さ**と **CRC-32**（要件 7.3）。
///
/// 長さだけでは同じ大きさの差し替えを見逃し、CRC だけでは長さの違いを 1/2^32 で
/// 取り違える。両方を名前に入れるので、原本を作り直す判定は名前の有無だけで済む。
fn stamp(bytes: &[u8]) -> String {
    format!("{}-{:08x}", bytes.len(), areka_nar::crc32(bytes))
}

/// [`CACHE`] の 1 件の名前を `(検体名, 刻印)` に割る。割れなければ `None`。
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

/// 検体 1 つの原本（[`CACHE`]`/<検体>-<刻印>/`）を用意して、その絶対パスを返す
/// （要件 7.2・7.3・7.6・7.7・7.8）。
///
/// 原本は**誰にも配らない**。配るのは [`fresh_root`] がここから取る複製である。
/// 掃除は取得の入口（[`fresh_root`]）が走らせるので、この関数は棚を片付けない。
///
/// 窓口 `SampleRoot::acquire` は複製を配る [`fresh_root`] だけを通るので、**原本を直に
/// 求める呼び手はテストだけ**である（多重プロセスの検査が、実物の `.nar` と
/// [`nar_dir`] の綴りを本物の経路で通す唯一の場所）。公開したままでは「検体を得る唯一の
/// 入口は `SampleRoot`」（要件 1.1）が嘘になるので、テスト専用と明示して閉じてある。
///
/// # Errors
///
/// `.nar` が読めない・原本を据え付けられないとき [`SampleError::Io`]、`.nar` が
/// 受理されないとき [`SampleError::Nar`]。古い原本の回収に失敗しても**失敗にはしない**
/// （標準エラーへ出して取得を続ける＝要件 7.3）。
#[cfg(test)]
pub(crate) fn cached_root(name: &str) -> Result<PathBuf, SampleError> {
    cached_root_in(&namespace_dir()?, name, &sample_nar(name)?.1)
}

/// 名前空間と `.nar` のバイトを明示して原本を用意する。
///
/// テストは私有の名前空間と自前で組んだ `.nar` を渡す（並走する他のテストを巻き込まずに
/// 「原本が無い」「刻印が変わった」「名前空間ごと消えた」を作れる）。
fn cached_root_in(namespace: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, SampleError> {
    let cache = namespace.join(CACHE);
    let stamp = stamp(bytes);
    let master = cache.join(format!("{name}-{stamp}"));

    // 名前が合う原本は完全（不変条件）。だから在るかどうかだけを見る。
    if !master.is_dir() {
        stage_into_cache(namespace, &cache, name, bytes, &master)?;
    }
    reclaim_stale(namespace, &cache, name, &stamp);
    Ok(master)
}

/// 空の根へ展開してから、`rename` 1 回で [`CACHE`] へ据え付ける。
///
/// 展開の間ずっと作業フォルダの札が開いているので、並走する別プロセスの掃除には
/// 消されない。`rename` が成功した後の作業フォルダの [`Drop`] は木が無いことを許容する。
///
/// `areka-nar` は書庫をパスからしか開けないので、確かめたバイトを別の札付きの作業フォルダの
/// 中の `<名>.nar` へ書いてから開く（audit-test-kits 決め D13。展開先に混ぜないために分ける）。
fn stage_into_cache(
    namespace: &Path,
    cache: &Path,
    name: &str,
    bytes: &[u8],
    master: &Path,
) -> Result<(), SampleError> {
    std::fs::create_dir_all(cache).map_err(|source| SampleError::Io {
        what: "create the cache shelf",
        path: cache.to_path_buf(),
        source,
    })?;
    let work = WorkDir::in_namespace(namespace)?;
    discard_older_cache_shelves(namespace);
    let scratch = WorkDir::in_namespace(namespace)?;
    let nar = scratch.path().join(format!("{name}.nar"));
    std::fs::write(&nar, bytes).map_err(|source| SampleError::Io {
        what: "write the verified .nar for staging",
        path: nar.clone(),
        source,
    })?;
    let archive = NarArchive::open(&nar)?;
    archive.install(&InstallRequest {
        roots: &SingleRoot(work.path()),
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
            what: "move the master copy into the cache",
            path: master.to_path_buf(),
            source,
        }),
    }
}

/// [`CACHE`] の数を上げる前の世代の棚（名前が `cache` で始まり [`CACHE`] でないフォルダ）を
/// 退ける（audit-test-kits 指摘 54）。数を上げるたびに古い棚が `cargo clean` まで残るのを防ぐ。失敗は
/// [`discard_tree`] と同じく人の読める形に出して続ける。
fn discard_older_cache_shelves(namespace: &Path) {
    let entries = match std::fs::read_dir(namespace) {
        Ok(entries) => entries,
        Err(err) => return report_cleanup("list the namespace", Err(err), namespace),
    };
    let shelf = namespace.join(WORK);
    for entry in entries.flatten() {
        let older = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with("cache") && name != CACHE);
        if older && entry.path().is_dir() {
            discard_tree(&shelf, &entry.path(), "older generation cache shelf");
        }
    }
}

/// 同じ検体の刻印違いの原本を回収する（要件 7.3・7.9）。
///
/// 先に `work/gc-…/` へ `rename` してから消すので、途中で落ちても [`CACHE`] の下に
/// 半端な木は残らない（残るのは `work/` の下の消し残しで、次の取得の掃除が拾う）。
///
/// 失敗しても**取得は続ける**。別のプロセスが複写中・ウイルス対策が掴んでいる、は
/// どれも次の取得で消せるようになる一時的な事情で、これで検体が得られないのは損だから。
///
/// 実測: Windows は**子孫のファイルが開かれているフォルダの `rename` を拒む**
/// （共有モードに書き込みと削除を足しても同じ）。掴まれている古い原本は最初の一歩で
/// 失敗するので、[`CACHE`] に無傷のまま残り、次の取得で改めて回収される（兄弟テスト
/// `a_reclaim_that_cannot_move_the_stale_copy_keeps_going_and_retries_on_the_next_acquire`）。
fn reclaim_stale(namespace: &Path, cache: &Path, name: &str, keep: &str) {
    let entries = match std::fs::read_dir(cache) {
        Ok(entries) => entries,
        Err(err) => return report_cleanup("list the old master copies", Err(err), cache),
    };
    let shelf = namespace.join(WORK);
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report_cleanup("list the old master copies", Err(err), cache);
                continue;
            }
        };
        let raw = entry.file_name();
        // 名前を読めない・刻印の形をしていないものは、この仕様が置いた原本ではない。
        // 黙って消さずに残す（`CACHE` の下は掃除の対象でもない）。
        let Some((sample, stamp)) = raw.to_str().and_then(split_stamp) else {
            continue;
        };
        if sample != name || stamp == keep {
            continue;
        }
        let stale = entry.path();
        if let Err(err) = std::fs::create_dir_all(&shelf) {
            report_cleanup("create the shelf for reclaiming", Err(err), &shelf);
            continue;
        }
        // `CACHE` の外へ出すのが先。「名前が合う原本は完全」を一瞬も破らない。
        discard_tree(&shelf, &stale, "old master copy");
    }
}

/// 木を作業の棚の中の `gc-…` へ**先に `rename` してから**消す（要件 7.7・7.9）。
///
/// 先に外へ出すので、途中で落ちても半端な木が元の名前で見えることがない。プロセス
/// 識別子を再利用した新しいプロセスが同じ名前の作業フォルダを作る瞬間との競合も、
/// 名前を変えた時点で消える。
///
/// 実測: Windows は**子孫のファイルが開かれているフォルダの `rename` を拒む**
/// （共有モードに書き込みと削除を足しても同じ）。掴まれている木は最初の一歩で失敗する
/// ので無傷のまま残り、次の取得で改めて片付けられる。
fn discard_tree(shelf: &Path, tree: &Path, what: &str) {
    discard_tree_with(shelf, tree, what, |gc| std::fs::remove_dir_all(gc));
}

/// [`discard_tree`] の手順。木を消す関数を受け取るのは [`WorkDir::release_with`] と同じ理由。
///
/// 退けた先の `gc-…` にも、移す**前に**札を作り、木を消し終えてから閉じて消す。札の無い
/// `gc-` の木は、消している間に並走する掃除から「札の無い残骸」に見え、同じ木を別の
/// `gc-` へ移そうとされる（spec: `areka-P0-ghost-session-test-load-flake` 要件 5.2・檻
/// `a_tree_moved_aside_keeps_a_held_lease_until_it_is_removed`）。
fn discard_tree_with(
    shelf: &Path,
    tree: &Path,
    what: &str,
    remove_tree: impl FnOnce(&Path) -> std::io::Result<()>,
) {
    let stem = format!(
        "gc-{}-{}",
        std::process::id(),
        NEXT_SERIAL.fetch_add(1, Ordering::Relaxed)
    );
    let gc = shelf.join(&stem);
    let lock = shelf.join(format!("{stem}{LOCK_SUFFIX}"));
    let lease = match open_lease(&lock) {
        Ok(lease) => lease,
        // 札が無いまま移すと上の窓が開く。移さずに残し、次の取得の掃除に任せる。
        Err(err) => {
            return report_cleanup(
                &format!("create the lease for moving aside the {what}"),
                Err(err),
                &lock,
            );
        }
    };
    match std::fs::rename(tree, &gc) {
        Ok(()) => report_cleanup(
            &format!("remove the moved-aside {what}"),
            remove_tree(&gc),
            &gc,
        ),
        Err(err) => report_cleanup(&format!("move aside the {what}"), Err(err), tree),
    }
    retire_lease(Some(lease), &lock);
}

/// 作業の棚の残骸を片付ける（要件 7.7・7.9）。取得のたびに 1 度走る。
///
/// 棚の並び順は決まっていないので、木より先に札を見るとは限らない。札を消してみる
/// 走査と、木を片付ける走査の 2 段に分ける。生きている利用者の木と組み上げ中の木は
/// 札が削除を拒むので残る。失敗は人の読める形に出して取得を続ける。
fn sweep(namespace: &Path) {
    let shelf = namespace.join(WORK);
    let entries = match std::fs::read_dir(&shelf) {
        Ok(entries) => entries,
        // 棚がまだ無いのは、片付ける残骸が無いのと同じ。
        Err(err) => return report_cleanup("list the work shelf", Err(err), &shelf),
    };
    let mut held = std::collections::HashSet::new();
    let mut trees = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report_cleanup("list the work shelf", Err(err), &shelf);
                continue;
            }
        };
        let raw = entry.file_name();
        // 名前を読めないものは、この仕様が置いた物ではない。黙って消さずに残す。
        let Some(name) = raw.to_str() else { continue };
        if let Some(stem) = name.strip_suffix(LOCK_SUFFIX) {
            // 札を消せた（`Ok`）ときだけ「持ち主は居ない」（要件 7.5・7.7）。失敗は**どれも
            // 触らない**——共有違反は生きている利用者が握っている札で、`NotFound` は別の
            // プロセスが同時に回収している最中である。後者を「持ち主が居ない」と読むと、
            // 回収中の相手の木を横から消してしまう。[`report_cleanup`] が `NotFound` を
            // 成功として通すので、この取り違えは机上の話ではない。
            if std::fs::remove_file(entry.path()).is_err() {
                held.insert(stem.to_owned());
            }
            continue;
        }
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => trees.push((name.to_owned(), entry.path())),
            // 木でないものも、この仕様が置いた物ではない（上と同じ扱い）。
            Ok(_) => {}
            Err(err) => report_cleanup("read the type of a shelf entry", Err(err), &entry.path()),
        }
    }
    for (name, tree) in trees {
        if !held.contains(&name) {
            discard_tree(&shelf, &tree, "leftover tree");
        }
    }
}

/// 検体 1 つの**使い捨ての複製**を配る（要件 6.5・7.4・7.5・7.7・7.9）。
///
/// 取得のたびに ⑴ 棚を掃除し、⑵ 原本を用意し、⑶ 札を先に開いてから原本を複写する。
/// 返った値を束縛している間だけ複製が生き、破棄で木と札の両方が消える。
///
/// 窓口 `SampleRoot::acquire` が内側から呼ぶ。crate の外へは出さない——検体を得る
/// 唯一の入口は `SampleRoot` である（要件 1.1）。
///
/// # Errors
///
/// 原本を用意できないとき [`cached_root`] と同じ失敗、複写できないとき
/// [`SampleError::Io`]。掃除の失敗は**失敗にしない**（人の読める形に出して続ける）。
pub(crate) fn fresh_root(sample: &'static Sample) -> Result<WorkDir, SampleError> {
    let namespace = namespace_dir()?;
    fresh_root_in(&namespace, sample.name, verified_bytes(sample, &namespace))
}

/// 検体 1 つの**手で使う根**（`manual/<検体>/`）を配り直して、その絶対パスを返す
/// （要件 1.9・9.7）。
///
/// 複製（[`fresh_root`]）と違って、**プロセスが終わっても木が残る**。印字した絶対パスを
/// 人が `areka.exe` へ渡すのはこの後だから、配った側の寿命に縛ってはならない。代わりに
/// 呼ぶたびに丸ごと消して原本から複写し直すので、実機の 2 周が 2 回とも起動記録の無い根で
/// 始まる。札を持たないので棚を [`WORK`] の外（[`MANUAL`]）に置き、掃除から外す。
///
/// # Errors
///
/// 原本を用意できないとき [`cached_root`] と同じ失敗、前回の木を消せない・複写できない
/// とき [`SampleError::Io`]。**消せない木を黙って配り直さない**——前回の起動記録が残った
/// 根を「新品」として印字するのは、この窓口が唯一避けたい事故である。
pub(crate) fn manual_root(sample: &'static Sample) -> Result<PathBuf, SampleError> {
    let namespace = namespace_dir()?;
    manual_root_in(&namespace, sample.name, verified_bytes(sample, &namespace))
}

/// 名前空間と `.nar` のバイトの出どころを明示して手で使う根を配り直す（テストが私有の
/// 名前空間を渡す）。出どころは原本が複写の途中で退けられたときにもう 1 度だけ呼ぶ。
pub(crate) fn manual_root_in(
    namespace: &Path,
    name: &str,
    nar: impl Fn() -> Result<Vec<u8>, SampleError>,
) -> Result<PathBuf, SampleError> {
    let master = cached_root_in(namespace, name, &nar()?)?;
    let manual = namespace.join(MANUAL).join(name);
    match std::fs::remove_dir_all(&manual) {
        Ok(()) => {}
        // まだ 1 度も配っていない（または前回の走行が消えている）。作り直しの目的は
        // 既に達成されている。
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(SampleError::Io {
                what: "remove the manual root before rebuilding it",
                path: manual,
                source,
            });
        }
    }
    copy_master(&master, &manual, || {
        cached_root_in(namespace, name, &nar()?)
    })?;
    Ok(manual)
}

/// 名前空間と `.nar` のバイトの出どころを明示して複製を配る（テストが私有の名前空間を渡す）。
///
/// 窓口の兄弟テストも、登記と食い違う展開結果を作るためにここから複製を取る。
pub(crate) fn fresh_root_in(
    namespace: &Path,
    name: &str,
    nar: impl Fn() -> Result<Vec<u8>, SampleError>,
) -> Result<WorkDir, SampleError> {
    sweep(namespace);
    let master = cached_root_in(namespace, name, &nar()?)?;
    // 札はフォルダより先に開かれる（`WorkDir` の約束）。複写はその後なので、組み上がる
    // 途中の複製が札の無い木として棚に見えることはない。
    let copy = WorkDir::in_namespace(namespace)?;
    copy_master(&master, copy.path(), || {
        cached_root_in(namespace, name, &nar()?)
    })?;
    Ok(copy)
}

/// 原本を複写する。複写が落ち、原本がもう無いときだけ、`again` で原本を用意し直して
/// 1 度だけ写し直す（audit-test-kits 決め D5）。
///
/// 原本が消えるのは、違うバイトを読んだ別のプロセスが刻印違いとして退けたとき。
/// `again` は今の書庫のバイトを読み直すので、勝った側の原本に合流して終わる。
fn copy_master(
    master: &Path,
    to: &Path,
    again: impl FnOnce() -> Result<PathBuf, SampleError>,
) -> Result<(), SampleError> {
    match copy_tree(master, to) {
        Err(_) if !master.exists() => {
            // 消えた原本の写しかけを新しい原本の複製に混ぜない。
            match std::fs::remove_dir_all(to) {
                Err(source) if source.kind() != std::io::ErrorKind::NotFound => {
                    return Err(SampleError::Io {
                        what: "remove the partial copy before copying again",
                        path: to.to_path_buf(),
                        source,
                    });
                }
                _ => {}
            }
            copy_tree(&again()?, to)
        }
        done => done,
    }
}

/// 原本の木をそっくり複写する。
///
/// `.nar` を展開し直すのではなく**ファイルを写す**のが要件 6.5 の「開発用の根は
/// 再インストールの経路を通らない」である（兄弟テストが、原本にだけ在るファイルが
/// 複製に現れることで判定する）。
pub(crate) fn copy_tree(from: &Path, to: &Path) -> Result<(), SampleError> {
    let mut stack = vec![(from.to_path_buf(), to.to_path_buf())];
    while let Some((src, dst)) = stack.pop() {
        std::fs::create_dir_all(&dst).map_err(|source| SampleError::Io {
            what: "create a folder of the copy",
            path: dst.clone(),
            source,
        })?;
        let entries = std::fs::read_dir(&src).map_err(|source| SampleError::Io {
            what: "list the master copy",
            path: src.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| SampleError::Io {
                what: "list the master copy",
                path: src.clone(),
                source,
            })?;
            let kind = entry.file_type().map_err(|source| SampleError::Io {
                what: "read the type of a master copy entry",
                path: entry.path(),
                source,
            })?;
            let target = dst.join(entry.file_name());
            if kind.is_dir() {
                stack.push((entry.path(), target));
            } else {
                std::fs::copy(entry.path(), &target).map_err(|source| SampleError::Io {
                    what: "copy a file of the master copy",
                    path: target,
                    source,
                })?;
            }
        }
    }
    Ok(())
}

/// 後始末の失敗を必ず人の読める形で残す。
///
/// [`Drop`]・古い原本の回収・掃除は失敗を返せない（後始末の失敗で取得を止めない＝
/// 要件 7.3・7.7）ので、
/// 呼び手へ伝える道がここしかない。本 crate はテスト専用で記録層に依存しない
/// （依存を足さない）ため、宛先は標準エラー出力にする。既に無いのは後始末の目的が
/// 達成された状態なので黙って通す。残った木と札は次の走行が回収する。
fn report_cleanup(what: &str, result: std::io::Result<()>, path: &Path) {
    if let Err(err) = result {
        if err.kind() == std::io::ErrorKind::NotFound {
            return;
        }
        // 頭の `sample-ghost-kit:` と `os error N` は `tools/load-flake.ps1` が字面で読む。
        eprintln!(
            "sample-ghost-kit: cleanup failed ({what}): {} ({})",
            crate::ascii(&path.to_string_lossy()),
            crate::ascii(&err.to_string())
        );
    }
}

#[cfg(test)]
#[path = "devroot_tests.rs"]
mod tests;

/// 原本（[`CACHE`]）の兄弟テスト。1 ファイル 1,000 行の上限に収めるために分けただけで、
/// 見ている物は同じ [`WorkDir`] と同じ名前空間。
#[cfg(test)]
#[path = "devroot_cache_tests.rs"]
mod cache_tests;

/// 複製と掃除の兄弟テスト。原本の兄弟テストの助けを借りる（固定入力の組み方と木の
/// 読み方は同じ物を使う）。
#[cfg(test)]
#[path = "devroot_sweep_tests.rs"]
mod sweep_tests;

/// 行から書庫のパスを決める解決（`sample_nar`）の兄弟テスト。URL の行は固定入力の行と私有の
/// 名前空間で作る。
#[cfg(test)]
#[path = "devroot_fetched_tests.rs"]
mod fetched_tests;
