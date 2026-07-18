# Implementation Plan

- [ ] 1. dola cue 語彙増分（Cursor・references・Window・キャリア正準形）
- [x] 1.1 Choice へ references フィールドを追加しワイヤ檻を追随させる
  - `Choice { id, text, references: Vec<String> }` へ `#[serde(default, skip_serializing_if = "Vec::is_empty")]` で拡張する
  - 既存ワイヤ檻の `Choice` 構築行を `references: vec![]` へ機械的追随させる（構築行のみ・期待 JSON リテラルは不変）
  - references 空のシリアライズ形が現行とバイト同一であること、references ありでも `default` で読めることを確認する新規檻を追加する
  - _Requirements: 1.3, 8.1_

- [x] 1.2 Cursor variant を追加する
  - `CueCommand::Cursor { x: String, y: String }` を追加する（単位付き・裸数値・`@` 相対・空の区別を保つ不透明転写）
  - Cursor のワイヤ形檻（`{"Cursor":{"x":"5em","y":"2lh"}}`）を追加する
  - _Requirements: 3.1, 3.2, 8.1_

- [x] 1.3 CueTarget::Window とキャリア正準形（command_carrier/as_command_carrier）を追加する
  - `CueTarget::Window` を additive unit variant として追加する
  - `CueCommand::command_carrier(name, tokens)` コンストラクタと `as_command_carrier(&self)` 抽出子を実装する（`Custom{command,params:Array<String>}` を正準形として単一箇所で構築）
  - 往復同一（`as_command_carrier(&command_carrier(n,t)) == Some((n,t))`）とキャリアのワイヤ形（`{"Custom":{"command":"move","params":["-353","","","0","base","base"]}}`）を檻で固定する
  - _Requirements: 4.1, 4.2, 8.1_

- [x] 1.4 名前権威表 command_target_of と relevance 文言の意図的更新を実装する
  - `command_target_of(name: &str) -> Option<CueTarget>` を新設し、M1 唯一のエントリ `"move" -> Some(CueTarget::Window)` を登記する（未知名は `None`）
  - `cue_target_of` へ `Cursor -> Balloon` アームを追加する
  - `Custom` の rustdoc を「型レベル `None` はコマンド名レベル選別（`command_target_of`）への委譲」へ改訂する（R8.7・settled 側の「誰も action しない」という内部矛盾の解消）
  - `command_target_of("move")==Some(Window)`・未知名==`None`・`as_command_carrier` の非正準 params では `None` を返すことを檻で確認する
  - _Requirements: 4.5, 8.7, 9.3b_
  - _Depends: 1.2, 1.3_

- [ ] 2. choice 配送モデルの意図的更新（案C＝配送列とバッグの責務二分）
- [x] 2.1 CuePlayer の Choice 配送を配送列合流へ変更する
  - `tick` の Choice アームを「`pending_choices` へ積み**かつ** `filtered_ready` へも積む」へ変更し、先積み分離を廃止する
  - bag への積みを配送ゲート（`remaining` 減少判定）と同一条件の内側へ移す（同一時刻の冪等再 tick で重複積みしないよう構造で保証する）
  - `pending_choices`／`resolve_choice`（id 照合・解決時 clear）の型・挙動は変更しない
  - rustdoc の「Choice 除外」文言を配送列合流の記述へ更新する
  - _Requirements: 1.8, 8.6_

- [x] 2.2 配送列檻の対置換とバッグ並存檻を追加する
  - `runtime_test.rs:156-163`（先積み一択檻）を、Choice が NewLine/Cursor と交互のまま配送列へ現れることを確認する檻へ置換する（削除でなく対置換＝非退行の観測を残す）
  - 「bag 内容は tick 列に不変」を assert するバッグ並存檻を新設する
  - 配送列に `\q \n \q \_l \q` 相当の交互配置が保存されることを確認する
  - _Requirements: 8.6, 9.7_
  - _Depends: 2.1_

