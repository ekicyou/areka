# 設計レビュー: areka-P0-mcp-get-property

> 実施: 2026-10-04・本ブランチ（`45aa4e69`）。非対話（開発者への質問なし）。設計の主張は信用せず、設計が最も頼っている前提を実物のソースで引き直した。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。コンパイルとテストの実行はしていない（ソースを変えない段のため。読んで確かめた範囲を下に書く）。

## 総評

設計は小さく、既存の部品を借りるだけの形に収まっており、実装に進める状態にある。設計が頼る前提（テスト 2 が「変えないファイル」に触れずに組めること・ログの捕まえ方・共有テストが緑のままであること・`handle` の形がコンパイルできること・要件の対応・触るファイルの上限）を 5 系統すべて実物で確かめ、食い違いは 0 件だった。致命的な指摘は 0 件で、下の「軽い所見」は設計ディスカッションで拾うか、タスクの注意書きに回せば足りる。

## 実物で確かめたこと（設計の前提）

| # | 設計の前提 | 実物 | 判定 |
|---|---|---|---|
| 1 | テストから起こした実行系の書き手とマウントに届く | `GhostSlot(pub(crate) Option<GhostSession>)`・`GhostSession::runtime()` は `pub(crate)`（`crates/areka/src/ghost_session.rs`）。`GhostRuntime::sylphya_publisher()`・`mount()` は `pub`（`crates/areka-ghost/src/runtime.rs`）。`MountModel.shiori`・`ShioriMount.dir` は `pub`（`crates/areka-parsers/src/package/model.rs`）。`SwitchRig.world` は `pub(crate)`、土台のモジュール `ghost_switch_test_support` は `#[cfg(test)] pub(crate)`（`crates/areka/src/emo2_boot/mod.rs`）。`mcp_tests.rs` の `logsink_fallen_unit_is_active` が同じ引き方（`rig.world.non_send::<GhostSlot>().0.as_ref()…runtime()`）を既に使っている | 合う |
| 2 | 書き手に `set`・`publish_static`・`barrier` がある | `SylphyaPublisher::set(AskerId, String, String)`・`publish_static(AskerId, Vec<(String,String)>, Vec<(String,String)>)`・`barrier() -> Result<(), ReplyError>`（`crates/areka-sylphya/src/actor.rs`）。`barrier` は同じ受け口の FIFO なので、起動の手順が先に投函した `baseware.*` もまとめて反映済みになる | 合う |
| 3 | 自由な名前 `test.key` は `set` で書ける | `classify_set` は「根が `DOTTED_ROOTS`」か「葉が `GENERIC_PROP_NAMES`」のときだけ `NotSettable`。根 `test`・葉 `key`／`empty`／`other_only` はどちらにも無い → `SetClass::StoreWrite` → `Effect::SetDottedPerAsker` | 合う |
| 4 | 空の値は捨てられず `Value("")` になる | `SylphyaCore::apply` の `StoreWrite` の枝は値をそのまま写し、`run_actor` は `dotted_per_asker` へ無条件に `insert`。読み手 `resolve_dotted_canonical` は表にあれば `Value(v.clone())`。`outcome::value("")` は `Text("")` 1 つ・`is_error: false` | 合う |
| 5 | `SwitchRig` の起動で `baseware.name`＝`areka` が全体に載る | `SwitchRig::boot` → `boot_ghost` → `boot_with_origin` が `publish_ghost_statics` を呼び、`("baseware.name", BASEWARE_NAME="areka")` を `publish_static` の点付き（`Effect::SetDottedGlobal`）で載せる（`crates/areka-ghost/src/sylphya_wiring.rs`） | 合う |
| 6 | 読む順は「問い手ごと → 全体」 | `SylphyaReader::resolve_dotted_canonical`（`dotted_per_asker.get(asker)` → `dotted_global`） | 合う |
| 7 | `log_capture_kit::capture` と `field_str` | `capture` は呼んだスレッドの全段を集め、番兵で窓が生きていることを自分で確かめる。`field_str` は `record_str` で渡った欄だけ `Some`。`event = "…"`（文字列の定数）と `…as_str()` はどちらも `record_str` に入る。`areka` は `log-capture-kit` を dev 依存済みで、`alert_tests.rs`・`app_exit_tests.rs` に同じ絞り方の前例がある | 合う |
| 8 | 共有テストが緑のまま | `get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure` は `World::new()` で `dispatch` を呼ぶ → 解決は通る → `handle` → 置き場が無い → `NG:Property system is not available`。`NG:Specified ghost is not active`・`NG:Cannot find active ghost from specified name` のどちらとも違う。`eight_with_no_active_ghost_are_not_active`・`eight_with_a_wrong_name_cannot_find`・`drain_resolves_from_the_world` は `handle` の手前で終わるので影響なし。`not implemented yet` を `get_property` に期待するテストは `get_property_tests.rs` の 1 本だけ（`crates/areka-mcp` 側の同じ文字列は偽の受け手と `INSTRUCTIONS` で、`get_property` の答えを見ていない） | 合う |
| 9 | `handle` の形がコンパイルできる | `world` は不変の再借用だけ（`get_non_send::<GhostSlot>()` は `Option<&GhostSlot>`＝`resolve::active` と `install/names.rs` の `current_runtime` が同じ書き方）。`args` は動かさず `as_str()`／`&` で借りる。`ReplyTo::send(self, ToolOutcome)` はどの道でも 1 回。`listed_value` は `pub(crate)` で `super::resolve` から届く。`ghost_asker_id` は `pub`（`areka_ghost::sylphya_wiring` は `pub mod`）。`AskerContext { pub asker: AskerId }`。`let outcome = match … { … outcome::value(…) … }` は、束縛が有効になるのが文の後で、モジュールのパスとは名前空間も別なので衝突しない | 合う |
| 10 | 触るファイルの上限（要件 5.1・5.3） | 上の 1〜9 で要る可視性はすべて今のままで足りる＝`mod.rs`・`resolve.rs`・`ghost_session.rs`・`ghost_switch_test_support.rs`・`Cargo.toml`・`areka-sylphya` を触る要は出ない。`runtime.rs` は 783 行、`sylphya_reader` という名前の関数は無い（欄と `GhostParts` の欄だけ）。`runtime.rs` の字面を読む見張りテストは無い | 合う |
| 11 | 実機確認の手がかり | 記録の行「MCP: 待受を始めた」（`crates/areka-mcp/src/server.rs`）・「MCP: ツールに答えた」（`bridge.rs`）・`AREKA_APP_SMOKE_EXIT_MS`・`AREKA_NO_ALERT`・`tools/package.ps1` は実在 | 合う |
| 12 | 申し送り（要件 5.6） | `.kiro/specs/areka-P0-property-name-case-fold/brief.md` の末尾に「`mcp-get-property` からの申し送り（2026-10-04）」の節がある。`mcp-ghost-name-match` の brief にはまだ無い（設計の記述どおり「タスクに残す」） | 合う |

