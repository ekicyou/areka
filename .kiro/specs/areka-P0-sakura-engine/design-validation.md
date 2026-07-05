# 設計検証レポート: areka-P0-sakura-engine

> 対象: 確定済み `requirements.md`（R1〜R11）＋ `design.md`（DD-1..8）。
> 検証種別: 実装前 設計品質レビュー（非対話・GO/NO-GO 判定）。
> 検証日: 2026-07-05 ／ 検証者: kiro-validate-design（subagent・非対話）
> 実シンボル照合: `areka_parsers::sakura::model`（`Instruction` 14 variant）・`areka-actor`（`spawn.rs` / `reply.rs`）を直読して散文主張を裏取り。

---

## 設計レビュー要約

三層構造（純粋展開 `expand` ／ 駆動 `playback` ／ 出力結線 `sink`）は brief の Boundary Candidates と要件（R2 決定性・R9 時刻注入・R7 単一 Close funnel・R6 reason 3 値）に忠実で、DD-1..8 の判断も研究材料に整合している。要件被覆は R1.1〜R11.4 全 ID が traceability とコンポーネントブロックに出現し、境界（暫定所在 DD-1・SurfaceArg 不透明・Duration 貫通）も明確。ただし **駆動層の対外インターフェース（`spawn_talk` の返り値と Close 配送経路）が実シンボル `spawn_actor` の返す `(Sender<M>, ActorHandle)` と齟齬**しており、R7（Close 即時中断）を成立させる結線が型として欠落している。これが唯一の実装前修正必須点である。

---

## Critical Issues（最大 3）

### 🔴 Critical Issue 1: `spawn_talk` の返り値が inbox `Sender` を捨てており、Close（R7）を届ける経路が型として存在しない

- **Concern**: `design.md` の駆動層 Service Interface は `pub fn spawn_talk(start, surface_sink, text_sink) -> areka_actor::ActorHandle;`（design.md「Service Interface（駆動）」）と定義するが、実シンボル `areka_actor::spawn_actor<M,F>(name, body) -> (Sender<M>, ActorHandle)`（`crates/areka-actor/src/spawn.rs`）は inbox 送信端 `Sender<M>` を必ず併せて返す。`SakuraMsg::Close` はこの `Sender<SakuraMsg>` へ送って初めてアクター body の受信ループへ届く。`spawn_talk` が `ActorHandle` のみを返すと、kanade は Close を投函する送信端を得られず、R7.1（即時停止）・R7.4（`Interrupted` ACK）を駆動する結線が存在しなくなる。
- **Impact**: 単一 Close funnel（R7・議題#1/#2 の確定契約）の中核が実装段階で破綻する。`ActorHandle` は非 RAII の join ハンドルにすぎず（`spawn.rs`・`is_finished`/`join` のみ）、中断入力の搬送能力を持たない。
- **Suggestion**: `spawn_talk` の返り値を `(areka_actor::Sender<SakuraMsg>, areka_actor::ActorHandle)` 相当（もしくは両者を包む `TalkHandle{ inbox: Sender<SakuraMsg>, actor: ActorHandle }`）へ改める。`StartTalk` を inbox 経由 `SakuraMsg::Start` として送る設計と、`spawn_talk(start, ...)` が `start` を引数で直接消費する設計が混在しているため、どちらか一方（推奨: spawn 時に `Sender` を返し `Start`/`Close` を共に inbox で送る一貫形）へ統一すること。
- **Traceability**: R7.1, R7.3, R7.4 ／ Adjacent「単一 Close funnel」。
- **Evidence**: design.md「再生駆動層（playback）> Service Interface（駆動）」の `spawn_talk` シグネチャ、および `SakuraMsg::{Start, Close}`（contract）と `spawn_actor` 実シグネチャの不一致。

### 🔴 Critical Issue 2: 注入時刻（R9.1）と Close 即時割込み（R7.1）を単一 body スレッドでどう両立させるかが未規定

