//! `SylphyaReader`: `resolve_flat` / `resolve_dotted` / `talk_snapshot`（同期・無待機）。
//!
//! 掲示板（マテリアライズド・ビュー）モデルの読み側。共有読みハンドル
//! [`crate::mirror::SharedMirror`] の現在鏡像を都度 `Arc` clone し、その不変像に対して
//! map 参照だけで解決する。読み口 2 形（per-talk 凍結スナップショット／逐次同期解決）を、
//! 問い合わせ元コンテキスト（[`AskerContext`]）第一級・由来非露出・無待機で提供する。
//!
//! ## 無待機の読み不変
//!
//! 読みが行うのは、写しの読みの鍵と `Arc` の複製、登録の表の読みの鍵と複製、
//! 登録された読み手（[`PullSource`]）の呼び出しだけである。他エンジン・他スレッドへの
//! ブロッキング照会を行わず、遅延しうる値源を同期読み経路に載せない。外部環境へのアクセスは
//! 外から登録された読み手（注入可能な値源シーム）を通す。読み手はその約束を守るものだけを登録する。
//! ゆえに読み経路はシステム全体の直列化点にならない。

use crate::asker::AskerContext;
use crate::key::{PropPath, parse_dotted};
use crate::mirror::SharedMirror;
use crate::pull::{PullRegistry, PullShape, PullSource};
use crate::value::{DottedResolution, FlatResolution};
use crate::vocab::DegradePolicy;
use crate::vocab::flat::FLAT_VOCAB;
use std::collections::BTreeMap;
use std::sync::Arc;

/// 共有読みハンドル: 鏡像（[`SharedMirror`]）を包み、同期・無待機の読み口 2 形を提供する。
///
/// `Clone` は内部 [`SharedMirror`]（`Arc` 共有）の clone のみ——複数の読者・消費エンジンが
/// 同一鏡像を安価に共有する。構築は [`SylphyaReader::new`]。
#[derive(Clone)]
pub struct SylphyaReader {
    shared: SharedMirror,
    pull: PullRegistry,
}

impl SylphyaReader {
    /// 共有鏡像ハンドルから読み口を構築する（登録の表は空）。
    pub fn new(shared: SharedMirror) -> Self {
        SylphyaReader {
            shared,
            pull: PullRegistry::default(),
        }
    }

    /// 名前の形に読み手を登録する（同じ形は置き換え）。複製どうしで共有される。
    ///
    /// 写しの区画に無い点付きの名前だけがこの読み手に届く。答えは写しにも保存にも書かない。
    pub fn register_pull(&self, shape: PullShape, source: Arc<dyn PullSource>) {
        self.pull.register(shape, source);
    }

    /// フラット窓の逐次解決（`%` 抜きの名を与える）。
    ///
    /// 当該 asker の per-asker 区画で名前を引き、値が実在すれば [`FlatResolution::Value`]。
    /// 不在なら台帳
    /// （[`FLAT_VOCAB`]）の縮退政策を返す（[`FlatResolution::Degraded`]）。台帳外の名は
    /// `Degraded(PassThroughRaw)`。
    ///
    /// 由来非露出: 返す型に backing 層情報は載らない。無待機。
    pub fn resolve_flat(&self, asker: &AskerContext, name: &str) -> FlatResolution {
        let img = self.shared.load();
        let hit = img
            .flat_per_asker
            .get(&asker.asker)
            .and_then(|m| m.get(name));
        if let Some(v) = hit {
            return FlatResolution::Value(v.clone());
        }
        let policy = flat_degrade_policy(name);
        tracing::debug!(
            target: "areka_sylphya::reader",
            asker = asker.asker.as_str(),
            name = name,
            policy = ?policy,
            "flat resolution degraded"
        );
        FlatResolution::Degraded(policy)
    }

    /// 点付き窓の逐次解決（パース済み [`PropPath`]）。
    ///
    /// `PropPath` を正準文字列形（[`PropPath::to_canonical_string`]）へ正規化し、
    /// per-asker → global の順で点付き区画を引き、どちらにも無い時だけ登録の表の読み手に聞く。
    /// 値が得られれば [`DottedResolution::Value`]、無ければ [`DottedResolution::NotFound`]。
    ///
    /// 無待機・由来非露出。
    pub fn resolve_dotted(&self, asker: &AskerContext, path: &PropPath) -> DottedResolution {
        let key = path.to_canonical_string();
        self.resolve_dotted_canonical(asker, path, &key)
    }

    /// 文字列 key の便宜口（sink 用）。
    ///
    /// [`parse_dotted`] に失敗した key は `tracing::warn!` を記録して
    /// [`DottedResolution::NotFound`] へ決定論的に縮退する（panic しない）。成功時は
    /// [`SylphyaReader::resolve_dotted`] へ委譲する。
    pub fn resolve_dotted_str(&self, asker: &AskerContext, key: &str) -> DottedResolution {
        match parse_dotted(key) {
            Ok(path) => self.resolve_dotted(asker, &path),
            Err(e) => {
                tracing::warn!(
                    target: "areka_sylphya::reader",
                    asker = asker.asker.as_str(),
                    key = key,
                    error = %e,
                    "dotted key parse failed; degrading to NotFound"
                );
                DottedResolution::NotFound
            }
        }
    }

