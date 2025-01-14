use std::any::{type_name, Any};
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Debug;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::thread::{self, ThreadId};

trait AsAny {
    fn as_any(self) -> Box<dyn Any>;
}

impl<T: Any> AsAny for T {
    fn as_any(self) -> Box<dyn Any> {
        Box::new(self)
    }
}

trait AsType {
    fn as_type<T: Any>(self, expect: &dyn Expect) -> Result<T, &'static str>;
}

impl AsType for Box<dyn Any> {
    fn as_type<T: Any>(self, expect: &dyn Expect) -> Result<T, &'static str> {
        self.downcast::<T>()
            .map_err(|_| expect.type_name())
            .map(|value| *value)
    }
}

trait Expect: Send {
    fn mock(&self, when: Box<dyn Any>) -> Result<Box<dyn Any>, &'static str>;
    fn type_name(&self) -> &'static str;
}

impl Debug for dyn Expect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.type_name())
    }
}

impl dyn Expect {
    fn on_mock<T: Any, U: Any>(&self, when: T) -> Result<U, &'static str> {
        let then = self.mock(when.as_any())?;
        Ok(then.as_type(self)?)
    }
}

impl<T: Any, U: Any> Expect for fn(T) -> U {
    fn mock(&self, when: Box<dyn Any>) -> Result<Box<dyn Any>, &'static str> {
        let then = self(when.as_type(self)?);
        Ok(then.as_any())
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<fn(T) -> U>()
    }
}

#[derive(Debug, Default)]
pub struct ExpectStore(Arc<Mutex<Vec<Box<dyn Expect>>>>);

impl Clone for ExpectStore {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl ExpectStore {
    fn add_expect<T: Any, U: Any>(&self, expect: fn(T) -> U) {
        self.0.lock().unwrap().insert(0, Box::new(expect));
    }

    fn next_expect(&self) -> Option<Box<dyn Expect>> {
        self.0.lock().unwrap().pop()
    }

    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }

    fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_empty()
    }
}

impl Drop for ExpectStore {
    fn drop(&mut self) {
        if !self.is_empty() {
            panic!("pending expects: {:?}", self.0.lock().unwrap())
        }
    }
}

fn type_error<T: Any + Debug, U: Any>(expect: &str) -> String {
    let received = type_name::<fn(T) -> U>();
    format!("expect type mismatch: expecting {expect:?}, received {received:?}")
}

pub trait Mockdown
where
    Self: Sized,
{
    fn store(&self) -> &ExpectStore;

    fn clear(self) -> Self {
        self.store().clear();
        self
    }

    fn expect<T: Any, U: Any>(self, expect: fn(T) -> U) -> Self {
        self.store().add_expect(expect);
        self
    }

    fn on_mock<T: Any + Debug, U: Any>(&self, args: T) -> Result<U, String> {
        let expect = self
            .store()
            .next_expect()
            .ok_or_else(|| type_error::<T, U>("nothing"))?;

        let result = expect
            .on_mock(args)
            .map_err(|expect| type_error::<T, U>(expect))?;

        Ok(result)
    }
}

pub struct StaticMock<M: Mockdown>(LazyLock<Arc<Mutex<HashMap<ThreadId, M>>>>);

impl<M: Mockdown + Clone + Default> StaticMock<M> {
    pub const fn new() -> StaticMock<M> {
        Self(LazyLock::new(|| Default::default()))
    }

    fn with(&self) -> (MutexGuard<'_, HashMap<ThreadId, M>>, ThreadId) {
        (self.0.lock().unwrap(), thread::current().id())
    }

    pub fn static_mock(&self) -> M {
        let (mut map, id) = self.with();
        if !map.contains_key(&id) {
            map.insert(id, M::default());
        }
        map.get(&id).unwrap().clone().clear()
    }

    pub fn on_mock<T: Any + Debug, U: Any>(&self, args: T) -> Result<U, String> {
        let (map, id) = self.with();
        map.get(&id).unwrap().on_mock(args)
    }
}
