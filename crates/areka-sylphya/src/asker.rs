//! 問い合わせ元コンテキスト第一級: `AskerContext` / `AskerId`。
//!
//! 読み口 API は問い合わせ元（どの SHIORI／ゴーストからの照会か）を第一級で受け取る（R2.6）。
//! M1 は単一ゴースト運行のため実挙動は単一だが、鏡像モデル・API 形とも問い合わせ元コンテキストを
//! 第一級で保持する（design §「鏡像＋SylphyaReader」Service Interface）。

/// 問い合わせ元 ID（R2.6・第一級）。ghost は `MountModel.shiori.dir` 由来の正準文字列で構築する。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AskerId(String);

impl AskerId {
    /// 正準文字列から構築する。
    pub fn new(id: impl Into<String>) -> Self {
        AskerId(id.into())
    }

    /// 正準文字列を借用で返す。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 問い合わせ元コンテキスト（R2.6・第一級）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskerContext {
    pub asker: AskerId,
}