    /// talk 凍結スナップショット素材。
    ///
    /// 鏡像に **値が実在するフラット名のみ** を `BTreeMap<String, String>` として返す。
    /// 当該 asker の per-asker 区画の写しで、縮退・不在の名や既定値は含めない（値実在分のみ）。
    /// 戻り値は所有コピー——取得後に新しい鏡像が
    /// publish されても、返した写像は不変（per-talk 凍結）。
    ///
    /// 無待機。ghost がこれを `SystemVarSnapshot` へ写像する（sylphya は sakura 非依存）。
    pub fn talk_snapshot(&self, asker: &AskerContext) -> BTreeMap<String, String> {
        let img = self.shared.load();
        img.flat_per_asker
            .get(&asker.asker)
            .cloned()
            .unwrap_or_default()
    }

    /// 正準文字列 key で点付き区画を引く（per-asker → global → 登録の表）。内部共通経路。
    fn resolve_dotted_canonical(
        &self,
        asker: &AskerContext,
        path: &PropPath,
        key: &str,
    ) -> DottedResolution {
        let img = self.shared.load();
        let hit = img
            .dotted_per_asker
            .get(&asker.asker)
            .and_then(|m| m.get(key))
            .or_else(|| img.dotted_global.get(key));
        if let Some(v) = hit {
            return DottedResolution::Value(v.clone());
        }
        // 写しに無い時だけ読み手に聞く（`find` は鍵を外してから返すので、呼ぶのは鍵の外）。
        match self.pull.find(path).and_then(|s| s.read(path)) {
            Some(v) => DottedResolution::Value(v),
            None => {
                tracing::debug!(
                    target: "areka_sylphya::reader",
                    asker = asker.asker.as_str(),
                    key = key,
                    "dotted resolution not found"
                );
                DottedResolution::NotFound
            }
        }
    }
}

/// フラット名の縮退政策を台帳から引く。台帳外の名は `PassThroughRaw`。
fn flat_degrade_policy(name: &str) -> DegradePolicy {
    FLAT_VOCAB
        .iter()
        .find(|e| e.token == name)
        .map(|e| e.degrade)
        .unwrap_or(DegradePolicy::PassThroughRaw)
}

#[cfg(test)]
mod tests {
    use crate::asker::{AskerContext, AskerId};
    use crate::key::parse_dotted;
    use crate::mirror::{MirrorImage, SharedMirror};
    use crate::reader::SylphyaReader;
    use crate::value::{DottedResolution, FlatResolution};
    use crate::vocab::DegradePolicy;
    use std::sync::Arc;

    fn ctx(id: &str) -> AskerContext {
        AskerContext {
            asker: AskerId::new(id),
        }
    }

    #[test]
    fn flat_value_present_per_asker() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Value("Alice".into())
        );
    }

    #[test]
    fn flat_absent_username_is_consumer_default() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "username"),
            FlatResolution::Degraded(DegradePolicy::ConsumerDefault)
        );
    }

    #[test]
    fn flat_absent_passthrough_token_is_passthrough() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
        );
    }

    #[test]
    fn flat_absent_table_outside_is_passthrough() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "not_a_real_token"),
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
        );
    }

    #[test]
    fn flat_other_asker_does_not_leak() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/b"), "selfname"),
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
        );
    }

    #[test]
    fn dotted_value_present_per_asker() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.dotted_per_asker
            .entry(a.clone())
            .or_default()
            .insert("areka.window.scope(0).x".into(), "10".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let path = parse_dotted("areka.window.scope(0).x").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &path),
            DottedResolution::Value("10".into())
        );
    }

    #[test]
    fn dotted_value_present_global() {
        let mut img = MirrorImage::empty();
        img.dotted_global
            .insert("baseware.name".into(), "areka".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let path = parse_dotted("baseware.name").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &path),
            DottedResolution::Value("areka".into())
        );
    }

    #[test]
    fn dotted_per_asker_precedes_global() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.dotted_global.insert("x.y".into(), "GLOBAL".into());
        img.dotted_per_asker
            .entry(a.clone())
            .or_default()
            .insert("x.y".into(), "PERASKER".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let path = parse_dotted("x.y").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &path),
            DottedResolution::Value("PERASKER".into())
        );
    }

    #[test]
    fn dotted_absent_is_not_found() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        let path = parse_dotted("system.no_such").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &path),
            DottedResolution::NotFound
        );
    }

    #[test]
    fn dotted_str_present() {
        let mut img = MirrorImage::empty();
        img.dotted_global
            .insert("baseware.version".into(), "0.1".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_dotted_str(&ctx("ghost/a"), "baseware.version"),
            DottedResolution::Value("0.1".into())
        );
    }

    #[test]
    fn dotted_str_malformed_is_not_found_no_panic() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        assert_eq!(
            reader.resolve_dotted_str(&ctx("ghost/a"), "a..b"),
            DottedResolution::NotFound
        );
        assert_eq!(
            reader.resolve_dotted_str(&ctx("ghost/a"), ""),
            DottedResolution::NotFound
        );
    }

    #[test]
    fn dotted_str_roundtrips_with_canonical_stored_key() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.dotted_per_asker
            .entry(a.clone())
            .or_default()
            .insert("areka.window.scope(2).y".into(), "512".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_dotted_str(&ctx("ghost/a"), "areka.window.scope(2).y"),
            DottedResolution::Value("512".into())
        );
    }

    #[test]
    fn talk_snapshot_contains_only_real_values() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let snap = reader.talk_snapshot(&ctx("ghost/a"));
        assert_eq!(snap.get("selfname").map(String::as_str), Some("Alice"));
        assert!(!snap.contains_key("username"));
        assert!(!snap.contains_key("month"));
    }

    #[test]
    fn talk_snapshot_is_frozen_after_publish() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let shared = SharedMirror::new(Arc::new(img));
        let reader = SylphyaReader::new(shared.clone());
        let snap = reader.talk_snapshot(&ctx("ghost/a"));
        let mut next = shared.load().successor();
        next.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "CHANGED".into());
        shared.publish(Arc::new(next));
        assert_eq!(snap.get("selfname").map(String::as_str), Some("Alice"));
        let snap2 = reader.talk_snapshot(&ctx("ghost/a"));
        assert_eq!(snap2.get("selfname").map(String::as_str), Some("CHANGED"));
    }
}

