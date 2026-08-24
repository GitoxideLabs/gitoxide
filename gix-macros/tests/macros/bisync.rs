mod synchronous {
    use gix_macros::{discard, keep, sync};

    macro_rules! sync_block {
        ($body:block) => {
            #[sync]
            pub async fn macro_generated_block() -> usize {
                (async move $body).await
            }
        };
    }

    sync_block!({ 42 });

    #[sync]
    pub async fn answer() -> usize {
        let value = async move { 41 };
        value.await + 1
    }

    #[sync]
    pub async fn owned_closure(value: String) -> impl FnOnce() -> usize {
        async move || value.len()
    }

    #[keep]
    pub fn mode() -> &'static str {
        "sync"
    }

    #[discard]
    fn mode() -> &'static str {
        "async"
    }
}

mod asynchronous {
    use gix_macros::{discard, keep};

    #[keep]
    pub async fn answer() -> usize {
        std::future::ready(42).await
    }

    #[discard]
    fn mode() -> &'static str {
        "sync"
    }

    #[keep]
    pub fn mode() -> &'static str {
        "async"
    }
}

#[test]
fn attributes_select_and_transform_async_code() {
    fn assert_future(_: impl std::future::Future<Output = usize>) {}

    assert_eq!(synchronous::answer(), 42, "async syntax is removed");
    assert_eq!(
        synchronous::macro_generated_block(),
        42,
        "macro-generated async blocks are transformed"
    );
    assert_eq!(
        synchronous::owned_closure("owned".into())(),
        5,
        "async closures retain move semantics"
    );
    assert_eq!(synchronous::mode(), "sync", "sync-only items are selected");
    assert_future(asynchronous::answer());
    assert_eq!(asynchronous::mode(), "async", "async-only items are selected");
}
