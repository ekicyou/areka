//! talk 起動契約の再エクスポート（正本は `areka-talk` クレート）。
//!
//! 本モジュールはかつて [`TalkId`] / [`StartTalk`] / [`TalkDone`] の物理定義を保持していたが、
//! 契約の単一定義化（Req 1.1・1.7）により、正本は `areka-talk` クレートへ移動した。
//! 本モジュールは `areka_kanade::talk::*` という既存の参照パスを維持するための再エクスポート
//! に徹し、二重定義は行わない。kanade（奏でる）と sakura（さくらスクリプト再生）はいずれも
//! `areka-talk` を直接の物理正本として参照する。
//!
//! 中断理由 3 値（[`TalkEndReason`]）の網羅テスト・型検証は `areka-talk` 側
//! （`crates/areka-talk/src/tests.rs`）に集約されている。本クレートは再エクスポートが
//! 既存の参照パスで機能することのみをスモークテストする。
pub use areka_talk::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// 再エクスポートが `areka_kanade::talk::*` 経由で機能し、型検証テスト本体は
    /// `areka-talk` 側が正本として保持することを確認する（Req 1.1・1.7）。
    #[test]
    fn reexported_types_are_usable_via_kanade_talk_path() {
        let start = StartTalk::new(TalkId(1), "hi");
        assert_eq!(start.talk_id, TalkId(1));

        let done = TalkDone {
            talk_id: TalkId(1),
            reason: TalkEndReason::Ended,
        };
        assert_eq!(done.reason, TalkEndReason::Ended);
    }
}
