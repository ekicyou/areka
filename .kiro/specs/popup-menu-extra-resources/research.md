# ギャップ分析: popup-menu-extra-resources

- 日付: 2026-10-31（main `9604677a` の上の作業ブランチ `4a22d51b`）
- 範囲: brief の ⑴ だけ（古い名前 `updatebuttoncaption` を `updatebutton.caption` が引けないときの項目名に使う）。⑵ は開発者の決めにより扱わない。
- 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の該当行）・下に挙げたコードと網羅台帳の一式。

## 1. 要約

- **骨組みは揃っている。** 問う名前の表（`ALLOWED_RESOURCE_IDS`）に 1 語足せば、送出の関門・SHIORI/3.0 の照会・SHIORI/2.x への組み替え（`GET String SHIORI/2.5`・`ID: updatebuttoncaption`）はどれも手を入れずに通る。2.x の資源の印は名前の形でなく同じ表から決まる（`crates/areka-kanade/src/shiori/real.rs:155,184` → `crates/shiori-host32-host/src/shiori2.rs:97-106`）。
- **足りないのは「新しい名前が引けないときに古い名前の値を使う」1 か所だけ。** 今の項目名は `plan.rs` の `caption_for` が `MenuItem.caption_resource` 1 つで `CaptionMap` を引いて決める。`menu/plan.rs`・`mod.rs` を変えずに済ませるには、置き換えを `menu/captions.rs`（`query_ids` で古い名前を足す・`interpret` か `CaptionMap::get` で新しい名前に落とす）に閉じ込める形になる。
- **台帳まわりは「1 行」では済まない。** `owner` を本 spec にすると `roadmap-draft.md` に `[[spec]]` の行が要り（`[briefs].count` 66→67）、状態を `implemented` にすると `briefing.md` の `[[barrier]]`（`list_shiori_resource` 14/145 → 15/144）と `report/shiori.md`・`report/summary.md` を作り直さないと `ukadoc-survey` のテストが赤になる。
- **台帳の決まりと要件 4.2 がぶつかる所が 1 つある。** `doc/ukadoc-coverage/README.md` の仕訳の規則は「同じ機能に複数の書き方があるときは最も新しい書き方を正典とし、それ以外を別名（`alias`）とする」と書く。ukadoc は 2 つの名前を同じ説明「ネットワーク更新の名称。」で載せているので、この規則どおりなら `updatebuttoncaption` は `alias`（`alias_of` は `updatebutton.caption`・`owner` と `priority` は空）になる。要件 4.2 は `implemented`・持ち主は本 spec と書いている。要件討議で決める。
- 規模は S（1〜2 日）・危うさは低い。残る未知は SSP で測る 4 点（測 1〜4）と、「．さくら」（華和梨・i686 helper 経由）で往復が 1 回増えたときに 1,000 ms の上限に収まるか、の実機の見え方だけ。

## 2. 今の姿（調べた事実）

### 2-1. 問う名前の表（kanade）

- `crates/areka-kanade/src/schedule/resources.rs:39-72` の `ALLOWED_RESOURCE_IDS` は 14 語。各要素の前に `// ukadoc: …#<名前>:1` の 1 行を置く（台帳の `implemented` の証拠＝README 3 章の「許可表の要素」）。
- 凍結のテスト `allowed_resource_ids_are_exactly_the_fourteen_names`（176-207 行）が 14 語を書き下している。名前・doc コメント（「14 名」「12→13」「13→14」）・失敗文（「枠 9 種の項目名」）も数を持つ。
- 表の doc コメント（25 行）は「利用者名＋メニュー 11 名＋更新 2 名」と書く（実数は 14 で、この内訳の書き方は既に実数と合っていない。直すときに揃える）。
- 否定側のテスト `unqueried_menu_names_are_not_allowed`（211-226 行）は `popupmenu.type` 系と n≧2 の表示可否と `other_homeurl_override` を「入れない」と確かめる。⑵ の名前を「入れない」側に足すかは任意（要件 5.3 は「⑵ の名前が表に無いこと」を凍結のテストで確かめると書く）。
- 表の読み手は `is_allowed_resource_id` だけ（`actor.rs:663` の送出の関門・`shiori/real.rs:155,184` の 2.x の印・テスト）。要素の数や並びを前提にする本番コードは無い。
- `crates/areka-kanade/tests/kanade/steady_test.rs:735-773` は「記録された id がイベント許可 ∨ リソース許可の部分集合」を見るだけで、数は見ない。

### 2-2. 照会の往復（kanade）