- **Concern**: 駆動は「注入式 tick で `elapsed` を進める」（R9.1）一方、body は inbox `Receiver<SakuraMsg>` を単独所有し Close を `run_inbox` の `Break` へ写像する（design.md「State Management」「Implementation Notes」）。しかし `run_inbox` は `rx.recv()` でブロックする受信ループ（`spawn.rs`）であり、「時刻注入（tick）で発火を進めながら、同時に Close を待つ」二重待機の具体機構（テスト時は注入列・本番は `recv_timeout` 刻み）が設計に明示されていない。テストが `run_inbox` を使うのか、`recv_timeout` ベースの自前ループ（`spawn.rs` docstring が言及する「周期 tick 等で自前ループ」）を使うのかで、Close の即時性と決定的観測の両立可否が変わる。
- **Impact**: R7.1（即時停止）と R9.1/9.4（sleep 非依存・決定的再現）の交点が実装者裁量に委ねられ、Close がタイムライン発火の合間でしか効かない／注入時刻が recv ブロックと干渉する等の非決定性を招く恐れがある。単体テスト主戦場は純粋 `expand` に閉じるため守られるが、playback 統合テスト（Close 中断）の決定性が担保されない。
- **Suggestion**: 駆動ループの正準形を 1 つ明記する。推奨は「テストは `run_inbox` を用いず、注入時刻列と inbox を交互に消費する自前ループ（`recv_timeout(0)` 相当で Close を非ブロック確認 → 未発火 `TimedFire` を `elapsed` まで flush）」を design で固定し、R7 の Close 検査点（各 tick 境界で必ず Close を先に見る）を Postcondition 化すること。
- **Traceability**: R7.1, R9.1, R9.4 ／ R10（body ローカル状態）。
- **Evidence**: design.md「State Management > Concurrency」「Implementation Notes（時刻注入は駆動ループが tick を消費する形）」と `areka-actor/src/spawn.rs` の `run_inbox`（`rx.recv()` ブロック）の突合。

### 🔴 Critical Issue 3: `StartTalk.reply` の consume による「高々 1 回」保証と、R7.5「既終端なら返さない」の型的成立条件が曖昧

- **Concern**: `TalkDone` の通算高々 1 回は `ReplySender::send(self)` の consume（`reply.rs`）で型強制される、と design は述べる（DD-7）。これは正しいが、design の Risks は「終端済みフラグ or `ReplySender` consume 済みで型的に防ぐ」と両論併記しており、`ReplySender` が `StartTalk` の一フィールドで body ローカルに move される以上、**consume 済みか否かを実行時に再判定する術は無い**（move 後は変数が使えない＝コンパイル時保証）。「終端済みフラグ」を併用すると、フラグと consume の二重管理が R7.5 の唯一結果性を却って曖昧化する。
- **Impact**: R6.4/R7.4/R7.5 の「自然終端後の Close は追加返信しない」を、型保証（consume）で閉じるのか実行時フラグで閉じるのかが未確定なまま実装へ渡ると、二重返信防止のロジックが冗長化・自己矛盾するリスクがある（軽微だが契約中核）。
- **Suggestion**: 「`ReplySender` の move-consume を唯一の高々1回機構とし、終端済みフラグは持たない」と design で一本化する。Close 受領時に `reply` が既に consume 済み（自然終端後）なら、body はそもそも `reply` を保持していない＝返信しようがない、という不変条件を Postcondition に明記すること。
- **Traceability**: R6.4, R7.4, R7.5。
- **Evidence**: design.md「再生駆動層 > Implementation Notes > Risks（終端済みフラグ or ReplySender consume 済み）」と `areka-actor/src/reply.rs`（`send(self)` consume）。

---

## 設計の強み（1〜2）

- **純粋展開層の切り出しが決定性を型で守る**: `expand(&[Instruction]) -> Timeline` を clock/sink/talk_id/アクター非依存の純粋関数に閉じ込め、R9.4（同一入力→同一観測）を単体テスト主戦場として確保した設計は、実シンボル（`Instruction` は `Clone/Debug/PartialEq` のみ・`Eq/Hash` 無し）とも整合し、`NewLineRatio(f32)` ゆえ `PartialEq` に留める判断まで正確（design.md line 364）。DD-2（Duration 貫通・f64 換算回避）と併せ、上流 `Wait(Duration)`（50ms 換算済み）を再計算しない R2.3 を素直に満たす。
- **DD-1 の暫定所在＋移譲シームが下流 import を守る**: `StartTalk`/`TalkDone` を `areka_sakura::contract` が暫定所有し、kanade 完成時に re-export へ差し替えて import パスを不変に保つ設計は、kanade 未実装という研究の最大未決事項（research §3.5）へ現実的なコンパイル可能解を与え、Revalidation Triggers にも移譲トリガを明記している。

---

## 最終判定

### 判定: NO-GO（軽微・1 箇所の結線修正で GO へ転じ得る）

### 根拠
要件被覆・アーキテクチャ健全性・境界・決定性はいずれも高水準だが、**Critical Issue 1（`spawn_talk` が Close 配送用 `Sender` を型として捨てている）は R7 単一 Close funnel の実装可能性を直接損なう対外インターフェース欠落**であり、実装前に必ず是正すべき。Issue 2/3 は駆動ループ正準形と二重返信防止機構の一本化で、いずれも設計文言レベルの明確化で解消できる。

### Next Steps
1. `kiro-design-discussion` で Issue 1〜3 を論点として解決する（特に Issue 1: `spawn_talk` 返り値へ inbox `Sender<SakuraMsg>` を含める／`Start` と `Close` の投函経路を一貫させる）。
2. 解決を design.md へ反映後、`/kiro-spec-tasks areka-P0-sakura-engine` でタスク生成へ進む。
3. 純粋 `expand` 層は現状のまま実装着手可（決定性・型契約とも問題なし）。
