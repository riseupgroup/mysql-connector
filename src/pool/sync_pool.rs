use {
    super::{PoolItem, PoolPut},
    crossbeam::queue::ArrayQueue,
    std::{fmt, mem::ManuallyDrop},
};

pub struct SyncPool<T: SyncPoolContent, const N: usize> {
    ctx: T::Ctx,
    pool: ArrayQueue<T>,
}

impl<T: SyncPoolContent, const N: usize> PoolPut<T> for SyncPool<T, N> {
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