- `crates/areka-kanade/src/actor_resources.rs:69-103` の `answer` は、id を 1 つずつ**順に** `round_trip_request` で往復し、入力と同じ順・同じ長さで 1 回だけ返す。`Steady` でないときは送らず全件 `NoContent`。
- 古い名前を足すと、メニュー 1 回の照会で SHIORI への往復が 1 回増える。待つ上限はメニュー側の `QUERY_TIMEOUT`（`menu/captions.rs:95`・1,000 ms・照会全体）で、kanade 側は 1 件ずつの上限を持たない。要件 1.6 は上限を延ばさないと書くので、増えた往復の分だけ「全件既定名」に落ちる余地がわずかに広がる（**実機で見る**）。

### 2-3. SHIORI/2.x の組み替え（完了 `shiori2-protocol`）

- `Shiori3Client::for_resource()` の印は `is_allowed_resource_id(id)` で決まる（`crates/areka-kanade/src/shiori/real.rs:154-157,184-186`）。
- 印の付いた GET は `route` で `Route::String` になり、`GET String SHIORI/2.5`・`ID: <名前>` を組み、答えは `String` の見出しから読む（`crates/shiori-host32-host/src/shiori2.rs:97-106,114-163,170-189`）。名前の形を見て分ける所は無い。
- 結論: **表に足した名前は今の名前と同じ道で 2.x の相手へ届く。2.x の書き方には手を入れなくてよい**（要件の Adjacent expectations の前提は成り立つ）。`real_resource_mark_tests.rs` は `for_resource(` の呼び出しの数と位置を見ており、本 spec はそこに触れない。

### 2-4. 項目名の引き方（areka `menu/`）

- `crates/areka/src/menu/captions.rs`
  - `FRAME_CAPTIONS`（30-63 行）: 枠 → (資源名, 既定名)。「ネットワーク更新」は `(Some("updatebutton.caption"), Frame::Update, "ネットワーク更新")`。並びは `Frame::ORDER` と同じで判別値が添字（`row` の `debug_assert`）。1 枠 1 資源名の形。
  - `query_ids`（133-139 行）: 先頭に表示可否の名前、続いて写しの `caption_resource` を出た順に（子も辿る）、重複は潰す。**古い名前を足す場所の候補 1**。
  - `CaptionMap`（157-172 行）: 非空の値だけを持つ `HashMap<&'static str, String>`。`get(id)` が無ければ `None`。**候補 2**（ここで新しい名前 → 古い名前に落とす）。
  - `interpret`（245-320 行）: 1 件ずつ ⑴ 非空＝表へ ⑵ 空・値なし＝`empty` へ ⑶ 失敗＝`failed` へ。最後に `menu_resource_empty`（`debug`）と `menu_resource_failed`（`warn`）を 1 行ずつ。返事なしは `menu_resource_query_unanswered`（`warn`）1 行で全件既定名。**候補 3**（ループの後で古い名前の値を新しい名前の鍵へ写す）。
  - `UNQUERIED_POPUPMENU_RESOURCES`（87-92 行）: 問わない 4 名。兄弟テストが「許可表に無い」を確かめる。
- `crates/areka/src/menu/plan.rs:124-130` の `caption_for`: `item.caption_resource.and_then(|id| captions.get(id))` が非空ならそのまま（`&` は素通し）、無ければ既定名を `&`→`&&` で逃がす。**古い名前の値も `CaptionMap` から同じ鍵で出てくれば、`&` の素通し・前後を削らない扱い（要件 1.5）は何もしなくても揃う。**
- `crates/areka/src/menu/update_frame.rs:22-33`: 項目は `caption_resource: captions::resource_for(Frame::Update)`。登記されない（窓の無い起動など）ときは写しに現れないので、`caption_resource` から古い名前を導けば要件 1.7（登記されていないときは問わない）は自然に満たす。
- `MenuItem.caption_resource` を組む所は本番・テストで 20 か所あまり（`grep caption_resource`）。`MenuItem` に欄を足す案はこの全部と `mod.rs`・`plan.rs` を触る。

### 2-5. 既存テストで影響を受けるもの

- `menu/captions_tests.rs:221-251` `query_ids_collects_submenu_children_and_collapses_duplicates` は孫の葉に `Some("updatebutton.caption")` を置き、期待の列に `updatebutton.caption` を含む。**古い名前を資源名から導く形にすると期待の列に `updatebuttoncaption` が 1 つ増える**（直すか、固定の葉を別の名前にする）。
- `menu/captions_tests.rs:172-196` は `FRAME_CAPTIONS` の資源名がすべて許可表にあることを見る。古い名前を別の表に置くなら、その表も許可表にあることを同じ形で確かめる行が要る。
- `menu/plan_tests.rs:286`・`menu/update_frame_tests.rs:122` は `updatebutton.caption` の綴りを見るだけで、古い名前の有無に左右されない。
- `trigger_wired_tests.rs:86-95` などの照会の列の固定は「説明書」「終了」の枠だけで組んでおり、「ネットワーク更新」の枠を含まない＝影響なし。
- `captions_tests.rs` は 444 行。組み合わせ 6 通り（要件 5.1）と照会の列 2 本（要件 5.2）を足しても 1,000 行の目安に収まる見込み。テーマで分けるなら `captions_<テーマ>_tests.rs` の兄弟（`structure.md:151-181`）。

