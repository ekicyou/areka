# 選択関連イベント 互換対応表（カスケード則・Reference 割付・タイムアウト）

> 出所 spec: `areka-P0-choice-select-events`（Req2.8 / Req7.8 / Req8.1 / Req8.2・設計裁定 DD-8 / DD-14 / DD-15）
> 正本: 当該 spec の `design.md`「カスケード則の正典裁定」1〜8。**正典引用は同 spec の `research.md` §5（2026-07-31 に ukadoc MCP で実測した一次記述）から転記**しており、本書で新たな引用を創作していない。
> 位置づけ: `COMPAT_ARCHITECTURE.md` §2「沈黙ルール」（ukadoc が沈黙/曖昧な箇所は二次参照＋areka 裁量で決定し、判断を対応表に明記する）の運用実体のうち、**選択関連イベント領域の詳細台帳**。同書 §8 の横断表は要約・本書が詳細。
> 生成物ではなく手編集の記録文書である（`shiori/fragments/` 配下は生成物ゆえ手編集不可・本書はその外に置く）。

---

## 0. 読み方（provenance の 3 値）

各行には**出所を表す 1 値**を必ず付ける。語彙は生成正本 `shiori/fragments/events/14.choice.toml` の既存 2 値に、areka 裁量を表す第 3 値を足したものである（DD-14）。

| provenance | 意味 |
|---|---|
| `ukadoc` | 正典（ukadoc）に直接の記述があり、areka はその記述どおりに動作する |
| `ssp_secondary` | 正典本文では二次的／実装依存（SSP・CROW 等）と位置づけられる記述に由来する |
| `areka_discretion` | 正典が沈黙している、または正典記述間に字面の揺れがあるため areka が裁定した |

**列の意味**: `挙動`＝areka が実際に行うこと／`provenance`＝上記 3 値／`正典引用・根拠`＝一次記述の転記または実装アンカー／`反証・注記`＝反対に読める記述・非対称点・追跡先。

**複数の出所が混じる裁定は行を分割**して記録する（1 行 1 provenance を崩さないため。例: 裁定 5・7・8）。

---

## 1. Reference 割付（Req8.1・正典 layout）

| # | 裁定項目 | 挙動 | provenance | 正典引用・根拠 | 反証・注記 |
|---|---|---|---|---|---|
| R-1 | `OnChoiceSelectEx` の Reference | Reference0=表示ラベル／Reference1=選択肢 ID／Reference2 以降=付随参照列（`\q` の 3 番目以降）を記述順 | `ukadoc` | research §5-f 実測: 「`OnChoiceSelectEx`＝Ref0 ラベル／Ref1 ID／Ref2+ 拡張（`\q` 3 番目以降）・『OnChoiceSelect よりも先に開始』」 | 生成正本 `shiori/fragments/events/14.choice.toml`（`[entry."OnChoiceSelectEx"]`）は当該エントリの `provenance` を `ssp_secondary` と記録している。**割付そのものは §5-f の ukadoc 実測に一致**するため areka は `ukadoc` として扱い、粒度差のみここに注記する |
| R-2 | `OnChoiceSelect` の Reference | Reference0=選択肢 ID のみ | `ukadoc` | research §5-f 実測: 「`OnChoiceSelect`＝Ref0 ID」 | `14.choice.toml` の `extra_choice_id`（Reference1 以降・可変長・`provenance = "ssp_secondary"`・「CROW のみ。選択肢の 2 番目以降の ID。」）は areka では M1 非対応（下表 7b-ii） |
| R-3 | `On` 始まり任意名イベントの Reference | Reference0 以降に付随参照列を記述順。表示ラベル・選択肢 ID は載せない | `ukadoc` | research §5-b 実測（`\q[タイトル,OnID,r0,r1,...]`）: 「ID が "On" で始まっている場合は、選択後、SHIORI イベント OnID(書いた通りのイベント) が開始される。それパラメータは r0,r1,... の順番に Reference0 以降に格納される。」 | research §9-2 が `\_a[OnID,r0,r1...]`（アンカー系）も同一規則を明記していることを実測記録している |
| R-4 | `OnChoiceTimeout` の Reference | Reference0=タイムアウトした選択肢を含むトークのスクリプト | `ukadoc` | research §5-f 実測: 「`OnChoiceTimeout`＝Ref0 タイムアウトしたスクリプト」。生成正本 `14.choice.toml` の `timeout_script`（reference=0・`provenance = "ukadoc"`）と一致 | 供給元は kanade が自ら組み立てた台本（`ActiveTalk.script`・DD-10）＝通知同梱ではない |
| R-5 | 付随参照列が空のときの位置 | 対応する Reference 位置を**付与しない**（空文字で埋めない） | `areka_discretion` | 正典は「r0,r1,... の順番に Reference0 以降に格納」と述べるのみで、空のときの位置の有無に沈黙（research §5-b） | 既存 `on_mouse_move` の「値なし → 空文字で位置を保持」とは**非対称**な規約（Req3.5）。非対称であることを実装コメントにも明記している |
| R-6 | 発生元 scope の扱い | Reference に載せない。解決対象の選択待ちの特定・検証・ログにのみ用いる | `areka_discretion` | 正典の `\q` 記述に scope 相当の Reference 位置は存在しない（research §5-a の 6 形いずれにも無い）＝沈黙 | M1 の talk は単一 slot ゆえ scope で解決対象を特定する実需が無い（DD-13）。将来の per-scope 化に備えて値の搬送だけは維持する縮退シーム |

