//! talk 起動契約の正本型（TalkId / StartTalk / TalkDone）。
//!
//! 本モジュールは kanade（奏でる）と sakura（さくらスクリプト再生）の間で交わす
//! talk 起動契約の唯一の定義であり、sakura-engine 側で再定義してはならない。
//!
//! # 依存規律（DD-1）
//! 本ファイルは **`std` のみ**に依存する。host32 型・areka-actor 型を一切 import
//! しない。これにより、2 例目の契約消費者（sakura-engine 実着手）が現れた時点で
//! 契約クレートへ切り出す作業は、このファイルの**機械的な移動だけ**で完結する。
//!
//! # script の不透明性（Req 2.4）
//! `StartTalk::script` は不透明な文字列として保持するのみで、kanade はその内容を
//! 一切解釈しない。`\-`（終了要求）の検出は sakura 側の責務であり、kanade は
//! `TalkDone::quit` フラグを消費するだけである。この非解釈性は「script は素の
//! `String`・解釈ロジックを持たない」という型設計そのもので保証される。

/// talk の一意識別子（kanade が単調増番で採番・再利用しない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TalkId(pub u64);

/// talk 起動要求（kanade → sakura）。script は不透明文字列（kanade は解釈しない）。
#[derive(Debug, Clone)]
pub struct StartTalk {
    pub talk_id: TalkId,
    pub script: String,
}

/// 再生完了通知（sakura → kanade・`KanadeMsg::TalkDone` に包んで送る）。
/// `\-` の検出は sakura 側の責務であり、kanade は quit フラグを消費するのみ。
#[derive(Debug, Clone, Copy)]
pub struct TalkDone {
    pub talk_id: TalkId,
    pub quit: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn talk_id_equality_and_inequality() {
        assert_eq!(TalkId(1), TalkId(1));
        assert_ne!(TalkId(1), TalkId(2));
    }

    #[test]
    fn talk_id_is_copy() {
        // Copy であること: 束縛後も元の値が move されず使える。
        let a = TalkId(7);
        let b = a;
        assert_eq!(a, b);
        assert_eq!(a.0, 7);
    }

    #[test]
    fn talk_id_usable_in_hashset() {
        let mut set = HashSet::new();
        assert!(set.insert(TalkId(1)));
        assert!(set.insert(TalkId(2)));
        // 同一 id の再挿入は false（Hash + Eq が効いている）。
        assert!(!set.insert(TalkId(1)));
        assert!(set.contains(&TalkId(2)));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn start_talk_holds_opaque_script_and_id() {
        let msg = StartTalk {
            talk_id: TalkId(42),
            script: r"\0こんにちは\e".to_string(),
        };
        assert_eq!(msg.talk_id, TalkId(42));
        // script は素の String として保持されるのみ（解釈しない）。
        assert_eq!(msg.script, r"\0こんにちは\e");
    }

    #[test]
    fn start_talk_is_cloneable() {
        let original = StartTalk {
            talk_id: TalkId(3),
            script: "hi".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(cloned.talk_id, original.talk_id);
        assert_eq!(cloned.script, original.script);
    }

    #[test]
    fn talk_done_carries_id_and_quit_flag() {
        let done = TalkDone {
            talk_id: TalkId(9),
            quit: true,
        };
        assert_eq!(done.talk_id, TalkId(9));
        assert!(done.quit);

        let keep_running = TalkDone {
            talk_id: TalkId(9),
            quit: false,
        };
        assert!(!keep_running.quit);
    }

    #[test]
    fn talk_done_is_copy() {
        let done = TalkDone {
            talk_id: TalkId(1),
            quit: false,
        };
        let copied = done;
        // Copy ゆえ done も引き続き有効。
        assert_eq!(done.talk_id, copied.talk_id);
        assert_eq!(done.quit, copied.quit);
    }
}
