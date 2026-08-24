//! `BalloonFrameSource`（balloon.rs）: バルーン枠画像を **シェルと同一の** compose/present 経路へ
//! 載せる入力適合層（R5.1）。
//!
//! バルーンの入力は **scope 別に解決した面画像**である（R1.1/R1.2）。本モジュールは scope 番号から
//! 接頭辞優先連鎖（[`prefix_chain`]）を導出し、面 ID 単位で連鎖を辿って採用面列
//! （[`ResolvedFace`]）を決め、そこから **synthetic surfaces.txt テキスト**（`surface{ID}` に単一
//! overlay element＝採用面の実ファイル名）を生成 → `areka_parsers::shell::parse` →
//! `areka_emo_atlas::bake` → `EmoWorld::build`＋`bind_atlas` と、シェルが辿るのと**寸分違わぬ
//! 公開 API 経路**で `(EmoWorld, AtlasTable)` を組み上げる。直 WIC バイパスは設けない（R5.1）。
//!
//! # 正典整理（本モジュールが従う分類）
//!
//! - **面画像のみ入力**（R5.3）: 列挙対象は当該 scope の連鎖に載る接頭辞の `{接頭辞}{ID}.png` に
//!   限る（scope 0 は `balloonp0def`/`balloons`、scope 1 はそれらに先立つ `balloonp1def`/`balloonk`）。
//!   `balloonc*`（入力ボックス）・`arrow*`（スクロール矢印）・`marker`（`\![*]` マーカー）・
//!   `online*`（受信アニメ）はどの連鎖にも載らず列挙されない。相方側 `balloonk*` は
//!   **scope 1 以上でのみ**採用される正規の面系列であり、scope 0 の連鎖には現れない。
//! - **PNG α 尊重**（R5.2）: `use_self_alpha,1` 相当＝[`UseSelfAlpha::On`] で bake する。emo2 kakukaku は
//!   `.pna` 無し・PNG α のみ（fixture 実測）で、`.pna` 対応は [`ElementDecoder::probe_pna`] の既存
//!   seam に委ね本 spec では追加しない。
//! - **surface id = ID**（`{接頭辞}{ID}` の ID をそのまま採用）。`balloon.defaultsurface` 既定 0 と整合。
//!
//! 失敗経路は log-first（`tracing::error!`＋`Err`・silent failure 禁止）。面 0 が解決できない／bake が
//! エラーを産んだ場合は、真因をログへ出したうえで [`PresentError::Compose`]
//! （[`ComposeError::EmptyComposition`]）へ畳む。EmptyComposition は下流で Hide 縮退として許容される
//! ため（設計ディスカッション #1）、バルーン構築失敗はゴーストごと殺さず穏当に縮退する。
//!
//! [`UseSelfAlpha::On`]: areka_emo_atlas::UseSelfAlpha::On
//! [`ElementDecoder::probe_pna`]: areka_emo_atlas::ElementDecoder::probe_pna
//! [`ComposeError::EmptyComposition`]: areka_emo_compose::ComposeError::EmptyComposition

use std::collections::BTreeMap;
use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, ElementDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{ComposeError, EmoWorld};
use areka_parsers::balloon::BalloonModel;
use areka_parsers::charset::{DefaultEncoding, decode};

use crate::command::PresentError;

/// 面画像の拡張子（小文字比較）。接頭辞は scope 番号から連鎖として導出するため定数を持たない。
const FRAME_SUFFIX: &str = ".png";
/// バルーン既定設定ファイル名（2 層マージの**基層**・シェル側 descript と同名別物）。
const DESCRIPT_TXT: &str = "descript.txt";

