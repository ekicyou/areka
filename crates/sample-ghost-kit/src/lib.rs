//! 検体ゴースト／バルーンを **名前で引く**窓口を、ワークスペースで **1 箇所**だけ定義する
//! crate（spec: `areka-P0-nar-install` 要件 1）。
//!
//! この doc は**利用手順**である。ここだけを読めば、別の crate のテスト・example から検体を
//! 使う書き方が分かる。実装を開く必要は無い。掲載している Rust の例は **doctest** として
//! `cargo test -p sample-ghost-kit` でコンパイルされる（手順書が黙って古びない）。
//!
//! # なぜ要るか
//!
//! 検体の在処を各テストが自前で綴ると、保管の形を変えるときに 38 か所を追いかけ回すことに
//! なる。窓口を 1 つ通しておけば、直すのは本 crate の中だけで済む。
//!
//! # 引き方
//!
//! 消費 crate の `Cargo.toml` に 1 行加える。**`[dev-dependencies]` からのみ**引く。
//!
//! ```toml
//! [dev-dependencies]
//! sample-ghost-kit = { path = "../sample-ghost-kit" }
//! ```
//!
//! # 使い方
//!
//! [`SampleRoot::acquire`] に検体名を渡し、**得た値を束縛したまま**パスを借りる。取得の
//! たびに `vendors/sample_ghost/<名>.nar` を展開した原本の**使い捨ての複製**が配られ、
//! 値を捨てた時点でその複製は消える（起動記録の無い新品で始まる＝要件 7.4）。
//!
//! ```rust,no_run
//! use sample_ghost_kit::SampleRoot;
//!
//! let emo2 = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
//!
//! // ベースウェアの根（`ghost/`・`balloon/` を直下に持つ）。
//! assert!(emo2.root().join("ghost").is_dir());
//!
//! // ゴースト／バルーンのフォルダ。
//! assert!(emo2.folder().join("ghost").join("master").is_dir());
//!
//! // 同時にインストールされるバルーンは名前で引く（自分でパスを継ぎ足さない）。
//! assert!(emo2.balloon("emo2-kakukaku").expect("emo2 の同梱バルーン").is_dir());
//! ```
//!
//! 上の例が `no_run`（組むだけで走らせない）なのは、doctest の実行ファイルだけが
//! ビルド成果物の置き場の外（OS の一時フォルダ）に置かれるため。展開先はその置き場の
//! 下に掘るので、doctest から呼ぶと設計どおり [`SampleError::TargetDirNotFound`] に
//! なる（`devroot` の doc を参照）。振る舞いの判定は兄弟テストが持ち、この例は
//! **窓口が crate の外から届くこと**（doctest は別 crate として組まれる）を固定する。
//!
//! 3 つの読み口は**全て借用を返す**ので、値を捨ててパスだけ取り出す書き方はコンパイル
//! できない（[`SampleRoot::folder`] の例を参照）。値の寿命が複製された木の寿命なので、
//! この型の強制が「消えた木のパスを渡す」事故を構造的に防ぐ。
//!
//! # 検体を足すとき
//!
//! 2 手で終わる（要件 1.5）——`vendors/sample_ghost/<名>.nar` を 1 つ置き、[`SAMPLES`] に
//! 1 行足す。検体ごとの専用関数は増やさない。
//!
//! # 返す位置
//!
//! [`SampleRoot::root`] が**ベースウェアの根**、[`SampleRoot::folder`] が
//! `<根>/ghost/<名>/`（バルーンの検体なら `<根>/balloon/<名>/`）、
//! [`SampleRoot::balloon`] が `<根>/balloon/<directory>/`。展開結果の要素が登記
//! （種別・フォルダ名・同梱バルーン）と食い違えば [`SampleError::RegistryMismatch`]
//! を返す（黙って存在しないパスを配らない）。

use std::path::{Path, PathBuf};

/// 検体の種別。展開結果が登記と一致するかの照合に使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleKind {
    /// ゴースト（`install.txt` の `type,ghost`）。
    Ghost,
    /// バルーン（`install.txt` の `type,balloon`）。
    Balloon,
}

impl SampleKind {
    /// 根の直下の格納先の名前（`<根>/ghost/`・`<根>/balloon/`）。
    fn store(self) -> &'static str {
        match self {
            SampleKind::Ghost => "ghost",
            SampleKind::Balloon => "balloon",
        }
    }
}

/// 検体 1 つの登記。足すときに書く 1 行がこれである。
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    /// `.nar` のファイル名（拡張子無し）＝ `install.txt` の `directory`。
    pub name: &'static str,
    /// 種別。
    pub kind: SampleKind,
    /// 同時にインストールされるバルーンの `directory` 名。
    pub balloons: &'static [&'static str],
}

