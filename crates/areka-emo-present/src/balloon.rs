//! `BalloonFrameSource`（balloon.rs）: バルーン枠画像を **シェルと同一の** compose/present 経路へ
//! 載せる入力適合層（R5.1）。
//!
//! M-boot のバルーンは fixture の枠画像（`balloons{N}.png`）だけを入力とする。本モジュールは
//! それらを列挙し、**synthetic surfaces.txt テキスト**（`surface{N}` に単一 overlay element
//! `balloons{N}.png`）を生成 → `areka_parsers::shell::parse` → `areka_emo_atlas::bake` →
//! `EmoWorld::build`＋`bind_atlas` と、シェルが辿るのと**寸分違わぬ公開 API 経路**で
//! `(EmoWorld, AtlasTable)` を組み上げる。直 WIC バイパスは設けない（R5.1）。
//!
//! # 正典整理（本モジュールが従う分類）
//!
//! - **枠画像のみ入力**（R5.3）: 列挙対象は `balloons{N}.png` に限る。`balloonc*`（入力ボックス）・
//!   `arrow*`（スクロール矢印）・`marker`（`\![*]` マーカー）・`online*`（受信アニメ）・相方側
//!   `balloonk*` は列挙しない。
//! - **PNG α 尊重**（R5.2）: `use_self_alpha,1` 相当＝[`UseSelfAlpha::On`] で bake する。emo2 kakukaku は
//!   `.pna` 無し・PNG α のみ（fixture 実測）で、`.pna` 対応は [`ElementDecoder::probe_pna`] の既存
//!   seam に委ね本 spec では追加しない。
//! - **surface id = N**（`balloons{N}` の N をそのまま採用）。`balloon.defaultsurface` 既定 0 と整合。
//!
//! 失敗経路は log-first（`tracing::error!`＋`Err`・silent failure 禁止）。枠が 1 枚も無い／bake が
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

use crate::command::PresentError;

/// 列挙されるバルーン枠画像のファイル名接頭辞（本体側吹き出し・相方側 `balloonk*` は対象外）。
const FRAME_PREFIX: &str = "balloons";
/// 列挙対象の拡張子（小文字比較）。
const FRAME_SUFFIX: &str = ".png";

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

/// `balloon_dir` から枠画像を列挙し `(surface_id, ファイル名)` を **surface id 昇順**で返す。
///
/// `balloons{N}.png`（N は非負整数）だけを枠として採り、`balloonc*`/`arrow*`/`marker*`/`online*`・
/// 相方側 `balloonk*` は名前段で除外する（R5.3）。ファイル名の大小は無視して判定するが、element
/// path として使う値は **実ファイル名を原形のまま**保持する（実 WIC デコードが実パスを読むため）。
///
/// ディレクトリ走査に失敗した場合は log-first で [`PresentError`] を返す。
fn enumerate_frames(balloon_dir: &Path) -> Result<Vec<(u32, String)>, PresentError> {
    let read_dir = std::fs::read_dir(balloon_dir).map_err(|e| {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            error = %e,
            "balloon: 枠画像ディレクトリの走査に失敗"
        );
        PresentError::Compose(ComposeError::EmptyComposition(0))
    })?;

    let mut frames: Vec<(u32, String)> = Vec::new();
    for entry in read_dir {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // 個別エントリの取得失敗は致命ではない（他エントリ継続・log-first）。
                tracing::warn!(error = %e, "balloon: ディレクトリエントリの取得に失敗（スキップ）");
                continue;
            }
        };
        let file_name = entry.file_name();
        let name = match file_name.to_str() {
            Some(n) => n,
            None => continue, // 非 UTF-8 名は枠画像規約外＝スキップ。
        };
        if let Some(id) = frame_id(name) {
            frames.push((id, name.to_string()));
        }
    }

    // surface id 昇順で決定化（ディレクトリ走査順は非決定ゆえ明示ソート）。
    frames.sort_unstable_by_key(|(id, _)| *id);
    Ok(frames)
}

