//! 記憶の sqlite の住み場所（置き場の口と、置き場から開く先への変換）。

mod backend;
mod locate;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the store core that calls these lands with the layered read"
    )
)]
mod schema;

pub use backend::{
    Access, FsBackend, Located, MemBackend, StoreBackend, UNUSABLE_APPLICATION_ID, WriterHold,
    fs_backend,
};
