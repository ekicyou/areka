//! 記憶の sqlite の住み場所（置き場の口と、置き場から開く先への変換）。

mod backend;
mod locate;

pub use backend::{
    Access, FsBackend, Located, MemBackend, StoreBackend, UNUSABLE_APPLICATION_ID, WriterHold,
    fs_backend,
};