- [x] 3. (P) sysvar 展開契約の実装（SystemVarSnapshot・DEFAULT_USERNAME・resolve_system_var）
  - `SystemVarSnapshot`（名前→値の決定論順序スナップショット・`get`/`insert`）を新規モジュール `sysvar` に実装する
  - `DEFAULT_USERNAME = "ユーザーさん"` を唯一の定義点として実装する（伺かの伝統的な未指定時デフォルト・対応表記録）
  - `ResolvedVar`（`Text`/`PassThrough`）と純関数 `resolve_system_var(name, vars)` を実装する（値あり→値／`username` 欠落→既定値／未対応名→素通し）
  - `sysvar` モジュールを `areka-sakura` の公開 API へ追加する
  - 値あり／`username` 欠落／未対応名の 3 経路と、同一入力→同一出力（no I/O・決定論）を確認する単体檻を追加する
  - `%username` 既定値「ユーザーさん」を `doc/COMPAT_ARCHITECTURE.md` の対応表へ登記する（正典沈黙・伺かの伝統的デフォルト・design.md「互換裁量の記録」の義務）
  - _Requirements: 7.1, 7.3, 7.4, 7.5, 7.6, 9.4_
  - _Boundary: sysvar 展開（areka-sakura）_

- [ ] 4. compile 増分（5 アーム＋barrier 発行＋除外集合の縮小）
- [x] 4.1 compile 署名を拡張し Choice/Cursor アームを実装する
  - `compile(instructions: &[Instruction], vars: &SystemVarSnapshot) -> CompiledTalk` へ署名変更する（純関数のまま）
  - `Choice{disp,target,references}` → `CueCommand::Choice{id:target,text:disp,references}` の写像を実装する（現在スコープ帰属・記述順保存）
  - `Cursor{x,y}` → `CueCommand::Cursor{x,y}` の写像を実装する（双方空でも発行）
  - `\q` 旧仕様形／`script:` 形は縮退のまま維持することを確認する（追加実装なし・parser 実測で既に `Raw`/不透明転写）
  - fixture のメインメニュー script 断片を直入力し、`\q`/`\_l` から期待どおりの Choice/Cursor cue が得られることを単体檻で確認する
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 1.6, 1.7, 3.1, 3.3, 3.4, 3.5_
  - _Depends: 1.1, 1.2, 3_

- [ ] 4.2 Move/GenericCommand/SystemVar アームと barrier 発行ヘルパを実装する
  - `Move(MoveArgs)`／`GenericCommand{name,raw_args}` → `command_carrier(name, tokens)` の写像を実装する（`\!` 全体が第一級で台本に載る）
  - `SystemVar(name)` → `resolve_system_var` の結果を `Text` cue へ写像する（`duration = text_playback_duration(展開文字列)`・`offset += D`・独立 cue とし隣接 Text と併合しない）
  - barrier 発行ヘルパを新設する: 走査終了後、choice cue が 1 個以上あれば `CuePayload::Barrier(BarrierKind::WaitForChoice{timeout:None})` を最終 offset へ 1 個 append する
  - 既存の台本規則（ClearAll 前置・D 焼込・絶対時刻整列・End/Quit 切詰め）が新アームにも一貫適用されることを確認する
  - compile は allowlist（時間指令系）外のコマンド意味を解釈しないことを再確認し、allowlist 自体は M1 非実導出（語彙保持のみ）であることを `doc/COMPAT_ARCHITECTURE.md` の対応表へ登記する
  - _Requirements: 2.1, 2.2, 2.5, 2.6, 4.1, 4.2, 4.3, 4.4, 7.2, 8.4_
  - _Depends: 4.1_

- [ ] 4.3 除外集合を Raw-only へ縮小し意図的更新の檻を対置換する
  - catch-all を `Raw`＋`#[non_exhaustive]` 未知 variant のみへ縮小する（Choice/Cursor/Move/GenericCommand/SystemVar は卒業）
  - 除外檻 `compile.rs:511-544` を「Raw＋未知 variant のみ 0 cue」を確認する檻へ書き換える（4 語彙の卒業を明示するコメント付き）
  - _Requirements: 8.2, 8.3_
  - _Depends: 4.2_