---

## 2. カスケード則・状態・タイムアウトの裁定（design「カスケード則の正典裁定」1〜8）

| # | 裁定項目 | 挙動 | provenance | 正典引用・根拠 | 反証・注記 |
|---|---|---|---|---|---|
| 1 | `On` 始まり ID の先行段 | `OnChoiceSelectEx` / `OnChoiceSelect` を**先行発火しない**（同名イベントの直接発火 1 段のみ） | `areka_discretion` | 正典の OnID 記述（research §5-b・上表 R-3 に転記）は「OnID が開始される」と述べるのみで、**Ex/無印が先行するか否かに沈黙**している | 適合対象ゴースト emo2 の実物メニューは直接発火に依存する。決定的で最も単純な読みとして 1 段のみを採用。SSP 実挙動の裏取りは得られなかった（research §8-1） |
| 2a | 正典形のカスケード順序 | `On` 始まりでない ID は `OnChoiceSelectEx` を先行段・`OnChoiceSelect` を後続段とする | `ukadoc` | research §5-f 実測に含まれる正典文「OnChoiceSelect よりも先に開始」（`14.choice.toml` `[entry."OnChoiceSelectEx"].description` にも同文が生成されている） | 順序そのものに揺れはない。揺れがあるのは**後続段を出す条件**（次行 2b） |
| 2b | 後続段の発行条件（204 ゲート） | **先行段が応答スクリプトを返したら後続段を発行しない**（204／失敗のときのみ次段へ進む） | `ukadoc` | research §5-c 実測: `\*` の記述「選択時は通常通り、OnChoiceSelectEx イベント(トークがなければ OnChoiceSelect イベント)が発生する。」／アンカー系の明文 `14.choice.toml:78`（`OnAnchorSelectEx`）「SHIORI が何も返さなかった場合のみ続けて OnAnchorSelect が発生する」 | **反証（字面の揺れ）**: research §5-c 実測の `\q[タイトル,ID,r2,r3...]` 記述「OnChoiceSelectEx が開始される。… OnChoiceSelectEx に続いて OnChoiceSelect も発生するが、Reference1 以降に何も入らずこの書き方では無意味。」は**無条件で後続**とも読める。areka は 204 ゲート説を採用（Req2.3/2.4）し、反証をここに保存する |
| 3 | カスケード最終段が 204 | トークは起動しない。**選択待ちのバリア解決は実行する**（選択待ちのまま台本を停止させない） | `areka_discretion` | 正典は最終段 204 のあとの選択待ちの帰趨に沈黙（Req2.8 の明示対象） | 解決を取りやめると台本がバリアで永久停止する（Req5.3）。204／失敗のいずれでも解決はちょうど 1 回発行する |
| 4 | 選択解決後の選択肢集合 | **破棄する**（解決時に先積みの候補集合をクリアし、再生を継続する） | `areka_discretion` | 正典沈黙（Req2.8 の明示対象）。areka は現行実装が先に答えを持っている＝`dola` の `CuePlayer::resolve_choice` が一致時に先積みをクリアする（`crates/dola/src/cue/runtime.rs` の resolve 経路） | 実装先行の裁定。維持側（解決後も選択肢が残る）に倒す正典根拠は見つからなかった |
| 5a | タイムアウトの計測起点 | 当該トークの**表示が全て完了した時点**から計測を開始する | `ukadoc` | research §5-e 実測（`\![set,choicetimeout,時間]`）: 「単位はミリ秒。時間のカウントはトークの表示が全て終わってから開始。そのスクリプト中のみ有効。選択肢より後ろに書いても有効。タイムアウト時 OnChoiceTimeout。」 | 起点値の出所は再生層の占有 horizon（duration 権威）であり、areka は独自の時間基準を新設しない（Req7.2・DD-9） |
| 5b | 無効化・省略の語彙 | `0` または `-1`＝計測を開始せず無期限に選択待ちを継続。時間指定の省略＝既定値へ戻す | `ukadoc` | research §5-e 実測: 「時間指定を省略：デフォルト値に戻す / 0 か -1：タイムアウトしない」 | areka の型上の写像は DD-8: 指令 `None`＝未指定（既定へ委譲）／`Some(v), v<=0.0`＝無効化／`Some(v), v>0.0`＝明示秒指定。M1 で実際に流れるのは既定値のみ（台本タグの解釈は追跡 spec `areka-P0-sakura-time-directives` の領分） |
| 5c | タイムアウト既定値 | **30,000ms（30 秒）**（`KanadeConfig.choice_timeout_default_ms`） | `areka_discretion` | **正典は既定値の数値を規定していない**——research §5-e が ukadoc 実測により「既定値の数値は ukadoc に記載なし」と確認済み（Req7.8 の「正典が数値を規定していない旨とともに記録する」に対応） | SSP の de-facto 値の裏取りは得られなかった（research §8-2）。値を変える場合は下流 e2e（`areka-P0-emo2-conformance-e2e`）の期待値に波及する |
| 5d | タイムアウトの権威の単一化 | 計測・発火・解除の権威は kanade に**一本化**する。`dola` の `TimedSchedule` が持つバリア自動解除機構（`barrier_timeout_offset`）は選択バリアには**使用しない** | `areka_discretion` | 正典沈黙（実装内部の権威配置）。当該 seam は `CuePlayer::tick` が選択待ちで早期 return するため choice バリアには到達不能であり、かつ自動解除は「無条件再開」で正典の `OnChoiceTimeout` GET → 204 で解除という手順と一致しない（research §10.1 の設計フェーズ実測） | 二重権威の禁止。将来 dola 側 seam を生かす場合は本行を改訂したうえで行うこと |
| 6 | 選択待ち中の `Status` 複合値 | `talking,choosing`（選択待ち中もトーク slot を占有し続けるため `talking` は真のまま・正典順で連結） | `areka_discretion` | 正典は各状態語を定義するが、**選択待ち中に `talking` を降ろすか否かに沈黙**。連結順序・区切り・空集合省略は既存の送出契約（`crates/areka-kanade/src/status.rs` の正典順ソート）に従い、本 spec は書式を新設しない | 状態語の集合そのものは正典由来（**本表外の出所**: `emo2-conformance-scope.md:18` が emo2 の読むヘッダとして `Status`（talking/choosing/online 等 9 種）を記録）。適合対象ゴーストが `status == "talking"` の完全一致比較に依存する場合、複合値では自発トークが抑止されない可能性があるため、areka は**自身の調停のみで**選択待ち中の自発トークを抑止する（Req6.5） |
| 7a-i | `script:` 前置形の存在 | 正典には `\q[タイトル,script:実行内容]` 形が存在する | `ukadoc` | research §5-a 実測（`\q` の 6 形）: 「`\q[タイトル,ID]` / `\q[タイトル,ID,r2,r3...]` / `\q[タイトル,ID1,ID2,ID3...]` / `\q[タイトル,OnID,r0,r1,...]` / `\q[タイトル,script:実行内容]` / `\q[ID][タイトル]`（旧仕様）」 | 語彙は完全な形で保持し、判定は「未対応カテゴリ」として第一級に表現する（次行） |
| 7a-ii | `script:` 前置の M1 縮退 | **M1 非対応**。SHIORI イベントを発行せず、未対応である旨を警告に記録したうえで選択待ちの解決は実行し、会話を停止させない | `areka_discretion` | 正典は当該形のスクリプト直接実行を規定するが、areka は M1 で実装しない明示縮退（Req2.7）。判定は純関数の第一級カテゴリ（`crates/areka-kanade/src/schedule/choice.rs` の `CascadePlan::Unsupported`） | 縮退の 4 点セット: ①完全語彙＝`CascadePlan` の 3 分岐に未対応カテゴリを持つ ②縮退シーム＝当該分岐を実行経路に残す ③追跡＝本表 ④スコープ明記＝出所 spec の Non-Goals。emo2 は当該形を使用しないため実機一周に影響しない |
| 7b-i | CROW 複数 ID 形の存在 | 正典には `\q[タイトル,ID1,ID2,ID3...]` 形が存在し、追加 ID が Reference に格納される | `ssp_secondary` | research §5-d 実測: 「`\q[タイトル,ID1,ID2,ID3...]` は『ID* が Reference* に格納』＝OnChoiceSelect の Ref1 以降に追加 ID が載る形」。生成正本 `14.choice.toml` の `extra_choice_id`（`provenance = "ssp_secondary"`・「CROW のみ。選択肢の 2 番目以降の ID。」） | 二次情報（CROW 実装由来）であることを ukadoc 側も明示している |
| 7b-ii | CROW 複数 ID 形の M1 縮退 | **M1 非対応**。areka のワイヤ形では Ex 形（`ID,r2,r3...`）と**区別不能**であるため、到来した追加 ID は付随参照列として扱う（`OnChoiceSelect` の Reference1 以降に追加 ID を載せる挙動は実装しない） | `areka_discretion` | research §5-d 実測: 「areka の `ChoiceSelection`／`CueCommand::Choice` は `id` ＋ `references` の 2 分割ゆえ、Ex 形（r2,r3…）と複数 ID 形をワイヤ形で区別できない」（DD-15） | 構造的縮退（判定材料がワイヤ形に存在しない）であり、意思による省略ではない。区別を可能にするには上流のコンパイル形（`\q` の分割規約）を変える必要があり、その改訂は本表と下流 e2e の期待値に波及する |
| 8a | 選択起源の逐語発火 | `\q` の ID に書かれたイベント名を**逐語で**発行する（事前の固定登録を要さない） | `ukadoc` | research §5-b 実測（上表 R-3 に転記）: 「ID が "On" で始まっている場合は、選択後、SHIORI イベント OnID(**書いた通りのイベント**)が開始される。」 | `On` 接頭辞は名前の制約ではなく、Ex 形と任意名形を分けるディスパッチ判定子である（research §9-2 の正典再確認） |
| 8b | スケジューラ起源の恒久禁止との非交差 | areka がスケジューラ起源で自発送出する固定表の恒久禁止（`OnTalk` / `OnHour`）を、**選択確定に由来する発行へは適用しない**（`\q[x,OnTalk]` は発火する） | `areka_discretion` | 禁止の根拠は「ベースウェアが自発的に周期発火すると消費側ゴーストの自発生成と二重駆動する」ことであり（`emo2-conformance-scope.md:27`「送ってはいけない: `OnTalk`/`OnHour`（emo2 が OnSecondChange 内で内部生成。二重発火になる）」）、ゴースト作者が選択肢に明示的に書いた 1 クリック = 1 回の発火はこの根拠に該当しない（Req2.9） | 正典に「禁止」という概念は無く、禁止自体が areka 裁量である。実装は型で分離する: 固定表はスケジューラ起源専用、選択起源は別カテゴリの受理規則（`On` 接頭）で検証する。両者は交差しない |

