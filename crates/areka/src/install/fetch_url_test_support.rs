//! 本体のテストで使い回す偽の取得口（design「Testing Strategy / URL の取得」）。
//!
//! エンジンの偽の取得口（`areka-update` の `testkit`）はテスト専用で本体から使えないので、
//! 本体の側に 1 つ置く。`\![execute,install,url]` の取得（4.1）と更新の手続き（6.1・6.6）が
//! これを差す。ネットワークには触れない。

use std::collections::BTreeMap;

use areka_update::{Fetch, FetchError};

/// URL → 本文または失敗の固定表。表に無い URL は `NotFound`（「無い」）。
#[derive(Debug, Clone, Default)]
pub(crate) struct FakeFetch {
    table: BTreeMap<String, Result<Vec<u8>, FetchError>>,
}

impl FakeFetch {
    pub(crate) fn new() -> FakeFetch {
        FakeFetch::default()
    }

    /// `url` に本文を返させる。
    pub(crate) fn serve(mut self, url: &str, bytes: &[u8]) -> FakeFetch {
        self.table.insert(url.to_owned(), Ok(bytes.to_vec()));
        self
    }

    /// `url` に失敗を返させる。
    pub(crate) fn fail(mut self, url: &str, err: FetchError) -> FakeFetch {
        self.table.insert(url.to_owned(), Err(err));
        self
    }
}

impl Fetch for FakeFetch {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.table
            .get(url)
            .cloned()
            .unwrap_or(Err(FetchError::NotFound))
    }
}
