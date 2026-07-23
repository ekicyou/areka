//! 永続層: `PersistScope` / `ScopeRoots` / `PersistKey`（4 key 族 typed）/ 載せ替え orchestration。

pub mod format;
pub mod io;

pub use io::{FakePersistIo, FsPersistIo, PersistIo};
