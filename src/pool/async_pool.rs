use {
    super::{AsyncPoolContent, AsyncPoolGetFuture, AsyncPoolTrait, PoolItem, PoolPut},
    crossbeam::queue::{ArrayQueue, SegQueue},
    std::{
        future::Future,
        mem::ManuallyDrop,
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        task::{self, Poll, Waker},
    },
};

pub struct AsyncPool<T: AsyncPoolContent<C>, C, const N: usize> {
    ctx: T::Ctx,
    items: AtomicUsize,
    pool: ArrayQueue<T>,
    wakers: SegQueue<Waker>,
}

impl<T: AsyncPoolContent<C>, C, const N: usize> PoolPut<T> for AsyncPool<T, C, N> {
    fn put(&self, value: T) {
        // As we won't create too many items, the pool won't be full
        let _ = self.pool.push(value);
    }
}

impl<T: AsyncPoolContent<C>, C, const N: usize> AsyncPool<T, C, N> {
    pub fn new(ctx: T::Ctx) -> Self {
        Self {
            ctx,
            items: AtomicUsize::new(0),
            pool: ArrayQueue::new(N),
            wakers: SegQueue::new(),
        }
    }
}

impl<T: AsyncPoolContent<C>, C, const N: usize> AsyncPoolTrait<T> for AsyncPool<T, C, N> {
    fn get(&self) -> Pin<Box<AsyncPoolGetFuture<'_, T>>> {
        Box::pin(PoolTake {
            pool: self,
            add: None,
            waker_added: false,
        })
    }
}

#[allow(clippy::type_complexity)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct PoolTake<'a, T: AsyncPoolContent<C>, C, const N: usize> {
    pool: &'a AsyncPool<T, C, N>,
    add: Option<Pin<Box<dyn Future<Output = Result<T, T::Error>> + 'a>>>,
    waker_added: bool,
}

impl<'a, T: AsyncPoolContent<C>, C, const N: usize> PoolTake<'a, T, C, N> {
    fn poll_add(&mut self, cx: &mut task::Context<'_>) -> Option<Poll<<Self as Future>::Output>> {
        self.add.as_mut().map(|add| {
            add.as_mut().poll(cx).map(|res| {
                res.map(|item| PoolItem {
                    item: ManuallyDrop::new(item),
                    pool: self.pool,
                })
            })
        })
    }
}

impl<'a, T: AsyncPoolContent<C>, C, const N: usize> Future for PoolTake<'a, T, C, N> {
    type Output = Result<PoolItem<'a, T>, T::Error>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let pool = this.pool;
        if let Some(res) = this.poll_add(cx) {
            return res;
        }
        match pool.pool.pop() {
            Some(item) => Poll::Ready(Ok(PoolItem {
                item: ManuallyDrop::new(item),
                pool,
            })),
            None => {
                let item_count = pool.items.load(Ordering::Relaxed);
                if item_count < N {
                    if pool
                        .items
                        .compare_exchange(
                            item_count,
                            item_count + 1,
                            Ordering::Relaxed,
                            Ordering::Relaxed,
                        )
                        .is_ok()
                    {
                        this.add = Some(Box::pin(T::new(&pool.ctx)));
                        if let Some(res) = this.poll_add(cx) {
                            return res;
                        }
                    } else {
                        cx.waker().wake_by_ref();
                    }
                } else if !this.waker_added {
                    this.waker_added = true;
                    pool.wakers.push(cx.waker().clone());
                    // wake for the rare case that an item was added after we tried to get one but before we registered the waker
                    cx.waker().wake_by_ref();
                }
                Poll::Pending
            }
        }
    }
}