### 2-6. 網羅台帳と検査（`ukadoc-survey`）

- 台帳の行: `doc/ukadoc-coverage/ledger/shiori.toml:15329-15348` `updatebuttoncaption` は `vocabulary-only`・`owner = ""`・`priority = "A11"`・`values = ["装い"]`。備考は「語彙のみ」の型の文（`SHIORI_RESOURCE_IDS` に名前はあるが `ALLOWED_RESOURCE_IDS` に無い）。
- 隣の行: 同 15303-15327 `updatebutton.caption` は `implemented`・`owner = "areka-P0-network-update"`。備考の「壊れ方」は 15317-15318 行「壊れ方: 無い。空・値なし・失敗・時間切れのときは既定名「ネットワーク更新」に落ち、メニューはそのまま出る。」で、本 spec の後は「新しい名前が引けないときは古い名前に落ち、それも引けないときに既定名」に変わる（要件 4.3 の対象）。同じ備考の「根拠の場所」は `ALLOWED_RESOURCE_IDS` と `FRAME_CAPTIONS` を挙げる。落とし先の表を `captions.rs` に別に置くなら、ここも事実と食い違う（要件 4.3 は「壊れ方」の文だけを直すと書く）。
- `implemented` の証拠: README 3 章。ソースの定義箇所（許可表の要素の前）に `// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#updatebuttoncaption:1` の 1 行があればよい（無ければ `ImplementedWithoutEvidence`）。カタログの id は `ukadoc:list_shiori_resource:updatebuttoncaption:1`（`catalog.toml:1695`・`versions = []`）なので `introduced = ""` のままでよい。
- 優先度: `derive::priorities`（`crates/ukadoc-survey/src/documents/derive.rs:157-172`）は `implemented` でも `vocabulary-only` でも同じ値を出す＝`A11` のまま。`alias` にすると空文字でなければならない。
- 束: `linkage.md` の `[bundle."メニュー"]`（482 行〜）の 2 か所（632・761 行）に `updatebuttoncaption` が入っている。`alias` と `not-applicable` は束から除く決まり（`tests/consistency/linkage_checks.rs:79` `EXCLUDED_STATUSES`）。
- 持ち主の検査（判定 ⑸・`tests/consistency/spec_checks.rs`）:
  - 腕 f: 台帳の非空の `owner` は `roadmap-draft.md` の `[[spec]]` の名前か、`briefing.md` の `[[owner_completed]]` の名前でなければならない。`popup-menu-extra-resources` はどちらにも無い＝**`[[spec]]` の行を 1 行足す必要がある**。
  - 腕 a: `[briefs].count`（`roadmap-draft.md:610-613`・今 66）＝ `[[spec]]` の行数。行を足すと 67 に直す。
  - 腕 b: `[[spec]]` の名前が `.kiro/specs/` の直下か `completed/` に在る（完了の後も通る）。
  - 腕 c: `owner_count` ＝ 台帳 4 本でその名前を持つ項目の数（本 spec なら 1）。
  - 腕 d: `bundle` が `linkage.md` の束名に在る（`"メニュー"` は在る）。
  - 先例: 2026-10-24 `shiori2-protocol`（`roadmap-draft.md:554-562` の説明の段落と 1104-1108 行の `[[spec]]`）、2026-10-31 `seriko-pattern-calls`（565-575 行・1110-1115 行）。どちらも「追加」の段落を書き、段階 A の表の該当の束の行（本 spec なら 1141 行「メニュー」）に spec 名と件数を足している。