/// `balloons{N}.png`（大小無視）なら surface id `N` を返す。枠画像でなければ `None`。
///
/// 接頭辞は `balloons` 固定ゆえ `balloonc*`/`balloonk*`（8 文字目が `s` でない）は自然に外れる。
fn frame_id(name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_prefix(FRAME_PREFIX)?.strip_suffix(FRAME_SUFFIX)?;
    // 接頭辞と拡張子の間は 10 進整数のみ（空・非数字は枠画像でない）。
    stem.parse::<u32>().ok()
}

/// 枠 `(surface_id, ファイル名)` 列から synthetic surfaces.txt テキストを生成する（転記層の流儀）。
///
/// 各枠は `surface{N}` ブロックに単一 overlay element（`element0,overlay,{ファイル名},0,0`）として
/// 転記する。`areka_parsers::shell::parse` が受理する surfaces.txt 文法に忠実で、独自構文は発明
/// しない（surface ヘッダ→`{`→element 行→`}` の登場順ストリーム）。
fn synthetic_surfaces_txt(frames: &[(u32, String)]) -> String {
    let mut text = String::new();
    for (id, file_name) in frames {
        // `surface{N}` ブロック・単一 overlay element（layer 0・オフセット 0,0）。
        text.push_str(&format!(
            "surface{id}\n{{\nelement0,overlay,{file_name},0,0\n}}\n\n"
        ));
    }
    text
}

