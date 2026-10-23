# 設計の検証: shiori2-protocol

- 検証日: 2026-10-23
- 対象: `.kiro/specs/completed/shiori2-protocol/design.md`（要件 `requirements.md`・調べ `research.md` 10 節〔SSP 2.9.23 の測り〕・11 節〔設計の決め〕）
- 照らした steering: `product.md`・`tech.md`・`structure.md`・`logging.md`・`roadmap.md`（C11 ⑧ の行と約束）
- 進め方: `kiro-validate-design`（分析 → 大きな問題 → よい点 → 判定）。対話なしで判定まで出す。

## まとめ

設計は、SSP 2.9.23 で測った形（研究記録 10 節）をほぼそのまま写しており、3.0 のゴーストへの違いを「読み込みの直後の `GET Version SHIORI/2.6` 1 本」だけに抑える筋道が、今のコードの上で成り立っている。大きな問題は 1 つだけで、要求の種類（台本か資源か）を「名前が `On` で始まるか」で分ける規則が、外から頼まれた任意の名前のイベント（SSTP・MCP の `raise_event`）で崩れる点。直しは小さく、設計の骨組みは変わらないので **GO** とする。

## コードと突き合わせて確かめたこと

| 確かめたこと | 結果 | 根拠 |
|---|---|---|
| areka が今 NOTIFY で送るもの | `OnInitialize`・`basewareversion`・`OnSecondChange`（台詞を出せないとき）・`OnClose`（強制終了のとき）・`OnDestroy`（読み直し）。`ownerghostname`・`otherghostname`・`capability` は送っていない | `crates/areka-kanade/src/schedule/events.rs` の表と `ALLOWED_EVENT_IDS` |
| SSP の形との対応 | `OnInitialize`・`OnDestroy`・`OnSecondChange` は SSP も `GET Sentence SHIORI/2.2`。`basewareversion` は SSP も 2.x の相手へ送らない（測 5）。SSP が `NOTIFY OwnerGhostName/OtherGhostName SHIORI/2.3` で送る 2 つと、送らない `capability` は、areka が送っていないので組まなくてよい。設計の Non-Goals と表の「送らない」の行はこれと合っている | 研究記録 10.2 節・設計「2.x の要求の形」 |
| 資源の問い合わせ | 14 名（`username`・`*.caption`・`*.popupmenu.visible`・`homeurl`・`useorigin1`）。どれも `On` で始まらず、どれも References を持たない | `schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`・`resource_get` |
| `basewareversion` を送らないと起動が止まらないか | 2.x の枝が `Ok(())` を返せば、kanade の起動の運行は今どおり `Notified` で起動を終える | `schedule/boot.rs`（BootVersion＋Notified） |
| 2.x の状態の行を今の読みで読めるか | 今の `parse_status_code` は版の字を見ず、最初に数として読める字句を状態とする。`SHIORI/2.6 200 OK` も読める | `shiori-host32-host/src/shiori3.rs` |
| `SecurityOrigin` の値 | 内部の要求の値は `null`。SSP の `GET String`・`TRANSLATE` の値 `null` と同じ | `request_headers.rs` の `RequestHeaders::internal` |
| 翻訳への 400 で台詞が失われないか | 今の 3.0 と同じく、エラー応答なら展開済みの元の台詞で進む。「．さくら」の `TRANSLATE` への 400（研究記録 10.2 節）でも話す | `schedule/translate.rs` の `read_reply` |
| 3.0 の道が確かめの 1 本のほかは変わらないか | 変わらない。版の分岐は `get_with_headers`・`notify_with_headers` の中だけで、3.0 の枝は今の `encode_and_note`・`parse_and_note` を通る。確かめの答えは `note_response` へ回さないので、文字コードの採用（emo2 の pasta で 3 本目から UTF-8）も今のまま | `client.rs`・`shiori_wiring.rs` の注記 |
| 読み直しで確かめ直すか | LOAD を送るのは `real_connect` の 1 か所だけなので、読み直しも切り替えも新しい接続になり、確かめ直す | `MsgTag::Load` の出現は `shiori_wiring.rs` だけ |
| 構造の検査を壊さないか | `the_one_way_event_body_contains_no_adoption_path`（`notify_with_headers` の本文に `encode_and_note(` があり、`parse_and_note`・`note_response`・`parse_response` が無い）と `the_connection_receives_the_already_decided_negotiator`（`CharsetNegotiator::new` が 1 回）は、設計の書き方なら緑のまま | `client.rs` の 722 行目付近・`shiori_wiring_charset_tests.rs` |
| `ShioriConnection` の書き下し | 設計が挙げる 3 ファイル（`common_window_actor.rs`・`idle_pump_test.rs`・`real_helper_test.rs`）と `shiori_wiring.rs` だけ。`shiori_inproc.rs` は説明の中で名前を出すだけで、組んでいない | `ShioriConnection {` の検索 |
| 3.0 の検体の確かめへの答え | YAYA の標準の受け口（emily4 の `yaya_shiori3.dic`）は、どの答えも先頭の行を `SHIORI/3.0` で書く。設計の規則なら 3.0 に決まる。里々は文書のとおり 3.0 の形で答える見込み（実機の 1 回で見る） | `vendors/sample_ghost/emily4.nar` の中 |

## 大きな問題

### 問題 1: 台本か資源かを「名前が `On` で始まるか」で分けると、外から頼まれた任意の名前のイベントが資源の問い合わせとして送られる