- 状態の分布（判定 ⑷ 腕 f・`tests/consistency/briefing_arms.rs`）: `briefing.md` の `[[barrier]]` `page = "list_shiori_resource"` は `implemented = 14`・`vocabulary_only = 145`（990-999 行付近）。`implemented` にすると **15・144** に直す必要がある（直前の段落に経緯の 1 文を足すのが先例）。
- 報告: `report/shiori.md:27` の `| list_shiori_resource | 14 | 145 | … |` は機械が作る。`cargo run -p ukadoc-survey -- report` と `-- report-summary` で作り直さないと `DomainReportStale` と判定 ⑹（`summary.md` の全文一致）が赤になる（README 4 章）。
- 人向けの経緯の文: `briefing-shiori.md:1043-1047` に `ghostinstallbutton.caption`・`updatebutton.caption` を実装済みにした日の追記があり、1039 行の一覧に `updatebuttoncaption` が並ぶ。検査の対象ではないが、先例に倣うなら 1 段落の追記が要る。
- 触らないもの: `crates/areka-sylphya/src/vocab/shiori_resource.rs:79`（語彙の表・既に `updatebuttoncaption` を持つ）、`doc/shiori/fragments/resources/02.ghost_info.toml:197`（正典の写し・areka の対応状態を持たない）、`catalog.toml`（機械だけが書く）。

## 3. 要件と資産の対応

| 要件 | 使える資産 | ギャップ |
|---|---|---|
| 1.1 1 回の照会で古い名前も問う | `query_ids` が写しから名前を集める | **Missing**: 新しい名前から古い名前を導く対応（表か 1 組）と、`query_ids` で足す処理 |
| 1.2 新しい名前が引けたら新しい名前 | `CaptionMap`・`caption_for` | なし（新しい名前を先に見る順を守るだけ） |
| 1.3 新しい名前が引けず古い名前が引けたら古い名前 | `interpret`・`CaptionMap` | **Missing**: 落とし先の解決（`interpret` の後で写すか `get` で落とすか） |
| 1.4 どちらも引けないとき既定名 | `caption_for` の既定名 | なし |
| 1.5 `&` の素通し・前後を削らない | `caption_for` | なし（同じ鍵で `CaptionMap` から出せば揃う） |
| 1.6 返事なし→全件既定名・上限を延ばさない | `QUERY_TIMEOUT`・`menu_resource_query_unanswered` | **Constraint**: kanade は順に往復するので往復が 1 回増える。上限は変えない。実機で見る |
| 1.7 枠が無いときは問わない | 写しから導く `query_ids` | なし（資源名から導けば自然に満たす） |
| 2.1〜2.3 記録の決まり | `interpret` の 2 行 | **Unknown（議題）**: 新しい名前が引けたときも古い名前の「空」が `debug` 行に載る。要件どおりだが、今のゴーストの記録の行の中身が変わる |
| 3.1〜3.3 今のゴーストを変えない | 既存の写し・計画 | なし（`debug` の行の中身を除く） |
| 3.4 3.0 と 2.x で同じ | 2-3 の通り | なし（確認済み） |
| 3.5 問わない名前の表 | `UNQUERIED_POPUPMENU_RESOURCES` | なし |
| 4.1 許可表に 1 語 | `ALLOWED_RESOURCE_IDS` | **Missing**: 1 語＋URL 1 行・凍結のテストを 15 語に |
| 4.2 台帳の行（`implemented`・持ち主） | 台帳・先例の備考の型 | **Constraint（議題）**: README の仕訳の規則では `alias` が素直。`implemented` を取るなら規則から外れる理由を備考に書く |
| 4.3 隣の行の「壊れ方」 | 15317-15318 行 | **Missing**: 文の直し。「根拠の場所」も食い違う可能性（議題） |
| 4.4 ⑵ の 11 行を変えない | — | なし |
| 4.5 整合の検査が通る | `spec_checks.rs`・`briefing_arms.rs`・報告の全文一致 | **Missing**: `roadmap-draft.md` の `[[spec]]`＋`[briefs].count`、`briefing.md` の `[[barrier]]`、`report/` 2 本の作り直し（要件本文は「1 行」と書くが、検査を通すにはこれらが要る） |
| 5.1〜5.4 決定論テスト | `captions_tests.rs` の `interpret_capturing`・`value` などの手伝い | **Missing**: 組み合わせ 6 通り・照会の列 2 本・凍結の更新・既存 1 本の期待の列の直し |
| 5.5 3 クレートのテスト | — | なし |
| 5.6〜5.8 実機（「．さくら」×SSP） | `vendors/sample_ghost/dot_sakura.nar`・排他テストの机・`target\` の下の根 | **Research**: 華和梨（i686 helper 経由）で往復 1 回増の時間、SSP の測 1〜4 |
| 6.1〜6.4 ⑵ を渡す | `roadmap.md` 46 行に「⑵ もここで待つ」と既に書いてある | なし（完了の棚卸の仕事） |

## 4. 実装の案

### 案 A: `menu/captions.rs` の中で落とす（既存の部品を広げる）

- `captions.rs` に「新しい名前 → 古い名前」の対応を置く（例: `(&str, &str)` の組の表を 1 本、要素は今 1 組。`FRAME_CAPTIONS` の行の形は変えない）。
- `query_ids`（`collect_caption_resources`）: 新しい名前を足したら、対応する古い名前をその直後に足す（重複は潰す）。並びは測 1 の結果で決める。
- 解決は 2 通りあり、どちらも `plan.rs` を変えない:
  - **A-1 `interpret` で写す**: ループの後で「新しい名前の鍵が無く、古い名前の鍵がある」なら古い名前の値を新しい名前の鍵へ入れる。記録は今のループがそのまま出す（古い名前の空は `debug`、失敗は `warn` の行に他と一緒に載る＝要件 2 を自然に満たす）。
  - **A-2 `CaptionMap::get` で落とす**: `get(new)` が無ければ対応する古い名前で引く。`interpret` は無改変。`get` が対応表を知る分だけ表の責務が広がる。
- 触るファイル: `resources.rs`（1 語＋URL＋凍結のテスト）・`menu/captions.rs`・`menu/captions_tests.rs`（または兄弟の新しいテストファイル）・台帳一式。
- ✅ 要件 6.2（`plan.rs`・`mod.rs`・`win32.rs` を変えない）をそのまま満たす。差分が最小。
- ✅ windows-reactor の後のメニューの作り直しで捨てる所に乗らない（「何を問い、どの値を使うか」は残る側）。
- ❌ 新しい名前の値がある今のゴーストでも、古い名前の照会と（値が無ければ）`debug` 行の 1 語が増える。

### 案 B: `MenuItem` に「落とし先の資源名」の欄を足す（新しい欄）

- `MenuItem` に `caption_fallback: Option<&'static str>` を足し、`update_frame.rs` が `Some("updatebuttoncaption")` を入れ、`query_ids` と `plan::caption_for` が 2 つを順に見る。
- ✅ 落とし先が枠の登記と同じ場所に見える。
- ❌ `mod.rs`・`plan.rs` を触る＝要件 6.2 と Boundary の「直しは問う名前の表と項目名の引き方の所に収める」に反する。
- ❌ `MenuItem` を組む 20 か所あまりに欄が増える。作り直しで捨てる側にコードが乗る。

