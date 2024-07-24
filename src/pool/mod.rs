use std::{fmt, future::Future, pin::Pin};

mod async_pool;
mod pool_item;
mod sync_pool;

pub use {async_pool::*, pool_item::PoolItem, sync_pool::*};

trait PoolPut<T> {
    fn put(&self, value: T);
}

type AsyncPoolGetFuture<'a, T> =
    dyn Future<Output = Result<PoolItem<'a, T>, <T as AsyncPoolContentError>::Error>> + 'a;

pub trait AsyncPoolTrait<T: AsyncPoolContentError> {
    fn get(&self) -> Pin<Box<AsyncPoolGetFuture<'_, T>>>;
}

pub trait AsyncPoolContent<T>: AsyncPoolContentError + Sized + 'static {
    type Ctx: fmt::Debug;
    fn new<'a>(ctx: &'a Self::Ctx)
        -> Pin<Box<dyn Future<Output = Result<Self, Self::Error>> + 'a>>;
}

pub trait AsyncPoolContentError: 'static {
    type Error: fmt::Debug;
}