/// バルーン枠画像を **シェルと同一の** compose/present 経路へ載せ `(EmoWorld, AtlasTable)` を返す。
///
/// `balloon_dir` 内の `balloons{N}.png` を枠として列挙（R5.3）→ synthetic surfaces.txt →
/// `shell::parse` → `bake`（PNG α 尊重＝[`UseSelfAlpha::On`]・R5.2）→ `EmoWorld::build`＋`bind_atlas`
/// と、直 WIC バイパス無しでシェルと同一機構に載せる（R5.1）。得た組を `attach_target` に渡すだけで
/// バルーン target がシェルと同じ提示経路へ乗る。
///
/// 枠が 1 枚も無い／bake がエラーを産んだ場合は log-first で真因をログし
/// [`PresentError::Compose`]（[`ComposeError::EmptyComposition`]・Hide 縮退許容）を返す。
pub fn build_balloon_target(
    balloon_dir: &Path,
    decoder: &impl ElementDecoder,
) -> Result<(EmoWorld, AtlasTable), PresentError> {
    let frames = enumerate_frames(balloon_dir)?;
    if frames.is_empty() {
        tracing::error!(
            balloon_dir = %balloon_dir.display(),
            "balloon: 枠画像（balloons{{N}}.png）が 1 枚も見つからない"
        );
        return Err(PresentError::Compose(ComposeError::EmptyComposition(0)));
    }

    // synthetic surfaces.txt をシェルと同一の parser で解釈する（転記層・R5.1）。
    let text = synthetic_surfaces_txt(&frames);
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
    // M-boot の枠は固定小集合ゆえ全枚デコード成功が前提＝脱落は制作者ミス/配置不備の兆候。
    if !baked.errors.is_empty() {
        for err in &baked.errors {
            tracing::error!(
                balloon_dir = %balloon_dir.display(),
                error = %err,
                "balloon: 枠画像の bake に失敗"
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
mod tests {
    use super::*;

    use areka_emo_atlas::MemoryDecoder;
    use areka_parsers::shell::parse;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    // ── テスト用一時ディレクトリ（新規 dev-dep を避け std のみで構成）─────────────
    // `std::env::temp_dir()` 配下へプロセス id ＋単調カウンタで一意なサブディレクトリを作り、
    // Drop で後始末する（tempfile 相当の最小実装）。

    static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Drop 時に自身を再帰削除する一時ディレクトリ。
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "areka-emo-present-balloon-{}-{}",
                std::process::id(),
                n
            ));
            std::fs::create_dir_all(&path).expect("一時ディレクトリ作成");
            TempDir { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        /// 空のプレースホルダファイルを作る（MemoryDecoder 経路ゆえ中身は不問・列挙対象のため名前のみ要）。
        fn touch(&self, name: &str) {
            std::fs::File::create(self.path.join(name)).expect("プレースホルダ作成");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// 不透明 1×1 PBGRA スペック（bake が placement を必ず産む＝非退化）。
    fn opaque_1x1() -> (u32, u32, u32, Vec<u8>, bool) {
        (1, 1, 4, vec![10u8, 20, 30, 255], true)
    }

    /// R5.1/R5.3 転記一致（観測完了基準）: synthetic surfaces.txt → `shell::parse` の往復で、
    /// 各枠の surface id（`{N}`）と element path（`balloons{N}.png`）が転記一致する。
    ///
    /// これはファイルシステム/デコードを一切要さない純粋な転記層の檻。
    #[test]
    fn synthetic_text_transcribes_frame_id_and_path() {
        let frames = vec![
            (0u32, "balloons0.png".to_string()),
            (1u32, "balloons1.png".to_string()),
        ];
        let text = synthetic_surfaces_txt(&frames);
        let shell = parse(&text);

        assert_eq!(shell.surfaces.len(), 2, "2 枠 → 2 surface");
        for (n, file_name) in &frames {
            let surface = shell
                .surfaces
                .iter()
                .find(|s| s.id == *n)
                .unwrap_or_else(|| panic!("surface id {n} が転記されていない"));
            assert_eq!(
                surface.elements.len(),
                1,
                "各枠は単一 overlay element へ転記される"
            );
            assert_eq!(
                surface.elements[0].path.as_str(),
                file_name,
                "element path が balloons{{N}}.png へ転記一致しない"
            );
            assert_eq!(surface.elements[0].layer, 0, "layer 0（element0）へ転記");
        }
    }

    /// 大小無視の枠判定と非枠除外（R5.3）: `frame_id` が `balloons{N}.png` からのみ N を得て、
    /// `balloonc*`/`balloonk*`/`arrow*`/`marker*`/`online*`・非数字・非 png を弾く。
    #[test]
    fn frame_id_matches_only_balloon_frames() {
        assert_eq!(frame_id("balloons0.png"), Some(0));
        assert_eq!(frame_id("balloons12.png"), Some(12));
        assert_eq!(frame_id("BALLOONS3.PNG"), Some(3), "大小無視");
        // 非枠（列挙対象外・R5.3）。
        assert_eq!(frame_id("balloonc0.png"), None, "入力ボックスは枠でない");
        assert_eq!(frame_id("balloonk0.png"), None, "相方側は枠でない");
        assert_eq!(frame_id("arrow0.png"), None);
        assert_eq!(frame_id("marker.png"), None);
        assert_eq!(frame_id("online0.png"), None);
        assert_eq!(frame_id("balloons.png"), None, "数字が無ければ枠でない");
        assert_eq!(frame_id("balloonsX.png"), None, "非数字は枠でない");
        assert_eq!(frame_id("balloons0.txt"), None, "非 png は枠でない");
    }

    /// R5.1/R5.2/R5.3 full build: `build_balloon_target` が枠のみを列挙し、シェルと同一の
    /// parse→bake→World 経路で `(EmoWorld, AtlasTable)` を返す。非枠ファイルは列挙されず
    /// アトラスにも World にも現れない。MemoryDecoder ゆえ実 PNG 不要で決定論。
    #[test]
    fn build_balloon_target_end_to_end_frames_only() {
        let dir = TempDir::new();
        // 枠 2 枚 ＋ 非枠 3 種を同ディレクトリへ配置。
        dir.touch("balloons0.png");
        dir.touch("balloons1.png");
        dir.touch("balloonc0.png"); // 入力ボックス（非枠）
        dir.touch("arrow0.png"); // スクロール矢印（非枠）
        dir.touch("marker.png"); // マーカー（非枠）

        // 枠のみデコーダへ登録（非枠は登録しない＝もし列挙されれば decode 失敗で露見する）。
        let mut dec = MemoryDecoder::new();
        let (w, h, stride, bytes, has_alpha) = opaque_1x1();
        dec.insert(dir.path().join("balloons0.png"), w, h, stride, bytes.clone(), has_alpha);
        dec.insert(dir.path().join("balloons1.png"), w, h, stride, bytes, has_alpha);

        let (world, table) =
            build_balloon_target(dir.path(), &dec).expect("枠 2 枚から Ok が返る");

        // アトラスに枠 2 枚のエントリがあり placement を持つ（PNG α 尊重で焼かれる・R5.2）。
        for rel in ["balloons0.png", "balloons1.png"] {
            let id = table
                .resolve(SetId(0), rel)
                .unwrap_or_else(|| panic!("{rel} がアトラスに解決されない"));
            assert!(
                table.entry(id).placement.is_some(),
                "{rel} は不透明ゆえ placement を持つ"
            );
        }
        // 非枠は列挙対象外ゆえアトラスに存在しない（R5.3）。
        assert_eq!(table.resolve(SetId(0), "balloonc0.png"), None);
        assert_eq!(table.resolve(SetId(0), "arrow0.png"), None);
        assert_eq!(table.resolve(SetId(0), "marker.png"), None);
        assert_eq!(table.len(), 2, "生存エントリは枠 2 枚のみ");

        // World は surface id = N（balloons{N} の N）を常駐させる。
        assert!(world.surface(0).is_some(), "surface id 0（balloons0）が World にある");
        assert!(world.surface(1).is_some(), "surface id 1（balloons1）が World にある");
        assert!(world.surface(2).is_none(), "存在しない id は None");
    }

    /// R5.5 多面バルーン fixture（偶数 id・正典準拠）: TempDir へ `balloons0.png`＋`balloons2.png`
    /// の 2 枚（偶数 id 0/2）を置き、`build_balloon_target` が **surface 0 と surface 2 の両面**を
    /// 列挙・構築した world を返すことを固定する。既存 `..._frames_only`（id 0/1）と対をなし、
    /// 面 id が飛び番（1 を欠く 0/2）でも各面が id=N でそのまま常駐することを実演する。
    /// MemoryDecoder ゆえ実 PNG 不要で決定論（既存流儀踏襲）。
    #[test]
    fn build_balloon_target_enumerates_multiple_even_id_faces() {
        let dir = TempDir::new();
        // 偶数 id の面 2 枚（1 を欠く飛び番）を test-local fixture として自前用意（R5.5）。
        dir.touch("balloons0.png");
        dir.touch("balloons2.png");

        // 両面をデコーダへ登録（MemoryDecoder 経路・実 PNG 不要）。
        let mut dec = MemoryDecoder::new();
        let (w, h, stride, bytes, has_alpha) = opaque_1x1();
        dec.insert(dir.path().join("balloons0.png"), w, h, stride, bytes.clone(), has_alpha);
        dec.insert(dir.path().join("balloons2.png"), w, h, stride, bytes, has_alpha);

        let (world, table) =
            build_balloon_target(dir.path(), &dec).expect("偶数 id 2 面から Ok が返る");

        // アトラスに 2 面が解決され placement を持つ（PNG α 尊重で焼かれる）。
        for rel in ["balloons0.png", "balloons2.png"] {
            let id = table
                .resolve(SetId(0), rel)
                .unwrap_or_else(|| panic!("{rel} がアトラスに解決されない"));
            assert!(
                table.entry(id).placement.is_some(),
                "{rel} は不透明ゆえ placement を持つ"
            );
        }
        assert_eq!(table.len(), 2, "生存エントリは偶数 id 面 2 枚のみ");

        // 多面列挙の要（本タスクの主張）: surface 0 と surface 2 の **両面** が world に常駐する。
        assert!(world.surface(0).is_some(), "surface id 0（balloons0）が World にある");
        assert!(world.surface(2).is_some(), "surface id 2（balloons2）が World にある");
        // 飛び番の欠番 id 1 は列挙対象に無いゆえ常駐しない（面 id=N の同一性を固定）。
        assert!(world.surface(1).is_none(), "欠番 id 1 は World に無い");
    }

    // ── 檻 1: scope→接頭辞優先連鎖の導出（R1.1/1.6/1.8/1.9/1.10・R7.1）─────────────

    /// `(接頭辞, tier)` の組で連鎖を突き合わせるための簡約形（表明の可読性のため）。
    fn chain_pairs(family: &SeriesFamily, scope: u32) -> Vec<(String, ChainTier)> {
        prefix_chain(family, scope)
            .into_iter()
            .map(|p| (p.prefix, p.tier))
            .collect()
    }

    /// R1.1: scope 0 の連鎖は `balloonp0def`（正規名）→ `balloons`（旧名）の 2 段で、
    /// 双方とも当該 scope 自身の候補＝tier `Own`。scope 0 は相方系列段もデフォルト段も持たない。
    #[test]
    fn prefix_chain_scope0_is_canonical_then_legacy_all_own() {
        assert_eq!(
            chain_pairs(&BALLOON_FAMILY, 0),
            vec![
                ("balloonp0def".to_string(), ChainTier::Own),
                ("balloons".to_string(), ChainTier::Own),
            ],
            "scope 0 は Own 段のみ（正規名が先頭・旧名が後続）"
        );
    }

    /// R1.1: scope 1 の連鎖は Own（`balloonp1def` → `balloonk`）＋ Default（`balloonp0def` →
    /// `balloons`）の 4 段。scope 1 は相方系列段を持たない（自身が相方系列そのもの）。
    #[test]
    fn prefix_chain_scope1_is_own_then_default() {
        assert_eq!(
            chain_pairs(&BALLOON_FAMILY, 1),
            vec![
                ("balloonp1def".to_string(), ChainTier::Own),
                ("balloonk".to_string(), ChainTier::Own),
                ("balloonp0def".to_string(), ChainTier::Default),
                ("balloons".to_string(), ChainTier::Default),
            ],
            "scope 1 は Own 2 段＋Default 2 段（KeroNamed 段なし）"
        );
    }

    /// R1.1/R1.6: scope 5（n≧2 の代表）の連鎖は Own（`balloonp5def` のみ＝旧名なし）＋
    /// KeroNamed（`balloonk`）＋ Default（`balloonp0def` → `balloons`）。
    #[test]
    fn prefix_chain_scope5_names_kero_then_default() {
        assert_eq!(
            chain_pairs(&BALLOON_FAMILY, 5),
            vec![
                ("balloonp5def".to_string(), ChainTier::Own),
                ("balloonk".to_string(), ChainTier::KeroNamed),
                ("balloonp0def".to_string(), ChainTier::Default),
                ("balloons".to_string(), ChainTier::Default),
            ],
            "scope 5 は Own 1 段＋KeroNamed 1 段＋Default 2 段"
        );
    }

    /// R1.1: 相方系列の段は正典が名指しした系列（`balloonk`）であって、scope 1 の解決へ再帰的に
    /// 縮退するものではない。ゆえに n≧2 の連鎖に scope 1 の正規名 `balloonp1def` は現れない。
    #[test]
    fn prefix_chain_ge2_excludes_scope1_canonical_name() {
        for scope in 2u32..=8 {
            let chain = prefix_chain(&BALLOON_FAMILY, scope);
            assert!(
                !chain.iter().any(|p| p.prefix == "balloonp1def"),
                "scope {scope} の連鎖に scope 1 の正規名が混入している: {chain:?}"
            );
            // 名指し相方系列は旧名 `balloonk` 1 本で、tier は KeroNamed。
            let kero: Vec<_> = chain
                .iter()
                .filter(|p| p.tier == ChainTier::KeroNamed)
                .collect();
            assert_eq!(kero.len(), 1, "scope {scope} の相方系列段は 1 段");
            assert_eq!(kero[0].prefix, "balloonk", "相方系列段は正典名指しの balloonk");
        }
    }

    /// R1.1/R1.8/R1.9: 各段は「正規名が先頭・旧名が後続」であり、scope を数値のみで扱う
    /// （2 値列挙もさくらスクリプト語彙も内部の正準表現にしない）ことを、任意 scope の
    /// 正規名 `balloonp{s}def` が常に連鎖先頭であることで固定する。
    #[test]
    fn prefix_chain_head_is_scope_canonical_name_for_any_scope() {
        for scope in [0u32, 1, 2, 3, 7, 42, 100] {
            let chain = prefix_chain(&BALLOON_FAMILY, scope);
            assert_eq!(
                chain[0],
                SeriesPrefix {
                    prefix: format!("balloonp{scope}def"),
                    tier: ChainTier::Own,
                },
                "scope {scope} の連鎖先頭は当該 scope の正規名（tier=Own）"
            );
        }
    }

    /// R1.9（縮退シーム）/R7.1(c): 装飾族の**一段深い旧名**（`arrows` → `arrow`）も、
    /// 同じ表構造（可変長の旧名候補列）で構造改変なしに表現できる。本仕様は吹き出し族のみ
    /// 実装するが、表データが族でパラメタ化されている事実をここで固定する。
    #[test]
    fn series_family_table_expresses_deeper_legacy_names() {
        const ARROW_FAMILY: SeriesFamily = SeriesFamily {
            base: "arrow",
            scope0_legacy: &["arrows", "arrow"],
            scope1_legacy: &["arrowk"],
        };

        assert_eq!(
            chain_pairs(&ARROW_FAMILY, 0),
            vec![
                ("arrowp0def".to_string(), ChainTier::Own),
                ("arrows".to_string(), ChainTier::Own),
                ("arrow".to_string(), ChainTier::Own),
            ],
            "scope 0 の旧名候補が 2 段でも構造改変なしに連鎖へ載る"
        );
        assert_eq!(
            chain_pairs(&ARROW_FAMILY, 2),
            vec![
                ("arrowp2def".to_string(), ChainTier::Own),
                ("arrowk".to_string(), ChainTier::KeroNamed),
                ("arrowp0def".to_string(), ChainTier::Default),
                ("arrows".to_string(), ChainTier::Default),
                ("arrow".to_string(), ChainTier::Default),
            ],
            "デフォルト段にも可変長の旧名候補列がそのまま展開される"
        );
    }

    // ── 檻 2-5: 面 ID 単位の連鎖探索（純核 `select_faces`・R1.2/1.3/1.4/1.5・R7.1）─────

    /// `(surface_id, 採用接頭辞, tier, ファイル名)` の簡約形（表明の可読性のため）。
    fn selected(names: &[&str], scope: u32) -> Vec<(u32, String, ChainTier, String)> {
        let chain = prefix_chain(&BALLOON_FAMILY, scope);
        select_faces(names, &chain)
            .into_iter()
            .map(|f| (f.surface_id, f.prefix, f.tier, f.file_name))
            .collect()
    }

    /// 檻 2（R1.2/R1.10）正規名優先: 正規名 `{base}p{s}def{ID}` と旧名が併存する場合、
    /// 連鎖先頭の正規名が採られる（scope 0 の `balloonp0def`・scope 1 の `balloonp1def` とも）。
    #[test]
    fn select_faces_prefers_canonical_name_over_legacy() {
        assert_eq!(
            selected(&["balloons0.png", "balloonp0def0.png"], 0),
            vec![(
                0,
                "balloonp0def".to_string(),
                ChainTier::Own,
                "balloonp0def0.png".to_string()
            )],
            "scope 0: 正規名 balloonp0def0.png が旧名 balloons0.png に優先する"
        );
        assert_eq!(
            selected(&["balloons0.png", "balloonk0.png", "balloonp1def0.png"], 1),
            vec![(
                0,
                "balloonp1def".to_string(),
                ChainTier::Own,
                "balloonp1def0.png".to_string()
            )],
            "scope 1: 正規名 balloonp1def0.png が旧名 balloonk0.png にも Default 段にも優先する"
        );
    }

    /// 檻 3（R1.3）ID 単位フォールバック: `balloonk0` があり `balloonk1` が無いとき、
    /// scope 1 の面 0 は `balloonk0`（tier=Own）・面 1 は `balloons1`（tier=Default）。
    /// ある ID の欠落を理由に系列全体を後段の接頭辞へ切り替えない。
    #[test]
    fn select_faces_falls_back_per_face_id_not_per_series() {
        let names = ["balloonk0.png", "balloons0.png", "balloons1.png"];
        assert_eq!(
            selected(&names, 1),
            vec![
                (
                    0,
                    "balloonk".to_string(),
                    ChainTier::Own,
                    "balloonk0.png".to_string()
                ),
                (
                    1,
                    "balloons".to_string(),
                    ChainTier::Default,
                    "balloons1.png".to_string()
                ),
            ],
            "面 0 は相方側・面 1 のみデフォルト段へ縮退する（系列一括切替でない）"
        );
        assert_eq!(
            selected(&names, 0),
            vec![
                (
                    0,
                    "balloons".to_string(),
                    ChainTier::Own,
                    "balloons0.png".to_string()
                ),
                (
                    1,
                    "balloons".to_string(),
                    ChainTier::Own,
                    "balloons1.png".to_string()
                ),
            ],
            "scope 0 の連鎖に balloonk は無く、相方側の面を一切採らない"
        );
    }

    /// R1.3（ID 集合の和）: 面 ID は連鎖内の**全**接頭辞にまたがる和であり、先頭接頭辞の
    /// ID 集合に閉じない。先頭にしか無い ID も後段にしか無い ID も双方が採られる。
    #[test]
    fn select_faces_unions_face_ids_across_all_prefixes() {
        assert_eq!(
            selected(&["balloonp1def2.png", "balloons0.png"], 1),
            vec![
                (
                    0,
                    "balloons".to_string(),
                    ChainTier::Default,
                    "balloons0.png".to_string()
                ),
                (
                    2,
                    "balloonp1def".to_string(),
                    ChainTier::Own,
                    "balloonp1def2.png".to_string()
                ),
            ],
            "後段にしか無い ID 0 と先頭にしか無い ID 2 の双方が面集合に入る"
        );
    }

    /// 檻 4（R1.4）後方互換収束: 連鎖の先頭側接頭辞の画像を 1 枚も含まないバルーンでは、
    /// 全 scope が `balloons` 系列へ解決され、同一の面集合（ID と採用ファイル）を得る。
    /// tier は連鎖上の地位ゆえ scope で異なる（scope 0 は Own・scope≧1 は Default）。
    #[test]
    fn select_faces_converges_to_legacy_series_for_all_scopes() {
        let names = ["balloons0.png", "balloons1.png", "balloons2.png"];
        let faces_of = |scope: u32| -> Vec<(u32, String, String)> {
            select_faces(&names, &prefix_chain(&BALLOON_FAMILY, scope))
                .into_iter()
                .map(|f| (f.surface_id, f.prefix, f.file_name))
                .collect()
        };

        let scope0 = faces_of(0);
        assert_eq!(scope0.len(), 3, "balloons 3 枚がそのまま面集合になる");
        for scope in [1u32, 2, 7] {
            assert_eq!(
                faces_of(scope),
                scope0,
                "scope {scope} の面集合が scope 0 と一致しない（後方互換収束の破れ）"
            );
        }
    }

    /// 檻 5（R1.5）非バルーン面除外: 入力ウィンドウ用 `balloonc*`・装飾用 `arrow*`/`marker*`/
    /// `online*`・非数字・非 png はどの連鎖でも採用されない。接頭辞は完全一致ゆえ
    /// `balloonc0.png` を `balloonk` の面と誤認する事故は構造的に起こり得ない。
    #[test]
    fn select_faces_rejects_non_balloon_faces() {
        let names = [
            "balloonc0.png", // 入力ウィンドウ（バルーン面でない）
            "balloonc1.png", // 同上
            "arrow0.png",    // スクロール矢印（装飾）
            "arrows0.png",   // 同上
            "marker.png",    // マーカー（装飾）
            "online0.png",   // 受信アニメ（装飾）
            "balloonsX.png", // 残余が非数字
            "balloons0.txt", // 非 png
            "balloons.png",  // 残余が空
            "balloonk.png",  // 残余が空（相方側）
        ];
        for scope in [0u32, 1, 5] {
            assert!(
                selected(&names, scope).is_empty(),
                "scope {scope} の連鎖が非バルーン面を採用した: {:?}",
                selected(&names, scope)
            );
        }
        assert_eq!(
            face_id_of("balloonk", "balloonc0.png"),
            None,
            "balloonc を balloonk と誤認しない（接頭辞完全一致）"
        );
    }

    /// R1.5（判定 3 段）: 接頭辞 strip（大小無視）→ `.png` strip → 残余の全数字化。
    /// いずれか 1 段でも満たさなければ面でない。
    #[test]
    fn face_id_of_is_strict_three_stage_predicate() {
        assert_eq!(face_id_of("balloons", "balloons0.png"), Some(0));
        assert_eq!(face_id_of("balloons", "balloons12.png"), Some(12));
        assert_eq!(face_id_of("balloonk", "balloonk1.png"), Some(1));
        assert_eq!(
            face_id_of("balloonp0def", "BalloonP0Def3.PNG"),
            Some(3),
            "大小無視で判定する"
        );
        assert_eq!(
            face_id_of("balloons", "balloonk0.png"),
            None,
            "接頭辞は完全一致（部分一致で拾わない）"
        );
        assert_eq!(face_id_of("balloons", "balloons.png"), None, "残余が空");
        assert_eq!(
            face_id_of("balloons", "balloonsX.png"),
            None,
            "残余が非数字"
        );
        assert_eq!(face_id_of("balloons", "balloons0.txt"), None, "非 png");
        assert_eq!(
            face_id_of("balloons", "balloons+0.png"),
            None,
            "符号付きは全数字でない"
        );
    }

    /// 決定論（設計 Postconditions）: 戻りは surface id **昇順**であり、入力（ディレクトリ
    /// 走査順）に依存しない。辞書順ではなく数値順である（`balloons10` は `balloons2` の後）。
    #[test]
    fn select_faces_is_deterministic_regardless_of_input_order() {
        let ascending = [
            "balloons0.png",
            "balloons1.png",
            "balloons2.png",
            "balloons10.png",
        ];
        let shuffled = [
            "balloons10.png",
            "balloons2.png",
            "balloons0.png",
            "balloons1.png",
        ];
        let a = selected(&ascending, 0);
        assert_eq!(a, selected(&shuffled, 0), "入力順に結果が依存しない");
        assert_eq!(
            a.iter().map(|f| f.0).collect::<Vec<_>>(),
            vec![0, 1, 2, 10],
            "surface id 昇順（辞書順でない）"
        );
    }

    /// ファイル名は**原形保持**（実 WIC デコードが実パスを読むため大小を正規化しない）。
    /// 同一 (ID, 接頭辞) に大小違いが併存する病的入力では辞書順最小を採り、走査順に
    /// 結果を左右させない。
    #[test]
    fn select_faces_preserves_original_file_name_case() {
        assert_eq!(
            selected(&["BALLOONS0.PNG"], 0),
            vec![(
                0,
                "balloons".to_string(),
                ChainTier::Own,
                "BALLOONS0.PNG".to_string()
            )],
            "採用ファイル名は原形のまま保持される"
        );
        let a = selected(&["BALLOONS0.PNG", "balloons0.png"], 0);
        let b = selected(&["balloons0.png", "BALLOONS0.PNG"], 0);
        assert_eq!(a, b, "大小違い併存でも走査順に依存しない");
        assert_eq!(a.len(), 1, "同一 ID の採用面は 1 つ");
    }

    /// 枠が 1 枚も無ければ log-first で `EmptyComposition`（Hide 縮退許容）を返す。
    #[test]
    fn no_frames_returns_empty_composition() {
        let dir = TempDir::new();
        dir.touch("balloonc0.png"); // 非枠のみ配置。
        let dec = MemoryDecoder::new();

        // `(EmoWorld, AtlasTable)` は Debug 非実装ゆえ expect_err を使わず match で判定する。
        match build_balloon_target(dir.path(), &dec) {
            Ok(_) => panic!("枠 0 枚なら Err のはず"),
            Err(err) => assert!(
                matches!(err, PresentError::Compose(ComposeError::EmptyComposition(0))),
                "枠不在は EmptyComposition(0) へ畳む: {err:?}"
            ),
        }
    }
}