/// 検体の登記表。**検体を足す作業はここに 1 行**（要件 1.5）。
pub const SAMPLES: &[Sample] = &[
    Sample {
        name: "emo2",
        kind: SampleKind::Ghost,
        balloons: &["emo2-kakukaku"],
    },
    Sample {
        name: "R_POST_and_KOMAINU",
        kind: SampleKind::Ghost,
        balloons: &[],
    },
    Sample {
        name: "emo2-kakukaku-offsetdpi",
        kind: SampleKind::Balloon,
        balloons: &[],
    },
    Sample {
        name: "emo2-kakukaku-wplimit",
        kind: SampleKind::Balloon,
        balloons: &[],
    },
    Sample {
        name: "konnoyayame",
        kind: SampleKind::Ghost,
        balloons: &[],
    },
];

/// 取得した検体。**この値を束縛している間だけ**パスを借りられる。
///
/// 値の寿命が複製された木の寿命なので、借用しか配らないことが「消えた木のパス」を
/// 構造的に防ぐ。破棄で複製の木と生存の札の両方が消える。
#[derive(Debug)]
pub struct SampleRoot {
    sample: &'static Sample,
    /// 配られた複製。破棄で木が消えるので、**この値が根の寿命そのもの**である。
    copy: WorkDir,
    folder: PathBuf,
    /// 同梱バルーンの `(directory 名, フォルダ)`。借用を返すため先に組んでおく。
    balloons: Vec<(&'static str, PathBuf)>,
}

impl SampleRoot {
    /// 検体名から検体を取得する（要件 1.1）。
    ///
    /// `vendors/sample_ghost/<名>.nar` を展開した原本から**使い捨ての複製**を作って配る。
    ///
    /// # Errors
    ///
    /// 未登録の名前なら [`SampleError::UnknownSample`]（既知の名前の一覧を含む・要件 1.4）。
    /// 展開結果が登記と食い違えば [`SampleError::RegistryMismatch`]。`.nar` の取り回しの
    /// 失敗は [`SampleError::Nar`]・[`SampleError::Io`]・[`SampleError::TargetDirNotFound`]。
    pub fn acquire(name: &str) -> Result<SampleRoot, SampleError> {
        let sample = registered(name)?;
        SampleRoot::from_copy(sample, devroot::fresh_root(sample.name)?)
    }

    /// 配られた複製を**登記と照合**してから窓口の形に組む（要件 1.1・1.2・1.3）。
    ///
    /// 照合が要るのは、位置（`<根>/ghost/<名>/` など）を登記の行から組み立てるからである。
    /// `.nar` の `install.txt` が別の `directory` や別の種別を名乗っていると、組んだ位置は
    /// 実在しないパスになる。黙って配らずに [`SampleError::RegistryMismatch`] を返す。
    ///
    /// 兄弟テストは私有の名前空間と自前の `.nar` から取った複製をここへ渡して、4 通りの
    /// 食い違い（フォルダ名・種別・同梱の欠け・同梱の余り）を通す。
    fn from_copy(sample: &'static Sample, copy: WorkDir) -> Result<SampleRoot, SampleError> {
        check_registry(sample, copy.path())?;
        let root = copy.path().to_path_buf();
        let folder = root.join(sample.kind.store()).join(sample.name);
        let balloons = sample
            .balloons
            .iter()
            .map(|balloon| (*balloon, root.join("balloon").join(balloon)))
            .collect();
        Ok(SampleRoot {
            sample,
            copy,
            folder,
            balloons,
        })
    }

    /// 検体を含む**ベースウェアの根**（`ghost/`・`balloon/` を直下に持つ）の絶対パス
    /// （要件 1.2）。
    ///
    /// 返すのは借用なので、取得した値を捨ててパスだけ持ち出すことはできない。
    ///
    /// ```rust,compile_fail
    /// use sample_ghost_kit::SampleRoot;
    ///
    /// fn main() -> Result<(), sample_ghost_kit::SampleError> {
    ///     // 一時値の破棄と借用が衝突する（E0716）。
    ///     let path = SampleRoot::acquire("emo2")?.root();
    ///     println!("{}", path.display());
    ///     Ok(())
    /// }
    /// ```
    pub fn root(&self) -> &Path {
        self.copy.path()
    }