/// 鏡像・読み口の決定論単体テスト。純関数的な解決なので I/O・環境・時計に触れない。
/// 点付き per-asker 非混同（フラット側は `flat_other_asker_does_not_leak`）を補う。
#[cfg(test)]
mod mirror_reader_criteria_cage {
    use crate::asker::{AskerContext, AskerId};
    use crate::key::parse_dotted;
    use crate::mirror::{MirrorImage, SharedMirror};
    use crate::reader::SylphyaReader;
    use crate::value::{DottedResolution, FlatResolution};
    use crate::vocab::DegradePolicy;
    use std::sync::Arc;

    fn ctx(id: &str) -> AskerContext {
        AskerContext {
            asker: AskerId::new(id),
        }
    }

    /// per-asker 区画が **フラット・点付きとも** asker 間で混ざらないこと（両方向）。
    ///
    /// 2 asker（A=ghost/a・B=ghost/b）へ各々固有のフラット値（selfname／keroname）と
    /// 固有の点付き値（同名 key の別値＋各自だけが持つ key）を据える。検証:
    /// - フラット両方向: A の selfname=Alice・B の selfname=Bob が独立（相互に混ざらない）。
    /// - フラット非漏洩: A のみが持つ keroname は B からは見えず政策縮退へ落ちる。
    /// - 点付き両方向: 同一 key `areka.window.scope(0).x` が A→10・B→99 と asker ごとに独立
    ///   （点付き per-asker 非混同の核）。
    /// - 点付き非漏洩: A 固有 key は B から `NotFound`、B 固有 key は A から `NotFound`
    ///   （per-asker 区画を取り違え／併合すれば別 asker の値が漏れて破れる）。
    #[test]
    fn per_asker_non_mixing_flat_and_dotted() {
        let a = AskerId::new("ghost/a");
        let b = AskerId::new("ghost/b");
        let mut img = MirrorImage::empty();
        {
            let pa = img.flat_per_asker.entry(a.clone()).or_default();
            pa.insert("selfname".into(), "Alice".into());
            pa.insert("keroname".into(), "KeroA".into());
        }
        {
            let da = img.dotted_per_asker.entry(a.clone()).or_default();
            da.insert("areka.window.scope(0).x".into(), "10".into());
            da.insert("areka.only_a".into(), "A1".into());
        }
        {
            let pb = img.flat_per_asker.entry(b.clone()).or_default();
            pb.insert("selfname".into(), "Bob".into());
        }
        {
            let db = img.dotted_per_asker.entry(b.clone()).or_default();
            db.insert("areka.window.scope(0).x".into(), "99".into());
            db.insert("areka.only_b".into(), "B2".into());
        }
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));

        // --- フラット ---
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Value("Alice".into())
        );
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/b"), "selfname"),
            FlatResolution::Value("Bob".into())
        );
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/b"), "keroname"),
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
        );

        // --- 点付き ---
        let shared_key = parse_dotted("areka.window.scope(0).x").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &shared_key),
            DottedResolution::Value("10".into())
        );
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/b"), &shared_key),
            DottedResolution::Value("99".into())
        );
        let only_a = parse_dotted("areka.only_a").unwrap();
        let only_b = parse_dotted("areka.only_b").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/b"), &only_a),
            DottedResolution::NotFound
        );
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &only_b),
            DottedResolution::NotFound
        );
    }
}
