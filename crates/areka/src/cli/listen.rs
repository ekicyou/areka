//! `--sstp-listen` の値を MCP の待受の候補にする（要件 8）。

/// MCP の待受の計画。
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum ListenPlan {
    /// `--sstp-listen` が渡されていない（今までどおり `AREKA_MCP_PORT` か既定・8.8）。
    #[default]
    NotGiven,
    /// `none` か値なし（8.4）。
    Off,
    /// 書いた順の番号（127.0.0.1 で試す・8.1〜8.3）。
    Ports(Vec<u16>),
    /// 待ち受け先はあったが全部はねた（8.7）。
    AllRejected,
}