/// 系列族の定義（**表データ**・候補追加が構造改変を伴わない形・R1.9）。
///
/// 正典のバルーン資産は族をまたいで同一の scope 別接尾辞体系を持つ——吹き出し
/// `balloons` / `balloonk` / `balloonp{n}def`、スクロール矢印 `arrows` / `arrowk` /
/// `arrowp{n}def`、マーカー `markers` / `markerk` / `markerp{n}def`……。本構造体は
/// その体系を **族名でパラメタ化**して保持するため、他族の scope 別対応（本仕様では
/// 対象外）が同じ機構をそのまま再利用できる。
///
/// 旧名候補は **可変長**である。装飾族には旧名がもう一段深く存在する（正典「`arrows` が
/// 本体用・旧バージョン対応のために `arrow` で代用を推奨」／`markers` に対する `marker` も
/// 同型）ため、`scope0_legacy` に `["arrows", "arrow"]` と積めば構造改変なしに表現できる
/// （R7.1(c) の語彙記録に対する縮退シーム）。
#[derive(Debug, Clone, Copy)]
pub struct SeriesFamily {
    /// 族の基底名（吹き出し族＝`"balloon"`・正規名は `{base}p{n}def`）。
    pub base: &'static str,
    /// scope 0 の旧名候補列（吹き出し族＝`["balloons"]`）。
    pub scope0_legacy: &'static [&'static str],
    /// scope 1 の旧名候補列（吹き出し族＝`["balloonk"]`）。
    pub scope1_legacy: &'static [&'static str],
}

/// 吹き出し族（本仕様で唯一実装する族）。
pub const BALLOON_FAMILY: SeriesFamily = SeriesFamily {
    base: "balloon",
    scope0_legacy: &["balloons"],
    scope1_legacy: &["balloonk"],
};

/// 連鎖内の 1 接頭辞（採用時の分類タグ付き）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesPrefix {
    /// 探索に用いる接頭辞（例: `"balloonp1def"` / `"balloonk"`）。
    pub prefix: String,
    /// この接頭辞が連鎖内で担う役割。
    pub tier: ChainTier,
}

/// 連鎖内での役割（フォールバック分類を運ぶ——採用時の警告判定・対応表記録の区分に用いる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainTier {
    /// 当該 scope 自身の候補（正規名＋当該 scope の旧名）。
    Own,
    /// n≧2 連鎖の**名指し**相方系列（`balloonk`）。正典が三人目以降の流用先として名指しした
    /// 系列であり、scope 1 の解決へ再帰的に縮退するものではない。
    KeroNamed,
    /// デフォルト定義（`balloonp0def` → `balloons`）。全連鎖の最終受け皿は scope 0 のみが
    /// 持つ地位であり、scope 1 はデフォルトの地位を持たない。
    Default,
}

impl SeriesFamily {
    /// 当該 scope の**正規名** `{base}p{scope}def`。
    ///
    /// 正典は p 系列を `\p[2]` 以降としてのみ記述しており、scope 0 / 1 の正規名
    /// （`balloonp0def` / `balloonp1def`）を先行探索することは areka 裁量の正規化拡張である
    /// （R1.10・対応表へ記録する対象）。
    fn canonical(&self, scope: u32) -> String {
        format!("{}p{scope}def", self.base)
    }

    /// 当該 scope の**旧名候補列**（scope 0 / 1 のみが持ち、scope 2 以上は旧名なし＝空）。
    fn legacy(&self, scope: u32) -> &'static [&'static str] {
        match scope {
            0 => self.scope0_legacy,
            1 => self.scope1_legacy,
            _ => &[],
        }
    }
}