### 案 C: kanade の照会の側で読み替える

- kanade の `answer` が `updatebutton.caption` を問われたら、引けないとき自分で `updatebuttoncaption` も問い、新しい名前の答えとして返す。
- ✅ `menu/` は無改変。
- ❌ kanade の照会は「渡された id を不透明に順に往復するだけ」（`actor_resources.rs` の設計・`resource_get` の契約）で、メニューの意味を kanade に持ち込む。記録の決まり（要件 2）も kanade 側で組み直しになる。
- ❌ 往復が 2 段（問うてから足す）になり、1 回の照会の形（要件 1.1）から外れる。

### 比べて

- 要件（特に 6.2・Boundary）にそのまま合うのは案 A。A-1 と A-2 の差は「落とし先を知る場所を `interpret` に置くか `CaptionMap` に置くか」で、どちらも 1 ファイルに収まる。案 B・C は要件を直さないと取れない。
- どの案でも台帳まわりの作業（2-6）は同じ量。

## 5. 規模と危うさ

- **規模: S（1〜2 日）**。コードは `resources.rs` に 1 語、`captions.rs` に十数行、テストが 8〜10 本。台帳の一式（行 2 つの文・`roadmap-draft.md`・`briefing.md`・報告の作り直し）と、SSP の測りと実機の見比べが時間の大半。
- **危うさ: 低**。既存の型と経路（許可表・2.x の組み替え・`CaptionMap`）をそのまま使う。残るのは実機の時間（往復 1 回増）と、台帳の状態の語の決め（`implemented` か `alias` か）だけ。

## 6. 設計の段への持ち越し

### 調べること（Research Needed）

