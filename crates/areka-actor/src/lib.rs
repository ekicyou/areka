//! areka-actor: エンジン非依存なアクター規約と薄いヘルパ群（規約正本）。
//!
//! 本クレートは areka M1 並行モデルの「機構」——全エンジン（kanade/sakura/seriko/emo 等）が
//! 共有するアクター通信の**原語**——を提供する。特定エンジンの知識を持たず、経路（kanade）や
//! 結線（ghost / ghost-setup）は下流ユニットの領分である。提供物は「規約＋薄いヘルパ＋UI 配送
//! ブリッジ」までに限定し、共通トレイトによる過剰抽象（actor framework 化）は行わない。抽象は
//! 2 例目の実物が要求するまで作らない（Req 7）。
//!
//! 構造は純粋層（[`spawn`]／[`reply`]＝std のみ・wintf/executor 非依存）と UI ブリッジ層
//! （[`ui`]）をモジュール境界で分離する。本モジュール（`lib.rs`）は規約の正本（rustdoc）と
//! 公開面の re-export のみを担い、実装ロジックを持たない。
//!
//! 以下 5 つの規約は全消費者を拘束する規範文である。各消費者は本規約の上に自分のメッセージ型を
//! 定義する。
//!
//! # inbox 規約
//!
//! （Req 1.5 / 2.1）
//!
//! アクター 1 個につき受信端（inbox）は単一の [`std::sync::mpsc::Receiver<XxxMsg>`] を 1 本のみ
//! とする。受信端はアクター本体スレッドが単独所有し（複数箇所から受信しない）、この構造は spawn
//! 原語が `Receiver` を body へ move することで保証される。
//!
//! アクター宛メッセージは、そのアクターごとの enum 型で表す。型は `XxxMsg` の命名規約に従う
//! （例: `KanadeMsg`・`EmoMsg`）。1 つの enum が当該アクターの受理可能な全メッセージを網羅する。
//!
//! # envelope 規約
//!
//! （Req 2.2 / 2.4 / 2.5）
//!
//! request/reply が要る variant は、返信端 `ReplySender<T>` をフィールドに同梱する（oneshot
//! 相当・応答は 1 回）。受信側はその `ReplySender` へ 1 度だけ応答を送り、送信側は対応する
//! `ReplyReceiver` で受け取る。返信端の生成は [`reply`] 層の `reply_channel` が担う。
//!
//! アクター間を渡るメッセージは `Send + 'static` な**所有データ**とし、借用（参照）を跨がせない。
//! この制約は spawn 原語の型境界（`M: Send + 'static`）で強制される。
//!
//! 大型データ（画素バッファ等）を含める場合はコピーせず、`Arc<T>` もしくは `Arc<[u8]>` 等の
//! フィールドで手渡す（共有所有権の移譲）。値のディープコピーを channel に載せない。
//!
//! # 停止規約
//!
//! （Req 3.1 / 3.3 / 3.7）
//!
//! 各 `XxxMsg` enum には横断制御としての停止メッセージ（Close 相当）を必ず含める。Close は
//! **即時停止**を意味する: 受信ループは Close を受領した時点で直ちに抜け、inbox に残る積み残し
//! メッセージは drain せず破棄する。積み残しを処理し切ってから止めたい送信側は、後続メッセージが
//! 無いことを確認したうえで Close を送る運用で対応する（graceful 停止はこの原語の上に送信側が
//! 構築する）。
//!
//! 積み残しメッセージが drop されると、それに同梱された `ReplySender` も drop され、対応する要求
//! 側の応答受信は切断（`Err`）として観測される（永久ブロックしない）。要求側は応答受信が `Err`
//! を返し得ることを許容する。
//!
//! 受信ループは**個別メッセージ処理の失敗（handler の `Err`）では終了しない**。基盤は
//! `tracing::error!` でエラーを記録したうえで受信を継続する。受信ループの正常な終了経路は
//! 「Close 受領」と「全 Sender drop（inbox 切断）」の 2 経路のみである。panic はエラー処理では
//! なくバグの観測として join 時検出に委ねる（黙って握り潰さない）。自前の受信ループを書く場合も
//! 同規約に従う。
//!
//! # 流量規約
//!
//! （Req 5.1 / 5.2）
//!
//! 制御メッセージ経路は unbounded とする（低レート前提・backpressure を持たない）。毎フレーム
//! 大量に発生するデータ（画素バッファ等）は channel に直接流さず、共有バッファ／`Arc` 手渡しで
//! 受け渡す（envelope 規約の大型データ手渡しと同旨）。channel は指令の搬送路であってデータ洪水の
//! 経路ではない。
//!
//! # 拡張シーム
//!
//! （Req 5.3）
//!
//! select／MPMC／有界キューは、実需（2 例目の実物）が現れるまで**凍結**する。本ユニットでは
//! 導入しない。将来これらが必要になった場合、crossbeam-channel 等の新規依存追加は**開発者承認の
//! 上で**行い、`ReplySender` 等の newtype の**内部実装差し替え**として導入する（公開面は据え置く）。
//! 監督ツリー・再起動戦略も同様に 2 例目駆動で M2 以降へ見送る。
//!
//! # 公開面（re-export）
//!
//! 本クレートの最終的な公開面は以下に限定される（これ以外の公開面を持たない・Req 7.1）。各シンボルは
//! 後続タスクで実装され次第、本モジュールから re-export される。
//!
//! - 純粋層 [`spawn`]: `spawn_actor`・`run_inbox`・`ActorHandle`・`ActorError`
//! - 純粋層 [`reply`]: `reply_channel`・`ReplySender`・`ReplyReceiver`・`ReplyError`
//! - UI ブリッジ層 [`ui`]: `spawn_ui`・`UiSender`・`UiSendError`・`UiSpawnError`

pub mod reply;
pub mod spawn;
pub mod ui;

pub use reply::{ReplyError, ReplyReceiver, ReplySender, reply_channel};
pub use spawn::{ActorError, ActorHandle, run_inbox, spawn_actor};
pub use ui::{UiSendError, UiSender, UiSpawnError, spawn_ui};
