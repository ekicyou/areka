# Implementation Plan

- [x] 1. 実装前提とスコープ境界の確認
  - `punctuation_wait` ハックと drive の生スクリプト診断ログが残存していないかを確認し、残っていれば撤去する
  - wintf `Typewriter` widget・テキストレイアウト/描画（縦書き・折返し・フォントメトリクス）・mayuna/bind 合成・実行時サーフェスリサイズ・動的制御フロー（一時停止・選択肢）が本 spec の変更範囲に含まれないことを確認する（隣接 spec への委譲を保つ）
  - 観測可能な完了条件: 前提確認結果（ハックが不在または撤去済み・対象外領域に変更が入っていない）が記録される
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [ ] 2. Foundation: cue envelope の拡張（duration・Wait・ClearAll・配送エンベロープ）
- [x] 2.1 cue envelope へ再生時間を第一級データとして追加し、既存資産の後方互換を保証する
  - あらゆる cue が再生時間（瞬時は明示的 0）を保持できるようにし、「再生時間フィールドを持たない cue」という概念を作らない
  - 再生時間データを SakuraScript 固有の意味に解釈しない不透明な秒数として扱う
  - 再生時間フィールドを持たない既存のシリアライズ済み cue データを読み込んだ場合、再生時間 0 として解釈できる
  - 観測可能な完了条件: 再生時間欠落の旧シリアライズ資産を読み込むと再生時間 0 として復元され、新規往復（roundtrip）でも値が保たれることがテストで示される
  - _Requirements: 1.1, 1.2, 1.5, 9.3, 9.4, 9.5_

- [x] 2.2 純粋な待ちと全消去を第一級コマンドとして追加する
  - 明示的な待ち（action を持たず再生時間のみを持つ）を独立したコマンド種別として表現できるようにする
  - 全スコープのバルーン表示を一括で消去するコマンドを、特定スコープのみを消去する既存コマンドと区別して追加する
  - 既存コマンド種別のワイヤ形式を変えない加算的な拡張として実装する
  - 観測可能な完了条件: 新設した 2 種のコマンドが既存コマンド一覧と共に往復シリアライズでき、既存コマンドのワイヤ形式が変わっていないことがテストで示される
  - _Requirements: 5.1, 5.2, 6.1, 6.3_
  - _Depends: 2.1_

- [x] 2.3 演者へ cue を届ける配送エンベロープを演出タイミング基盤側の型として定義する
  - 発火時刻・演者・コマンド・再生時間を運ぶ搬送体を、さくらスクリプト固有の層でなく演出タイミング基盤側に置く
  - 観測可能な完了条件: 搬送体が再生時間フィールドを保持し、コマンド由来の値を無変形で運べることがテストで示される
  - _Requirements: 1.1_
  - _Depends: 2.1, 2.2_

- [ ] 3. Foundation: 自己完結した絶対時刻台本と占有終了判定
- [x] 3.1 台本に絶対開始時刻を持たせ、自己完結した絶対時刻台本にする
  - 台本自身が絶対開始時刻を保持できるようにし、各 cue の相対発火時刻と組み合わせて絶対発火時刻・talk の絶対終了時刻を台本のみから復元可能にする
  - 観測可能な完了条件: 絶対開始時刻を持つ台本から、任意の cue の絶対発火時刻と talk の絶対終了時刻が計算で導出できることがテストで示される
  - _Requirements: 1.7_
  - _Depends: 2.1_

- [x] 3.2 台本から時刻スケジュールへの変換を単一の経路へ統合し、占有終了に基づく完了判定を成立させる
  - 台本の先頭の待ちを消してしまう既存の正規化と、さくらスクリプト側の独自実装という 2 つの変換を廃し、絶対アンカーと相対発火時刻を保ったまま時刻スケジュールへ変換する唯一の経路にする
  - 変換時に再生時間が非有限（NaN・無限大）または負である場合、安全な既定値（瞬時＝0）へ丸める
  - 変換した各 cue の発火時刻と再生時間から占有終了時刻を導出し、スケジュール側で保持する
  - スケジュールの完了判定を、全 cue を配り終えた瞬間でなく、この占有終了時刻に到達したかどうかへ改める
  - 観測可能な完了条件: 先頭に待ちを含む台本を変換しても待ちが消えないこと、非有限/負の再生時間を与えた cue が丸められた値で下流へ届くこと、末尾に待ちを持つ台本を注入時刻で駆動したとき全 cue の配送完了時点ではまだ未完了と判定され占有終了時刻に達して初めて完了と判定されることがテストで示される
  - _Requirements: 1.3, 1.6, 1.8, 2.5, 11.1_
  - _Depends: 3.1_

- [ ] 4. Core: cue 再生ランタイムの一本化
- [x] 4.1 演者非依存の出力契約と relevance 判定の単一権威を演出タイミング基盤へ定義する
  - どの演者にも実装できる単一の出力契約（cue を受け取る手段）を用意し、既存の 2 種類に分かれていた出力契約を統合する
  - どの cue がどの演者の担当かを判定する権威を単一の場所へ集約する
  - 観測可能な完了条件: 単一の出力契約を実装したダミー演者が、relevance 判定結果に基づいて担当 cue を選別できることがテストで示される
  - _Requirements: 2.4, 11.3_
  - _Depends: 2.3_

- [x] 4.2 cue 再生ランタイムの状態機械骨格（バリア seam・選択肢先積み）を構築する
  - 既存の 2 箇所に分かれていた cue 再生の状態管理（再生中・停止中・入力待ち・選択肢待ち・完了）を、演出タイミング基盤側の受動的なランタイムへ一本化する
  - 外部解決待ちの停止点（バリア）と選択肢の先積みを、動的な一時停止/再開の状態は持ち込まずに最小限の範囲で移植する
  - 観測可能な完了条件: バリアに到達すると停止し、外部からの解決通知で再開することがテストで示される
  - _Requirements: 11.2_
  - _Depends: 3.2, 4.1_