    /// 検体がインストール済み形で置かれたフォルダの絶対パス
    /// （`<根>/ghost/<名>/` またはバルーンの検体なら `<根>/balloon/<名>/`・要件 1.1）。
    ///
    /// 返すのは借用なので、取得した値を捨ててパスだけ持ち出すことはできない。
    ///
    /// ```rust,compile_fail
    /// use sample_ghost_kit::SampleRoot;
    ///
    /// fn main() -> Result<(), sample_ghost_kit::SampleError> {
    ///     // 一時値の破棄と借用が衝突する（E0716）。
    ///     let path = SampleRoot::acquire("emo2")?.folder();
    ///     println!("{}", path.display());
    ///     Ok(())
    /// }
    /// ```
    ///
    /// 値を束縛すれば通る（上の例が別の理由で落ちているのではないことの対）。
    ///
    /// ```rust,no_run
    /// use sample_ghost_kit::SampleRoot;
    ///
    /// fn main() -> Result<(), sample_ghost_kit::SampleError> {
    ///     let emo2 = SampleRoot::acquire("emo2")?;
    ///     let path = emo2.folder();
    ///     println!("{}", path.display());
    ///     Ok(())
    /// }
    /// ```
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// 同時にインストールされるバルーンのフォルダ `<根>/balloon/<directory>/` の絶対パス
    /// （要件 1.3）。
    ///
    /// `folder()` に名前を継ぎ足して自分でパスを作る代わりにこれを呼ぶ。
    ///
    /// # Errors
    ///
    /// その検体が同時にインストールしないバルーン名なら [`SampleError::UnknownBalloon`]
    /// （既知の一覧を含む・要件 1.4）。
    ///
    /// こちらも借用なので、取得した値を捨てる書き方はコンパイルできない。
    ///
    /// ```rust,compile_fail
    /// use sample_ghost_kit::SampleRoot;
    ///
    /// fn main() -> Result<(), sample_ghost_kit::SampleError> {
    ///     // 一時値の破棄と借用が衝突する（E0716）。
    ///     let path = SampleRoot::acquire("emo2")?.balloon("emo2-kakukaku")?;
    ///     println!("{}", path.display());
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ```rust,no_run
    /// use sample_ghost_kit::SampleRoot;
    ///
    /// fn main() -> Result<(), sample_ghost_kit::SampleError> {
    ///     let emo2 = SampleRoot::acquire("emo2")?;
    ///     let path = emo2.balloon("emo2-kakukaku")?;
    ///     println!("{}", path.display());
    ///     Ok(())
    /// }
    /// ```
    pub fn balloon(&self, directory: &str) -> Result<&Path, SampleError> {
        self.balloons
            .iter()
            .find(|(name, _)| *name == directory)
            .map(|(_, path)| path.as_path())
            .ok_or(SampleError::UnknownBalloon {
                sample: self.sample.name,
                requested: directory.to_owned(),
                known: self.sample.balloons,
            })
    }
}

/// 検体名を登記の 1 行に引き当てる（要件 1.1・1.4）。
///
/// 未登録の名前を受けたら既知の名前の一覧を添えて断る。窓口も下の [`manual_paths`] も
/// ここを通るので、「黙って空のパスを返さない」の判断は 1 か所にしかない。
fn registered(name: &str) -> Result<&'static Sample, SampleError> {
    SAMPLES
        .iter()
        .find(|candidate| candidate.name == name)
        .ok_or_else(|| SampleError::UnknownSample {
            requested: name.to_owned(),
            known: known_sample_names(),
        })
}

/// 配られた根の中身が登記の 1 行と合っているかを照合する（要件 1.1・1.2・1.3）。
fn check_registry(sample: &'static Sample, root: &Path) -> Result<(), SampleError> {
    let declared = declared_elements(sample);
    let installed = installed_elements(root)?;
    if installed != declared {
        return Err(SampleError::RegistryMismatch {
            sample: sample.name,
            expected: declared.join(", "),
            installed,
        });
    }
    Ok(())
}

/// 検体を**手で使う根**へ配り直し、印字する `key=value` の組を並べて返す（要件 1.9）。
///
/// 実機走行は絶対パス起動が定石なので、開発者はコマンド `nar-sample-path` でこれを得る。
/// 配る先は [`SampleRoot::acquire`] の使い捨ての複製ではなく、**プロセスが終わっても残る**
/// `manual/<名>/`（印字した絶対パスを人が使うのはコマンドが終わった後だから）。呼ぶたびに
/// 丸ごと作り直すので、実機を 2 周すれば 2 回とも起動記録の無い根で始まる。
///
/// 鍵は `root`・`folder`・同梱バルーンごとの `balloon.<directory>` の順。値は
/// [`SampleRoot`] の 3 つの読み口と同じ位置である（実機と自動テストが同じ木を見る）。
///
/// # Errors
///
/// [`SampleRoot::acquire`] と同じ。未登録の名前なら既知の名前の一覧を含む
/// [`SampleError::UnknownSample`]、展開結果が登記と食い違えば
/// [`SampleError::RegistryMismatch`]（実在しないパスを印字しない）。
pub fn manual_paths(name: &str) -> Result<Vec<(String, PathBuf)>, SampleError> {
    let sample = registered(name)?;
    let root = devroot::manual_root(sample.name)?;
    check_registry(sample, &root)?;
    let mut printed = vec![
        ("root".to_owned(), root.clone()),
        (
            "folder".to_owned(),
            root.join(sample.kind.store()).join(sample.name),
        ),
    ];
    printed.extend(sample.balloons.iter().map(|balloon| {
        (
            format!("balloon.{balloon}"),
            root.join("balloon").join(balloon),
        )
    }));
    Ok(printed)
}

