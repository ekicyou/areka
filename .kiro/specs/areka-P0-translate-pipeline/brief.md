# Brief: areka-P0-translate-pipeline

> 起票: 2026-09-02（`/kiro-discovery`・Path D の 1 本目・**M 規模**）。開発者要望「ukadoc を確認し MAKOTO/2.0 対応を実装する spec を立ち上げよ。ブリーフィング段階で要件議論は大体済ませておきたい」を受け、翻訳（トランスレート）経路を 2 spec に分けた **前半**。後半は [areka-P0-makoto-dll-host](../areka-P0-makoto-dll-host/brief.md)（MAKOTO/2.0 DLL のホスティング）。
> **種別**: 互換機能の新設（SHIORI が返した台詞を表示前に加工する「翻訳」の継ぎ目と、SHIORI イベント `OnTranslate`）。emo2 は使わない＝**M2 ゲート扱い**（e2e をブロックしない）。
> **ブリーフィング段階の裁定（2026-09-02・開発者）**: ⑴ OnTranslate を範囲に含める ⑵ 2 spec に分割（本 spec → makoto-dll-host） ⑶ 環境変数の展開は正典順（展開してから翻訳）へ寄せる ⑷ 実機は本物の MAKOTO DLL 1 本＋自前テスト DLL（後半 spec） ⑸ 任意 charset は常に欲しい（後半 spec で SHIORI 側 wire にも適用） ⑹ シェル側 MAKOTO も後半 spec に含める ⑺ spec 名は本名で確定。

## Problem