- [x] 4.3 cue 再生ランタイムの broadcast 配送と占有終了検知を実装する
  - 準備完了した cue を、登録された全出力先へ同一絶対時刻で一斉配送する
  - スケジュールの占有終了判定を用いて、ランタイム自身が完了を検知できるようにする
  - 外部から時刻を注入して進行させ、出力先の登録、完了問い合わせ、中断・破棄を行うための操作を確定する
  - 観測可能な完了条件: 1 つの台本を複数の登録済み出力先へ流すと全出力先が同一の cue 列を同一絶対時刻で受信し、末尾の待ちを含む台本では占有終了に達するまで完了通知が出ないことがテストで示される
  - _Requirements: 2.1, 9.1_
  - _Depends: 4.2_

- [ ] 5. Core: テキスト再生時間の計算と台本への焼き込み
- [x] 5.1 (P) テキストから暗黙の再生時間を算出する単一の純関数を実装する
  - 同一テキスト入力に対して常に同一の再生時間を返す、実時間・描画状態に依存しない純粋な計算を実装する
  - 1 文字あたりのノミナル時間定数をこの計算の中だけで定義し、他層で重複定義させない
  - 明示的な待ちの時間は畳み込まず、暗黙の文字数由来の時間のみを算出する
  - 演出タイミング基盤側は per-char の意味を内包しない汎用のまま保つ（定数はこの計算専用）
  - 観測可能な完了条件: 空文字列・複数バイト文字混在・同一入力の再計算という代表的な入力について、期待される再生時間が単体テストで固定される
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 9.2_
  - _Boundary: areka-sakura duration 計算_

- [x] 5.2 コンパイルが再生時間を台本へ焼き込み、明示的な待ちを第一級 cue として発行し、talk 冒頭へ全消去を前置する
  - テキスト cue へ算出した再生時間を付与し、後続 cue の発火時刻をテキスト再生完了後へ確定させる
  - 明示的な待ちを、offset へ吸収して消すのでなく、独立した第一級の待ち cue として発行し、後続整列のため時間を進める
  - talk 台本の先頭へ、全スコープを一括消去するコマンドを単一件だけ前置する
  - 観測可能な完了条件: 文字送り直後に明示的な待ちが続く台本をコンパイルすると、待ちが cue として台本に残り、その待ちを含めた分だけ後続 cue の発火時刻が遅れ、末尾のみの待ちでも台本から消えないことがテストで示される
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.3, 6.1, 6.2, 11.4_
  - _Depends: 2.2, 3.1, 5.1_

- [ ] 6. Core: 演者が duration honor 契約と出力契約を実装する
- [x] 6.1 (P) バルーンテキスト表現者が配送された再生時間へ reveal を服従させ、honor 契約と全消去/対象消去を実装する
  - 独自の文字送り時間定数を捨て、配送された cue の再生時間から文字送りのタイミングを導出する（文字数に対して概ねその時間で表示し切る）
  - 対象スコープのみを消去するコマンドと全スコープを消去するコマンドを区別して処理する
  - 自身の担当でない cue を受け取った場合、その動作は無視しつつ再生時間は honor し、新たなローカルな遅延を生じさせない
  - 単一の出力契約を実装し、relevance 判定で自身の担当 cue を選別する
  - 観測可能な完了条件: 再生時間 0 かつ文字数 1 以上の cue で全文字が同時可視になること、文字数 0 の cue で追記が起きず割り算エラーが起きないこと、担当外の cue（表情切替や待ち）を受けても文字送りに余計な遅延が乗らないことがテストで示される
  - _Requirements: 1.8, 2.2, 2.3, 5.4, 6.4, 7.1, 7.2, 7.3, 7.4, 7.5, 11.6_
  - _Boundary: areka-emo-text_
  - _Depends: 4.1_

- [x] 6.2 (P) シェル表現者が単一の出力契約を実装し、担当外 cue の再生時間を honor する
  - 表情・面切替系の cue のみを自身の担当として処理し、それ以外（テキスト・待ち・消去等）は動作を無視しつつ再生時間を honor する
  - 単一の出力契約を実装し、relevance 判定で自身の担当 cue を選別する
  - 観測可能な完了条件: 担当外 cue の受信が異常でなく正常な経路として扱われ、ログレベルが実害のない水準に保たれることがテストで示される
  - _Requirements: 2.2, 2.3, 5.4, 11.6_
  - _Boundary: areka-seriko_
  - _Depends: 4.1_

- [ ] 7. Integration: さくらスクリプト前段を front-end と talk アクター glue へ縮小する
- [x] 7.1 talk アクターが cue 再生ランタイムを包み、注入時刻を送り、絶対開始時刻を刻印する
  - talk の再生開始時にコンパイル済み台本へ絶対開始時刻を刻印し、cue 再生ランタイムへ渡す
  - talk アクターは配送・状態機械・完了判定を自前実装せず、cue 再生ランタイムへ委譲する
  - さくらスクリプト前段に残っていた搬送体と relevance 判定の型を、演出タイミング基盤へ移設済みのものへ差し替えて前段の型定義を縮小する
  - 観測可能な完了条件: 同一台本を 2 回異なる時刻で再生開始すると、それぞれ異なる絶対発火時刻で cue が配送されることがテストで示される
  - _Requirements: 9.1, 11.4_
  - _Depends: 4.3, 5.2_