/// 登記の 1 行が主張する**展開結果の要素**を `<格納先>/<名前>` の形で名前順に並べる。
///
/// 本体が 1 つと同梱バルーンが 0 個以上。照合の期待値であり、位置を組み立てる式でもある。
fn declared_elements(sample: &Sample) -> Vec<String> {
    let mut declared = vec![format!("{}/{}", sample.kind.store(), sample.name)];
    declared.extend(
        sample
            .balloons
            .iter()
            .map(|balloon| format!("balloon/{balloon}")),
    );
    declared.sort();
    declared
}

/// 展開された根の直下に**実際に**在る要素を `<格納先>/<名前>` の形で名前順に並べる。
///
/// 片方の格納先しか作らない検体（同梱バルーンの無いゴースト・単体のバルーン）があるので、
/// 棚が無いのは「その格納先の要素が 0 件」と読む。
fn installed_elements(root: &Path) -> Result<Vec<String>, SampleError> {
    let mut installed = Vec::new();
    for store in ["ghost", "balloon"] {
        let shelf = root.join(store);
        let entries = match std::fs::read_dir(&shelf) {
            Ok(entries) => entries,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(SampleError::Io {
                    what: "展開結果の走査",
                    path: shelf,
                    source,
                });
            }
        };
        for entry in entries {
            let entry = entry.map_err(|source| SampleError::Io {
                what: "展開結果の走査",
                path: shelf.clone(),
                source,
            })?;
            installed.push(format!("{store}/{}", entry.file_name().to_string_lossy()));
        }
    }
    installed.sort();
    Ok(installed)
}

/// 登記されている検体名の一覧（失敗の理由に載せる・要件 1.4）。
///
/// [`SAMPLES`] から毎回導くので、名前の一覧が第 2 の手書きの表になることはない（要件 1.5）。
fn known_sample_names() -> Vec<&'static str> {
    SAMPLES.iter().map(|sample| sample.name).collect()
}

/// 窓口が返す失敗。黙って空のパスを返さない（要件 1.4）。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SampleError {
    /// 登記されていない検体名。
    #[error("unknown sample {requested:?}; known samples: {known:?}")]
    UnknownSample {
        /// 渡された名前。
        requested: String,
        /// 登記されている検体名の全て。
        known: Vec<&'static str>,
    },
    /// その検体が同時にインストールしないバルーン名。
    #[error("sample {sample:?} does not install balloon {requested:?}; known balloons: {known:?}")]
    UnknownBalloon {
        /// 検体名。
        sample: &'static str,
        /// 渡されたバルーン名。
        requested: String,
        /// その検体が同時にインストールするバルーンの全て（無ければ空）。
        known: &'static [&'static str],
    },
    /// 展開結果の要素が登記（種別・フォルダ名・同梱バルーン）と食い違う。
    #[error("sample {sample:?} installed {installed:?} but the registry declares {expected}")]
    RegistryMismatch {
        /// 検体名。
        sample: &'static str,
        /// 登記が主張する要素（`<格納先>/<名前>` を名前順に並べたもの）。
        expected: String,
        /// 実際に置かれた要素。
        installed: Vec<String>,
    },
    /// ビルド成果物の置き場が決まらない（`CARGO_TARGET_DIR` も祖先の `target` も無い）。
    #[error("build output directory not found from {started_from:?}; set CARGO_TARGET_DIR")]
    TargetDirNotFound {
        /// 祖先を辿り始めた場所（実行ファイル）。
        started_from: PathBuf,
    },
    /// 検体の `.nar` が受理されなかった（原本を作れない）。
    #[error("sample archive refused: {0}")]
    Nar(#[from] areka_nar::NarError),
    /// 開発用の根のファイル操作の失敗。何をしようとしたのかを添える。
    #[error("{what} failed at {path:?}: {source}")]
    Io {
        /// 何をしようとしたか。
        what: &'static str,
        /// 対象のパス。
        path: PathBuf,
        /// 元の失敗。
        #[source]
        source: std::io::Error,
    },
}

mod devroot;
pub use devroot::WorkDir;

mod nar_writer;
pub use nar_writer::{Corrupt, Damage, EntryBuilder, NarBuilder, fold_tree, install_txt};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