/// scope 番号から**接頭辞優先連鎖**を導出する（純関数・表データ駆動・R1.1/1.8/1.9）。
///
/// 連鎖は 3 段の連結である——
///
/// ```text
/// chain(s) = Own(s) ++ KeroNamed(s≧2 のみ) ++ Default(s≧1 のみ)
///   Own      : 正規名 {base}p{s}def ＋ 当該 scope の旧名候補列
///              （s=0 → [balloonp0def, balloons] / s=1 → [balloonp1def, balloonk] / s≧2 → [balloonp{s}def]）
///   KeroNamed: 正典が名指しする相方系列の旧名候補列（[balloonk]）
///   Default  : 正規名 {base}p0def ＋ scope 0 の旧名候補列（[balloonp0def, balloons]）
/// ```
///
/// 各段では**正規名を先頭・旧名を後続**に置く。結果として
/// scope 0 は `balloonp0def` → `balloons`、scope 1 は `balloonp1def` → `balloonk` →
/// `balloonp0def` → `balloons`、scope n（n≧2）は `balloonp{n}def` → `balloonk` →
/// `balloonp0def` → `balloons` となる。
///
/// 相方系列の段は**正典が名指しした系列**であって scope 1 の解決へ再帰的に縮退するもの
/// ではないため、scope 2 以上の連鎖に scope 1 の正規名 `balloonp1def` は含めない（R1.1）。
///
/// scope は最後まで**数値のみ**で扱う。`Sakura`/`Kero` 等の 2 値列挙も、さくらスクリプト側の
/// 別語彙（`\h`/`\u`）も内部の正準表現としない（R1.9）。ゆえに解決規則は M1 の実行時 scope
/// （0 と 1）に閉じず、scope 番号一般（n≧2 を含む）で定義される（R1.6）。
pub fn prefix_chain(family: &SeriesFamily, scope: u32) -> Vec<SeriesPrefix> {
    let mut chain: Vec<SeriesPrefix> = Vec::new();

    // (i) 当該 scope 自身の候補: 正規名を先頭に、当該 scope の旧名を後続に置く。
    chain.push(SeriesPrefix {
        prefix: family.canonical(scope),
        tier: ChainTier::Own,
    });
    for name in family.legacy(scope) {
        chain.push(SeriesPrefix {
            prefix: (*name).to_string(),
            tier: ChainTier::Own,
        });
    }

    // (ii) 相方系列（scope 2 以上のみ）: 正典が名指しした系列そのもの＝旧名候補列のみ。
    // scope 1 の解決への再帰縮退ではないため、scope 1 の正規名はここに含めない。
    if scope >= 2 {
        for name in family.scope1_legacy {
            chain.push(SeriesPrefix {
                prefix: (*name).to_string(),
                tier: ChainTier::KeroNamed,
            });
        }
    }

    // (iii) デフォルト定義（scope 1 以上のみ）: 最終受け皿は scope 0 の系列。
    if scope >= 1 {
        chain.push(SeriesPrefix {
            prefix: family.canonical(0),
            tier: ChainTier::Default,
        });
        for name in family.scope0_legacy {
            chain.push(SeriesPrefix {
                prefix: (*name).to_string(),
                tier: ChainTier::Default,
            });
        }
    }

    chain
}

/// 解決済みの 1 面（連鎖探索の採用結果）。
///
/// 解決の**一時値**であり、構築後に保持する必要はない——採用面から上書きファイル名を導出する
/// 規則は本モジュール（系列解決の単一権威）にあり、いつでも再導出できるため。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFace {
    /// 面 ID（`{接頭辞}{ID}.png` の `{ID}` ＝ surface id）。
    pub surface_id: u32,
    /// 採用接頭辞（連鎖内で最初に画像が存在したもの）。
    pub prefix: String,
    /// 採用接頭辞が連鎖内で担っていた役割（scope≧1 での [`ChainTier::Default`] 採用＝
    /// 本体側への縮退であり、警告記録の判定に用いる）。
    pub tier: ChainTier,
    /// 実ファイル名（**原形保持**——実 WIC デコードが実パスを読むため大小を正規化しない）。
    pub file_name: String,
}

impl ResolvedFace {
    /// 採用接頭辞に対応する**面別上書きファイル名** `{採用接頭辞}{面 ID}s.txt`（R2.2/R2.3）。
    ///
    /// 上書き層は**採用した画像に対応するもの**を引く——scope 1 が `balloonk0` を採ったなら
    /// `balloonk0s.txt` であり、連鎖の他の接頭辞の `balloons0s.txt` ではない（R2.2）。ID 単位
    /// フォールバックで後段接頭辞へ落ちた面についても同様に、その後段接頭辞の同一 ID の
    /// 上書き層が対応する（R2.3——正典が面別上書きを「対応する ID のサーフェスに対して」
    /// 適用すると定めることの帰結）。
    ///
    /// 導出元は [`Self::file_name`]（実ファイル名・原形保持）ではなく **連鎖上の接頭辞**
    /// [`Self::prefix`] である。実ファイル名の大小は資産側の都合で揺れるが、上書きファイルの
    /// 探索名は連鎖から一意に決まる小文字正準形でなければ、採用画像と上書き層の対応が
    /// 資産の表記揺れに左右されてしまうため。
    pub fn override_file_name(&self) -> String {
        format!("{}{}s.txt", self.prefix, self.surface_id)
    }
}

