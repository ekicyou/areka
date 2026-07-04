//! toy(a) 統合試験のプレースホルダ。
//!
//! worker⇄worker request/reply・Close→join 決定的完走・積み残し破棄→reply Err・
//! 全 Sender drop 終了・panic join 観測は後続タスクで実装される。