---

## 3. 実装層で確定した裁定（正典・設計いずれも沈黙）

`crates/areka-kanade/src/schedule/choice.rs` の `choice_deadline`（DD-8 の写像の実装点）で確定した細目。設計本文に明文が無く、決定論のために実装層で裁定した。

| # | 裁定項目 | 挙動 | provenance | 正典引用・根拠 | 反証・注記 |
|---|---|---|---|---|---|
| 9 | 秒→ミリ秒の丸め規則 | **四捨五入**（端数 0.5 は絶対値の大きい側／`f64::round`）。0ms へ丸まる微小な正値も「期限あり（即時）」であり、無期限とは区別する | `areka_discretion` | 正典は `\![set,choicetimeout]` の単位をミリ秒と規定する（research §5-e）が、areka 内部の入口値は秒（`Option<f64>`）であり、**秒→ミリ秒変換の丸め規則には正典・設計とも沈黙**している | 切り捨て（`trunc`）を採ると「0.9995 秒指定が 999ms」となり指定値より早く期限が来る。四捨五入は誤差が両側対称で、決定論檻で境界を固定しやすい。飽和加算により無限大・巨大値はオーバーフローせず上限に留まる |
| 10 | タイムアウト指令が `NaN` の場合 | **無期限（期限なし）へ畳む**——無効化（`0` / `-1`）と同じ側 | `areka_discretion` | 正典の語彙は「省略／`0` または `-1`／正の時間」の 3 値のみで（research §5-e）、`NaN` は**語彙の外**。DD-8 の 3 値語彙も `NaN` を規定していない | `NaN` は `v <= 0.0` にも `v > 0.0` にも一致しない（順序が部分的）ため、明示判定を置かないと分岐が処理系依存に見える。無期限側へ畳むのは「解釈できない指令で会話を打ち切らない」という選択系の一貫方針（棄却より継続）に沿う。逆側（既定値を適用）に倒す根拠は無い |