1. **測 1〜4（要件の「測りで決める点」）**: SSP の MCP と `ghost\master\ssp_shiori_log.txt` の差分で。測 1 は照会の並び（新しい名前の直後に古い名前か、別の順か）と問う時、測 2・3 は優先と空の値の扱い、測 4 は `readmebuttoncaption`。検体の根はワークツリーの `target\` の下。
2. **往復 1 回増の時間**: 「．さくら」（華和梨・SHIORI/2.x・i686 helper 経由）でメニューを開いたときの照会全体の所要が 1,000 ms に収まるか。実機の確かめ（要件 5.6）のついでに記録の時刻で読む。
3. **`roadmap-draft.md` の `[[spec]]` の `wave`**: 正本のロードマップの写しとして何と書くか（ウェーブ C15 の割り込みの席か、「版に縛らない」段か）。`stage = "A"`・`bundle = "メニュー"`・`owner_count = 1` は束の決まりから決まる。

### 決めること（要件討議へ）

下の「議題」の節。

## 7. 議題（要件討議の材料）

1. **台帳の状態の語: `implemented` か `alias` か。** README の仕訳の規則（「同じ機能に複数の書き方 → 最も新しい書き方を正典、それ以外を `alias`」）と ukadoc の同じ説明からは `alias`（`alias_of = "ukadoc:list_shiori_resource:updatebutton.caption:1"`・`owner = ""`・`priority = ""`）が素直。ただし `alias` にすると ⑴ 持ち主が付かない（要件 4.2・4.5 と食い違う）⑵ `linkage.md` の「メニュー」束の 2 か所から外し、段階と順位の数（`briefing.md` の `[stage.A]`・`[[rank]]` など）を数え直す仕事が増える ⑶ `[[barrier]]` は `alias` 1・`vocabulary_only` 144 になる。`implemented` を取るなら、規則から外れる理由（「areka は古い名前を別の名前として引き、新しい名前が無いときの落とし先に使う＝写像だけでなく引きに行く実装がある」など）を備考に書く。
2. **落とし先を解決する場所**: 案 A-1（`interpret` の後で新しい名前の鍵へ写す）か A-2（`CaptionMap::get` で落とす）か。どちらも `plan.rs` は無改変。
3. **対応の持ち方**: 「新しい名前 → 古い名前」の組の表（今 1 組・`readmebuttoncaption` などを足すなら 1 行足すだけ）にするか、`updatebutton.caption` 1 つに決め打ちするか。表にするなら「その表の名前がすべて許可表にある」兄弟テストを足す。
4. **古い名前を導く鍵**: 資源名（`caption_resource == "updatebutton.caption"`）から導くか、枠（`Frame::Update`）から導くか。資源名なら `query_ids` の入力の形は今のまま（ただし既存テスト `query_ids_collects_submenu_children_and_collapses_duplicates` の期待の列が 1 つ増える）。枠からなら写しの `(Frame, MenuItem)` の `Frame` を使う。
5. **今のゴーストの `debug` 行**: 新しい名前が引けたときも、古い名前が値なしなら `menu_resource_empty` の `ids` に `updatebuttoncaption` が載る（要件 2.1 の字面どおり）。今のゴーストの記録の行の中身が 1 語変わることを受け入れるか、新しい名前が引けたときは古い名前を空の記録から外すか（要件 2.1 と 3.1 の読み方）。
6. **網羅台帳で触る範囲**: 検査を通すには、行 2 つ（`updatebuttoncaption`・`updatebutton.caption` の「壊れ方」）のほかに、`roadmap-draft.md`（`[[spec]]` 1 行・`[briefs].count` 66→67・「追加」の段落・段階 A の表の「メニュー」の行）、`briefing.md`（`[[barrier]]` 14/145→15/144 と経緯の 1 文）、`report/shiori.md`・`report/summary.md` の作り直しが要る。要件 4・Boundary の「台帳の 1 行」をこの範囲まで広げてよいか（先例 `shiori2-protocol`・`seriko-pattern-calls` はこの形）。`briefing-shiori.md` の追記の段落も先例に倣って書くか。
7. **`updatebutton.caption` の行の「根拠の場所」**: 落とし先の表を `captions.rs` に別に置くと、「根拠の場所」（`ALLOWED_RESOURCE_IDS` と `FRAME_CAPTIONS`）も事実と食い違う。要件 4.3 の「壊れ方の文だけ」を「壊れ方と根拠の場所」に広げるか。
8. **照会の並び**: 古い名前を新しい名前の直後に置くか、列の最後に置くか（測 1 の結果で決める）。並びは往復の順になり、SSP の記録と見比べる材料になる。
9. **`readmebuttoncaption`（要件の「要件討議で決める点」の 1）**: 案 A の組の表にすれば仕組みは同じで足りる。ただし ukadoc の説明が `readmebutton.caption` と違う（「Read Me!の名称。」と「キャラクタについての名称。」）ので、同じ項目の古い名前と言えるかは測 4 で決める。
10. **凍結のテストの否定側**: 要件 5.3 の「⑵ の名前が表に無いこと」を、今の `unqueried_menu_names_are_not_allowed` に ⑵ の 11 語を足して確かめるか、別のテストにするか。

## 7-2. SSP の測りの結果（2026-10-31・要件討議の前）

`requirements.md` の「測った結果」に書いた。設計に効くのは次の 2 点。

- 照会の並び（議題 8）: SSP は古い名前を新しい名前の**直前**に問う（`updatebuttoncaption` → `sakura.updatebuttoncaption` → `updatebutton.caption`）。SSP の記録と見比べやすい並びは「古い名前を新しい名前の直前」。
- `readmebuttoncaption`（議題 9）: SSP は `readmebutton.caption` の直前に同じ形で問う。使い道は値を返す検体が無く読めない。

## 7-3. 要件討議の結果（2026-11-01・全議題を推奨案で承認）

- 議題 1（状態の語）→ `implemented`・`alias` にしない理由を備考に。議題 6・7（台帳の範囲・根拠の場所）→ 検査を通すのに要る所まで広げ、`updatebutton.caption` の行は「壊れ方」と「根拠の場所」の 2 文を直す。議題 5（`debug` 行）→ 受け入れる。議題 9（`readmebuttoncaption`）→ 取らず ⑵ と一緒に渡す。`sakura.updatebuttoncaption` → 取らない。
- 設計の段に残るのは議題 2・3・4・8・10（落とす場所・対応の持ち方・導く鍵・照会の並び・凍結テストの否定側）。

## 8. 次の段

- 要件討議（`/kiro-requirements-discussion popup-menu-extra-resources`）で 7 章の議題を決め、必要なら要件を直す。その前に測 1〜4 を SSP の MCP で測る（要件の決まり）。
- 決まったら `/kiro-design popup-menu-extra-resources` で設計へ。

## 9. 設計の段（2026-11-01・`kiro-spec-design -y`）

### 9-1. 調べの種類と使った手引き

- 種類: 軽い調べ（既存の仕組みへの追加・`design-discovery-light.md`）。新しい依存も外の API も無いので、外の調べ（WebSearch）は要らなかった。調べは 2 章のギャップ分析の事実を、設計に効く所だけコードで読み直した。
- 使った手引き: `kiro-spec-design` の `rules/`（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）と `.kiro/settings/templates/specs/design.md`。メモの決まり「配線は再テストしない」「零は明示的に書く」を、テストの組み方（9-3 の D5 と C1 の注記）に使った。

### 9-2. 設計の段で読み直した事実

- `menu/captions.rs` の `interpret` は `Ok` の枝だけで 1 件ずつ写し、`Err` の枝は空の `CaptionMap` を返して終わる。落とし先の解決を `Ok` の枝のループの後に置けば、時間切れの全件既定名（要件 1.6）は手を入れずに保たれる。
- `CaptionMap` は非空の値しか入れない（`insert` が空を捨てる）ので、「鍵がある」＝「引けた」。空の値・値なし・失敗の 3 通りは、どれも「鍵が無い」として同じに扱える。
- `plan.rs` の `caption_for` は `item.caption_resource` の鍵で引くだけなので、古い名前の値を新しい名前の鍵へ入れれば、`&` の素通し・前後を削らない扱い（要件 1.5）がそのまま揃う。
- `doc/ukadoc-coverage/README.md` 3 章: 証拠は定義箇所の `ukadoc:` 1 行。ページ URL の行の直後のスライス定数は、要素の最初の文字列を名前として拾う。古い名前の組の表に `ukadoc:` の行を置くと、最初の文字列＝新しい名前が拾われ、「呼び出し側には書かない」にも反するので、置かない。証拠は許可表の 1 行だけ。
- 検査の種類（`check/finding.rs`）に「語彙のみの行に証拠がある」を咎めるものは無い。`SourceUrlNotInCatalog` はカタログの URL `#updatebuttoncaption:1` に当たるので通る。
- `roadmap-draft.md` の先例 `shiori2-protocol` は `wave = "C11-⑧"`（ウェーブと席の番号）。本 spec は roadmap の C15 の ⑥ なので `wave = "C15-⑥"`（6 章の調べること 3 の答え）。
- 往復の配線のテスト（`actor_resources_tests.rs` の `reset_balloon_position_caption_is_looked_up_from_the_ghost` の型）は、送出の関門と 2.x の印がどちらも `is_allowed_resource_id` 1 つで決まるので、名前ごとに足さない。

