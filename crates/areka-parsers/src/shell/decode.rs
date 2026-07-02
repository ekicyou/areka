//! 意味層（Decode）— 構文トークン → 値正規化済み `Shell`（emo2 subset 限定）。
//!
//! Lexer（`lexer::lex`）が産んだ構文トークン列 `Vec<Token>` を消費し、シェルサーフェス
//! モデル `Shell` を組み立てる（依存方向 `model ← lexer ← decode`）。下流が再パース
//! 不要なのは、この層が値を decode しきるため（要件 1.2）。sakura 同様に `Result` を
//! 返さず・パニックせず・空入力で空を返す寛容決定的パスを採る（要件 2.x）。
//!
//! 本ファイル（タスク 4.1）が担うのは **decode スケルトンの枠組みのみ**:
//! - トップレベルのトークン走査と、ブロックヘッダによる**ブロックディスパッチ**
//!   （設計 System Flows「iterate top level tokens → 各ブロック種別」）。
//! - charset 行の寛容スキップ（要件 3.1）・descript ブロックの寛容スキップ（要件 3.2）。
//!   いずれもモデルに保持しない（`Shell` に charset/descript フィールドは存在しない）。
//! - `surfaceNNN` ブロックから surface ID を取り出し、（当面空の）構成要素枠を持つ
//!   `Surface` を積む（要件 4.1）。ID 抽出失敗は `unwrap_or(0)` で既定 0 に倒す（要件 3.3）。
//! - 空入力・ヘッダのみ・ヘッダ欠落・未知ブロック・未知行・`Raw` を寛容吸収し
//!   走査を中断しない（要件 3.3/9.2）。
//!
//! **ディスパッチ優先順位（重要）**: `kero.surface.alias` と `surface.append10` は
//! いずれも部分文字列 "surface" を含むため、素朴な `surfaceNNN` 判定より**先**に
//! これらを分類する。優先順位は
//! `descript` → `kero.surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
//!
//! **スコープ境界（タスク 4.3〜4.6 のシーム）**: 本ファイルは surface の element/collision
//! まで decode する（タスク 4.2 実装済み）。残りはシーム:
//! - animationN 集約（interval＋pattern 群）→ タスク 4.3 が `decode_surface_body` 内へ実装する。
//! - `surface.append*` ターゲット捕捉＋追記 collision/animation → タスク 4.4 が
//!   `decode_append_block` を実装し `Shell.appends` を充填する。
//! - `kero.surface.alias` 写像 → タスク 4.5 が `decode_alias_block` を実装し
//!   `Shell.aliases` を充填する。
//! - subset 外・不正の寛容吸収の最終化 → タスク 4.6。
//! 現段階ではこれらのシーム関数は枠のみ（空を返す/何もしない）に留める。

use super::lexer::Token;
use super::model::{Animation, Collision, CollisionName, Element, ElementPath, Shell, Surface};