---

## 4. 変更時の波及

- 本表の裁定を変更する場合、下流 spec `areka-P0-emo2-conformance-e2e` の期待値に波及する（出所 spec の Revalidation Triggers に明記）。
- 適合スコープの記述（`emo2-conformance-scope.md` §1 の送出イベント一覧）と本表 §1 は同一の割付を指す。片方だけを変えないこと。
- `shiori/fragments/events/14.choice.toml` は生成物であり手編集しない。正典側の記述が更新された場合は生成元を更新し、本表の `provenance` と反証欄を見直す（`COMPAT_ARCHITECTURE.md` §2「ukadoc 更新時は正典に従い是正」）。

## 5. 正典引用の出所一覧

本表の「正典引用・根拠」欄に転記した一次記述は、すべて出所 spec の `research.md` §5（2026-07-31 ukadoc MCP 実測）および §9-2 の正典再確認に実在する。

| 本表の行 | 転記元 |
|---|---|
| 7a-i | research §5-a（`\q` の 6 形） |
| R-3・8a | research §5-b（OnID 形の正典文） |
| 2b | research §5-c（`\*` の記述・`\q[…,r2…]` の反証・`14.choice.toml:78` のアンカー系明文） |
| 7b-i | research §5-d（CROW 複数 ID 形とワイヤ形の区別不能） |
| 5a・5b・5c・9・10 | research §5-e（`\![set,choicetimeout]` の記述・既定値の数値は記載なし） |
| R-1・R-2・R-4・2a | research §5-f（Reference 割付・「OnChoiceSelect よりも先に開始」） |
| R-3 の注記・8a の注記 | research §9-2（`\_a[OnID,…]` も同一規則・`On` 接頭はディスパッチ判定子） |
| 5d | research §10.1（設計フェーズ実測・`TimedSchedule` の死んだ seam） |
| 6 の注記（**本表外の出所**） | `emo2-conformance-scope.md:18`（`Status` ヘッダの状態語 9 種）・`emo2-conformance-scope.md:27`（`OnTalk`/`OnHour` の送出禁止根拠）——research §5 には無い出所であることを明示する |