/// `{prefix}{ID}.png`（大小無視）なら面 ID を返す。バルーン面でなければ `None`（R1.5）。
///
/// 判定は**厳密 3 段**である——
///
/// 1. **接頭辞の完全一致 strip**（大小無視）
/// 2. **`.png` の strip**
/// 3. **残余の全数字化**（空でなく、すべて ASCII 数字であること）
///
/// 接頭辞は完全一致ゆえ、入力ウィンドウ用 `balloonc0.png` を `balloonk` の面と誤認する事故は
/// 構造的に起こり得ない。装飾用の `arrow*` / `marker*` / `online*` も吹き出し族のどの接頭辞にも
/// 一致しない。3 段目を `u32::parse` 任せにせず全数字を明示検査するのは、`parse` が符号
/// （`balloons+0.png` の `+0`）を受理してしまうためで、正典の面 ID 表記は符号を持たない。
fn face_id_of(prefix: &str, name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    // (1) 接頭辞 strip（大小無視）→ (2) 拡張子 strip。
    let digits = lower
        .strip_prefix(prefix.to_ascii_lowercase().as_str())?
        .strip_suffix(FRAME_SUFFIX)?;
    // (3) 残余の全数字化（空・非数字・符号付きは面でない）。
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok()
}

/// ファイル名リストと接頭辞連鎖から採用面列を決める**純核**（fs 非依存・R1.2/1.3/1.4/1.5）。
///
/// 手順は 2 段である——
///
/// 1. **面 ID 集合の和**: 連鎖内の**全**接頭辞にまたがって面 ID を集める。ある接頭辞にしか
///    存在しない ID も候補に入り、先頭接頭辞の ID 集合には閉じない。
/// 2. **ID 単位の連鎖走査**: 各 ID について連鎖を**先頭から**辿り、最初に画像が存在した
///    接頭辞の面を採用する（R1.2）。ある ID の欠落は当該 ID の採用先を後段へずらすだけで、
///    当該 scope の系列全体を後段の接頭辞へ切り替えない（R1.3——`balloonk0` があり
///    `balloonk1` が無ければ、scope 1 の面 0 は `balloonk0`・面 1 は `balloons1`）。
///
/// 連鎖の末尾は常に族の scope 0 旧名（吹き出し族＝`balloons`）ゆえ、先頭側接頭辞の画像を
/// 1 枚も含まないバルーンでは全 scope が `balloons` 系列へ収束し、本仕様適用前と同一の
/// 面集合を得る（R1.4）。
///
/// 戻りは **surface id 昇順**——`(面 ID, 連鎖位置)` をキーとする [`BTreeMap`] の順序で決定化して
/// おり、入力（ディレクトリ走査順）に依存しない。走査順は非決定ゆえ、同一 (ID, 接頭辞) に大小
/// 違いの複数ファイルが併存する病的入力では**ファイル名の辞書順最小**を採り、ここでも走査順に
/// 結果を左右させない。
pub fn select_faces<S: AsRef<str>>(names: &[S], chain: &[SeriesPrefix]) -> Vec<ResolvedFace> {
    // (面 ID, 連鎖位置) → 実ファイル名。BTreeMap のキー順が「ID 昇順・連鎖先頭優先」と
    // 一致するため、和の構築と ID 単位走査を 1 本の走査で両立できる。
    let mut hits: BTreeMap<(u32, usize), String> = BTreeMap::new();
    for name in names {
        let name = name.as_ref();
        // 1 つの名前が複数の接頭辞に一致し得る（族の旧名が入れ子な場合）ため全段を見る。
        for (index, candidate) in chain.iter().enumerate() {
            let Some(id) = face_id_of(&candidate.prefix, name) else {
                continue;
            };
            hits.entry((id, index))
                .and_modify(|adopted| {
                    if name < adopted.as_str() {
                        *adopted = name.to_string();
                    }
                })
                .or_insert_with(|| name.to_string());
        }
    }

    let mut faces: Vec<ResolvedFace> = Vec::new();
    for ((surface_id, index), file_name) in hits {
        // 同一 ID の 2 件目以降は連鎖のより後段＝不採用（先頭から最初の一致のみを採る）。
        if faces.last().is_some_and(|f| f.surface_id == surface_id) {
            continue;
        }
        faces.push(ResolvedFace {
            surface_id,
            prefix: chain[index].prefix.clone(),
            tier: chain[index].tier,
            file_name,
        });
    }
    faces
}

