use async_std::stream::Stream;
use async_std::task::{Context, Poll};
use std::collections::VecDeque;
use std::pin::Pin;
use tendermint_rpc::query::Query;

pub struct TxSearchRequest {
    pub query: Query,
    pub page: u32,
}

pub trait TxHelper {
    fn transactions_for_heights(start: u64, stop: u64) -> Query;
}

impl TxHelper for Query {
    fn transactions_for_heights(start: u64, stop: u64) -> Self {
        let key = "tx.height";
        Query::gte(key, start).and_lt(key, stop)
    }
}

impl TxSearchRequest {
    pub fn new() -> Self {
        let key = "tx.height";
        let query = Query::gte(key, 1).and_lt(key, 2);
        TxSearchRequest::from_query_and_page(query, 1)
    }

    pub fn from_query_and_page(query: Query, page: u32) -> Self {
        TxSearchRequest { query, page }
    }
}

impl Default for TxSearchRequest {
    fn default() -> Self {
        Self::new()
    }
}

pub struct QueryStream {
    queue: VecDeque<Box<TxSearchRequest>>,
}

impl QueryStream {
    pub fn new() -> Self {
        let queue = VecDeque::new();
        QueryStream { queue }
    }

    pub fn enqueue(&mut self, request: Box<TxSearchRequest>) {
        self.queue.push_back(request)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for QueryStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Stream for QueryStream {
    type Item = Box<TxSearchRequest>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let item = self.queue.pop_front();
        Poll::Ready(item)
    }
}

#[tokio::test]
async fn test_query_stream() {
    use async_std::stream::StreamExt;
    let mut qs = QueryStream::default();
    let query = Query::transactions_for_heights(1, 2);
    let tx1 = TxSearchRequest::from_query_and_page(query.clone(), 1);
    let tx2 = TxSearchRequest::from_query_and_page(query.clone(), 2);
    let tx3 = TxSearchRequest::from_query_and_page(query, 3);
    let mut tx3_to_requeue: Option<Box<TxSearchRequest>> = None;
    qs.enqueue(Box::from(tx1));
    qs.enqueue(Box::from(tx2));
    qs.enqueue(Box::from(tx3));
    let mut re_queued = false;
    let mut transaction_index = 0;
    while let Some(tx_request) = qs.next().await {
        let mut expected_page = transaction_index + 1;
        if re_queued && transaction_index == 3 {
            expected_page = 2;
        }
        transaction_index += 1;
        assert_eq!(expected_page, tx_request.page);
        if tx_request.page == 3 && tx3_to_requeue.is_none() {
            tx3_to_requeue = Some(tx_request);
        } else if tx_request.page == 2 && !re_queued {
            // Simulate re-queuing a failed request
            qs.enqueue(tx_request);
            re_queued = true;
        }
    }

    qs.enqueue(tx3_to_requeue.unwrap());

    while let Some(tx_request) = qs.next().await {
        assert_eq!(3, tx_request.page, "Expected transaction not re-queued");
    }
}