- [ ] 4.4 compile 決定論檻（メニュー・キャリア・sysvar 展開）を追加する
  - メインメニュー script 直入力→期待列 `[ClearAll, Choice(頻度), NewLine, Choice(位置調整), Cursor(5em,2lh), Choice(閉じる), Barrier(WaitForChoice)]`（順序・at・duration・scope・barrier 唯一性/最終位置）を確認する檻を追加する
  - `\q` 無し台本では barrier が発行されないことを確認する
  - `\1\![move,-353,,,0,base,base]` 直入力→6 トークン（空 2 個保持）・scope"1" のキャリアが得られることを確認する
  - 未知名 `\![raise,OnBoot]`・`\![*]` 単独形→キャリア発行（無音落ちしない）ことを確認する
  - sysvar スナップショット値あり／なしの展開檻を確認する（4 の実装が本テストで初めて実時間非依存・script 直入力から検証可能になる）
  - _Requirements: 9.1, 9.2, 9.3, 9.4_
  - _Depends: 4.3_

- [ ] 5. 選択解決の口とアクター境界の実装
- [ ] 5.1 SakuraMsg::ResolveChoice アームと spawn_talk 署名を実装する
  - `SakuraMsg`（`#[non_exhaustive]`）へ `ResolveChoice { id: String }` を additive アームとして追加する
  - `spawn_talk` の署名を `sinks: Vec<Box<dyn CueSink + Send>>`（S-3・登録順＝broadcast 順）・`system_vars: SystemVarSnapshot`（talk 起動時手渡しの凍結像）を受け取る形へ変更する
  - `TalkDriver::on_start`（`drive.rs:170`）の `compile(&instructions)` 呼び出しを、4.1 で拡張された 2 引数署名 `compile(&instructions, &system_vars)` へ更新する（このタスクは 4.1 の compile 署名変更が着地していないとビルドできない＝並列不可）
  - _Requirements: 2.7, 7.3_
  - _Depends: 3, 4.1_

- [ ] 5.2 ResolveChoice ハンドラと即時 settle を実装する
  - `Driving` 状態で `ResolveChoice{id}` を受けたら `player.resolve_choice(&id)` を呼び、`Some` かつ `is_completed()` ならその場で `TalkDone` を送出する（次 Tick を待たない）
  - `None`（id 不一致・非待機）は記録して継続する。`Armed`/`Idle` への誤投函は警告して継続する（防御枝）
  - barrier 待機中に horizon 越えまで `Tick` を注入しても `TalkDone` が出ないこと（既存構造の再確認・R2.3）と、`ResolveChoice` 成立で再開・即時完了することを統合檻で確認する
  - _Requirements: 2.3, 2.4, 9.8_
  - _Depends: 5.1_

- [ ] 6. ghost boot S-3（sink Vec・provider シーム）
- [ ] 6.1 BootCueSink trait と GhostBootOptions の可変長 sink 化を実装する
  - `BootCueSink: CueSink + Send`（`clone_box`）を定義し、`CueSink + Clone + Send + 'static` への blanket impl を実装する（既存 sink は無改変で適合）
  - `GhostBootOptions.sinks: Vec<Box<dyn BootCueSink>>` へ変更する（`boot` の generic `<S,T>` 境界を撤去）・「2 スロット構造」文言を意図的に更新する
  - 既存 boot 呼出（spine/emo2_boot/tests）を Vec 形へ機械的追随させ、既存 spine テストが緑であることを確認する
  - _Requirements: 8.5_

- [ ] 6.2 system_vars provider シームと既定 provider を実装する
  - `SystemVarSource = Box<dyn Fn() -> SystemVarSnapshot + Send>` を定義する
  - `default_system_vars()`（W1 暫定 provider＝`{"username": DEFAULT_USERNAME}`）を実装する（既定値の二重定義をしない・sysvar 側定数を re-export）
  - dispatcher の `on_start` で `sinks.iter().map(clone_box)` と `(system_vars)()` を取得し `spawn_talk` へ渡す（凍結像の刻印点）
  - talk 開始のたび凍結されたスナップショットが sakura 側へ渡ることを確認する統合檻を追加する
  - _Requirements: 7.3, 7.4_
  - _Depends: 6.1_

