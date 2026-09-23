//! ネットワーク更新の本番クレート（spec: areka-P0-update-engine）。
//!
//! 更新定義ファイル（`updates2.dau`／`updates.txt`）を読み、手元との差分だけを
//! 取得・照合して、ファイル単位で確定する。失敗したら逆順に戻す。

mod diff;
mod error;
mod fetch;
mod manifest;
mod md5;
mod outcome;
mod paths;
#[cfg(test)]
mod testkit;
mod urlpath;

pub use error::{
    DeleteWhy, FailReason, FetchError, InvalidWhy, Stage, Stuck, UpdateError, UpdateWarning,
};
pub use fetch::Fetch;
pub use outcome::{ManifestName, Progress, Undeletable, UpdateOutcome};