- **何が起きるか**: 設計の `route` は、GET のうち `On` で始まらない名前をすべて `GET String SHIORI/2.5`（`ID:` に名前）で送る。ところが SSTP の NOTIFY と MCP の `raise_event` は、どんな名前のイベントでも受け付けて GET で送る（`EventId::Sstp`・入口の検査は「空でない・改行や NUL を含まない」だけ。`crates/areka-kanade/src/sstp.rs` の `validate_notify`、`actor.rs` の送出の関門）。`On` で始まらない名前のイベントを 2.x のゴーストへ頼むと、台本を求める `GET Sentence`＋`Event:` ではなく資源の問い合わせになり、ゴーストは答えられない。
- **なぜ大事か**: v0.1.0 のゴールは「SSP 並みの自動試験」で、MCP の `raise_event` はその主な道具。要件 3 は「areka が今送っている要求」をすべて 2.x の書き方に組み替えることを求めており、外から頼まれたイベントもその 1 つ。また、資源の名前の一覧を kanade の `ALLOWED_RESOURCE_IDS` とは別の「`On` で始まらないもの」という暗黙の規則でもう一度持つことになり、二つが食い違っても誰も気づかない。
- **直し方の案**: 資源かどうかを、kanade が持つ正本の一覧で決める。kanade の `shiori/real.rs`（本 spec が既に直すファイル）は `crate::schedule::resources::is_allowed_resource_id(id)` を読める（`actor.rs` の送出の関門と同じ判定・`schedule/` も `msg.rs` も直さない）。そこで決めた種類を `Shiori3Client` へ渡し（例: `with_version` と同じ形の 1 つの手、または `ShioriVersion::Shiori2` に添える値）、`shiori2::route` は「資源と言われたら `GET String`・それ以外の GET は `GET Sentence`（`OnTranslate` は `TRANSLATE`）」とする。`ShioriBackend` の口と偽物は変わらない。この形を採らないなら、少なくとも §8 の表に「`On` で始まらない外からのイベントは `GET String` になる（SSP では測っていない）」と書き、その振る舞いを決定論のテストで固定する。
- **要件**: 3.1・3.3・3.7・7.1
- **設計の場所**: 「Components and Interfaces」→「shiori2」→「私の手 `route`」と、その下の「決めた根拠」。「System Flows」の「1 本の要求の組み替え」の図

## よい点

1. **3.0 のゴーストへの違いを 1 本の要求に閉じ込める筋が、コードの事実で裏づけられている**。確かめを `real_connect` の LOAD の直後（kanade の最初の要求より前）に置き、答えを文字コードの採用に使わないと決めたことで、emo2 の pasta の要求の `Charset` の並び（2 本が Shift_JIS・3 本目から UTF-8）まで今のまま保てる。既存の 2 つの構造の検査の綴りの条件まで先に読んで守っている。
2. **測った形をそのまま写し、測っていない所と SSP との違いを表で分けている**。`BaseEvent` を書かない理由・`Sender: areka` のまま・確かめの `Charset` を SSP の UTF-8 固定にしない理由・`OnClose` が見込みであることが、それぞれ根拠つきで書かれている。新しい依存は無く、2.x の書き方は純粋な関数として新しいファイルに分け、1 ファイル 1,000 行の目安と兄弟のテストの決まりにも合っている。

## 軽い指摘（議題にしなくてもよい・実装のタスクで拾えば足りる）

- **記録の「ゴーストの名前」**: 要件 2.4 は「ゴーストの名前」を INFO に残すと書くが、設計はフォルダ名（例 `dot_sakura`）を残す。`runtime.rs` に触らない理由は筋が通っているので、`verification.md` に「名前の欄はフォルダ名」と一言書けば足りる。
- **確かめが時間切れになったとき**: 要件 2.5 のとおり 3.0 で続けるが、SHIORI が止まっていれば次の要求も上限（既定 60 秒）まで待つので、起動の失敗に気づくまでが今より最大 60 秒延びる。設計の「LOAD の待ちと同じ種類の危うさ」の注に、この延び方も書いておくとよい。
- **表の注の書きすぎ**: 「2.x の要求の形」の資源の行の「Reference の付く資源（`homeurl` など）」は、今の areka では資源の問い合わせに References を付けないので起きない。「付く資源は今は無い」と直すと読み違えない。
- **ウェーブの約束との照らし**: 設計が触るファイルは、ロードマップの C11 ⑧ の行に挙がるものより広い（kanade の `shiori/real.rs`・`areka-ghost` の `shiori_wiring.rs`・kanade のテスト 3 ファイル・`README.md`）。C11 のほかの 7 本の持ち物とは重ならない（`runtime.rs` は ④ の持ち物で、設計は触らない）。逆に ⑧ の行が挙げる `shiori_inproc.rs`・`actor.rs` には触らない。どちらも約束に反しない。

## 判定

- **判定: GO**
- **理由**: 測った SSP の形と今のコードの事実の上に、3.0 のゴーストを変えずに 2.x を足す筋が通っている。問題 1 は送る行の選び方の 1 か所で、直しても部品の分け方・依存の向き・試験の組み立ては変わらない。
- **次の段**: 設計の話し合い（`kiro-design-discussion`）で問題 1 の直し方（正本の一覧で決めるか、`On` の規則のまま限界を書いて固定するか）を決め、設計に反映してから `/kiro-spec-tasks shiori2-protocol` へ進む。