/// `balloon_dir` 直下のファイル名を **1 回の走査**で集める（面判定は行わない）。
///
/// 面判定を課さずに全名を返すのは、連鎖内の接頭辞ごとに走査を繰り返さないためである
/// （列挙 1 回・選択は純核 [`select_faces`] が担う）。非 UTF-8 名は面のファイル名規約
/// （ASCII の接頭辞＋数字＋拡張子）を満たし得ないためこの段で落とす。
///
/// ディレクトリ走査に失敗した場合は log-first で [`PresentError`] を返す（R6.4）。
fn enumerate_file_names(balloon_dir: &Path) -> Result<Vec<String>, PresentError> {
    let read_dir = std::fs::read_dir(balloon_dir).map_err(|e| {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            error = %e,
            "balloon: バルーンディレクトリの走査に失敗"
        );
        PresentError::Compose(ComposeError::EmptyComposition(0))
    })?;

    let mut names: Vec<String> = Vec::new();
    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // 個別エントリの取得失敗は致命ではない（他エントリ継続・log-first）。
                tracing::warn!(error = %e, "balloon: ディレクトリエントリの取得に失敗（スキップ）");
                continue;
            }
        };
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

/// `balloon_dir` から当該 `scope` の**採用面列**を解決する（全消費者共通の単一施行点）。
///
/// 手順は 3 段——[`prefix_chain`] で連鎖を導出し、[`enumerate_file_names`] で
/// ディレクトリを **1 回だけ**走査し、純核 [`select_faces`] で面 ID 単位の連鎖探索を行う
/// （R1.2/R1.3）。面 ID は連鎖内の全接頭辞にまたがる ID 集合の和であり、戻りは
/// **surface id 昇順**・同一入力に対し決定論的（ディレクトリ走査順に依存しない）。
///
/// # 面 0 必在契約（R1.7）
///
/// 面 ID 0 が連鎖のどの接頭辞でも解決できない場合、理由を `error!` で記録したうえで
/// [`PresentError`] を返す。この契約を**権威側の 1 箇所**で施行することで、消費者
/// （placement / boot）それぞれが独自に面 0 の有無を判定する必要をなくし、無言で空の
/// バルーンを表示するログ無し経路を構造的に作らない。返した [`PresentError::Compose`]
/// （[`ComposeError::EmptyComposition`]）は消費者側の既存の縮退経路（バルーン未配線・
/// ダミー窓）へそのまま伝播する——プロセス終了ポリシー自体は本関数で変更しない。
///
/// # 観測（R6.1/R6.2）
///
/// - 解決完了時に `info!` で scope・連鎖・採用面一覧 `(id, prefix, file)` を記録する（R6.1）。
/// - scope 1 以上で [`ChainTier::Default`]（デフォルト定義側＝本体側）へ縮退した面について、
///   scope・面 ID・採用ファイルを面ごとに `warn!` で記録する（R6.2）。これは失敗ではなく
///   正典準拠のフォールバック動作の観測であり、解決自体は成功として続行する。
pub fn resolve_balloon_faces(
    balloon_dir: &Path,
    scope: u32,
) -> Result<Vec<ResolvedFace>, PresentError> {
    let chain = prefix_chain(&BALLOON_FAMILY, scope);
    // 列挙は 1 回のみ（接頭辞ごとに走査を繰り返さない）。
    let names = enumerate_file_names(balloon_dir)?;
    let faces = select_faces(&names, &chain);

    // ログ用の簡約形（連鎖＝接頭辞列・採用面＝(id, prefix, file) 列）。
    let chain_view: Vec<&str> = chain.iter().map(|p| p.prefix.as_str()).collect();

    // 面 0 必在契約（R1.7）: 施行点は本関数のみ。
    if !faces.iter().any(|f| f.surface_id == 0) {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            scope = scope,
            chain = ?chain_view,
            resolved_faces = faces.len(),
            "balloon: 面 ID 0 が連鎖のどの接頭辞でも解決できない"
        );
        return Err(PresentError::Compose(ComposeError::EmptyComposition(0)));
    }

    // R6.2: デフォルト定義側へ縮退した面を面ごとに記録する（scope 0 はデフォルト段を
    // 持たない＝自身が最終受け皿ゆえ縮退の概念がない）。
    if scope >= 1 {
        for face in faces.iter().filter(|f| f.tier == ChainTier::Default) {
            tracing::warn!(
                scope = scope,
                surface_id = face.surface_id,
                prefix = %face.prefix,
                file = %face.file_name,
                "balloon: 面がデフォルト定義側（本体側）の系列へ縮退した"
            );
        }
    }

    // R6.1: 解決結果（どの scope がどの系列・どの面ファイルを採ったか）。
    let faces_view: Vec<(u32, &str, &str)> = faces
        .iter()
        .map(|f| (f.surface_id, f.prefix.as_str(), f.file_name.as_str()))
        .collect();
    tracing::info!(
        balloon_dir = %balloon_dir.display(),
        scope = scope,
        chain = ?chain_view,
        faces = ?faces_view,
        "balloon: scope 別バルーン系列の解決が完了"
    );

    Ok(faces)
}