## 要件の対応

要件 1.1〜1.6・2.1〜2.3・3.1〜3.3・4.1〜4.8・5.1〜5.7 の 27 項目は、設計の「Requirements Traceability」にすべて行があり、欠けは 0 件。テストで固定しないと明記された項目（1.4・1.6・4.5・4.7・4.8）は、要件の側がそう求めているもの（1.6・4.5・4.7・4.8）か、手直しの処理を書かないことで満たすもの（1.4）で、対応の抜けではない。

## 致命的な指摘

0 件。

## 軽い所見（設計ディスカッションかタスクの注意書きで拾えば足りる）

1. **`runtime.rs` の欄の説明文を「1 行直してよい」とする記述**（File Structure Plan）。要件 5.2 は「読み口 1 本の追加に限り、既存の欄…を変えない」。説明文だけなら振る舞いは変わらないが、差分の目視（5.2 の確かめ方）を単純にするなら直さないほうが楽。どちらにするかをタスクで 1 つに決めておくとよい。
2. **要件 1.4（名前を手直ししない）に檻が無い**。設計は「コードに手直しの処理 0」で満たすとしており、後で誰かが `trim` を足しても赤くならない。要件 4.7 が禁じるのは英字の大小だけなので、望むならテスト 2 に「前に空白を付けた名前（例 ` test.key`）→ `NG:Cannot find such property name.`」を 1 行足せば、起こす回数を増やさずに固定できる（SSP 2.9.07 も同じ答え・survey §7.1）。判断の分岐ではないので足さない選択も筋は通る。
3. **問い手の檻は「同じ組み方を 2 か所で使う」ことに頼る**。テスト 2 はゴースト自身の問い手を `ghost_asker_id(&runtime.mount().shiori.dir)` で組み、`handle` も同じ式で組む。`handle` が別の元（`ActiveGhost.root` など）から組めば赤くなるので要件 4.1⑸ の檻としては足りるが、起動の手順の組み方が変わったときはテストと `handle` の両方を直す必要がある（設計の Revalidation Triggers に書いてある）。
4. **共有テストの中でも `warn!` が 1 件出るようになる**。`get_log_and_seven_omitted_…` が実行系の無い道を通るため。`warn!` の件数で落ちる見張りは無く、害は無い（知っておくだけでよい）。
5. **実機の展開先の長さ**。ワークツリーのパスが長い（`…\.claude\worktrees\areka-p0-mcp-get-property-11e315\target\…`）ので、emo2 の短いパスの上限に収まるよう、設計の例（`target\signoff-prop\x\`）のとおり短い名前で展開すること。
6. **テストは読んで確かめただけ**。この段ではソースを変えないため、テスト 2 を実際に組んで走らせてはいない。実装の最初のタスクでテスト 2 を先に書き、`sylphya_reader()` が無い・`handle` がダミーのままの状態で赤になることを見てから中身を入れるとよい。

## 設計の強み

- **本物の実行系を 1 回だけ起こすテストで、問い手を組む元の取り違えを捕まえる形**。純粋な関数へ切り出す案では捕まえられない間違い（`mount().shiori.dir` でなく別のフォルダから問い手を組む）を、同名の値を 3 か所（ゴーストごと・全体・別の問い手）に置くことで 1 つの `assert_eq!` で赤にできる。重さは既存の 1 本分に抑えてある。
- **境界がはっきりしている**。「変えないファイル」を名指しし、触る要が出たら止めて報告すると決め、足さないテストも 0 本と明記している。後続の 2 つの spec（`property-name-case-fold`・`mcp-ghost-name-match`）の着地を赤で妨げない形になっている。

## 判定

**GO**

理由: 設計が頼る既存コードの前提を 12 項目すべて実物で確かめて食い違いが無く、要件 27 項目の対応に欠けが無く、触るファイルは要件 5.1 の 3 つで閉じる。残る点は軽い所見だけで、設計を作り直す要は無い。

次の手: 設計ディスカッションで軽い所見の 1 と 2 を決め（どちらも答えで増減するのは 1 行）、`/kiro-spec-tasks areka-P0-mcp-get-property` へ進む。
