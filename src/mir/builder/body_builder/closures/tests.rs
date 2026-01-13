#![cfg(test)]

use super::analysis::{CaptureCache, LambdaLoweringBody, capture_cache_key};
use crate::syntax::expr::ExprNode;
use std::collections::HashSet;

#[test]
fn capture_cache_records_hits_and_misses() {
    let mut cache = CaptureCache::default();
    let body = LambdaLoweringBody::Expression(Box::new(ExprNode::Identifier("x".into())));
    let key = capture_cache_key(&body);

    assert!(cache.get(&key).is_none());
    cache.insert(key.clone(), Vec::new());
    assert!(cache.get(&key).is_some());

    let metrics = cache.metrics();
    assert_eq!(metrics.misses, 1);
    assert_eq!(metrics.hits, 1);
}

#[test]
fn capture_cache_keys_distinguish_lambda_variants() {
    let expr_body = LambdaLoweringBody::Expression(Box::new(ExprNode::Identifier("x".into())));
    let other_body = LambdaLoweringBody::Expression(Box::new(ExprNode::Identifier("y".into())));

    let keys: HashSet<String> = [
        capture_cache_key(&expr_body),
        capture_cache_key(&other_body),
    ]
    .into_iter()
    .collect();

    assert_eq!(keys.len(), 2);
}