### 9-3. 設計の決め（7 章の議題 2・3・4・8・10）

#### D1: 落とし先を解決する場所＝`interpret`（議題 2・案 A-1）
- **Alternatives**: A-1 `interpret` のループの後で新しい名前の鍵へ写す ／ A-2 `CaptionMap::get` で新しい名前が無ければ古い名前を引く。
- **Selected**: A-1。
- **Rationale**: 「引ける・引けない」を決めている所に優先の決めも置ける。`CaptionMap` は「名前 → その名前自身の値」の表のままで、`get` に意味を足さない（テストが直接組む `CaptionMap` の振る舞いも変わらない）。記録は今のループが古い名前も数えるので、記録の手当ては要らない。
- **Trade-offs**: `CaptionMap` に古い名前の鍵も残る（引く項目が無いので害は無い）。コードの大きさは A-2 と同じ。

#### D2: 対応の持ち方＝1 行の組の表（議題 3）
- **Alternatives**: 組の表 `&[(&str, &str)]` ／ `updatebutton.caption` に決め打ちの 1 組の定数。
- **Selected**: `LEGACY_CAPTION_RESOURCES: &[(&str, &str)] = &[("updatebutton.caption", "updatebuttoncaption")]`。
- **Rationale**: 同じファイルの名前の表（`FRAME_CAPTIONS`・`UNQUERIED_POPUPMENU_RESOURCES`）と同じ形で、兄弟テストが許可表との突き合わせを同じ書き方でできる。決め打ちとコードの大きさは同じ。行を足す予定は無い（`readmebuttoncaption` は取らない裁定）。