- [x] 7.2 talk の完了通知を占有終了まで遅らせ、早期終了しないことを注入時刻テストで固定する
  - talk の完了通知（次の talk へ進行してよい合図）を、cue の配送完了でなく cue 再生ランタイムの占有終了検知に基づいて発火させる
  - 末尾の待ちや最終テキストの再生時間が、talk の終端で切り捨てられないことを、実際に時刻を注入して進行させるテストで確認する
  - 観測可能な完了条件: 末尾に明示的な待ちを持つ talk と、待ちを持たない末尾テキストのみの talk の両方について、完了通知が cue 配送完了時点でなく再生時間を含めた絶対終了時刻に達した時点で発火することが、時刻注入テストで示される
  - _Requirements: 2.5_
  - _Depends: 7.1_

- [ ] 8. Integration: 実配線（kanade→dispatcher→ランタイム→演者）と旧世代撤去
- [x] 8.1 診断既定の配線を単一出力契約へ改め、実配線の前提を broadcast 対応へ更新する
  - 診断用の既定出力先が同一 cue を二重にログしないよう、単一の出力契約による一本の配線へ改める
  - 実配線が新設コマンド種別を扱えるようにし、実配線のテストが「1 つの cue は 1 つの出力先だけに届く」という前提から「全出力先が全 cue を受け取り、担当のみ動作する」という前提へ更新される
  - 観測可能な完了条件: 実配線を通した統合テストで、表情切替を含む台本を流すと出力先ログが cue ごとに 1 回だけ記録され、テキスト出力先も表情切替 cue を受信するが動作はしない（再生時間のみ honor する）ことが確認される
  - _Requirements: 2.1, 2.4, 9.3_
  - _Depends: 6.1, 6.2, 7.1_

- [x] 8.2 (P) 旧世代の並行 cue 再生エンジンを撤去する
  - 生きたアプリケーションに配線されていない旧世代の cue 配送・状態管理一式（および台本正規化の旧実装）を削除する
  - 観測可能な完了条件: 撤去対象モジュールへの参照がワークスペース全体から消え、他クレートのビルド・既存テストに影響が無いことが確認される
  - _Requirements: 11.5_
  - _Boundary: wintf ecs/cue_
  - _Depends: 4.3, 3.2_

- [ ] 9. Validation: 横断統合テストと決定論回帰
- [ ] 9.1 broadcast 配送と relevance 選別の統合テストを追加する
  - 1 つの台本を cue 再生ランタイムへ流し、登録した複数の出力先が全て同一の cue 列を受信することを確認する
  - 観測可能な完了条件: 表情切替とテキストが混在する台本で、両出力先が受信する cue 列が一致することがテストで示される
  - _Requirements: 2.1, 2.4_
  - _Depends: 8.1_

- [ ] 9.2 honor 契約の統合テストを追加する（葉の no-op・ライフサイクル早期終了防止・relevance の網羅整合）
  - 担当でない cue を受け取っても新たなローカル遅延が生じないこと、末尾の待ちを含む talk がライフサイクルレベルで早期終了しないこと（時刻注入で確認）、全コマンド種別について relevance 判定と各演者の動作対象判定が食い違わないことを確認する
  - talk のコンパイル時に確定する終端理由（正常終了/中断等の区別）と、ライフサイクルレベルの絶対終了「時刻」が別概念であることを型で固定し、両者を混同する回帰を防ぐ
  - 観測可能な完了条件: 担当外 cue 受信後も担当 cue の発火時刻が遅延しないこと、末尾待ちを含む talk の完了通知が絶対終了時刻まで発火しないこと、全コマンド種別で動作する演者が高々 1 つであることがそれぞれテストで示される
  - _Requirements: 2.2, 2.3, 2.5, 5.3, 5.4, 7.5_
  - _Depends: 7.2, 8.1_

- [ ] 9.3 表情切替の再生同期を確認する統合テストを追加する
  - テキスト再生完了後の絶対時刻で表情切替 cue が発火し、全出力先が同一絶対時刻の同一台本を受け取ることを確認する
  - 観測可能な完了条件: テキストと表情切替を含む台本で、表情切替の発火時刻がテキストの再生完了時刻と一致することがテストで示される
  - _Requirements: 1.4_
  - _Depends: 8.1_

- [ ] 9.4 talk 冒頭の全消去とその配送順序を確認する統合テストを追加する
  - 複数スコープに書き込む talk の後に単一スコープへ書き込む talk が続く場合、書き込まれなかったスコープの残存テキストも新しい talk の開始時に消えることを確認する
  - 同一時刻に並ぶ全消去とテキストの配送順序が台本記述順どおりであることを確認する
  - 観測可能な完了条件: 前の talk が複数スコープへ書き込み、次の talk が一部スコープにしか書き込まない場合でも、次の talk 開始時に前の talk の全スコープの表示が消えることがテストで示される
  - _Requirements: 6.2_
  - _Depends: 8.1_

- [ ] 9.5 再生時間搬送のエンドツーエンド決定論テストを追加する
  - コンパイルされた再生時間が、台本正規化・cue 再生ランタイム・出力契約を経て、演者側の reveal タイミングまで無変形に届くことを確認する
  - 観測可能な完了条件: コンパイル時に付与した再生時間の値と、演者側で観測される reveal 完了時刻が、同一の算術で導かれた期待値と一致することがテストで示される
  - _Requirements: 1.1, 7.1_
  - _Depends: 8.1_

