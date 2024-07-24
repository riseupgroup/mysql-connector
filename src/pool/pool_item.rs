use {
    super::PoolPut,
    std::{fmt, mem::ManuallyDrop, ops},
};

pub struct PoolItem<'a, T> {
    pub(super) item: ManuallyDrop<T>,
    pub(super) pool: &'a dyn PoolPut<T>,
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
