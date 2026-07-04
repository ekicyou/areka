//! 純粋層: request/reply（oneshot 相当）の器。
//!
//! `reply_channel`・`ReplySender`・`ReplyReceiver`・`ReplyError` は後続タスクで
//! per-request な `std::sync::mpsc::channel()` を薄い newtype で包んで実装される。
