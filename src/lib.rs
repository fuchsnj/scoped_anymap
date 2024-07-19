use ahash::AHashMap;
use core::marker::PhantomData;
use core::mem;
use std::any::TypeId;

#[derive(Default)]
pub struct ScopedAnyMap<'a> {
    map: AHashMap<TypeId, Box<dyn EmptyTrait + 'a>>,
}

impl<'a> ScopedAnyMap<'a> {
    pub fn new() -> Self {
        Self {
            map: AHashMap::new(),
        }
    }

    pub fn insert<T: 'a>(&mut self, value: T) {
        self.map.insert(non_static_type_id::<T>(), Box::new(value));
    }

    pub fn get_ref<T>(&self) -> Option<&T> {
        let type_id = non_static_type_id::<T>();
        self.map.get(&type_id).map(|x| {
            let val = x.as_ref();

            // Safety: Same as `get_mut`
            unsafe { &*(val as *const dyn EmptyTrait as *const T) }
        })
    }

    pub fn get_mut<T>(&mut self) -> Option<&mut T> {
        let type_id = non_static_type_id::<T>();
        self.map.get_mut(&type_id).map(|x| {
            let val: &mut dyn EmptyTrait = x.as_mut();

            // Safety: This is safe because the TypeId was already checked to match.
            // The lifetime of the type when inserted must be at least that of self,
            // and it is being reduced to the same as self here.
            unsafe { &mut *(val as *mut dyn EmptyTrait as *mut T) }
        })
    }
}

// This is just a trait that _anything_ implements, so anything can be stored as a trait in a box
trait EmptyTrait {}
impl<T: ?Sized> EmptyTrait for T {}

fn non_static_type_id<T: ?Sized>() -> TypeId {
    // Powered by dtonlay magic:
    // https://github.com/rust-lang/rust/issues/41875#issuecomment-317292888

    trait NonStaticAny {
        fn get_type_id(&self) -> TypeId
        where
            Self: 'static;
    }

    impl<T: ?Sized> NonStaticAny for PhantomData<T> {
        fn get_type_id(&self) -> TypeId
        where
            Self: 'static,
        {
            TypeId::of::<T>()
        }
    }

    let phantom_data = PhantomData::<T>;
    NonStaticAny::get_type_id(unsafe {
        mem::transmute::<&dyn NonStaticAny, &(dyn NonStaticAny + 'static)>(&phantom_data)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        {
            let mut value: i32 = 5;

            let mut map = ScopedAnyMap::new();

            // This is storing a _reference_ to the value
            map.insert(&mut value);

            **map.get_mut::<&mut i32>().unwrap() += 1;
            assert_eq!(**map.get_ref::<&mut i32>().unwrap(), 6_i32);

            drop(map);

            assert_eq!(value, 6);
        }
    }
}
