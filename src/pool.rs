use {
    crossbeam::queue::{ArrayQueue, SegQueue},
    std::{
        fmt,
        future::Future,
        mem::ManuallyDrop,
        ops,
        pin::Pin,
        sync::atomic::{AtomicUsize, Ordering},
        task::{self, Poll, Waker},
    },
};

trait Pool<T> {
    fn put(&self, value: T);
}

pub struct SyncPool<T: SyncPoolContent, const N: usize> {
    ctx: T::Ctx,
    pool: ArrayQueue<T>,
}

impl<T: SyncPoolContent, const N: usize> Pool<T> for SyncPool<T, N> {
    fn put(&self, mut value: T) {
        // if there are too many items, they will be dropped
        value.reset(&self.ctx);
        let _ = self.pool.push(value);
    }
}

impl<T: SyncPoolContent, const N: usize> SyncPool<T, N> {
    pub fn new(ctx: T::Ctx) -> Self {
        Self {
            ctx,
            pool: ArrayQueue::new(N),
        }
    }

    pub fn get(&self) -> PoolItem<'_, T> {
        let item = self.pool.pop().unwrap_or_else(|| T::new(&self.ctx));
        PoolItem {
            item: ManuallyDrop::new(item),
            pool: self,
        }
    }
}

pub trait SyncPoolContent: Sized {
    type Ctx: fmt::Debug;
    fn new(ctx: &Self::Ctx) -> Self;
    fn reset(&mut self, ctx: &Self::Ctx);
}

#[derive(Debug)]
pub struct VecPoolCtx {
    pub size_cap: usize,
    pub init_size: usize,
}

impl<T> SyncPoolContent for Vec<T> {
    type Ctx = VecPoolCtx;

    fn new(ctx: &Self::Ctx) -> Self {
        Self::with_capacity(ctx.init_size)
    }

    fn reset(&mut self, ctx: &Self::Ctx) {
        unsafe {
            self.set_len(0);
        }
        self.shrink_to(ctx.size_cap);
    }
}

pub struct AsyncPool<T: AsyncPoolContent, const N: usize> {
    ctx: T::Ctx,
    items: AtomicUsize,
    pool: ArrayQueue<T>,
    wakers: SegQueue<Waker>,
}

impl<T: AsyncPoolContent, const N: usize> Pool<T> for AsyncPool<T, N> {
    fn put(&self, value: T) {
        // As we won't create too many items and this trait isn't public, the pool won't be full
        let _ = self.pool.push(value);
    }
}

impl<T: AsyncPoolContent, const N: usize> AsyncPool<T, N> {
    pub fn new(ctx: T::Ctx) -> Self {
        Self {
            ctx,
            items: AtomicUsize::new(0),
            pool: ArrayQueue::new(N),
            wakers: SegQueue::new(),
        }
    }

    pub fn get(&self) -> PoolTake<'_, T, N> {
        PoolTake {
            pool: self,
            add: None,
            waker_added: false,
        }
    }
}

pub struct PoolTake<'a, T: AsyncPoolContent, const N: usize> {
    pool: &'a AsyncPool<T, N>,
    add: Option<Pin<Box<dyn Future<Output = T> + 'a>>>,
    waker_added: bool,
}

impl<'a, T: AsyncPoolContent, const N: usize> PoolTake<'a, T, N> {
    fn poll_add(&mut self, cx: &mut task::Context<'_>) -> Option<Poll<<Self as Future>::Output>> {
        self.add.as_mut().map(|add| match add.as_mut().poll(cx) {
            Poll::Ready(item) => Poll::Ready(PoolItem {
                item: ManuallyDrop::new(item),
                pool: self.pool,
            }),
            Poll::Pending => Poll::Pending,
        })
    }
}

impl<'a, T: AsyncPoolContent, const N: usize> Future for PoolTake<'a, T, N> {
    type Output = PoolItem<'a, T>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let pool = this.pool;
        if let Some(res) = this.poll_add(cx) {
            return res;
        }
        match pool.pool.pop() {
            Some(item) => Poll::Ready(PoolItem {
                item: ManuallyDrop::new(item),
                pool,
            }),
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

pub trait AsyncPoolContent: Sized {
    type Ctx: fmt::Debug;
    fn new<'a>(ctx: &'a Self::Ctx) -> Pin<Box<dyn Future<Output = Self> + 'a>>;
}

pub struct PoolItem<'a, T> {
    item: ManuallyDrop<T>,
    pool: &'a dyn Pool<T>,
}

impl<T> ops::Deref for PoolItem<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<T> ops::DerefMut for PoolItem<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<T> AsRef<T> for PoolItem<'_, T> {
    fn as_ref(&self) -> &T {
        &self.item
    }
}

impl AsRef<[u8]> for PoolItem<'_, Vec<u8>> {
    fn as_ref(&self) -> &[u8] {
        &self.item
    }
}

impl<T> AsMut<T> for PoolItem<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.item
    }
}

impl AsMut<[u8]> for PoolItem<'_, Vec<u8>> {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.item
    }
}

impl<T> ops::Drop for PoolItem<'_, T> {
    fn drop(&mut self) {
        let item = unsafe { <ManuallyDrop<T>>::take(&mut self.item) };
        self.pool.put(item);
    }
}

impl<T: fmt::Debug> fmt::Debug for PoolItem<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.item.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for PoolItem<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.item.fmt(f)
    }
}