- [ ] 7. move 末端（純粋解釈＋UI 適用）
- [ ] 7.1 MoveDirective 型と parse_move_directive 純関数を実装する
  - `MoveDirective`（scope・x/y の `AxisSpec`(Fix|Px)・duration_ms・base(`MoveBase`)・base_offset/move_offset(`RefPoint`)）の完全語彙型を定義する
  - `parse_move_directive(scope, tokens) -> Result<MoveDirective, MoveDegradation>` を実装する（正典省略既定 fix/fix/0/screen/left.top・裸 `base`≡`base.base`）
  - 名前付き `--` 形・`time>0` は記録付き縮退（`MoveDegradation` として分類・語彙は保持）とする
  - 正典省略既定・裸 base 等価・縮退分類のそれぞれを確認する単体檻を追加する
  - 裸 `base`≡`base.base` 等価・名前付き `--` 形/基準 `screen`等/`time>0` の縮退を `doc/COMPAT_ARCHITECTURE.md` の対応表へ登記する
  - _Requirements: 5.2, 5.4_

- [ ] 7.2 basepos 型シームと座標算出を実装する
  - `BaseposResolver` trait と `CanonDefaultBasepos`（x=幅÷2・y=下端）を実装する（宣言 `point.basepos` は追跡 spec `areka-P0-surfaces-basepos` への差替シームとして型のみ予約）
  - 座標算出式を物理 px のみ（`WindowPos.size` のみを源とする）で実装する
  - fixture 検算式（`\1\![move,-353,,,0,base,base]` → `x' = pos0.x + w0/2 - 353 - w1/2`・Y 現状維持）が成立することを確認する
  - 宣言 `point.basepos` の型シーム予約（追跡 spec `areka-P0-surfaces-basepos` への差替点）を `doc/COMPAT_ARCHITECTURE.md` の対応表へ登記する
  - _Requirements: 5.2_
  - _Depends: 7.1_

- [ ] 7.3 MoveCueSink（talk スレッド純粋解釈）を実装する
  - `CueSink` を実装する `MoveCueSink`（`Sender<MoveDirective>`）を新設する
  - `as_command_carrier` と `command_target_of(name)==Some(Window)` かつ `name=="move"` の場合のみ解釈し mpsc へ送出、他は記録付き良性スキップとする
  - `emo2_boot` モジュールへ `move_cue` の mod 宣言を追加する
  - 名前選別が正しく機能する（`"move"` のみ解釈・他は skip）ことを確認する単体檻を追加する
  - _Requirements: 4.5, 8.5_
  - _Depends: 7.1, 1.4_

- [ ] 7.4 apply_move_directive（UI スレッド適用）を実装する
  - `apply_move_directive(world, directive) -> bool` を実装する: scope→`GhostWindows` 解決→basepos シーム経由の座標算出→`move_window_to` 呼出
  - 対象・基準窓が解決できない場合は警告を記録して継続する（`false` を返す）
  - `move_window_to` の `#[allow(dead_code)]` を撤去する
  - headless World（`fake_handle` パターン）＋既知 `WindowPos` で fixture 検算式どおりの物理座標が得られること、バルーン随伴 offset が維持されること、対象不在で warn+false となることを確認する統合檻を追加する
  - `apply_move_directive` の前後で `Anchored`（ドラッグ確定系の単一真実源）がビット同一であることを assert する構造檻を追加する（第二の位置ライター非混入・R6/9.5）
  - _Requirements: 5.1, 5.3, 5.5, 6.1, 6.2, 9.5_
  - _Depends: 7.2, 7.3_

- [ ] 7.5 move 決定論檻の全網羅を仕上げる
  - 7.1〜7.4 で追加した純関数・適用檻を Testing Strategy の「parse_move_directive 檻」「move 経路檻」の全項目（基準語彙の受理/縮退分類・対象不在・バルーン随伴・Anchored 不変）で網羅されていることを確認し、不足があれば追加する
  - _Requirements: 9.5_
  - _Depends: 7.4_

