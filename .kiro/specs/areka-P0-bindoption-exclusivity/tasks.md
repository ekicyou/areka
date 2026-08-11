# Implementation Plan

- [x] 1. 実行前提の整備とベースライン確認
  - 是正前の実機観測ログ（`target/bindopt-debug-observation.log`）を、cargo が消去しない位置（`%LOCALAPPDATA%\areka-diag\` 配下の日付付きディレクトリ）へ**最初に**複製し、その絶対パスを控える（実装後は是正前ログを再採取できないため、較正の唯一の材料）
  - i686 host-32 成果物をビルドし、`cargo test --workspace` がベースラインで exit 0 の全緑になることを確認する（steering 既知の前提: host-32 成果物が無いと workspace テストが赤になる）
  - 観測可能な完了状態: 複製先の絶対パスに保全ログが存在し（是正前の握り潰しを含む内容）、ベースライン全緑の実行結果が記録されている
  - _Requirements: 3.5_

- [x] 2. bindoption 3 値読み取りとモデル拡張（採取層）
  - `bindoption*.group` の値をカテゴリ名とオプション欄に分け、オプション欄を `+` 区切りで分解して mustselect / multiple を個別に認識する読み取りへ置換する（併記時は両方へ転記し情報を落とさない）
  - 転記モデルへ multiple カテゴリ名のスコープ別保持を追加し、mustselect と対称の所属照会を備える（採取層の既存所属照会は正典下でも正しいため退役させない）
  - 未知オプション語は読み流し、カテゴリ名空・オプション欄欠落は収録対象外とする寛容パースを維持する
  - 宣言ゼロの shell でも全カテゴリ既定として成立させ、同一入力に同一結果を返す決定論を保つ
  - 読み取りの決定論檻を 3 値・`+` 区切り・未知語・不完全値・宣言ゼロのマトリクスへ拡張し、旧「multiple は取り込まない」を謳う既存テストは期待値ごと反転させる
  - 観測可能な完了状態: multiple 宣言が転記モデルへスコープ別に収録され、拡張した採取層の決定論テストが全緑
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 4.2, 4.5_

- [x] 3. 【統合】3 値ポリシー語彙の導入と全呼出元の atomic 追随（挙動不変）
  - カテゴリの選択ポリシーを 3 値で明示する型と、bindoption 宣言をスコープ別に運ぶ名前付き搬送型を新設する
  - 判定器の構築を名前表 2 本＋搬送型の形へ変更し、併記は複数可を優先する導出順でポリシーを返す単一アクセサを備える
  - **判定器側の**旧 2 値述語のみを退役させ、その消費者を新アクセサでの同値判定へ機械的に置換する（この段階では挙動を変えない）。採取層の同名の所属照会は存置する
  - 退役述語の消費者は判定器のクレート内に閉じない——適用分岐・判定器の in-crate テストに加え、**起動時資産構築の檻がクレート越しに消費している**ため、これらを同一変更で追随させる（漏らすとコンパイルが通らない）
  - 判定器構築の全呼出元（本番 1・テスト 7）を同一変更で追随させ、中間の暫定実装を挟まない
  - 起動時資産構築で multiple 集合を転記モデルから構築し搬送型に載せる
  - 空構築の署名は変更しない——並走 spec の無干渉前提であり、アプリ側の空構築呼出 4 箇所を無改変のままコンパイルが通ることで検証される
  - 観測可能な完了状態: workspace がコンパイルを通り、既存テストが挙動変更なしで全緑（本タスクは意味を変えない機械的置換）
  - _Requirements: 1.1, 3.5_
  - _Boundary: policy 判定, 適用結線, 資産構築_

- [x] 4. 適用分岐の 3 値化と mustselect 解除不可の実装
  - 着衣指示では、複数可と宣言されていないカテゴリ（mustselect／非宣言）を排他置換へ流し、非宣言カテゴリの bind を高々 1 個に保つ
  - mustselect カテゴリへの脱衣指示は bind 集合を変えずに読み飛ばし、実機の既定ログ水準で見える警告として痕跡を残す
  - 複数可カテゴリの着衣は従来どおり加算、mustselect 以外の脱衣は従来どおり除去へ流す
  - 分岐後段（冪等ガード・単一発行点・変更時のみ発行・非表示/未知 scope の縮退・名前解決不能の読み飛ばし）は無改変で通す
  - 観測可能な完了状態: 非宣言カテゴリへ 2 パーツ連続で着衣指示を流すと bind 集合が後勝ち 1 個になり、mustselect への脱衣指示で集合が変わらず警告ログが出る。既定の意味が反転しても既存テストは全緑のまま（既存構成は 1 カテゴリ 1 パーツで排他と加算が集合同値のため期待値不変）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 5. 決定論テストの檻
- [x] 5.1 (P) ポリシー導出のマトリクス檻
  - mustselect 宣言・複数可宣言・非宣言・併記・未知カテゴリの各ケースで導出結果を固定し、本体／相方の名前空間隔離も併せて固定する
  - 空構築ではすべて既定ポリシーとなり、名前解決が常に不成立で適用に到達しないことを檻に固定する
  - 既存の名前解決・カテゴリ ID 収集・加算の檻は無改変で維持する（構築形の追随はタスク 3 で完了済み。本タスクは檻の追加のみ）
  - GPU 実描画・実窓・実 DPI・実 SHIORI・sleep・実時間待機に依存しない
  - 観測可能な完了状態: 導出マトリクスの全ケースが決定論テストで緑
  - _Requirements: 4.2, 4.3, 4.4_
  - _Boundary: policy 判定の檻_
  - _Depends: 3_

- [x] 5.2 (P) 適用経路の最小再現檻とポリシー×着脱の全網羅
  - 非宣言カテゴリの同一カテゴリ 2 パーツへ着衣指示を 2 回流し、後勝ち 1 個になることを最小再現として固定する（旧欠陥挙動からの反転の檻）
  - ポリシー 3 種×着衣／脱衣の 6 組合せをすべて固定する（mustselect の脱衣は集合不変かつ警告の文言・水準まで固定、複数可カテゴリの同一カテゴリ 2 パーツ共存を含む）
  - 排他置換でも変更時のみ発行・同値適用は非発行・非表示 scope は状態のみ更新・解決不能は読み飛ばしとなる既存流儀を檻で維持する
  - 旧 2 値語彙で命名された異カテゴリ加算の檻を新語彙へ改名し、検証実体は保持する
  - 観測可能な完了状態: 6 組合せと最小再現が決定論テストで緑になり、旧語彙のテスト名が残っていない
  - _Requirements: 4.1, 4.3, 4.4, 4.5, 2.1, 2.2, 2.4, 2.5, 2.6, 2.7, 3.1, 3.2, 3.3, 3.4_
  - _Boundary: 適用結線の檻_
  - _Depends: 4_

- [x] 5.3 (P) 貫通シナリオの回帰錨
  - 既存の貫通シナリオの期待値が不変であることを確認し、旧 2 値前提のコメント語彙を新正典へ更新する（構築形の追随はタスク 3 で完了済み）
  - 複数可宣言カテゴリで 2 パーツが共存したまま貫通することを新規シナリオとして追加する
  - 観測可能な完了状態: 貫通テストが全緑で、複数可カテゴリの共存錨が新規に緑
  - _Requirements: 3.3, 3.5, 4.4_
  - _Boundary: 貫通シナリオ檻_
  - _Depends: 4_

- [ ] 6. 文書整合と全体検証
- [x] 6.1 旧 2 値前提の記述と外部引用の一掃（対象は bind 経路に限る）
  - 「非 mustselect は加算」「複数可／非宣言は収録しない」を前提とする doc コメントを 3 値正典の記述へ更新する
  - 完了済み spec を根拠に引く裸の識別子引用を、本 spec の識別子を冠した引用へ差し替える（完了済み spec の文書自体は改変しない）
  - 対象は設計書の変更ファイル一覧に列挙された bind 経路のファイル群に**閉じる**——採取層・判定器・適用・状態の doc とその檻、資産構築とその檻のみ。同じ識別子文字列は workspace 全体で他 spec 領分にも多数現れるため、bind 経路外には触れない
  - 排他置換の実体側は doc の限定文言のみ汎用へ改め、コードは無改変に保つ
  - 観測可能な完了状態: bind 経路のファイル群から旧 2 値前提の主張と裸の外部引用が消え、bind 経路外の同名引用は一件も変更されておらず、workspace が緑のまま
  - _Requirements: 6.1, 4.5_

- [x] 6.2 既定の意味反転に対する全数監査と workspace 全緑
  - 複数可集合が空の非空判定器を使う全既存テストを列挙し、同一カテゴリ複数着衣を暗黙前提にしていないことを実測で確認する
  - i686 host-32 成果物のもとで workspace テスト全体を実行し、決定論的な緑を確認する
  - 観測可能な完了状態: 監査台帳が更新され `cargo test --workspace` が exit 0
  - _Requirements: 3.5, 4.4_

- [ ] 7. 実機サインオフと完了登記
- [x] 7.1 サインオフ判定の走査手順と既知ケース較正
  - 保全ログを時刻順 1 パスで走査し、各まばたき発火の直前の適用痕跡と id が一致すること・痕跡なしの発火を赤とすることを判定する手順を用意する
  - 飽和不在の判定（カテゴリ別の変更回数差と末尾時刻の近接）を、対象ゴースト固有の較正値として明記した形で用意する
  - タスク 1 で保全した是正前ログに対して両判定が必ず赤になることを較正として確認する
  - 観測可能な完了状態: 是正前ログで赤・想定形のログで緑となる判定手順が再実行可能な形で存在する
  - _Requirements: 5.1, 5.2, 5.4_
  - _Depends: 1_

- [ ] 7.2 emo2 実機実走とサインオフ受け入れ記録
  - 実 pasta.dll・辞書込みフルゴーストを絶対パスで起動し、有界の自動終了と適用痕跡が写る記録水準で実走する
  - 実走ログへ判定手順を適用し、共存痕跡の不在と飽和パターンの不在を判定する
  - ジト目へ切り替わった後に別の表情変更で正しく切り替わることを目視で確認する
  - 判定・実測値・実施条件・ログの所在を受け入れ記録として残し、不一致があれば記録して是正まで完了としない
  - 観測可能な完了状態: 受け入れ記録に全判定の結果が記され、共存痕跡と飽和パターンがいずれも不在
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 2.3_
  - _Depends: 7.1_
  - _Blocked: J1・J2 は PASS へ反転したが J3（目視）が FAIL——bind 集合から外れた bind 種アニメの最終コマ残留を掃除する経路が存在せず、外れた ID のコマが無条件に合成される（looper.rs:304-315 で残留・:244-247 で以後無評価／state.rs:394 が古い pattern をそのまま再発行／plan.rs:302-309 が bind 非所属の ID も描画対象へ合流）。根因は現行 Boundary の外（design.md :42/:74/:206 が looper.rs を無改変と宣言）ゆえスコープ拡大の開発者裁定が要る。詳細は real-machine-signoff.md §4_

- [x] 7.3 完了登記の起草と覆し記録の検証
  - 覆した完了済み spec の該当判断と本 spec による是正、および mustselect の起動時充足を shell 宣言へ委譲する既存縮退の語彙を、完了時にロードマップへ載せる追記文として起草する
  - 覆しの根拠（正典・実機証拠）と裁定の記録が設計文書に残っていることを確認する
  - 観測可能な完了状態: 追記文の草案が spec 配下に用意され、設計文書の覆し記録と裁定記録の所在が確認済み
  - _Requirements: 6.2, 6.3, 6.4_

- [ ] 8. 【スコープ追加】bind から外れたパーツの残留コマの掃除（2026-08-11 裁定・Requirement 7）
- [x] 8.1 発行前の残留除去（状態側）
  - bind 集合の変化時に「外れた ID の集合」を求め、表示指令を組む**前**に保持コマを取り除く
  - 除去対象は bind 集合由来の ID に限り、bind に属さないアニメの保持コマへ影響を与えない
  - 取り除いた事実を実機の既定ログ水準で見える形で残す（無言の状態変更を作らない）
  - 排他置換で外れたパーツが同じ発行で消えることを決定論テストで固定し、除去ログの文言・水準も固定する
  - 観測可能な完了状態: 保持コマのある ID を排他置換で外すと、発行される表示指令にその ID が含まれない
  - _Requirements: 7.1, 7.2, 7.4, 7.5, 7.6_
  - _Boundary: 状態側の発行前処理_

- [x] 8.2 再生中に外れた ID の停止（再生側）
  - 再生の進行段で、bind 種でありながら現在の bind 集合に属さない ID を停止相当（保持コマ除去＋再生除去）へ落とす
  - 発火ゲート自体は無改変で通し、bind に属さないアニメの再生には影響を与えない
  - 再生中に bind から外した ID が次の評価で復活しないことを決定論テストで固定する
  - 観測可能な完了状態: 再生中の ID を bind から外すと次の評価でコマと再生がともに消える
  - _Requirements: 7.3, 7.4, 7.5, 7.6_
  - _Boundary: 再生の進行段_
  - _Depends: 8.1_

- [x] 8.3 合成計画の合流条件の補強（最後の砦）
  - 描画対象へ合流する条件に「bind 種のアニメなら現在の bind 集合に属すること」を加える
  - 合成順・重ね順は無改変に保ち、bind に属さないアニメは従来どおり無条件で合流させる
  - bind 非所属の ID の保持コマを渡しても合成計画にその層が現れないことを決定論テストで固定し、既存の合成順の検証は無改変で維持する
  - 観測可能な完了状態: 上流 2 段が漏れても表示が壊れない不変量が檻で成立する
  - _Requirements: 7.1, 7.4, 7.6_
  - _Boundary: 合成計画の合流条件_
  - _Depends: 8.1_

- [ ] 8.4 全体検証と実機サインオフの再実施
  - i686 成果物のもとで workspace テスト全体を実行し決定論的な緑を確認する
  - 実機を再実走し、判定手順で共存痕跡と飽和パターンの不在を再確認したうえで、ジト目からの復帰を目視で確認する
  - 受け入れ記録を再実走の実測値で更新し、不一致があれば記録して是正まで完了としない
  - 観測可能な完了状態: 受け入れ記録の全判定が合格で `cargo test --workspace` が exit 0
  - _Requirements: 7.7, 5.3, 5.5, 5.6, 3.5_
  - _Depends: 8.2, 8.3_

## Implementation Notes

- **task 1（2026-08-11）**: 是正前の実機観測ログを `C:\Users\maz-o\AppData\Local\areka-diag\bindopt-20260811-101835\bindopt-debug-observation.log` へ保全済み（465,055 bytes・md5 `d910e4dc7d1ebd350ec0b1fa6bb8f4df`・worktree の `target/bindopt-debug-observation.log` と一致）。タスク 7.1 の既知ケース較正はこの絶対パスを使う。
- **task 1（2026-08-11）**: ベースライン全緑を確認。手順＝PowerShell で `cargo build -p shiori-host32-helper -p shiori-host32-testdll --target i686-pc-windows-msvc` → `cargo test --workspace`（exit 0）。i686 ビルドは必ず PowerShell（Git Bash の coreutils `link.exe` が MSVC link を遮蔽する）。
- **task 2（2026-08-11）**: `BindGroupDefaults` は転記順保持・重複可（design.md:490）。同一カテゴリの重複宣言は Vec へ重複 push されるので、集合化はタスク 3 の `BindOptionDecls` 構築（BTreeSet 変換）が担う——既存 mustselect と同型。
- **task 2（2026-08-11）**: D8 の引用差し替え対象は `D11`/`R4.5`/`Req 4.5` の 3 文字列に限る。同じファイルに残る `D2`・`R1.x`・`Req 1.x`/`Req 3.x` 等は bind 経路外（`.name`/`.default`/マウント解決）の別 spec 由来ゆえ無改変が正しい（タスク 6.1 でも同じ線引きを守ること）。
- **task 3（2026-08-11）**: 設計の変更ファイル台帳に `crates/areka-seriko/src/lib.rs` が漏れていた。`lib.rs` の `mod bind;` は非公開なので、`BindChoicePolicy`/`BindOptionDecls` を `pub use` で再エクスポートしないと areka クレートと `tests/bind_e2e.rs` からコンパイル不能。タスク 6.1 で設計台帳へ追記すること。
- **task 3（2026-08-11）**: `cargo test --workspace` に**別 spec 所有の間欠赤**がある——`actor_dispatch_tests.rs:741` の `non_shell_broadcast_reception_is_benign_debug_no_warn_error` が約 1/6〜1/8 の頻度で捕捉ログ 0 件になる。原因は `areka-P0-test-cage-determinism`（W6.9）に登記済みの tracing callsite 毒化（seriko の `capture_logs`/`capture_logs_flow` が `rebuild_interest_cache()` 未硬化）。本 spec の変更とは因果独立。全緑判定時は再実行して切り分けること。
- **task 4（2026-08-11）**: RED で旧欠陥挙動を実測採取した——非宣言カテゴリへ 2 度着衣すると `BindSet([1100,1207,1400,1402])`（まばたき 2 パーツ共存）になり、正典期待値 `BindSet([1100,1207,1402])` と乖離。実機観測の症状そのもの。是正後は後勝ち 1 個。設計の「テスト影響監査」は正しく、既存テストは期待値無改変で全緑のまま（既存フィクスチャは全て 1 カテゴリ 1 パーツ）。
- **task 4（2026-08-11）**: mustselect 脱衣の前段ガードは `return ControlFlow::Continue(())` で `match outcome` より手前に置いてあり、`commit_bind`／`emit_display`／info マーカーのいずれにも到達しない。集合不変かつ発行なしが構造的に保証される。タスク 5.2 で檻に固定する warn 文言は `actor.rs:383` の逐語 `seriko: mustselect カテゴリの脱衣指示を無視（正典・解除不可・bindopt 3.2）`。
- **task 5.1（2026-08-11）**: **間欠赤の実体はテスト 1 本ではなく 3 本**だった（レビューの独立実測）。`actor_dispatch_tests.rs:741` に加え、`actor_bind_loop_tests.rs:125` の `bind_apply_on_shown_emits_show_and_info_marker`（`level=INFO` 捕捉）と `actor::dispatch_tests::wait_broadcast_reception_is_benign_debug_no_warn_error` も同じログ捕捉クラスで落ちる（本 spec の新規テストを除外した状態でも 10 回中 2 回再現＝因果独立）。`areka-P0-test-cage-determinism`（W6.9）の登記対象を 1 本から 3 本へ広げる材料。
- **task 5.2（2026-08-11）**: 直積 6 セルの所在——MustSelect×on=`bind_mustselect_second_on_replaces_prior_part_in_category`（既存）／Default×on=`bind_default_category_second_on_replaces_prior_part`（task 4）／Multiple×on=`bind_multiple_category_two_parts_coexist_via_actor`（新規・`hair_multiple_resolver`）／MustSelect×off=`bind_mustselect_off_is_ignored_with_warn`（task 4）／Default×off=`bind_default_category_off_removes_part`（新規）／Multiple×off=`bind_multiple_category_off_removes_only_that_part`（新規）。既存の `bind_apply_on_shown_emits_show_and_info_marker` は 1 カテゴリ 1 パーツ構成で置換が実際に起きないため、排他置換の Changed=info は `bind_default_exclusive_replace_emits_show_and_info_marker` で別途固定した。
- **task 5.2（2026-08-11）**: **旧語彙のテスト名が `tests/bind_e2e.rs:434` に 1 本残っている**——`non_mustselect_explicit_on_off_is_additive_end_to_end`。本タスクの境界外（貫通シナリオ檻）ゆえ**タスク 5.3 で改名すること**（要件 4.5・bindopt D6）。`actor_bind_loop_tests.rs` 側の旧語彙テスト名はゼロ。
- **task 5.2（2026-08-11）**: 間欠赤の再実測でレート更新——`cargo test -p areka-seriko --lib` 20 回反復で 4 回 RED（内訳: `bind_apply_on_shown_emits_show_and_info_marker` 2／`non_shell_broadcast_reception_is_benign_debug_no_warn_error` 1／`wait_broadcast_reception_is_benign_debug_no_warn_error` 1）。**新規に追加した檻は 1 度も落ちていない**。W6.9 への申し送り対象は seriko 3 本＋`areka-emo-compose` の `scale::ratio_tests::mul_degradation_emits_warn_log` 1 本の計 4 本へ拡大。
- **task 5.3（2026-08-11）**: 貫通シナリオの改名は `non_mustselect_explicit_on_off_is_additive_end_to_end` → `default_category_explicit_on_then_off_removes_part_end_to_end`（実体＝既定カテゴリ「紅」の唯一パーツが明示 on で載り明示 off で外れる＝正典の解除可）。resolver・スクリプト・ticks・期待値は逐語同一。新規錨は `multiple_category_two_parts_coexist_end_to_end`（`sakura_multiple={髪飾り}`・発行列 `[{1207},{1207,1700},{1207,1700,1701}]`）。
- **task 5.3（2026-08-11）**: **`R8.1` の引用が 1 件消えた**（`completed/areka-P0-mayuna-compose` の Requirement 8＝非退行/additive 制約）。`R4.5` と同一文に同居し、その文の主張「紅は排他置換を受けず加算」が新正典下で虚偽になったため文ごと書き換えた巻き添え。置換後の文が非退行の含意をより正確に明示しており情報の喪失なし——**タスク 6.1 で復活させる必要はない**（レビューで裏取り済み）。
- **task 6.1（2026-08-11）**: 書き換えた主張は 4 件——`actor.rs` の `empty()` 根拠（主張は真だが根拠文言が旧前提）／`state.rs:329` の「mustselect カテゴリの」限定（**虚偽化していた**——`apply_bind_exclusive` は MustSelect と Default の両方から呼ばれる）／`bind.rs` の `category_ids` doc（mustselect 専用の道具に読める）／`actor_bind_loop_tests.rs` のモジュール doc（実体は 3 分岐）。いずれもレビューが実コードで裏取り済み。
- **task 6.1（2026-08-11）**: bind 経路に残った `R4.5`／`要件 4.5` 5 件（`actor.rs:439/444/449/453`・`state.rs:272`）は **`completed/areka-P0-balloon-face-cue` の Requirement 4 の 5**（バルーン面 key が解決できないときの縮退）であり mayuna-compose の R4.5 とは別物。**触らないのが正しい**（レビューが completed spec の現物で裏取り済み）。
- **【開発者裁定待ち・本 spec 未処理】** bind 経路のファイルに mayuna-compose 由来と思われる裸引用が残る（`actor.rs:171` の `D4`・`:312` の `D1/D10`・`:325/328` の `D8②`・`:342/346/349` の `D7`・`:414` の `D5`、`actor_bind_loop_tests.rs:11/15` の `D1`・`D8`）。**本 spec の `D1`〜`D8` と番号が正面衝突**しており、設計 D8 が防ごうとした事象そのもの。ただし実測すると出所が単一 spec に定まらない（`D4`＝broadcast/cue 系・`D7`＝scope 写像・`D8②〜⑤`＝bind 類別など複数由来が混在）。**誤った spec 名を冠するのは裸で残すより有害**なため本 spec では無改変とした。設計 D8 の対象集合は `D11`/`R4.5`/`Req 4.5` の 3 文字列に明示限定されており受入条件は満たしている。出所の全数特定と接頭辞付与は別途裁定すること。
- **task 6.2（2026-08-11）**: 全数監査の結論は**設計の主張どおり**——同一カテゴリ複数着衣を暗黙前提にする既存テストはゼロ。ただし設計時点の台帳（research.md §7.1）に**穴が 2 件**あった: ①実 emo2 の本番構築経路の判定器が数えられていなかった（実 fixture は非宣言カテゴリに**まばたき 4・キラリ 2・髪飾り 2** パーツを持ち seriko 側 fixture より条件が厳しい）／②判定軸に「**静的既定 bind 集合との交わり**」が立っていなかった（排他置換は `現在集合 − category_ids ∪ {対象}` ゆえ、同カテゴリの別パーツが `static_binds` で載っていれば加算と食い違う）。いずれも結論は変わらない。再監査台帳は research.md §11。
- **task 6.2（2026-08-11）**: `BindResolver::new` の呼出元は設計時点の **8 → 現在 13** 箇所（本 spec が檻を追加した結果）。`empty()` は 40 箇所でレビューが独立に数え直して台帳と完全一致。
- **task 6.2（2026-08-11）**: **本番挙動の記録**——既定の意味反転で実 emo2 において新たに排他となるのは非宣言かつ複数パーツの **まばたき・キラリ・髪飾り** の 3 カテゴリ。髪飾りは `1800.default,1` で起動時オンのため、今後 `\![bind,髪飾り,ボンボン,1]` を送ると既定オンのリボンが自動で外れる（正典どおりの意図した是正）。
- **task 6.2（2026-08-11）**: 要件 3.5 の「決定論的緑」の実態——`cargo test --workspace` 単発では**約 1/3〜1/4 の確率で exit 101** になり得る（既知間欠赤 4 本のいずれか）。本 spec が追加した檻は一度も落ちていない。完了ゲートで exit 0 を主張する際は再実行での切り分けが要る。
- **task 7.1（2026-08-11）**: 判定手順は `signoff-scan.py`（標準ライブラリのみ・PASS で exit 0／FAIL で 1／判定不能で 2）と `signoff-procedure.md`。既知ケース較正＝是正前保全ログで **J1=FAIL（違反 109/169・不一致で発火した id は 1400 と 1402）・J2=FAIL（Changed まばたき 3 / 目 25＝差 22・末尾時刻差 316.848 秒）**。レビュアーが自作の赤/緑 8 ケースで J1・J2 が独立判定であること・沈黙を緑としないこと・時刻順 1 パスであることを実証済み。
- **task 7.1（2026-08-11）**: ログ実形の実測——適用痕跡は `scope=0 category=<素の日本語> part=… id=… on=…`、発火は `scope="0" … animation_id=…`（**引用符の表記ゆれあり**）。発火は `loop 抽選発火` のみを数える（`loop 末尾残留` は必ず同 id の抽選発火に後続するため除外しても欠陥を隠さない）。requirements.md §決定的証拠 の「1400×156・1402×182」は抽選発火＋末尾残留の**合算値**で、抽選発火のみなら 78/91。
- **【開発者裁定待ち・7.2 の着手前に必要】** **J2 条件A（Changed 回数差 ≤ 2）は是正が正しく効いても赤になり得る**。実装者とレビュアーが独立に、保全ログから排他置換を再現した想定形で **まばたき 22 / 目 25＝差 3 > 2** を再現した。原因は emo2 の目→まばたきが多対一（目 べそ1300・笑顔1303・静観1304 がいずれも まばたき ----1403 へ写る＝descript.txt:41-53 で裏取り済み）で、目だけ変わりまばたきが据え置きになる遷移が数回生じるため。合成の忠実性は「同じ手法を目カテゴリへ適用すると保全ログの実測 25 と完全一致する」ことで検算済み。是正前は差 22・末尾差 316.8 秒なので飽和とは桁が違う。**閾値を 2→3〜4 へ改訂するか、条件Aを「片側恒久沈黙の不在」へ言い換えるかの裁定が要る**（設計 J2 は無断で緩めていない）。
- **task 7.3（2026-08-11）**: 覆し記録は `design.md:144-152`（§mayuna-compose 覆しの記録）、裁定記録は `design.md:110-113`（D1 ＋残余の正典乖離の登記）。**覆される当の判断の現物と設計の引用行は完全一致**（`completed/areka-P0-mayuna-compose/requirements.md:85` の R4.5・`design.md:68` の 3 分類表・`:142` の D11——実装者とレビュアーが独立に逐語照合）。
- **task 7.3（2026-08-11）**: **roadmap:57 の完了 spec 総数 151 は stale**。`7df461b`（file-slimming・PR#103）が completed へ 1 本足しながら更新を落としたため。計数規約は `.kiro/specs/completed/` の**直下エントリ数**（ディレクトリ 151 ＋ 単体ファイル `graphics-rendering-stability.md` 1 ＝ **現在 152**）で、ディレクトリ数 151 を採ると 1 ずれる。**本 spec 着地後は 153**。`/kiro-complete` はこの値を書くこと。
- **task 7.3（2026-08-11）**: **`/kiro-complete` は `roadmap.md:136` の追記台帳を編集しないこと**。実運用は「追記全文を history へ退避する棚卸のときに台帳へ 1 行足す」で、完了時に足すと本文の全文と重複する。実測: 台帳は (51)〜(57)＋(59) のみ（(55) 欠番・**(58) は全文が `:144` に現存するため不在**）で (60)(61)(62) も不在（全文が `:138`/`:140`/`:142`）。次回棚卸では **(58) と (60)〜(62)** をまとめて足すこと。
- **task 7.3（2026-08-11）**: 台帳の軽微なずれ 1 件——本ファイルの task 4 の申し送りが警告文言の所在を `actor.rs:383` と記していたが、現物は **`actor.rs:386`**（3 行ドリフト）。
- **task 7.2（2026-08-11・不合格）**: 実機サインオフは **J1=PASS・J2=PASS・J3=FAIL**。bind 層は実機で証明された（共存痕跡 109/169 → **0/66**・まばたき Changed 3 回 → **20 回**・J2 差 22 → **1**・末尾差 316.8 秒 → **0.000 秒**）が、**表情固着は再現**。要件 5.6 により未完了。ログ＝`%LOCALAPPDATA%\areka-diag\bindopt-signoff-20260811-181137\bindopt-signoff.log`（md5 `5B14F166078FD67A4B9D2D8A49C28233`）。受け入れ記録＝`real-machine-signoff.md`。
- **task 7.2（2026-08-11）**: **要件の因果モデルに一段の誤りがあった**——`requirements.md:21` は固着の機構（不透明最終コマが覆う）を正しく書きながら「排他置換すれば積み上がりが消える」と結論した。しかし emo2 の 14xx は `pattern0` を持たず**視覚寄与が `PatternState` 単独**（`surfaces.txt:84-86`）なので、bind 集合を正しても**既に置かれた残留コマには何の影響も及ばない**。裁定でスコープを広げる場合、requirements.md:21 の因果記述と design.md :42/:74/:206 の「looper.rs 無改変」宣言・Boundary Context を**同時に**改訂すること（片肺にしない）。
- **task 7.2（2026-08-11）**: スロー再生と CPU 20% 超は **`recompose-budget`（W6.75）の領分**と確定。seriko の進行は壁時計どおり（183ms／定義 172ms）で、遅いのは presenter 側の **1 コマ約 500ms**・`ShowSurface` 404 件すべて `cache_hit=false`・`areka-emo-present` に throttle 実装なし＝実コスト。**固着の根因ではないが症状を悪化させている**（閉じ目コマが約 500ms 滞留）。roadmap:90 が予告した「bind 着地後に budget の CPU 上昇を (a)bind 同根/(b)活性集合へ切り分ける」材料が本実走ログで揃った。
- **task 8.1（2026-08-11）**: RED が実機の固着を逐語で捉えた——排他置換後の pattern に `1402: PatternFrame { surface_id: 1413 }`（＝ジト目の不透明コマ）が残存。除去は `state.rs` の `drop_residual_frames` で、**`current_pattern(...).clone()` の前**・`removed` は **`dynamic_binds.insert` の前**に旧集合から確定（どちらかを取り違えると修正が無意味化する）。除去ログの文言＝`seriko: bind から外れた ID の保持コマを除去`（info）。
- **task 8.1（2026-08-11）**: **7.4（非 bind への無影響）は構造的に成立**——`BindSet` への流入は `assets.rs:338` の bindgroup default 由来と `actor.rs:356` の名前解決済み ID の 2 経路のみで、純 `random` の `interval` ID は混じらない（`apply_bind_exclusive` の `category_ids` は filter にしか使われず加算しない）。レビュアーが独立に裏取り済み。
- **task 8.1（2026-08-11）**: **8.1 単体では「再生中に外れた ID」は直らない**——次の評価で再生側がコマを置き直すため。8.1 が直すのは「再生が既に終わって残留している ID」（emo2 の 1402 がこれ）。**タスク 8.2 が必須**。
- **task 8.1（2026-08-11）**: `capture_logs` を `state_test_support.rs` へ複製した（既存 15 箇所と同じ流儀）。`areka-P0-test-cage-determinism`（W6.9）のログ捕捉ハーネス是正対象が 1 本増える。
- **task 8.2（2026-08-11）**: RED が **8.1 と 8.2 の両方が必要な理由を直接観測**した——8.1 が除去した `1412` のコマを進行相が次 tick で置き直す。停止は `looper.rs:288-311` で**コマ除去と playback 除去の両方**を行い、位置は `frame_at` の**手前**。bind 種の判定は `LoopTrigger::BindRandom`（`table.rs:36-47`）で、`Interval::Bind`・`Other` は `table.rs:110-136` で非採録＝**完全かつ唯一の判定**（純 random を巻き込まない）。slot 限定はしない——発火ゲートが slot 非区別なので進行相はその鏡映にする。
- **task 8.2（2026-08-11）**: **空虚な檻を 1 本作って差し戻された**。負の錨 `progress_phase_emits_no_drop_marker_while_bind_holds` が、健全系で捕捉が空（`logs=[]`）ゆえ不在主張が**恒真**になっていた。是正＝**同一捕捉内の陽性対照**（`loop 抽選発火` の存在主張）＋**ログ経路に依らない状態反証**（保持コマの残存）の併置。ガード条件反転の変異で looper セットの検出が **5/6 → 6/6** へ。**負の錨には必ず陽性対照を同一捕捉内に置くこと**（`actor_dispatch_tests.rs:741-745` が既存の手本）。
- **【W6.9 申し送り・要追加】** `areka-P0-test-cage-determinism` の間欠赤対象へ **`bind_default_exclusive_replace_emits_show_and_info_marker`**（`actor_bind_loop_tests.rs`・本 spec がタスク 5.2 で追加）を追加すること——full-lib 120 回反復で ×2 を実測。既知 4 本に入っていない。本 spec が新規追加した `progress_phase_bind_drop_emits_info_marker` と `residual_frame_removal_emits_info_marker` も同じ脆弱クラス（`capture_logs` の `rebuild_interest_cache()` 未硬化）。`capture_logs` の複製も `state_test_support.rs`／`looper_tests.rs` の 2 本増えて計 16 箇所。
- **task 8.3（2026-08-11）**: 合流ゲートは `plan.rs:326` の `if is_bind_animation(master, id) && !binds.contains(id) { debug!; continue; }`。**依存の逆流なし**——`SurfaceMaster.animations`（`normalized.rs:85`）が転記層の `Interval` を保持しており、`Bind`/`BindRandom` と `Random`/`Other`（`areka-parsers/src/shell/model.rs:119-136`）を emo-compose 内だけで区別できる。seriko の `LoopTrigger::BindRandom` と同じ区別が得られる。
- **task 8.3（2026-08-11）**: `&& !binds.contains(id)` は**論理冗長（等価変異）**——層(i) の filter が pattern0 の有無を条件にせず「bind 種かつ bind 所属」を先取りするため、ゲート到達時には恒真。設計 D9-3 の字義どおりの自己文書化として残してある（層(i) 変更に対する防御）。落としても全緑になるのは正常。
- **task 8.3（2026-08-11）**: 変異検査は**両側**で荷重を確認済み——①ゲート撤去で負の錨が赤（漏らし過ぎ側）／③`is_bind_animation` を落として全コマに bind 所属を要求すると新規 3 本＋既存 2 本が赤（**締め過ぎ側**＝純 random が描画されなくなる回帰を検出）。負の錨には非空の陽性対照を同一検証内に併置してある（8.2 の空虚な檻の再発防止）。
- **task 8.3（2026-08-11）**: 「最後の砦」の守備範囲は**当 surface に定義のある bind 種 ID に限る**（未定義 ID は fail-open で従来どおり合流）。7.4 を守るための必然で、仕様どおりの限界として記録。