/// バルーン記述ファイルを文字コード解決つきで読む（既定 Ansi・宣言優先＝emo2 は `charset,UTF-8`）。
///
/// 戻りは **読めたテキスト**か **失敗の [`std::io::Error`]** をそのまま返す。層ごとに要求される
/// ログレベルが異なる（D8——基層は `warn!`・上書き層の不在は `debug!`）ため、レベル判断は
/// 呼び出し側の 2 つの薄いラッパへ委ね、本関数は判断を持たない。
fn read_decoded(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read(path).map(|bytes| decode(&bytes, DefaultEncoding::Ansi))
}

/// **基層**（`descript.txt`）を寛容に読む。読取失敗は `warn!`＋`None`（空層で継続・D8）。
///
/// 基層の欠落はバルーン既定設定が丸ごと得られない事態であり、正常縮退ではない（相方側で
/// 毎起動鳴る類の事象でもない）。ゆえに現行の `warn!` を維持し `debug!` へ降格しない。
fn read_descript_layer(path: &Path) -> Option<String> {
    match read_decoded(path) {
        Ok(text) => Some(text),
        Err(err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "balloon: バルーン既定設定（descript.txt）の読取に失敗（空層で継続）"
            );
            None
        }
    }
}

/// **面別上書き層**（`{採用接頭辞}{ID}s.txt`）を寛容に読む（D8 のレベル階層）。
///
/// - 不在（[`std::io::ErrorKind::NotFound`]）→ `debug!`。上書き層を持たない面は正典上まったく
///   正常であり（R2.4「欠落を失敗として扱わない」）、既定設定のみで定義が確定する。相方側の
///   バルーンが上書き層を持たないだけで毎起動 `warn!` が鳴る事故を構造的に防ぐ。
/// - その他の入出力エラー（権限・I/O 障害等）→ `warn!`。こちらは資産配置の異常であり、
///   既定設定のみで継続はするが観測可能に残す。
fn read_face_override_layer(path: &Path, scope: u32, file_name: &str) -> Option<String> {
    match read_decoded(path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(
                scope = scope,
                file = %file_name,
                path = %path.display(),
                "balloon: 面別上書き設定が無いため既定設定のみを用いる（正常縮退）"
            );
            None
        }
        Err(err) => {
            tracing::warn!(
                scope = scope,
                file = %file_name,
                path = %path.display(),
                error = %err,
                "balloon: 面別上書き設定の読取に失敗（既定設定のみで継続）"
            );
            None
        }
    }
}

