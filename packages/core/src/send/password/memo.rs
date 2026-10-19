//! Remembers that the keychain is locked, so a dismissed unlock prompt doesn't come straight
//! back: reads answer "locked" from memory until something the user did asks again.

use std::sync::Mutex;

use super::store::StoreError;

#[derive(Default)]
pub struct LockMemo(Mutex<Option<StoreError>>);

impl LockMemo {
    pub const fn new() -> Self {
        LockMemo(Mutex::new(None))
    }

    /// `read`'s answer, or the remembered lock without calling it.
    pub fn read<T>(&self, read: impl FnOnce() -> Result<T, StoreError>) -> Result<T, StoreError> {
        match self.0.lock().ok().and_then(|memo| memo.clone()) {
            Some(locked) => Err(locked),
            None => self.write(read),
        }
    }

    /// Always runs `op` (writes come from the user saving); its answer is what is remembered.
    pub fn write<T>(&self, op: impl FnOnce() -> Result<T, StoreError>) -> Result<T, StoreError> {
        let answer = op();
        let locked = match &answer {
            Err(e @ StoreError::Locked(_)) => Some(e.clone()),
            _ => None,
        };
        if let Ok(mut memo) = self.0.lock() {
            *memo = locked;
        }
        answer
    }

    /// The next read asks the keychain again, and may bring up its unlock prompt.
    pub fn forget(&self) {
        if let Ok(mut memo) = self.0.lock() {
            *memo = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_locked_answer_is_remembered_until_forgotten() {
        let memo = LockMemo::new();
        let asked = Cell::new(0);
        let locked = || {
            asked.set(asked.get() + 1);
            Err::<(), _>(StoreError::Locked("dismissed".into()))
        };
        for _ in 0..3 {
            assert_eq!(
                memo.read(locked),
                Err(StoreError::Locked("dismissed".into()))
            );
        }
        assert_eq!(asked.get(), 1);
        memo.forget();
        assert!(memo.read(locked).is_err());
        assert_eq!(asked.get(), 2);
        assert_eq!(memo.write(|| Ok(())), Ok(()));
        assert_eq!(memo.read(|| Ok(7)), Ok(7));
    }

    #[test]
    fn other_failures_are_asked_again() {
        let memo = LockMemo::new();
        let asked = Cell::new(0);
        for _ in 0..2 {
            let _ = memo.read(|| {
                asked.set(asked.get() + 1);
                Err::<(), _>(StoreError::Unavailable("no keychain".into()))
            });
        }
        assert_eq!(asked.get(), 2);
    }
}
