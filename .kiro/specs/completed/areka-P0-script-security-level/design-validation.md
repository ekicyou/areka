# 設計の検証: areka-P0-script-security-level

- 検証日: 2026-10-22
- 対象: `design.md`（`requirements.md` 確定版に対して）
- 判定: **GO**（下の 2 点を設計の討議かタスクの生成で詰める）

## 設計の総評

出どころの語彙を `areka-talk` に 1 つ置き、要求のヘッダ・応答のレベルの読み・台本の印・タグの止めを、すべて同じ値から導く筋の通った設計である。手で確かめた設計の主張（`EventId` の 3 つの出所カテゴリ、`ShioriBackend` の実装の数、`begin_talk` の呼び手、運行表の出口の規則 `capture`、`build_request` の今の並び、触るファイルの行数）は、コードとほぼ一致した。実装へ進めてよい。残る心配は 2 つで、どちらも設計の作り直しではなく、テストの組み方を書き足せば済む。

## コードで確かめたこと

| 設計の主張 | コードで見たこと | 判定 |
|---|---|---|
| `EventId` は `Static`・`Choice`・`Sstp` の 3 つで、`match` に既定の腕が無い | `msg.rs` 316 行目。`match` は `msg.rs`（`as_str`）と `actor.rs`（506・660・709 行目）だけ。`areka` は `Choice` を作るだけ | 一致 |
| MCP の `raise_event` は `EventId::Sstp`、UI などの「汎用の入口」は `EventId::Static` | `mcp/raise_event.rs` は `KanadeMsg::SstpNotify`、`RaiseEvent` を送るのは更新・インストール・ファイルドロップ・メニュー・切替だけ | 一致（ゴースト自身の扱いで正しい） |
| `ShioriBackend` の実装は 21 か所（本物 2・`EmptyShiori`・残りはテストの偽物） | 22 か所（`recorder.rs` の `impl<B> ShioriBackend for Recorder<B>` を入れて）。製品の実装は `ShioriConnection`・`InProcBackend`・`EmptyShiori` の 3 つだけ | ほぼ一致（数え漏れ 1） |
| `begin_talk` の追随は 6 か所 | 呼び手は `dispatcher.rs` 340 行目とテスト 3 か所、上書きは `talk_lifecycle.rs` とテスト 1 か所 | 一致 |
| `ShioriOutcome::Value(` の追随は 133 か所 | 42 ファイルで 133 か所 | 一致 |
| 運行表の出口で 1 回だけ付けられる（`before`→`route`→`after`） | `schedule/mod.rs` の `step`（133〜147 行目）。`capture` は「一括の再生の開始はその応答の台詞」を前提に、最初の `StartTalk` を預ける | 一致 |
| `capture` の「`OnTranslate` の応答を再び翻訳しない」は `EventId::Translate` でも効く | 判定は `source.id.as_str() == "OnTranslate"` の文字列比較なので、新しいカテゴリでも同じに効く | 一致 |
| `SecurityLevel: local` は今 `Reference*` の後ろに固定 | `shiori3.rs` 137 行目 | 一致 |
| 止めは受け口 1 か所で足りる | kanade の `no_user_break` は UI の旗の写しで、`\![enter,nouserbreakmode]` を自分では読まない | 一致 |
| `ConsumerLedger::consumer_of("execute", Some("filewatch"))` は担当なしを返す | 名簿の `execute` は `install`・`resetwindowpos` の選別子つきだけで、名前だけの登記が無い | 一致（`enforced: false` と食い違わない） |
| 触るファイルは 1,000 行以内に収まる | 大きいものは `steady.rs` 957・`events.rs` 886・`actor.rs` 837・`shiori3.rs` 821・`client.rs` 775。どれも数十行の増えで収まる。設計の表に無い `spine.rs` は 940 行 | 収まる（ただし下の心配 2） |
| 通しのテストの組み方（`runtime_boot_tests.rs`）は GPU を要さない | 窓も GPU も使わない。要るのは COM の初期化と、リポジトリが持つ検体 `emo2` の使い捨ての複製（`target` の下）。`#[ignore]` は無く、普段の全体テストで走っている | 一致 |
| 依存の向きは変わらない | `areka-kanade`・`areka-ghost`・`areka-sakura` は既に `areka-talk` に依存。`areka` は直接は依存しないが、`areka-sakura` の再輸出で受ける設計 | 一致 |

## 大事な心配（2 件）

### 心配 1: 「実装済みなのに止めが無い」を、旗だけで確かめている