- [ ] 8. (P) emo-text 演者追随の実装
  - `state.rs`/`actor.rs` の網羅 match へ `Cursor` の warn-once スキップアームを追加する（状態不変・記録あり）
  - `Choice` アームの文言を「配送列に第一級で現れる（R8.6 仕様変更）・表示消費は W4」へ更新する（挙動は warn-once スキップのまま＝実機の見た目は不変）
  - `LogSink::command_kind`（ghost）へ Cursor アームを追加する
  - 既存の表示テストが無改変で緑のまま、Cursor cue が記録付きで良性スキップされることを確認する檻を追加する
  - _Requirements: 8.5_
  - _Boundary: 演者追随（areka-emo-text）, ghost LogSink（areka-ghost/src/sink.rs・command_kind アームのみ）_
  - _Depends: 1.2, 1.3_

- [ ] 9. emo2_boot 結線（move 末端の配線）
- [ ] 9.1 MoveCueSink の登録とチャンネル配線を実装する
  - `emo2_boot/mod.rs` で `mpsc::channel::<MoveDirective>()` を生成し `MoveCueSink` を `GhostBootOptions.sinks` へ第 3 要素として登録する
  - `Receiver<MoveDirective>` を `Emo2Wiring` へ受け渡す（`PresentBridge` と同型の配線）
  - _Requirements: 5.1_
  - _Depends: 6.2, 7.3_

- [ ] 9.2 frame 相での drain と適用を配線する
  - frame 相（`emo2_frame_system`）で `MoveDirective` チャンネルを drain し `apply_move_directive` を呼び出す配線を実装する
  - 配線がコンパイル・既存 frame テストと共存することを確認する（決定論的な存在確認は 9.3 の spine e2e で行う・手動実機確認はしない）
  - _Requirements: 5.1_
  - _Depends: 9.1, 7.4_

- [ ] 9.3 spine/boot 呼出テストの S-3 形機械的追随を完了する
  - `spine_e2e_test.rs` ほか boot 呼出テストを新しい Vec-sinks 署名・3 sink 構成へ機械的に追随させる
  - `CueTarget::Window` 追加による網羅 match 波及が `spine_e2e_test.rs` のみであることを確認する（他クレートは catch-all で吸収）
  - 「cue ごと 1 回ログ」診断檻が S-3 後も維持されることを確認する
  - spine e2e へ move cue 発火→`MoveDirective` チャンネル配線が実際に drain されることの決定論的な存在確認（headless・sleep 不使用）を追加する（9.2 の配線が生きていることの自動檻・手動実機確認は Task 11 に一本化）
  - _Requirements: 8.1, 5.1_
  - _Depends: 9.2_

- [ ] 10. 統合決定論檻（配送・barrier解決・未知名縮退・move経路・ワイヤ互換）
- [ ] 10.1 配送列檻を追加する
  - compile→`CuePlayer`＋記録 sink 複数→broadcast 観測順が compile 順と一致すること（Choice が NewLine/Cursor と交互のまま現れる）を確認する
  - 同一 Choice がバッグへも同時に積まれる（責務二分）ことを確認する
  - _Requirements: 9.1, 9.7_
  - _Depends: 2.2, 4.4_

- [ ] 10.2 barrier 停止と解決の統合檻を追加する
  - `spawn_talk` へメニュー script を投入し、horizon 越え Tick でも `TalkDone` が出ないことを確認する
  - `SakuraMsg::ResolveChoice{id}` 投函→再開→`TalkDone{Ended}` 到達（追加 Tick 不要）を確認する
  - 不一致 id の投函では状態が変化しないことを確認する
  - _Requirements: 2.3, 2.4, 9.8_
  - _Depends: 5.2, 10.1_

- [ ] 10.3 未知コマンド名の第一級縮退檻を追加する
  - 未知コマンド名を含む script を配送し、全 sink が受領した上で良性スキップ（記録あり）し talk が完了することを確認する
  - `command_target_of` の partition 不変条件（1 コマンド名の担当は高々 1）を確認する檻を追加する
  - _Requirements: 8.2, 9.3b_
  - _Depends: 1.4, 8, 10.2_