#### D3: 古い名前を導く鍵＝資源名（議題 4）
- **Selected**: `caption_resource` が表の新しい名前に等しいとき。`Frame` は使わない。
- **Rationale**: `interpret` は名前の列しか持たず、サブメニューの子には `Frame` が無い。資源名から導けば要件 1.7 が分岐なしで満たされ、`query_ids` の型も変わらない。
- **Follow-up**: 既存テスト `query_ids_collects_submenu_children_and_collapses_duplicates` の期待の列に古い名前が 1 つ増える（直す）。

#### D4: 照会の並び＝古い名前を新しい名前の直前（議題 8）
- **Rationale**: SSP の並び（測 1）と同じで、記録を見比べやすい。優先は往復の後にまとめて決めるので、並びは優先に効かない。

#### D5: 凍結のテストの否定側＝kanade に別のテストを 1 本（議題 10）
- **Selected**: `resources.rs` の `mod tests` に `deferred_menu_item_names_are_not_allowed`。⑵ の 8 つ・`char*.recommend*` の 3 つ（台帳の綴りと `char2.` の綴りの両方）・取らない裁定の `readmebuttoncaption`・`sakura.updatebuttoncaption` が許可されないことを明示して確かめる。
- **Rationale**: 今の `unqueried_menu_names_are_not_allowed` は「問い合わせない裁定」の名前で、⑵ は「windows-reactor 待ち」の名前。理由の違う名前を同じ失敗文に混ぜない。完全一致の凍結のテストでも無いことは導けるが、無いことは明示して書く（メモの決まり）。

### 9-4. まとめ方の見直し（`design-synthesis.md`）

- **一般化**: 「新しい名前が引けないとき古い名前で引く」は他の項目名にもありうる形（`readmebuttoncaption` など）なので、仕組みを組の表で受けた。ただし行は 1 つだけで、足す仕組み・選択肢は作らない（今の要件が求めるのは 1 組だけ）。
- **作るか借りるか**: 外の部品で解く問題ではない。既存の `query_ids`・`interpret`・`CaptionMap`・許可表をそのまま使う。
- **削ったもの**: `MenuItem` に欄を足す案（案 B）・kanade で読み替える案（案 C）は取らない（要件 6.2 と kanade の「名前を不透明に往復する」契約に反する）。名前ごとの往復の配線テストは足さない。`briefing-shiori.md` の経緯の段落は検査の対象でなく要件 4.5 の挙げる周りにも入らないので書かない。

### 9-5. 危うさ

- 同じウェーブの `shell-scale-linkage` も `roadmap-draft.md`・`briefing.md`・報告に触れる。数を共有する所（`[briefs].count`・行数・`[[barrier]]`・報告）は、後から着地する側が main に合わせて数え直し、報告を作り直す。
- 往復が 1 回増える。上限 1,000 ms は変えない。「．さくら」（i686 helper 経由）での所要を実機の確かめで記録の時刻から読む。

## 10. 設計討議の結果（2026-11-01・全議題を推奨案で承認）

- 議題 1（SSP の側の見比べ）→ SSP を使っているセッションに断り、排他テストの机の中で、ぱすたさんを降ろさず `\![call,ghost,．さくら]` で横に呼び出す。「．さくら」の `ssp_shiori_log.txt` のバイト位置を控えて、2.x でも SSP が古い名前を問うか・並びを差分で読む。済んだら閉じて元のままを確かめてから机を返す（design.md C5）。
- 議題 2（所要の合否の線）→ areka でメニューを 3 回以上開いて毎回の所要と上限までの余裕を記録し、時間切れが 1 回でも出たら実装を止めて報告する（design.md C5・Performance）。