/// 当該 scope 専用の**マージ済みバルーン定義**を組む（R2.1〜R2.5・全消費者共通の単一施行点）。
///
/// バルーン既定設定 `descript.txt`（基層）へ、`face0` が採用した面に**対応する**面別上書き
/// ファイル `{採用接頭辞}{ID}s.txt`（[`ResolvedFace::override_file_name`]）を重ねた 2 層を、
/// 既存パーサ `areka_parsers::balloon::parse_str` へそのまま渡す。マージ規則（後勝ち・未指定
/// 項目の基層継承＝R2.5）はパーサ側の既存契約であり、本関数はパーサを改造せず**層を用意する
/// だけ**である。上書き層は採用面に対応するもの 1 つのみを引くため、ID 単位フォールバックで
/// 後段接頭辞へ落ちた面にはその後段接頭辞の上書き層が対応する（R2.2/R2.3）。
///
/// # 失敗の扱い（D8・log-first）
///
/// `parse_str` は `Result` を返さず panic しない寛容写像ゆえ、本関数も常に [`BalloonModel`] を
/// 返す（読取失敗は空層で継続＝欠落キーは当該スカラ `None`）。層ごとのログレベルは
/// [`read_descript_layer`]／[`read_face_override_layer`] のとおりで、上書き層の**不在のみ**が
/// `debug!`（正常縮退）である。
///
/// # scope 引数について
///
/// 設計の署名は `(balloon_dir, face0)` だが、観測（R6.3）は確定値を **scope とともに**記録する
/// ことを求める。scope は [`ResolvedFace`] から逆算できない——本体側へ縮退した相方の面は採用
/// 接頭辞が `balloons` になり、scope 0 の面と区別が付かないためである。ゆえに scope を明示的に
/// 受け取る（列挙側 [`resolve_balloon_faces`] と同じ `(dir, scope, ...)` の引数順に揃える）。
///
/// # 観測（R6.3）
///
/// 確定した `windowposition` / `validrect` の実値を scope・採用上書きファイル名とともに `info!`
/// で記録する。placement／boot の 2 呼出点から scope あたり 2 行出るが、**値が一致すること自体**が
/// 権威一元化の生き証人になる。
pub fn load_scope_balloon_model(
    balloon_dir: &Path,
    scope: u32,
    face0: &ResolvedFace,
) -> BalloonModel {
    // 基層: バルーン既定設定（読取失敗は warn!＋空層）。
    let descript = read_descript_layer(&balloon_dir.join(DESCRIPT_TXT)).unwrap_or_default();

    // 上書き層: 採用面に対応する 1 つのみ（不在は debug! の正常縮退・D8）。
    let override_name = face0.override_file_name();
    let face_override =
        read_face_override_layer(&balloon_dir.join(&override_name), scope, &override_name);

    // マージ規則そのものは既存パーサの契約（後勝ち・未指定は基層継承＝R2.5）に委ねる。
    let model = areka_parsers::balloon::parse_str(&descript, face_override.as_deref());

    // R6.3: 確定した表示位置指定と文字範囲の実値を scope とともに記録する。
    let wp = model.windowposition();
    let vr = model.validrect();
    tracing::info!(
        balloon_dir = %balloon_dir.display(),
        scope = scope,
        file = %override_name,
        windowposition_x = ?wp.x(),
        windowposition_y = ?wp.y(),
        validrect_top = ?vr.top(),
        validrect_bottom = ?vr.bottom(),
        validrect_left = ?vr.left(),
        validrect_right = ?vr.right(),
        "balloon: scope 別バルーン定義の 2 層マージが確定"
    );

    model
}

/// 採用面列から synthetic surfaces.txt テキストを生成する（転記層の流儀）。
///
/// 各面は `surface{ID}` ブロックに単一 overlay element（`element0,overlay,{実ファイル名},0,0`）として
/// 転記する。`areka_parsers::shell::parse` が受理する surfaces.txt 文法に忠実で、独自構文は発明
/// しない（surface ヘッダ→`{`→element 行→`}` の登場順ストリーム）。element path に用いるのは
/// [`ResolvedFace::file_name`]（原形保持の実ファイル名）であり、連鎖上の接頭辞ではない——
/// bake が `base_dir.join(rel)` で実パスを開くため。
fn synthetic_surfaces_txt(faces: &[ResolvedFace]) -> String {
    let mut text = String::new();
    for face in faces {
        // `surface{ID}` ブロック・単一 overlay element（layer 0・オフセット 0,0）。
        let (id, file_name) = (face.surface_id, &face.file_name);
        text.push_str(&format!(
            "surface{id}\n{{\nelement0,overlay,{file_name},0,0\n}}\n\n"
        ));
    }
    text
}

