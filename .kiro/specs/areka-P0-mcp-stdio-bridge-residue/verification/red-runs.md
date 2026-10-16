# 赤の実走と確かめの記録

## 確かめ C（要件 2.3・タスク 2.1）: 偽の受け口の自己確認を部品のテストの束でだけ走らせる

直す前と直した後に、ワークツリーの根で同じコマンドを取った。

```
cargo test -p areka-mcp-bridge -- --list
```

控えはワークツリーの `target\residue-2.1\list-before.txt`（前）と `target\residue-2.1\list-after.txt`（後）。

### 束ごとの数

| 束 | 前のテストの数 | 前の自己確認 | 後のテストの数 | 後の自己確認 |
|---|---|---|---|---|
| 部品のテスト（`src\lib.rs`） | 86 | 2 | 86 | 2 |
| `src\main.rs` | 0 | 0 | 0 | 0 |
| 通しのテスト（`tests\exe_test.rs`） | 10 | 2 | 8 | 0 |
| 通しのテスト（`tests\real_receiver_test.rs`） | 6 | 0 | 6 | 0 |
| 文書のテスト | 0 | 0 | 0 | 0 |
| 合計 | 102 | 4 | 100 | 2 |

- 自己確認の 2 本（`one_request_gets_the_scripted_reply_and_is_recorded`・`the_handshake_defaults_and_every_reply_mode_answer_as_named`）は、前は部品の束と `exe_test` の束に 1 回ずつ（道筋は両方とも `fake_server::tests::<名前>`）、後は部品の束に 1 回だけ（道筋は `fake_server_tests::<名前>`）、通しの束では 0 回。
- 合計が 102 から 100 へ減った 2 本は、`exe_test` の束で 2 回目に走っていた自己確認の 2 本そのもの。

### 関数の名前の集まり

道筋（`::` の前）を外した関数の名前を重ねずに並べると、前も後も 100 個で、2 つの並びに違いは無い（`diff` の出力が空）。どの関数の名前も直した後に少なくとも 1 回走る。

### テストの結果

直した後の `cargo test -p areka-mcp-bridge` は緑（部品 86・`main.rs` 0・`exe_test` 8・`real_receiver_test` 6・文書 0、失敗 0）。`cargo fmt --all -- --check` も差分なし。テストの本体は変えていない（替えたのは自己確認のファイルの頭の説明と `use`、偽の受け口の頭の説明と末尾の `mod tests` の宣言、`src\lib.rs` の兄弟の宣言だけ）。