- [ ] 10.4 (P) move 経路の統合檻を追加する
  - `MoveCueSink`→channel→`apply_move_directive`→`move_window_to` のフルパイプラインを headless 環境で駆動する
  - バルーン随伴 offset の維持、対象不在時の warn+false、`Anchored` ビット同一を通しで確認する
  - _Requirements: 5.3, 5.5, 6.1, 6.2, 9.5_
  - _Depends: 9.2_
  - _Boundary: move 末端（areka bin）_

- [ ] 10.5 (P) ワイヤ互換回帰檻を追加する
  - 既存 8 variant のワイヤ檻が期待 JSON リテラル不変のまま緑であること（`references`/`Cursor`/キャリア形追加後も）を確認する
  - 旧資産（`references` フィールドなしの `Choice` シリアライズ）が `default` で読めることを確認する
  - _Requirements: 8.1_
  - _Depends: 1.1, 1.2, 1.3, 9.3_
  - _Boundary: dola cue 語彙増分_

- [ ] 11. 実機での初回起動位置調整サインオフ
  - workspace をビルドし、i686 版 `shiori-host32-helper.exe` を `target/debug/` の areka.exe 隣へ上書きコピーする
  - 実 emo2・実 pasta.dll・実 DPI（≠96 推奨）・絶対パスで起動し、OnFirstBoot 経路でエモ（相方側）がむらさきの左隣へ横移動することを目視確認する
  - dpi=96 は自己整合して座標欠陥を隠すため、実 DPI での確認を省略しない
  - _Requirements: 9.6_
  - _Depends: 10.4_

## Implementation Notes
- 1.2: `CueCommand::Cursor` variant 追加時、dola 内網羅 match `cue_target_of`（sink.rs）が非網羅コンパイルエラーを出すため、機械的追随として `Cursor→Balloon` アームを先行追加済み（設計:159 と一致）。→ Task 1.4 は当該アームが既に存在する前提で、`command_target_of` 新設・`Custom` rustdoc 改訂・檻の追加に集中すること。
- 検証コマンドは crate 単位 `cargo test -p <crate>`（worktree root から）。`--workspace` は host-32 i686 成果物前提のため実装中は使わない。worktree 絶対パス `...\.claude\worktrees\areka-p0-balloon-face-cue-18b0ad` 必須。
- 3: `doc/COMPAT_ARCHITECTURE.md` に既存の対応表が無かったため §8「沈黙ルール対応表」を新設し `%username`→`ユーザーさん` を登記済み。→ Task 4.2/7.1/7.2 の対応表登記は §8 の既存テーブルへ行追記する形でよい。`DEFAULT_USERNAME` の定義点は sysvar.rs のみ（Task 6.2 の provider は re-export で二重定義しない）。
- 4.1: (a) `drive.rs` on_start と一部テストの compile 呼び出しに `&SystemVarSnapshot::default()` の暫定ブリッジを差した → **Task 5.1** で実 snapshot threading へ差し替える。(b) drive.rs の空シート即時TalkDone檻2件のフィラーを `\q`→`%username`/`\![raise,OnBoot]` へ差し替えた。これらは 4.1 では無 cue だが **Task 4.2** で SystemVar/GenericCommand が cue 化するため再破綻する → 4.2 で恒久的に無視される `Raw(...)` フィラーへ置換すること。
- 【重要・横断】additive な dola 変更（1.1 references／1.2 Cursor／1.3 Window）により **areka-ghost・areka-emo-text・areka(bin) は現時点でテストビルドが赤**（網羅 match の Cursor 欠落・`Choice` の references 欠落・`command_target_of`未配線・`sink.rs:320` の 1 引数 compile 呼び出し等）。これは設計:164 の「横断・機械的追随」通り想定内。各下流 crate はそれを touch するタスク（areka-ghost=Task 6、emo-text/ghost LogSink=Task 8、areka bin=Task 9）が走る時に機械的追随で緑化する。→ **Task 6.1 は areka-ghost をビルドさせるため必要な機械的追随を全て行う（`command_kind` の Cursor アーム含む・cage は Task 8 が追加）**。1.2/1.4 で `Cursor→Balloon` アームを先行追加し cage を後続に回したのと同型。