/// 構文トークン列を値正規化済みの `Shell` へ写像する（mod 内・`parse` が結線する）。
///
/// - 入力 `tokens` は lexer 出力（構文区切り済み）。
/// - トップレベルを走査し、ブロックヘッダでディスパッチする。charset/descript は
///   読み飛ばし保持しない（要件 3.1/3.2）。`surfaceNNN` は枠を積む（要件 4.1）。
/// - 失敗しない（`Vec`/`Shell` を返す・`Result` でない・要件 2.x）。空入力で空 `Shell`。
/// - 同一入力で同一出力（決定的）。
pub(crate) fn decode(tokens: Vec<Token>) -> Shell {
    let mut shell = Shell {
        surfaces: Vec::new(),
        appends: Vec::new(),
        aliases: Vec::new(),
    };

    // ブロックの本体行（BlockStart..BlockEnd の間の Line 群）を蓄えるカーソル。
    // BlockStart でヘッダを開き、以降の Line を積み、BlockEnd でヘッダ＋本体を
    // ディスパッチする。ブロック外の Line は崩れ入力ゆえ寛容に読み飛ばす（要件 3.3/9.2）。
    let mut open: Option<(Vec<String>, Vec<Vec<String>>)> = None;

    for token in tokens {
        match token {
            Token::BlockStart(header) => {
                // 直前の BlockStart が BlockEnd を見ずに再び開いた崩れ入力でも走査を
                // 止めない。開いていた分をここで確定させてから新ブロックを開く。
                if let Some((prev_header, prev_body)) = open.take() {
                    dispatch_block(&mut shell, &prev_header, &prev_body);
                }
                open = Some((header, Vec::new()));
            }
            Token::Line(fields) => {
                // ブロック本体行はカーソルに蓄える。ブロック外の Line は吸収する。
                if let Some((_, body)) = open.as_mut() {
                    body.push(fields);
                }
            }
            Token::BlockEnd => {
                // ブロック終端でヘッダ＋本体をディスパッチする。
                if let Some((header, body)) = open.take() {
                    dispatch_block(&mut shell, &header, &body);
                }
            }
            // トップレベル行・不正断片は個別処理しない。charset 行の非保持（要件 3.1）を
            // 含め寛容に読み飛ばす（要件 3.3/9.2）。値化はタスク 4.3〜4.6 の領分。
            Token::TopLevel(_) | Token::Raw(_) => {}
        }
    }

    // BlockEnd を見ずに EOF に達した崩れ入力（未閉じブロック）でも、開いていた分を
    // 取りこぼさずディスパッチする（走査を中断しない・要件 9.2）。
    if let Some((header, body)) = open.take() {
        dispatch_block(&mut shell, &header, &body);
    }

    shell
}

/// ブロックヘッダ（`BlockStart` の CSV フィールド列）でブロック種別を判定し振り分ける。
///
/// 優先順位は `descript` → `kero.surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
/// alias/append は "surface" を含むため plain `surfaceNNN` より先に判定する（誤分類回避）。
///
/// `body` は BlockStart..BlockEnd 間の本体行（CSV フィールド列）。surface ブロックの
/// element/collision decode に用いる。
fn dispatch_block(shell: &mut Shell, header: &[String], body: &[Vec<String>]) {
    let head = header.first().map(String::as_str).unwrap_or("");

    if head == "descript" {
        // descript ブロックは寛容スキップ・非保持（要件 3.2）。本体も破棄する。
        return;
    }
    if head == "kero.surface.alias" {
        // alias 写像はタスク 4.5 の領分。現段階では吸収のみ（何も積まない）。
        decode_alias_block(shell, header);
        return;
    }
    if head.starts_with("surface.append") {
        // append 追記定義はタスク 4.4 の領分。現段階では吸収のみ（何も積まない）。
        decode_append_block(shell, header);
        return;
    }
    if let Some(rest) = head.strip_prefix("surface") {
        // `surfaceNNN` ブロック（要件 4.1）。NNN を取り出し、本体を decode して積む。
        // 非数値・欠落は `unwrap_or(0)` で既定 0 に倒す（要件 3.3・パニックしない）。
        let id = rest.parse::<u32>().unwrap_or(0);
        let (elements, collisions, animations) = decode_surface_body(body);
        shell.surfaces.push(Surface {
            id,
            elements,
            collisions,
            animations,
        });
        return;
    }

    // 未知ブロック（未対応ヘッダ）は寛容に吸収する（要件 9.2）。何も積まない。
}