/// 当該 `scope` のバルーン面を **シェルと同一の** compose/present 経路へ載せ
/// `(EmoWorld, AtlasTable)` を返す。
///
/// 系列解決（[`resolve_balloon_faces`]・R1.1/R1.2/R1.3）を内包する薄いラッパであり、構築本体は
/// [`build_balloon_target_from_faces`] が担う。ゆえに **World の面は当該 scope が解決した系列の
/// 面**であり、全 scope が本体側 `balloons` 系列へ畳み込まれることはない（scope 1 が
/// `balloonk0.png` を採ったなら World の面 0 はその画像である）。
///
/// 解決済み面列を既に手元へ持つ消費者（起動時資産構築・窓配置採寸）は、二重列挙を避けるため
/// [`build_balloon_target_from_faces`] を直接呼べる。
///
/// 失敗（面 0 不在・ディレクトリ走査失敗・bake 脱落）は log-first で真因をログしたうえで
/// [`PresentError::Compose`]（[`ComposeError::EmptyComposition`]・Hide 縮退許容）を返す。
pub fn build_balloon_target(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
    scope: u32,
) -> Result<(EmoWorld, AtlasTable), PresentError> {
    let faces = resolve_balloon_faces(balloon_dir, scope)?;
    build_balloon_target_from_faces(balloon_dir, decoder, &faces)
}

/// 解決済み面列から `(EmoWorld, AtlasTable)` を組む（構築本体・R5.1/R5.2/R5.3）。
///
/// 採用面列 → synthetic surfaces.txt → `shell::parse` → `bake`（PNG α 尊重＝
/// [`UseSelfAlpha::On`]・R5.2）→ `EmoWorld::build`＋`bind_atlas` と、直 WIC バイパス無しで
/// シェルと同一機構に載せる（R5.1）。得た組を `attach_target` に渡すだけでバルーン target が
/// シェルと同じ提示経路へ乗る。
///
/// 系列解決を引数として受け取るため、**どの scope の面列であるかは呼び出し側が決める**。
/// 面列が空／bake がエラーを産んだ場合は log-first で真因をログし
/// [`PresentError::Compose`]（[`ComposeError::EmptyComposition`]）を返す。
pub fn build_balloon_target_from_faces(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
    faces: &[ResolvedFace],
) -> Result<(EmoWorld, AtlasTable), PresentError> {
    if faces.is_empty() {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            "balloon: 採用面列が空（構築する面が 1 枚も無い）"
        );
        return Err(PresentError::Compose(ComposeError::EmptyComposition(0)));
    }

    // synthetic surfaces.txt をシェルと同一の parser で解釈する（転記層・R5.1）。
    let text = synthetic_surfaces_txt(faces);
    let shell = areka_parsers::shell::parse(&text);

    // PNG α 尊重（use_self_alpha,1 相当・R5.2）で bake。base_dir は balloon_dir（実パスは
    // base_dir.join(rel) で一度だけ実体化される）。
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: balloon_dir,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], decoder, PackConfig::default());

    // bake の脱落（decode/normalize 失敗）は log-first で真因を出し、構築失敗として畳む。
    // 採用面は解決済みの固定小集合ゆえ全枚デコード成功が前提＝脱落は制作者ミス/配置不備の兆候。
    if !baked.errors.is_empty() {
        for err in &baked.errors {
            tracing::error!(
                balloon_dir = %balloon_dir.display(),
                error = %err,
                "balloon: 面画像の bake に失敗"
            );
        }
        return Err(PresentError::Compose(ComposeError::EmptyComposition(0)));
    }

    // シェルと同一の compose 前段: World 構築 → アトラス束縛（SetId(0)・resolve は本呼び出し限り）。
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&baked.table, SetId(0));

    Ok((world, baked.table))
}

#[cfg(test)]
#[path = "balloon_model_tests.rs"]
mod model_tests;
#[cfg(test)]
#[path = "balloon_series_tests.rs"]
mod series_tests;
#[cfg(test)]
#[path = "balloon_target_tests.rs"]
mod target_tests;
#[cfg(test)]
#[path = "balloon_test_support.rs"]
mod test_support;
