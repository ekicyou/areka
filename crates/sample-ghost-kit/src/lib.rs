//! 検体ゴースト／バルーンを **名前で引く**窓口を、ワークスペースで **1 箇所**だけ定義する
//! crate（spec: `areka-P0-nar-install` 要件 1）。
//!
//! この doc は**利用手順**である。ここだけを読めば、別の crate のテスト・example から検体を
//! 使う書き方が分かる。実装を開く必要は無い。掲載している Rust の例は **doctest** として
//! `cargo test -p sample-ghost-kit` でコンパイル・実行される（手順書が黙って古びない）。
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
//! [`SampleRoot::acquire`] に検体名を渡し、**得た値を束縛したまま**パスを借りる。
//!
//! ```rust
//! use sample_ghost_kit::SampleRoot;
//!
//! let emo2 = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
//!
//! // ゴースト／バルーンのフォルダ。
//! assert!(emo2.folder().join("ghost").join("master").is_dir());
//!
//! // 同時にインストールされるバルーンは名前で引く（自分でパスを継ぎ足さない）。
//! assert!(emo2.balloon("emo2-kakukaku").expect("emo2 の同梱バルーン").is_dir());
//! ```
//!
//! 3 つの読み口は**全て借用を返す**ので、値を捨ててパスだけ取り出す書き方はコンパイルできない
//! （[`SampleRoot::folder`] の例を参照）。段 ③ 以降は値の寿命が展開された木の寿命になるため、
//! この型の強制が「消えた木のパスを渡す」事故を構造的に防ぐ。
//!
//! # 検体を足すとき
//!
//! 2 手で終わる（要件 1.5）——`vendors/sample_ghost/<名>.nar` を 1 つ置き、[`SAMPLES`] に
//! 1 行足す。検体ごとの専用関数は増やさない。
//!
//! # 段 ① の中間形
//!
//! 現在の窓口は**追跡済みの展開形**を指すだけで、展開は行わない。`folder()` は
//! `<リポジトリ根>/<登記の checked_in_parent>/<名>` を、`balloon()` はその直下を返す。
//! 根を返す `root()` は段 ③（`.nar` からの展開）で入る。段 ③ で返す先は
//! `<根>/ghost/<名>`・`<根>/balloon/<名>` に変わるが、**呼び手の 1 行は変わらない**。

use std::path::{Path, PathBuf};

/// 検体の種別。段 ③ では展開結果が登記と一致するかの照合に使う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleKind {
    /// ゴースト（`install.txt` の `type,ghost`）。
    Ghost,
    /// バルーン（`install.txt` の `type,balloon`）。
    Balloon,
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
    /// **段 ① 限定**——追跡済みの展開形が置かれている親フォルダ（リポジトリ根から見た相対）。
    ///
    /// 検体ごとに置き場が違う（emo2 系は example の fixtures・里々の標準テンプレートは
    /// `vendors/`）ので、登記の同じ行に持たせて「1 行足すだけ」を保つ。段 ③ で保管が
    /// `vendors/sample_ghost/<名>.nar` に統一されると、この欄は消える。
    pub checked_in_parent: &'static str,
}

/// 検体の登記表。**検体を足す作業はここに 1 行**（要件 1.5）。
pub const SAMPLES: &[Sample] = &[
    Sample {
        name: "emo2",
        kind: SampleKind::Ghost,
        balloons: &["emo2-kakukaku"],
        checked_in_parent: "crates/pilot/examples/shiori-host-32/fixtures",
    },
    Sample {
        name: "R_POST_and_KOMAINU",
        kind: SampleKind::Ghost,
        balloons: &[],
        checked_in_parent: "vendors/sample_ghost",
    },
    Sample {
        name: "emo2-kakukaku-offsetdpi",
        kind: SampleKind::Balloon,
        balloons: &[],
        checked_in_parent: "crates/pilot/examples/shiori-host-32/fixtures",
    },
    Sample {
        name: "emo2-kakukaku-wplimit",
        kind: SampleKind::Balloon,
        balloons: &[],
        checked_in_parent: "crates/pilot/examples/shiori-host-32/fixtures",
    },
];

/// リポジトリ根の絶対パス。本 crate だけがこの綴りを持つ。
fn workspace_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// 取得した検体。**この値を束縛している間だけ**パスを借りられる。
///
/// 段 ③ 以降は値の寿命が展開された木の寿命になるので、借用しか配らないことが
/// 「消えた木のパス」を構造的に防ぐ。
#[derive(Debug)]
pub struct SampleRoot {
    sample: &'static Sample,
    folder: PathBuf,
    /// 同梱バルーンの `(directory 名, フォルダ)`。借用を返すため先に組んでおく。
    balloons: Vec<(&'static str, PathBuf)>,
}

impl SampleRoot {
    /// 検体名から検体を取得する（要件 1.1）。
    ///
    /// # Errors
    ///
    /// 未登録の名前なら [`SampleError::UnknownSample`]（既知の名前の一覧を含む・要件 1.4）。
    pub fn acquire(name: &str) -> Result<SampleRoot, SampleError> {
        let sample = SAMPLES
            .iter()
            .find(|candidate| candidate.name == name)
            .ok_or_else(|| SampleError::UnknownSample {
                requested: name.to_owned(),
                known: known_sample_names(),
            })?;
        let folder = workspace_root()
            .join(sample.checked_in_parent)
            .join(sample.name);
        let balloons = sample
            .balloons
            .iter()
            .map(|balloon| (*balloon, folder.join(balloon)))
            .collect();
        Ok(SampleRoot {
            sample,
            folder,
            balloons,
        })
    }

    /// 検体がインストール済み形で置かれたフォルダの絶対パス（要件 1.1）。
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
    /// ```rust
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

    /// 同時にインストールされるバルーンのフォルダの絶対パス（要件 1.3）。
    ///
    /// `folder()` に名前を継ぎ足して自分でパスを作る代わりにこれを呼ぶ。段 ③ で
    /// バルーンの置き場が `<根>/balloon/<名>` に変わっても呼び手は書き換えずに済む。
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
    /// ```rust
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
pub use devroot::{WorkDir, cached_root, fresh_root};

mod nar_writer;
pub use nar_writer::{Corrupt, Damage, EntryBuilder, NarBuilder, fold_tree, install_txt};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