- **何が問題か**: 要件 4.7 の照合は「名簿に担当が在る」と「表の `enforced` が真」が一致するかだけを見る。後の spec が `\![execute,filewatch]` を実装して赤を見たとき、表の `enforced` を真にするだけで緑に戻り、受け口が `permits` を呼んでいなくても通ってしまう。
- **なぜ困るか**: 要件 4.7 が守りたいのは「実装済みのタグが制限を受けずに実行される状態」で赤になることだ。今の照合では、外部扱いの台本の止め忘れが黙って通る道が残る。設計自身も「運ぶ道のどこかで既定の値に落ちても黙って通る」を心配に挙げているが、手当ては `\![enter,nouserbreakmode]` の通しのテスト 1 本だけである。
- **どう直すか**: `script_restriction_tests.rs` の照合に、振る舞いの確かめを足す。`enforced` が真の各行について、名簿が返す担当（`CommandConsumer`）の受け口を作り、`begin_talk` で外部扱いの出どころを渡してその行のタグを流し、受け口の線にそのタグの合図が出ないことを確かめる。担当から受け口を作る `match` に既定の腕を置かなければ、新しい担当を足した spec はコンパイラに確かめの手を求められる。名簿のファイル（`consumer_ledger.rs`）は読むだけで済み、C10 の約束にも触れない（受け口を本物の線で動かす先例は `consumer_ledger_agreement_tests.rs`）。
- **要件**: 4.7（4.2・4.6 も）
- **設計の箇所**: 「Testing Strategy」の統合テストの `script_restriction_tests.rs` の照合、「判定の表」

### 心配 2: 既定の実装つきの新しいメソッドが、中継の型で黙ってヘッダと応答のレベルを落とす。通しのテストの偽の SHIORI も同じ

- **何が問題か**: `get_with_headers`・`notify_with_headers` の既定の実装は、ヘッダを捨てて `get`・`notify` を呼び、応答のレベルを「無し」で返す。中身へ素通しする中継の型（`areka-ghost` のテストの `BoxedBackend` 3 つと `Recorder<B>`）は `get`・`notify` しか素通ししないので、本物の `InProcBackend` を包んでも、新しいヘッダも応答の `SecurityLevel` も届かない。本物の pasta を相手にするテスト（`real_pasta_online.rs`・`inproc_e2e_test.rs`）が、新しい線を確かめないまま緑になる。また、通しのテスト（`script_security_e2e_tests.rs`）が「`runtime_boot_tests.rs` と同じ組み方」で使う `ScriptedShioriBackend`（`spine.rs`）も新しいメソッドを上書きしないので、応答に `external` を付けられない。設計はその手を書いておらず、`spine.rs` は変えるファイルの表に無い。
- **なぜ困るか**: 要件 3.1・4.2 の通しのテストが、組み方の段で詰まる。`spine.rs`（940 行）に手を足すと 1,000 行に近づく。中継の型が黙って値を落とす形は、「ログの無い失敗の経路を作らない」の決まりの外で、要件 2.8 の「2 つの経路で同じヘッダ」を本物の SHIORI の段で確かめる手が無くなる。
- **どう直すか**: ⑴ 中継の型（`BoxedBackend` 3 つと `Recorder<B>`）は新しい 2 メソッドも素通しする、を追随の表に書き足す。⑵ 通しのテストは、`ScriptedShioriBackend` を包んで `get_with_headers` だけを上書きし、`OnBoot` の応答に `security_level: Some("external")` を付ける小さな偽物をテストのファイルの中に置く、と書く（`spine.rs` は触らない）。⑶ `ShioriBackend` の実装の数を 22 に直す。
- **要件**: 2.1・2.8・3.1・4.2
- **設計の箇所**: 「ShioriBackend の新しいメソッド」、「Modified Files」の機械的な追随、「Testing Strategy」の E2E

## 設計のよいところ

1. **出どころを 1 つの値から導く形**: 要求の 4 つのヘッダ・台本の実効のレベル・止めの判断が、すべて `ScriptOrigin` と `EventId` の出所カテゴリから導かれる。`EventId` の `match` に既定の腕を置かないので、SSTP の受信など下流の spec がカテゴリを足した時点で、コンパイラが写しの決め直しを求める。`ShioriCall`・`ShioriMsg` の形を変えず、腕（`boot.rs`・`change.rs`・`close.rs`・`steady.rs`）にも触らずに、運行表の出口 1 か所で付ける点も、既存の `capture` の前提と揃っていて無理が無い。
2. **「下げるだけで上げない」を型で守る**: `lowered_by` だけでレベルを動かし、応答・翻訳の応答・選択肢の引き継ぎのどこでも上がらない。構造上起きない異常では止める側（外部扱い）へ倒し、`error!` を残す。安全の向きがそろっている。

## 最終の判定

- **判定: GO**
- **理由**: 既存の層の分け方（語彙は `areka-talk`、規則は kanade、線の書き読みは host32、止めは受け口）と食い違わず、要件 1〜5 がすべて部品とテストへたどれる。2 つの心配は、どちらもテストの組み方と追随の表を書き足せば済み、設計の形は変わらない。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion areka-P0-script-security-level`）で心配 1・2 を議題にし、設計に書き足すかタスクの生成に申し送るかを決める。
  2. そのあと `/kiro-spec-tasks areka-P0-script-security-level` でタスクを作る。規模の見立て 17 に、心配 2 の中継の型の追随（数か所）が加わっても 20 を超えない見込み。