ukadoc の正典では、SHIORI が返した台詞（さくらスクリプト）は **バルーンに表示される直前に「翻訳」を通る**（[トランスレータ](https://ssp.shillest.net/ukadoc/manual/manual_translator.html)）。順序は **SHIORI イベント `OnTranslate` → ゴースト側 MAKOTO → シェル側 MAKOTO**。`OnTranslate` は「SHIORI がゴースト側 MAKOTO を兼ねる」仕組みで、Reference0 に**ベースウェアが環境変数を展開した後の台詞**が入り、SHIORI が返した台詞が最終の台詞になる（[OnTranslate](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnTranslate:1)・自分自身では再発火しない）。

里々／YAYA の標準辞書はこの `OnTranslate` で敬称の重なり（「さんさん」）の除去・語尾の変化・自動ウェイト挿入などを行う（YAYA wiki「OnTranslate の使い方」・里々 wiki「OnTranslate」）。areka は今日 `OnTranslate` を一度も送らず、翻訳の継ぎ目も無い。**利用者から見える結果**: 既存ゴーストを入れると、作者が `OnTranslate` で直しているはずの台詞（「ユーザーさんさん」・語尾違い）が**正常な顔で間違ったまま表示される**＝toolkit 規則 6 の壊れ方で最上位。emo2（pasta）は `OnTranslate` を持たないので M1 適合には無害。

さらに順序の問題がある。正典は「展開してから翻訳」だが、areka の `%username` 等の展開は再生側の compile（[compile.rs:181](../../../crates/areka-sakura/src/compile.rs:181)）で行われ、翻訳を置ける位置より**後**にある。このままでは翻訳側が `太郎さん` ではなく `%usernameさん` を見る。

## Current State（2026-09-02 実測・着手時に再検証）

- **SHIORI 応答が台詞になる場所は 7 か所**、すべて kanade の純粋な状態機械（`schedule` 層）にある: `boot.rs:241`（OnFirstBoot/OnBoot の挨拶）・`boot.rs:260`（204＋epilogue のみ）・`close.rs:70`（OnClose の別れ）・`steady.rs:776`（毎秒ポンプの Value）・`steady.rs:814`（マウス起源の Value）・`steady.rs:407`（選択肢連鎖の応答）・`steady.rs:627`（OnChoiceTimeout の応答）。
- **7 か所は 1 つの出口に集まる**: `Action::StartTalk` → `send_talk_command`（[actor.rs:183](../../../crates/areka-kanade/src/actor.rs:183)・[actor.rs:366](../../../crates/areka-kanade/src/actor.rs:366)）。SSTP・communicate・プラグインは未実装なので、今日の再生経路はこの 1 か所で 100% 捕まえられる。
- **SHIORI へ出る唯一の実行点**は `round_trip_request`（[actor.rs:242](../../../crates/areka-kanade/src/actor.rs:242)）。送出許可表は 11 件（[events.rs:70](../../../crates/areka-kanade/src/schedule/events.rs:70)）で `OnTranslate` は含まれない。設計規律「**同時に進行中の SHIORI 往復は 1 つまで**」（[actor.rs:11](../../../crates/areka-kanade/src/actor.rs:11)）があり、Value を受けた直後にもう 1 往復（OnTranslate）を出すには**新しい相**（翻訳待ち）を状態機械に足す必要がある。
- **環境変数**: `%username` は字句解析で `Instruction::SystemVar` になり、compile 時に per-talk 凍結スナップショットで純粋展開される（[sysvar.rs:65](../../../crates/areka-sakura/src/sysvar.rs:65)・凍結点は [dispatcher.rs:301](../../../crates/areka-ghost/src/dispatcher.rs:301)）。文字列段階の前処理は存在しない。`%(...)` は未実装。M1 の対応名は `username` のみ（他は `%名前` のまま素通し）。
- **応答 Reference0 の再利用**: `ActiveTalk.script`（[schedule/mod.rs:129](../../../crates/areka-kanade/src/schedule/mod.rs:129)）が OnChoiceTimeout の Reference0 に使われる＝翻訳の前後どちらを記録するかを決める必要がある。
- `doc/shiori/fragments/events/28.other.toml:34-47` に `OnTranslate` の正典定義（Reference0〜3）は登記済み・Rust 側の消費者は 0。
- テスト: mock SHIORI 配線（`ShioriWiring::Custom`・[runtime.rs:557](../../../crates/areka-ghost/src/runtime.rs:557)）と kanade の状態機械テストが既存＝DLL なしで決定論テストが組める。

## Desired Outcome

1. **翻訳の継ぎ目が 1 か所**にある: SHIORI 応答（Value）を受けてから `StartTalk` を出すまでの間に `translate(台詞, 出所) -> 台詞` を通す。7 か所すべてが通る（1 か所でも素通りしたら赤になる変異テスト）。epilogue のみの talk（areka 内部生成）は翻訳しない。
2. **順序が正典と同じ**: 展開（`%username` 等）→ `OnTranslate` → 〔MAKOTO 鎖のフック＝本 spec では恒等・後半 spec が差し込む〕→ 再生。展開は kanade 側で翻訳の前に 1 回行い、その台詞を翻訳へ渡す。展開結果は compile 経由の展開と**同じ関数**（`resolve_system_var`）で得た同じ文字列（既知名の最長一致の文字列走査・未知名は `%名前` のまま素通し・**lexer には触れない**）。compile 側の `SystemVar` 腕は残す（展開済みなら到達しない＝恒等）。
3. **`OnTranslate` を GET で送る**: Reference0＝展開済みの台詞、Reference2＝元のイベント ID、Reference3＝元の Reference 群（バイト値 1 区切り）、Reference1 は areka に該当する出所（communicate／SSTP／plugin／notranslate）が無いので**欠番**。応答 200＋台詞 → それを最終台詞にする（空文字列も採用）。204／失敗／タイムアウト → **元の台詞をそのまま使う**（沈黙しない＝`warn!`／`error!` を残す）。`OnTranslate` の結果に対して `OnTranslate` を再発火しない。送出許可表に `OnTranslate` を追加。
4. **「同時 1 往復まで」の規律を守る**: 翻訳待ちの相を状態機械に足し、Value 受領 → OnTranslate 送出 → 応答 → StartTalk の順を Action バッチで表現する（例外扱いにしない）。
5. **記録は翻訳後**: `ActiveTalk.script`（OnChoiceTimeout の Reference0）は表示された台詞＝翻訳後を保持する（SSP は明記なし＝COMPAT §8 に登記）。
6. **決定論テスト**: ⑴ 7 か所の通過（変異＝継ぎ目を外すと赤） ⑵ 順序（展開→翻訳＝翻訳側 mock が `太郎さん` を受け取る） ⑶ 応答行列（200 置換／200 空／204／失敗／タイムアウト） ⑷ 再帰しない ⑸ Reference の内容と欠番 ⑹ ログ語彙。全て mock SHIORI と純粋関数で組む。
7. **挙動不変の証跡**: `OnTranslate` に 204 を返す SHIORI（pasta）では表示される台詞が bit 同一（emo2 の e2e／実機の台詞ログで確認）。1 talk あたり SHIORI 往復が 1 回増える（数 ms・毎秒ポンプは Value のときだけ）。
8. `cargo test --workspace` 緑（i686 helper 先ビルドが前提・記憶 workspace-test-needs-i686-host32-artifacts）。

## Approach

**継ぎ目を kanade の状態機械に置く**（純粋層で決定論・7 か所が 1 出口に集まる実測に基づく）。設計で 2 案から選ぶ:

- ⒜ **`Action::StartTalk` の手前に「翻訳フェーズ」を挟む**: 各 Value 受領地点で `Action::Translate{script, origin}` を積み、actor が展開→OnTranslate 往復→（MAKOTO フック）→ `StartTalk` を再投入する。状態機械には `Translating{next: 元の遷移}` の相を足す。7 か所の遷移を「翻訳後に続きを行う」形に書き換える。
- ⒝ **actor の `send_talk_command` 直前で同期的に翻訳する**: 状態機械は無改変で actor が OnTranslate を出す。ただし Action バッチの中で 2 往復目を出す＝「同時 1 往復」の規律に例外を作る（[actor.rs:11](../../../crates/areka-kanade/src/actor.rs:11) の唯一の例外 ForceQuit と同じ形）。

推奨は ⒜（記憶 canonical-not-minimal-lifecycle＝規律に例外を作らず正規の相で表す）。要件定義で「翻訳待ち中に来る入力（マウス・毎秒・Close）の扱い」を決める（推奨: Close は翻訳を打ち切って元台詞で進む／他は既存の待ち規則に従う）。

## Scope

- **In**: kanade の翻訳フェーズ（相・Action・msg）・展開の前倒し（文字列走査・名前表は `areka-sakura::sysvar` を公開して共有）・`OnTranslate` の送出（許可表・Reference 組立＝欠番対応）・MAKOTO 鎖の**フック 1 本**（`Box<dyn Translator>` 相当・本 spec は恒等実装のみ・語彙完備で後半 spec が差し込む）・決定論テスト・COMPAT §8 の裁量登記（Reference1 欠番／記録は翻訳後／失敗は元台詞）。
- **Out**: MAKOTO/2.0 DLL のロード・wire・charset・`\![load|unload|reload,makoto]`（→ `makoto-dll-host`）・SSTP／communicate／plugin 由来の Reference1・`%(...)` 展開・`OnTranslate` 以外の新イベント・`\![reload,shiori]`。

## Boundary Candidates

- **展開の前倒し**（S・純粋関数＋kanade 1 か所）と **翻訳フェーズ＋OnTranslate**（M・状態機械の相）で切れる。前者は単独 PR でも着地可（挙動不変＝compile 側と同じ文字列）。
- 後半 spec との境界＝`Translator` フック（台詞 in → 台詞 out・出所付き）。DLL・プロセス・charset は一切持ち込まない。

## Out of Boundary

- MAKOTO DLL のホスティングと付け外し命令（後半 spec）。
- SSTP・communicate・プラグインなど**新しい台詞の出所**を作らない。
- 翻訳結果の再パース以外の再生側改変（sakura／emo は無改変）。

## Upstream / Downstream

- **Upstream**: なし（現行 main で着手可）。kanade の状態機械（完了 spec `kanade`・`idle-talk`・`choice-select-events`・`sakura-dialogue-tags` の epilogue）が前提。
- **Downstream**: `makoto-dll-host`（フックに DLL 鎖を差し込む＝本 spec が rebase 源）・`ukadoc-survey-shiori`（`events.rs` の定義箇所へ ukadoc URL コメント＝後着 rebase）・将来の SSTP／communicate（Reference1 の出所語彙）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec `kanade` の設計規律「同時 1 往復」を相の追加で守る）。
- **Adjacent**: `property-query-channels`（`areka-kanade/schedule/*` を触る＝**共有あり・直列**）／`ukadoc-survey-shiori`（同ファイルへ doc コメント 1 行＝後着 rebase）／`emo2-conformance-e2e`（tests のみ＝共有 0）／`cursor-tag-canon`・`text-decoration-canon`（emo-text＝共有 0）／`ukadoc-survey-toolkit`（新規 crate＝共有 0）。
- **編集集合（2026-09-02 実測）**: `crates/areka-kanade/src/{actor.rs, msg.rs, schedule/{mod,boot,close,steady,events}.rs}`＋新規 `crates/areka-kanade/src/translate.rs`（兄弟テスト）＋`crates/areka-sakura/src/sysvar.rs`（名前表の公開のみ）＋`doc/COMPAT_ARCHITECTURE.md` §8。**`areka-parsers`・`areka-sakura/compile.rs`・`areka-ghost`・`consumer_ledger.rs` は非接触**。

## Constraints

- ログ無し失敗経路の禁止（翻訳失敗は `warn!`/`error!`＋元台詞）。
- 1 ファイル 1,000 行未満・兄弟テスト配置・`file_length_guard_test.rs` の例外表には触れない。
- 「同時 1 往復まで」の規律に例外を作らない（⒜ 推奨）。
- 実装済みの証拠（ukadoc URL コメント）は置かない＝`ukadoc-survey-shiori` の仕事。
- 正典の根拠: ukadoc [トランスレータ](https://ssp.shillest.net/ukadoc/manual/manual_translator.html)・[OnTranslate](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnTranslate:1)・YAYA wiki「Tips/OnTranslate の使い方」・里々 wiki「OnTranslate」。