/// surface ブロック本体（element/collision/animation 行）を対応型へ decode する。
///
/// タスク 4.2 が element overlay ＋ collision（レイヤ昇順・矩形）を充填する。
/// - element overlay: `elementN,overlay,PATH,X,Y` を `Element` へ（画像パスは無加工・
///   レイヤ昇順で安定ソート・要件 4.2/4.3/4.4）。
/// - collision: `collisionN,始点X,始点Y,終点X,終点Y,ID` を `Collision` へ（left/top/
///   right/bottom ＋不透明領域名・出現順・要件 6.1/6.2）。
///
/// animation 行（`animationN.interval` / `animationN.patternM`）はタスク 4.3 の
/// シームゆえここでは充填しない（読み飛ばす）。overlay 以外の element メソッド・
/// collisionex 等の寛容吸収はタスク 4.6 の領分ゆえ、扱わない行は単に読み飛ばす
/// （パニックしない・要件 9.2）。
fn decode_surface_body(body: &[Vec<String>]) -> (Vec<Element>, Vec<Collision>, Vec<Animation>) {
    let mut elements: Vec<Element> = Vec::new();
    let mut collisions: Vec<Collision> = Vec::new();
    // animation はタスク 4.3 で充填するシーム。本タスクでは常に空。
    let animations: Vec<Animation> = Vec::new();

    for fields in body {
        let key = fields.first().map(String::as_str).unwrap_or("");

        // element overlay 行（要件 4.2/4.3/4.4）。field[1] == "overlay" のみ扱う。
        // overlay 以外のメソッドはタスク 4.6 の寛容吸収に委ね、ここでは読み飛ばす。
        if let Some(rest) = key.strip_prefix("element") {
            let is_overlay = fields.get(1).map(String::as_str) == Some("overlay");
            if is_overlay {
                elements.push(Element {
                    // element の N をレイヤインデックスに用いる。非数値は既定 0（要件 3.3）。
                    layer: rest.parse::<u32>().unwrap_or(0),
                    // 画像パス（field[2]）は無加工保持（区切り正規化なし・要件 4.3）。
                    path: ElementPath::new(field_string(fields, 2)),
                    x: field_i64(fields, 3),
                    y: field_i64(fields, 4),
                });
            }
            continue;
        }

        // collision 矩形行（要件 6.1/6.2）。ukadoc 順序 始点X/始点Y/終点X/終点Y。
        if let Some(rest) = key.strip_prefix("collision") {
            // `collisionex`（要件 6 のタスク 4.6 寛容吸収）は扱わない。純 collisionN のみ。
            if rest.chars().all(|c| c.is_ascii_digit()) {
                collisions.push(Collision {
                    index: rest.parse::<u32>().unwrap_or(0),
                    left: field_i64(fields, 1),
                    top: field_i64(fields, 2),
                    right: field_i64(fields, 3),
                    bottom: field_i64(fields, 4),
                    // 領域名（field[5]）は opaque・無加工保持（要件 6.2）。
                    name: CollisionName::new(field_string(fields, 5)),
                });
            }
            continue;
        }

        // animation 行・その他はタスク 4.3 / 4.6 のシーム。ここでは読み飛ばす（要件 9.2）。
    }

    // element はレイヤインデックス昇順・安定ソート（同レイヤは出現順維持・要件 4.4）。
    elements.sort_by_key(|e| e.layer);

    (elements, collisions, animations)
}

/// 本体行の `idx` フィールドを無加工の `String` として取り出す（欠落は空文字列）。
fn field_string(fields: &[String], idx: usize) -> String {
    fields.get(idx).cloned().unwrap_or_default()
}

/// 本体行の `idx` フィールドを `i64` として取り出す（欠落・非数値は既定 0・要件 3.3）。
fn field_i64(fields: &[String], idx: usize) -> i64 {
    fields
        .get(idx)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

/// `surface.append*` 追記ブロックを `SurfaceAppend` へ decode する枠（タスク 4.4）。
///
/// タスク 4.1 では何も積まない（吸収のみ）。タスク 4.4 がヘッダのターゲット指定捕捉
/// （`parse_targets`・展開しない）と、本体の collision/animation 充填を実装し、
/// `shell.appends` へ push する。
fn decode_append_block(_shell: &mut Shell, _header: &[String]) {
    // タスク 4.4 で実装。現段階は寛容吸収（要件 9.2）。
}

/// `kero.surface.alias` ブロックを `SurfaceAlias` 群へ decode する枠（タスク 4.5）。
///
/// タスク 4.1 では何も積まない（吸収のみ）。タスク 4.5 が各 `KEY,[id,...]` 行を
/// opaque キー＋順序付き数値 ID リストへ写像し（重複キー保持）、`shell.aliases` へ
/// push する。
fn decode_alias_block(_shell: &mut Shell, _header: &[String]) {
    // タスク 4.5 で実装。現段階は寛容吸収（要件 9.2）。
}
