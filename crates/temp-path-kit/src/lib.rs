//! テスト用の一時パスを **プロセス間で一意**に組み立てる窓口を、ワークスペースで
//! **1 箇所**だけ定義する crate。
//!
//! この doc は**利用手順**である。ここだけを読めば、別の crate から一時ディレクトリ・
//! 一時ファイルを使うテストが書ける。実装を開く必要は無い。
//!
//! 掲載している Rust の例は **doctest** として `cargo test -p temp-path-kit` で
//! コンパイル・実行される。しかも doctest は本 crate を外から `use` する**別 crate**として
//! 組まれるので、そのまま消費側で成立する形になっている（手順書が黙って古びない）。
//!
//! # なぜ要るか
//!
//! `std::env::temp_dir()` の下に**固定名**を組み立てると、同じテストを複数プロセスで
//! 同時に走らせたときに互いの一時ファイルを奪い合う。プロセス内だけで一意な連番も同じで、
//! 隣のプロセスが同じ名前を先に作り、先に消す。実際にこの仕様の反復検証で
//! `cargo test -p areka` の同時 4 プロセス 30 回中 3 回が赤になっている。
//!
//! [`TempPath`] は名前に**プロセス識別子**（`std::process::id()`）と**単調増加の連番**の
//! 両方を含めるので、プロセス内でもプロセス間でも衝突しない。
//!
//! # 引き方
//!
//! 消費 crate の `Cargo.toml` に 1 行加える。**`[dev-dependencies]` からのみ**引く。
//!
//! ```toml
//! [dev-dependencies]
//! temp-path-kit = { path = "../temp-path-kit" }
//! ```
//!
//! 本 crate の依存は **0**（標準ライブラリのみ）なので、引いても消費側に何も足さない。
//!
//! # 使い方
//!
//! 窓口が配るのは**ディレクトリ 1 つ**である。単一ファイルが要るときも別の型は使わず、
//! 配られたディレクトリの下へ [`TempPath::child`] で置く（宛先の種類を増やさない）。
//!
//! 引数の札は「どのテスト群の一時パスか」が失敗時のディレクトリ名から分かるようにするための
//! ものである。`-` と英数字だけの短い識別子にすること。
//!
//! ```rust
//! use temp_path_kit::TempPath;
//!
//! let dir = TempPath::new("demo-ghost");
//!
//! // ディレクトリは `new` の時点で実在する。
//! assert!(dir.path().is_dir());
//!
//! // 単一ファイルの宛先はその下に取る。
//! let descript = dir.child("descript.txt");
//! std::fs::write(&descript, "charset,UTF-8\n").expect("一時ディレクトリへ書けるはず");
//! assert!(descript.is_file());
//!
//! // 破棄で中身ごと消える（後始末を呼び忘れる余地が無い）。
//! let kept = dir.path().to_path_buf();
//! drop(dir);
//! assert!(!kept.exists());
//! ```
//!
//! 値を持ち回る間だけ実体が生きるので、**束縛を `_` で捨ててはならない**
//! （`let _ = TempPath::new(..)` はその場で破棄される）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// 同一プロセス内で単調増加する連番。プロセス識別子と組にして名前を一意にする。
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(0);

/// 一時パスの名前を組み立てる唯一の式。
///
/// プロセス識別子と連番を**引数で受ける**ので、別プロセスを起こさずに
/// 「識別子が違えば別名になる」を決定論的に確かめられる（要件 12.1 の自己テスト）。
/// 実際に作る側（[`TempPath::new`]）もこの関数を通る。
fn compose_name(label: &str, process_id: u32, serial: u32) -> String {
    format!("areka-{label}-{process_id}-{serial}")
}

/// 破棄時に自身を再帰削除する、プロセス間で一意な一時ディレクトリ。
pub struct TempPath {
    path: PathBuf,
}

impl TempPath {
    /// 一時ディレクトリを作って配る。
    ///
    /// 名前は `areka-{札}-{プロセス識別子}-{連番}`。`label` は失敗時の見分けが付く
    /// 短い識別子にすること。
    ///
    /// # Panics
    ///
    /// ディレクトリを作れないとき（黙って縮退しない）。
    pub fn new(label: &str) -> Self {
        let serial = NEXT_SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(compose_name(label, std::process::id(), serial));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("一時ディレクトリ作成: {} ({err})", path.display()));
        TempPath { path }
    }

    /// 配られたディレクトリそのもの。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 配られたディレクトリの下の宛先を 1 つ取る（単一ファイルはここへ置く）。
    ///
    /// 返すのはパスだけで、実体は作らない。
    pub fn child(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