- [ ] 10. Validation: 実機受け入れ（人間サインオフ）
  - 実 emo2 ゴーストを実 pasta.dll・実 DPI・絶対パスで起動し、明示的な待ちが一時停止として体感できること（#3）、改行直前の待ちが改行を遅らせること（#4）、新しい talk 開始で前の会話のバルーンテキストが消えること（#6）、表情切替がテキスト再生完了後に同期して発火することを、人間が観測しサインオフする
  - 観測可能な完了条件: 実機での #3・#4・#6・表情同期の観測結果と人間サインオフの記録が残る
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_
  - _Depends: 9.1, 9.2, 9.3, 9.4, 9.5_

## Implementation Notes

- **Task 1（前提確認・Requirements 10.1–10.5）**: `punctuation_wait` は `crates/**` で grep ゼロ（ヒットは `vendors/pasta` サブモジュールと spec/steering 文書のみ＝areka 実コード外）。`crates/areka-sakura/src/drive.rs` の生スクリプト診断ログも不在——`script` は `drive.rs:148` で `areka_parsers::sakura::parse(&script)` へ渡されるのみで、既存 `tracing` 呼び出し（78/141/159/194/200/208/221/235/265/273/303 行）はいずれも構造化エラー/デバッグ経路であり raw script 本文を出力しない。撤去作業は発生せず。対象外領域（wintf `Typewriter`・テキストレイアウト/描画・mayuna/bind・サーフェスリサイズ・動的制御フロー）への変更もゼロ（作業ツリー clean からの着手）。
- **検証コマンド**: `cargo test --workspace`（ベースライン全緑・exit 0 を着手前に確認済）。host-32 の i686 成果物（`target/i686-pc-windows-msvc/debug/{shiori.dll,shiori-host32-helper.exe}`）はビルド済みゆえ workspace テストの前提は充足。
- **リポジトリは rustfmt-clean でない**（HEAD 時点で `cargo fmt --all` は 248 ファイル・`cargo fmt -p dola -- --check` は 40 件の既存 Diff を出す）。`cargo fmt --all` を実行すると本 spec と無関係な整形チャーンが大量混入するため**使わないこと**。整形の是非は本 spec のスコープ外。
- **Task 5.x への申し送り（Task 2.1 レビュー FINDINGS 2）**: `crates/areka-sakura/src/compile.rs` のテストヘルパ `cue_eq` は `actor`/`start_time`/`payload` のみを比較し `duration` を見ない。全 cue が `duration: 0.0` の現状では無害だが、compile がテキスト cue へ D を焼き込む時点で**必ず `duration` 比較へ拡張が必要**（さもなくば compile の決定性テストが D の回帰を素通しする）。
- **emo-text の `ClearAll` 全スコープ消去は Task 2.2 で実装済み**（design.md:169「`ClearAll` は自身の全 actor_states を消去」＋ D4 の relevance 単一権威が強制。`cue_target_of(ClearAll)==Balloon` ゆえ ignore-arm は D4 と矛盾する嘘のコメントになり、`todo!()` は禁止マーカー）。Task 6.1 は reveal の duration 駆動・`char_wait` 撤去・`CueSink` 実装・7.1/7.2/7.3/7.5 を引き続き所有する。
- **`ClearAll` は `actors.clear()` でなく `values_mut()` で各エントリを中身だけ空にすること（load-bearing 不変条件）**: `crates/areka-emo-text/src/actor.rs` の `present_frame` は `state.actors()` を走査して再描画対象を決めるため、マップからエントリを**削除**すると当該スコープが再描画対象から落ち、既に描いたテキストが画面へ残留する＝#6 欠陥そのものを再現する。既存 `Clear` と同じイディオム（エントリは残し中身を空に）を守ること。檻 `clear_all_erases_every_actor_scope_unlike_clear` が変異テストで固定済み。
- **搬送体 `TalkCue` は dola に唯一定義・sakura は re-export**（Task 2.3）: `dola::cue::TalkCue { at, actor, command, duration }`。`Cue`＝ワイヤ型（Serialize/Deserialize・PartialEq 非導出）と `TalkCue`＝実行時投影（PartialEq・serde 非導出）の derive 分割は設計どおりゆえ崩さないこと。`areka_sakura::contract::TalkCue` は dola 型の re-export で、Task 7.1 が前段の型定義を最終的に片付ける。
- **Task 3.2 への申し送り（Task 2.3 レビュー FINDINGS 1）**: `crates/dola/src/cue/command.rs` の `talk_cue_envelope_carries_cue_duration_untransformed` と `talk_cue_duration_is_uniform_across_every_command_variant` は、テスト本体で `TalkCue { .., duration: cue.duration }` を組み立ててから等価を主張する**自己充足的な檻**（Rust の構造体代入が自分自身を主張しているだけ・変異テストで素通しを実測済）。構造の檻としては有効だが「無変形」の behavioral な証明は `crates/areka-sakura/src/drive.rs` 側の `to_bits()` 檻が担っている。**3.2 で canonical 変換を dola へ移す際に、この 2 テストを実変換を通す形へ書き直して檻を load-bearing にすること**。
- **`CueSheet` は named struct・`absolute_start_time` は `#[serde(default)]`**（Task 3.1）: tuple `(Vec<Cue>)`→`{ absolute_start_time: f64, cues: Vec<Cue> }`（serde 形が配列→オブジェクト・永続化資産無しゆえ後方互換対象外）。導出アクセサ `absolute_fire_time(&cue) = absolute_start_time + cue.start_time`、`absolute_end_time() = absolute_start_time + max(start_time + duration)`（空台本はアンカーそのもの＝相対 horizon の下限 0.0 の fold）。`with_absolute_start_time(t)` で dispatch 時刻を刻印（呼び出しは Task 7.1）。**duration の NaN/±inf/負値 clamp はここで行わず 3.2 の canonical 変換（dola ingress 単一権威）が担う**——`absolute_end_time` へ 2 個目の clamp を足さないこと。`CompiledTalk.end`（`TalkEndReason` enum＝終端理由）と `absolute_end_time`（f64＝終了時刻）は別概念（D6）。
- **canonical 変換は `dola::cue::to_talk_schedule(&CueSheet) -> TimedSchedule<TalkCue>`**（Task 3.2）: アンカー=`sheet.absolute_start_time()`（7.1 刻印まで 0.0）、相対 `start_time` 保存（min 正規化しない＝先頭待ちを食わない）、per-cue insert で同一 at FIFO、`clamp_duration(d) = if d.is_finite() && d >= 0.0 { d } else { 0.0 }`（NaN/±inf/負→0・+inf も畳む）を**エンベロープ duration と horizon 累積の両方**へ適用＝dola ingress の単一 clamp 権威（R1.8）。Command→Payload(TalkCue)、Barrier→Barrier、Routing→Routing にルーティング（旧 sakura の skip は廃止）。相対 horizon = `max(start_time + clamp(duration))`（空台本 0.0）。
- **`TimedSchedule` に `horizon: f64` フィールド＋`with_horizon(start, horizon)` ctor**（Task 3.2）: `new(start)` は horizon 0.0 既定（**死んだ wintf `CueQueue` パスの `is_completed` 4 箇所を挙動不変に保つため必須**）。`is_completed() = entries.is_empty() && current_barrier.is_none() && current_offset >= horizon`。`Entry::Payload` シグネチャは不変（duration はエンベロープが保持）。0-duration cue では horizon=最終 offset ゆえ従来挙動を保存（5.2 が非零 duration を焼くと horizon-gating が発効）。
- **sakura の独立変換 `fn to_schedule` は削除済み・drive は `to_talk_schedule` へ委譲**（Task 3.2）。drive のアクター構造（on_tick/完了検知/Close）は据え置き＝shrink は Task 7.1。`compile_sheet` は据え置き（消費者は死んだ wintf ecs/cue のみ・Task 8.2 が撤去）。
- **Task 2.3 の自己充足檻 2 件は sheet_test.rs へ移設し実変換を通す形へ書き直し済み**（Task 3.2 で load-bearing 化・`to_bits()` で無変形を固定）。command.rs には NOTE breadcrumb を残置。
- **`dola::cue::CueSink` 単一トレイト＋`cue_target_of` は dola に唯一定義**（Task 4.1）: `sink.rs` に `pub trait CueSink { fn emit(&mut self, cue: TalkCue); }`（infallible・SurfaceSink/TextSink の役割統合＝broadcast＋演者側 relevance ゆえ役割分割不要）と `cue_target_of`（exhaustive・catch-all 無し・10 variant 全分類）を移設。`areka_sakura::contract::cue_target_of` は dola からの re-export（`drive.rs::on_tick` の import パス不変）。**SurfaceSink/TextSink は据え置き**（消費者 emo-text/seriko/ghost の移行は 6.1/6.2/8.1）。演者側 action ゲートは `cue_target_of` に一致させること（seriko=Shell/emo-text=Balloon・D4 単一権威）。
- **`CuePlayer` は `crates/dola/src/cue/runtime.rs`・受動的注入時刻ランタイム**（Task 4.2）: `from_sheet(&CueSheet)`（内部で唯一の canonical `to_talk_schedule` を通す）／`from_schedule(TimedSchedule<TalkCue>)`、`tick(t)`／`ready()`（Choice 除外の action cue）／`resolve_click`／`resolve_choice(id)`／`skip_barrier`／`state()`／`pending_choices()`／`current_barrier()`／`remaining()`（テスト内観用・完了契約ではない）。`CuePlayerState::{Playing, WaitingForInput, WaitingForChoice, Completed}`（**Paused なし**＝pause/resume は Non-Goal・dola へ持ち込まない）。Choice cue は `pending_choices` へ先積みし `ready()` から除外。`Timeout` バリアは `Playing` 維持＋継続 tick で schedule 自動解除。**dola には logger（tracing）依存が無い**（新規依存禁止）ゆえ wintf の `Error(EmptyChoiceBarrier)` 状態は移植せず、空 choice の `WaitForChoice` は観測可能に `WaitingForChoice` へ入り `skip_barrier()` で脱出可能（silent dead-end でない・compile は現状バリア非生成の防御パス）。
- **`CuePlayer` の broadcast＋完了＋制御 API は Task 4.3 で実装済み**: `register_sink(Box<dyn CueSink>)`（登録順保持）、`tick` 内で ready action cue（Choice 除外）を**全 sink へ選別なく broadcast**（中央 router なし・演者側 relevance が action 選別＝D4）、`is_completed()`（horizon-gated＝entry 枯渇 AND 注入時刻 ≥ 占有 horizon）、`stop()`（`schedule.clear()` で残 entry 破棄・terminal 化）。**配送 1 回ゲート**は `if schedule.remaining() < remaining_before { emit filtered_ready }`＝entry を pop した tick に限り配送（冪等再 tick・Timeout 継続の early-return では ready_buffer 据え置き→remaining 不変→非配送）。`TalkCue.at` は**相対 offset**（無変形）・絶対時刻＝`absolute_start_time + at` を各 sink が同一導出（配送時導出は禁忌）。`Box<dyn CueSink>` は Debug 非実装ゆえ手動 Debug impl。
- **`text_playback_duration` は `crates/areka-sakura/src/duration.rs`**（Task 5.1）: `pub const CHAR_NOMINAL_MS: u64 = 50`（**ワークスペース唯一**の per-char 定数・parser `WAIT_UNIT_MS` とも emo-text `char_wait` とも別概念）、`pub fn text_playback_duration(text: &str) -> f64 = Duration::from_millis(text.chars().count() as u64 * CHAR_NOMINAL_MS).as_secs_f64()`。char（`.chars().count()`）カウント＝byte/UTF-16 でない。空文字→0.0。純粋・決定論（`to_bits()` 檻）・`\_w` は畳まない（`&str` のみゆえ観測不能・合成は 5.2 の compile が担う）。**dola へ per-char 意味を漏らさない**（9.2）。Task 5.2 は compile Text arm でこの D を焼き込む際、Task 2.1 レビューが申し送った `compile.rs` の `cue_eq` を `duration` 比較へ拡張すること。
- **compile は D 焼き込み・Wait 第一級発行・ClearAll 前置済み**（Task 5.2）: Text arm は `duration = text_playback_duration(t)` を焼き込み `offset += D`（後続 cue はテキスト再生完了後に発火）。Wait arm は吸収を廃し `CueCommand::Wait`（action 空・`duration = d.as_secs_f64()`）を発行し `offset += d`（末尾・単独の待ちも台本に残る＝自己完結・`absolute_end_time()` で extent 復元可）。content ≥1 の talk のみ冒頭へ `ClearAll`@0.0/duration0 を単一前置（**空 content talk は空 sheet 維持**＝drive `on_start` の `is_empty()`→即 TalkDone 契約を壊さない）。`emit(scope, offset, duration, command)` にシグネチャ拡張。`cue_eq` テストヘルパは `duration` 比較へ拡張済み。drive.rs/dispatcher.rs/spine_e2e_test.rs の変更は**全て `#[cfg(test)]` 内の期待値 ripple＋import**（本番ロジック不変）。
- **emo-text は reveal を配送 duration へ服従・`dola::CueSink` 実装済み**（Task 6.1）: `state.rs` の reveal は `interval = if N>0 { cue.duration / N } else { 0.0 }`（`extend_chunk` 第3引数 `char_wait`→`interval`）、`TextLayerConfig.char_wait` 撤去（`line_pitch_factor` 残置）、`apply_cue` から `config` 引数除去。D=0→interval 0→全グリフ即時可視、N=0→追記なし・除算しない。担当外 cue（Emote/Wait 等）は action 無視＋新ローカル遅延なし（葉の否定的 no-op）。`actor.rs::apply_cue` は exhaustive match で `Clear`→cue.actor render の `request_clear()`／`ClearAll`→**全 render** の `request_clear()`（`surfaces.values_mut()`・#6 の描画層消去）。`EmoTextSink` は `dola::cue::CueSink` を実装（private `deliver` 共有）＋**転置用に `areka_sakura::TextSink` も暫定併存**（ghost 注入が TextSink 経由ゆえ・**Task 8.1 で TextSink impl を撤去**）。emo-text Cargo.toml に `dola = { path = "../dola" }` 追加（workspace パス依存・循環なし）。reveal テストは D/N 算術で期待値再計算（旧 0.05 リテラル不使用・FP 規律）。
- **seriko は `dola::CueSink` 実装・非 Shell 受信を benign debug 化済み**（Task 6.2）: `SerikoSink` は `dola::cue::CueSink` を実装（private `deliver` 共有）＋**転置用に `areka_sakura::SurfaceSink` も暫定併存**（ghost 注入が SurfaceSink 経由・**Task 8.1 で SurfaceSink impl を撤去**）。broadcast で正常となった非 Shell（Balloon: Text/NewLine/Clear/ClearAll/Choice）・純 Wait・Custom 受信は `warn!`→良性 `debug!`（メッセージも是正・Wait は「どの演者の担当でもない」）。honor＝skip のみ（seriko は reveal/timeline を持たず新ローカル遅延を生じない・状態不変）。**genuine anomaly は据え置き**: Unresolved/Invalid `error!`・NameForm/EntityRef/unknown-Shell `warn!`。seriko Cargo.toml に `dola = { path = "../dola" }` 追加（workspace パス依存・循環なし）。
- **Task 7.1 への申し送り（Wait の None 腕ノイズ）**: 現状 drive の pre-broadcast `on_tick` は `cue_target_of(Wait)==None` を `tracing::error!("unclassifiable cue command; skipping")` 腕でスキップする（action 空ゆえスキップ自体は正しいが、5.2 で Wait cue が発行され始めた今、**Wait ごとに error レベルの誤解を招くノイズ**が出る・完了は horizon が Wait の duration を含むので無影響）。7.1 で drive を `CuePlayer` broadcast へ張り替える際、この None 腕を Wait 用の明示的 action-less no-op 経路へ置き換えてノイズを止めること（6.2 の seriko warn→debug と同系の是正）。
- **drive は `CuePlayer` 委譲の glue へ縮小済み**（Task 7.1）: `TalkDriver` は `TalkPhase::{Idle,Armed,Driving}` を持ち、**最初の注入 Tick** で `sheet.with_absolute_start_time(t)`（dispatch anchor 刻印）→`CuePlayer::from_sheet`→両 sink を `register_sink`（broadcast）→`player.tick(t)`。以降 Tick は `player.tick(t)`。完了は `player.is_completed()`（horizon-gated・自前 entry 枯渇でない）。Close は `player.stop()`。**非有限/単調 tick ガードは維持**（NaN broadcast ハザード遮断）。drive から `cue_target_of` 中央 fan-out は撤去済み（broadcast＋演者側 relevance）。`spawn_talk` の sink 境界は `SurfaceSink`/`TextSink`→`dola::cue::CueSink`（2 引数・両 register）。本番 0-based 初回 tick=0.0→anchor 0.0 で従来挙動と等価。観測: 同一 sheet を anchor 10.0/20.0 で駆動→"bye" が 10.6/20.6 に配送（`same_sheet_started_at_different_times...`）。
- **7.1 の ghost/areka 波及は bound-swap＋test のみ**（本番ロジック不変）: `spawn_talk`→dispatcher→`boot`→areka `ClockedTextSink` の型鎖ゆえ dispatcher/runtime/sink の境界を `CueSink` へ、test sink（RecordingSink/ChannelSink/NoopSink/LogSink）と `ClockedTextSink` に `CueSink` 転送 impl 追加。spine_e2e/dispatcher の中央振り分け前提テストは broadcast-aware（両 sink が全 cue 受信・relevance で action）へ更新。**LogSink は `CueSink` impl 追加のみ・dual-wiring 潰しは未実施（broadcast 下で二重ログ・Task 8.1 で潰す）**。`command_kind` の Wait/ClearAll 腕は既存（7.1 追加でない）。
- **Task 8.1 への申し送り**: 診断既定 `LogSink` は SurfaceSink/TextSink/CueSink の 3 trait を実装し `boot` が両スロットへ dual-wire するため broadcast で cue ごと二重ログ。8.1 で単一 `CueSink` 一本配線へ collapse（cue ごと 1 回ログ）。dispatcher/spine_e2e に per-sink partition assertion（全 variant で action する演者が高々 1・`cue_target_of` 分類と一致）を追加。転置用の `TextSink`/`SurfaceSink` impl（emo-text `EmoTextSink`・seriko `SerikoSink`・areka `ClockedTextSink`・ghost 各 test sink）を撤去し `CueSink` 一本へ。
- **Task 7.2 は drive-level 注入時刻檻を追加（test-only・本番不変）**: 7.1 で既に `settle_after_tick` が `player.is_completed()`（horizon-gated）で `TalkDone` を発火するため、7.2 は 4 つの回帰檻を drive.rs `#[cfg(test)]` へ追加したのみ。(1) 末尾 Wait（`\s[10]hello\_w[800]\e`・horizon 1.05）で entry 枯渇（0.25）では発火せず horizon 1.05 で発火、(2) 末尾 Text（`\s[10]hello\_w[500]world\e`・world@0.75 dur0.25・horizon 1.0）で配送時刻 0.75 でなく start+D=1.0 で発火、(3) tick 源 liveness（entry 枯渇 0.1・horizon 未満 0.5 で withhold・0.7 で発火）、(4) D6 型区別（`done.reason == compile().end == TalkEndReason::Quit`＝理由でなく時間、発火時刻は horizon 由来＝独立 2 事実）。非発火の race-free 証明は `recv_timeout(200ms).is_err()`。期待値は `text_playback_duration`/`Duration::from_millis` 算術で計算（decimal 直書きなし）。変異検証（`is_completed()`→`remaining()==0`）で 4 檻全 fail＝load-bearing 確認済み・ワークフローで実装＋3 レビュー＋変異検証を orchestrate。
- **Task 7.1/7.2 への申し送り（drive glue）**: sakura talk アクターが `CuePlayer` を包む際、`register_sink` で演者 sink を登録し、dispatch 時刻を `CueSheet::with_absolute_start_time` で刻印してから `from_sheet`（または刻印済み sheet を `to_talk_schedule`→`from_schedule`）で構築し、注入時刻を `tick` へ送る。`TalkDone`（自然終了）は `CuePlayer::is_completed()`（horizon-gated）で発火＝**entry 枯渇でなく horizon 到達で完了**（早期終了しない・R2.5）。tick 源（kanade）は entry 枯渇後も horizon 到達まで tick を送り続けること。Close/中断は `stop()` を呼ぶ（`Completed` terminal・interrupt-vs-natural の区別は drive 側 `TalkEndReason` が持つ）。
- **Task 7.2 への申し送り**: drive-level の `TalkDone` を horizon まで遅らせる注入時刻檻と「tick 源は entries 枯渇後も horizon 到達まで tick を送り続ける」liveness 契約は Task 7.2 の領分（3.2 は schedule-level の `is_completed` horizon-gating のみ固定）。
- **GPU/UI-pump 系の稀な flaky（既知・本 spec 起因でない）**: areka-ghost `spine_e2e_test::s5_close_deadline_exceeded_forces_termination_via_tick_injection` 等の多アクター Tick 注入系も fail-fast 全走時にごく稀に単発落ちする（単独/再走で緑・cue 発火は horizon 変更と独立）。 `cargo test --workspace` を fail-fast で回すと、ごく稀に初期の GPU/UI-pump 実行系スイートが 1 件落ちることがある（`--no-fail-fast` で再現せず・dola/sakura 差分と無関係）。dola のみの変更で緑判定する際は `--no-fail-fast` を用い、GPU 系の単発落ちは本 spec の回帰と混同しないこと（memory: areka-no-ci-gpu-tests-in-cargo-test / 低頻度 race）。
- **Task 6.2 への申し送り**: `crates/areka-seriko/src/actor.rs` の `None` 腕は「分類できない cue command を受領」と `warn!` する。`Wait => None` は「分類不能」でなく「どの演者の担当でもない」ゆえ、5.2 が Wait を発行し始めると意味的に不正確な warn がノイズになる。6.2 は warn→良性 debug の格下げと**併せてメッセージ文言も**直すこと。
- **要件 9.3 括弧内の字句は緩い（実装への指摘ではない）**: 「`#[serde(default)]`＝0 はワイヤ省略」は機構として不正確（省略には `skip_serializing_if` が要る）。規範部は「既存 variant のワイヤ形不変・additive 拡張」であり、design §Data Contracts の「新 JSON は 4 フィールド目を持つ」が権威。**素の `#[serde(default)]` が設計正解**で、`skip_serializing_if` の追加こそが逸脱。
- **Task 8.1 で `CueSink` 一本化完了・旧 `SurfaceSink`/`TextSink` は workspace から消滅**: `crates/areka-sakura/src/sink.rs`（trait 2 本＋`MockSink`）を削除し `lib.rs` の `pub mod sink`／`pub use sink::{MockSink, SurfaceSink, TextSink}` を除去。転置並存 impl（emo-text `EmoTextSink`／seriko `SerikoSink`／areka `ClockedTextSink`／ghost `LogSink`）を撤去し `dola::cue::CueSink` 一本へ。**2 スロット boot API（`GhostBootOptions<S, T>`／`boot<S,T>`／`spawn_dispatcher<S,T>`／`DispatcherState<S,T>`）は不変**——dispatcher が talk ごとに sink を `clone()` する（`S: Clone`）ため `Vec<Box<dyn CueSink>>`（非 Clone）へは畳めない＝2 スロット generic が load-bearing。二重ログ解消は**配線層**で: `main.rs::ghost_boot_options` を `GhostBootOptions<LogSink, DiscardSink>` へ（surface=LogSink・text=**新設 production no-op `DiscardSink`**＝`crates/areka-ghost/src/sink.rs`）＝診断 LogSink は cue ごと 1 回ログ（型が「記録 sink は 1 本」をコンパイル時に固定）。檻: ghost `sink.rs::diagnostic_default_wiring_logs_each_cue_exactly_once_through_broadcast`（実 `CuePlayer::from_sheet`→`register_sink`×2→broadcast で ClearAll/Emote/Text が各 1 回・二重配線なら 6）＋ `spine_e2e_test.rs::broadcast_relevance_partition::{every_variant_has_at_most_one_acting_performer_consistent_with_cue_target_of, emote_acts_only_on_seriko_text_performer_receives_but_does_not_act}`（全 10 variant を `cue_target_of` owner table 照合・catch-all 無し compile 檻付き）。`areka-emo-text`/`areka-seriko` の `Cargo.toml` 変更はコメントのみ（新規外部依存なし）。
- **Task 8.2 への申し送り**: 旧世代 wintf `ecs/cue`（`CueQueue`/`dispatch`/`tracker`/`compile_sheet` 消費一式＋関連 `*_test.rs`）撤去は 8.1 のスコープ外＝**未着手**（8.1 は `wintf/src/ecs/cue` に一切触れていない）。8.2 は生きた App 未配線の旧世代を削除する（design §File Structure「wintf/src/ecs/cue [撤去]」・R11.5）。撤去後に他クレートのビルド・既存テスト無影響を確認すること。
- **Task 9.x への申し送り（partition 檻の防御深度・8.1 レビュー観察）**: `spine_e2e_test.rs` の partition 檻は owner table を `cue_target_of` と照合する形で seriko の action gate（＝`cue_target_of` そのもの）と `cue_target_of` 自体を固定する。emo-text の `apply_cue` arm を（`cue_target_of` を触らず）act/ignore 間で動かす発散は本檻では捕まらず、emo-text 側 task 6.1 の honor/state 檻＋網羅 match compile 檻が担う。Task 9.2（honor 契約の網羅整合）で「全 variant について relevance 判定と各演者の動作対象判定が食い違わない」を演者実 arm 側からも固定すると防御深度が上がる（design §Testing honor 契約 ③・任意の増分）。
- **Task 8.2 で旧世代 wintf `ecs/cue` 全撤去＋dola `compile_sheet` 系撤去完了**: `crates/wintf/src/ecs/cue/` ディレクトリ一式（command/component/dispatch/error/mod/queue/registry/systems/tracker）＋`ecs/mod.rs` の `pub mod cue;`＋`tests/ecs.rs` の cue test 8 宣言＋`tests/ecs/cue_*_test.rs` 8 本を削除。旧世代は生きた App に未配線（撤去前の cue/ 外参照は `pub mod cue;` のみ）ゆえ他クレート無影響。「台本正規化の旧実装」= dola `compile_sheet`＋`CompiledCue`＋`CuePayload::into_entry`（compile_sheet 専用ヘルパ）を `sheet.rs` から撤去し `mod.rs` re-export も除去（新 runtime は `to_talk_schedule` を使い compile_sheet 不使用）。`sheet_test.rs` の `compile_sheet_*`/`into_entry_*` 陳腐化テストを除去（`cue_sheet_new_with_nan_start_time_does_not_panic`＝`CueSheet::new` 検証は保全・`to_talk_schedule_preserves_leading_wait_unlike_compile_sheet`→`to_talk_schedule_preserves_leading_wait` に改修し compile_sheet 対照ブロックのみ除去、to_talk_schedule assertion は温存）。`compile_sheet` 言及コメント（`areka-sakura/src/compile.rs`・dola `sheet.rs`）も撤去済みで `git grep -E "compile_sheet|CompiledCue|into_entry|ecs::cue" -- crates/` は ZERO。**残存の `CueQueue`/`EntityRegistry` は dola 新 runtime（`command.rs`/`runtime.rs`/`schedule.rs`）の設計系譜ドキュメント散文のみ（コード参照ゼロ・タスク 4.x 由来・8.2 撤去境界外・「旧 CueQueue」は意図的な設計履歴）＝将来の任意 tidy 対象で 8.2 の欠陥ではない**。`TimedSchedule`/`Entry`/`CueSheet`/`to_talk_schedule`/`CuePlayer`/`CueSink`/`cue_target_of` 等の生き runtime は無傷。全 workspace テスト緑（既知の低頻度 GPU/多アクター flake `spine_e2e_test::s3_helper_liveness_detected` は単独再走で緑・本撤去と無関係）。
